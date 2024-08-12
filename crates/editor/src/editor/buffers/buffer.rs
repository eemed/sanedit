mod change;
mod diagnostic;
mod filetype;
mod options;
mod snapshots;
mod sorted;

use std::{
    borrow::Cow,
    fs,
    io::{self, Write},
    ops::{Range, RangeBounds},
    path::{Path, PathBuf},
};

use anyhow::Result;
use anyhow::{bail, ensure};
use sanedit_buffer::{PieceTree, PieceTreeSlice, ReadOnlyPieceTree};
use sanedit_lsp::lsp_types::Diagnostic;
use sanedit_utils::key_type;
use thiserror::Error;

use crate::common::{dirs::tmp_file, file::FileDescription};

use self::snapshots::Snapshots;
pub(crate) use change::Change;
pub(crate) use filetype::Filetype;
pub(crate) use options::Options;
pub(crate) use snapshots::{SnapshotData, SnapshotId};
pub(crate) use sorted::SortedRanges;

key_type!(pub(crate) BufferId);

/// A range in the buffer
pub(crate) type BufferRange = Range<usize>;

#[derive(Debug)]
pub(crate) struct Buffer {
    pub(crate) id: BufferId,
    pub(crate) filetype: Option<Filetype>,
    pub(crate) options: Options,
    pub(crate) read_only: bool,
    pub(crate) diagnostics: Vec<Diagnostic>,

    pt: PieceTree,
    /// Snapshots of the piecetree, used for undo
    snapshots: Snapshots,
    last_saved_snapshot: SnapshotId,

    is_modified: bool,
    last_change: Option<Change>,

    /// Path used for saving the file.
    path: Option<PathBuf>,
}

impl Buffer {
    pub fn new() -> Buffer {
        let pt = PieceTree::new();
        let snapshot = pt.read_only_copy();
        Buffer {
            id: BufferId::default(),
            filetype: None,
            read_only: false,
            diagnostics: vec![],
            pt,
            is_modified: false,
            snapshots: Snapshots::new(snapshot),
            options: Options::default(),
            path: None,
            last_change: None,
            last_saved_snapshot: 0,
        }
    }

    pub fn from_file(file: FileDescription, options: Options) -> Result<Buffer> {
        if file.is_big() {
            Self::file_backed(file, options)
        } else {
            Self::in_memory(file, options)
        }
    }

    fn file_backed(file: FileDescription, options: Options) -> Result<Buffer> {
        log::debug!("creating file backed buffer");
        let path = file.path();
        let pt = PieceTree::from_path(path)?;
        let snapshot = pt.read_only_copy();
        Ok(Buffer {
            id: BufferId::default(),
            read_only: file.read_only(),
            diagnostics: vec![],
            pt,
            filetype: file.filetype().cloned(),
            is_modified: false,
            snapshots: Snapshots::new(snapshot),
            options,
            path: Some(path.into()),
            last_change: None,
            last_saved_snapshot: 0,
        })
    }

    fn in_memory(file: FileDescription, options: Options) -> Result<Buffer> {
        log::debug!("creating in memory buffer");
        let path = file.path();
        let ffile = fs::File::open(path)?;
        let mut buf = Self::from_reader(ffile)?;
        buf.filetype = file.filetype().cloned();
        buf.path = Some(path.into());
        buf.read_only = file.read_only();
        buf.options = options;
        Ok(buf)
    }

    fn from_reader<R: io::Read>(reader: R) -> Result<Buffer> {
        let pt = PieceTree::from_reader(reader)?;
        let snapshot = pt.read_only_copy();
        Ok(Buffer {
            id: BufferId::default(),
            diagnostics: vec![],
            pt,
            filetype: None,
            read_only: false,
            is_modified: false,
            snapshots: Snapshots::new(snapshot),
            options: Options::default(),
            path: None,
            last_change: None,
            last_saved_snapshot: 0,
        })
    }

    pub fn name(&self) -> Cow<'_, str> {
        self.path
            .as_ref()
            .map(|p| p.to_string_lossy())
            .unwrap_or(Cow::from("scratch"))
    }

    pub fn len(&self) -> usize {
        self.pt.len()
    }

    /// Get mutable access to extra data for a snapshot
    pub fn snapshot_data_mut(&mut self, id: SnapshotId) -> Option<&mut SnapshotData> {
        self.snapshots.data_mut(id)
    }

    /// Get access to extra data for a snapshot
    pub fn snapshot_data(&self, id: SnapshotId) -> Option<&SnapshotData> {
        self.snapshots.data(id)
    }

    /// Get the last change done to buffer
    pub fn last_change(&self) -> Option<&Change> {
        self.last_change.as_ref()
    }

    /// Creates undo point if it is needed
    fn create_undo_point(&mut self, mut change: Change) -> Change {
        let last = self.last_change.as_ref();
        if self.is_modified && change.needs_undo_point(last) {
            let snap = self.pt.read_only_copy();
            let id = self.snapshots.insert(snap);
            change.created_snapshot = Some(id);
        }

        change
    }

    pub fn undo(&mut self) -> Result<&Change> {
        let change = Change::undo();
        let mut change = self.create_undo_point(change);

        let node = self.snapshots.undo().ok_or(BufferError::NoMoreUndoPoints)?;
        change.restored_snapshot = Some(node.id);
        self.is_modified = node.id != self.last_saved_snapshot;
        self.last_change = change.into();
        self.pt.restore(node.snapshot);

        let change = self.last_change.as_ref().unwrap();
        Ok(change)
    }

    pub fn redo(&mut self) -> Result<&Change> {
        let change = Change::redo();
        let mut change = self.create_undo_point(change);

        let node = self.snapshots.redo().ok_or(BufferError::NoMoreRedoPoints)?;
        change.restored_snapshot = Some(node.id);
        self.is_modified = node.id != self.last_saved_snapshot;
        self.last_change = change.into();
        self.pt.restore(node.snapshot);

        let change = self.last_change.as_ref().unwrap();
        Ok(change)
    }

    pub fn remove(&mut self, range: Range<usize>) -> Result<&Change> {
        let ranges = vec![range.clone()].into();
        self.remove_multi(&ranges)
    }

    pub fn append<B: AsRef<[u8]>>(&mut self, bytes: B) -> Result<&Change> {
        self.insert_multi(&[self.pt.len()], bytes)
    }

    pub fn insert<B: AsRef<[u8]>>(&mut self, pos: usize, bytes: B) -> Result<&Change> {
        self.insert_multi(&[pos], bytes)
    }

    pub fn insert_multi<B: AsRef<[u8]>>(&mut self, pos: &[usize], bytes: B) -> Result<&Change> {
        ensure!(!self.read_only, BufferError::ReadOnly);
        let bytes = bytes.as_ref();
        let ranges: Vec<BufferRange> = pos.iter().map(|pos| *pos..pos + bytes.len()).collect();
        let change = Change::insert(&ranges.into(), bytes);
        let change = self.create_undo_point(change);

        self.pt.insert_multi(pos, bytes);
        self.is_modified = true;
        self.last_change = change.into();
        Ok(self.last_change.as_ref().unwrap())
    }

    pub fn remove_multi(&mut self, ranges: &SortedRanges) -> Result<&Change> {
        ensure!(!self.read_only, BufferError::ReadOnly);

        let change = Change::remove(ranges);
        let change = self.create_undo_point(change);

        for range in ranges.iter().rev() {
            self.pt.remove(range.clone());
        }

        self.is_modified = true;
        self.last_change = change.into();
        Ok(self.last_change.as_ref().unwrap())
    }

    pub fn set_path<P: AsRef<Path>>(&mut self, path: P) {
        self.path = Some(path.as_ref().to_owned());
        self.is_modified = true;
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_ref().map(|p| p.as_path())
    }

    pub fn slice<R: RangeBounds<usize>>(&self, range: R) -> PieceTreeSlice {
        self.pt.slice(range)
    }

    pub fn read_only_copy(&self) -> ReadOnlyPieceTree {
        self.pt.read_only_copy()
    }

    pub fn is_modified(&self) -> bool {
        self.is_modified
    }

    /// Save the buffer by copying it to a temporary file and renaming it to the
    /// buffers path
    pub fn save_rename(&mut self) -> Result<Saved> {
        ensure!(!self.read_only, BufferError::ReadOnly);
        // No save path before unmodified to execute save as even if buffer is
        // unmodified without path
        let path = self.path().ok_or(BufferError::NoSavePath)?;
        ensure!(self.is_modified, BufferError::Unmodified);

        let cur = self.read_only_copy();
        let copy = Self::save_copy(&cur)?;

        // Rename backing file if it is the same as our path
        if self.pt.is_file_backed() {
            let backing = self.pt.backing_file().unwrap();
            if backing == path {
                let (path, _file) = tmp_file().ok_or(BufferError::CannotCreateTmpFile)?;
                self.pt.rename_backing_file(&path)?;
            }
        }

        // TODO does not work across mount points
        fs::rename(copy, path)?;

        self.is_modified = false;
        let snap = self.snapshots.insert(cur);
        self.last_saved_snapshot = snap;
        Ok(Saved { snapshot: snap })
    }

    fn save_copy(buf: &ReadOnlyPieceTree) -> Result<PathBuf> {
        let (path, mut file) = tmp_file().ok_or(BufferError::CannotCreateTmpFile)?;

        let mut chunks = buf.chunks();
        let mut chunk = chunks.get();
        while let Some((_, chk)) = chunk {
            let bytes = chk.as_ref();
            file.write(bytes)?;
            chunk = chunks.next();
        }

        file.flush()?;
        Ok(path)
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if self.pt.is_file_backed() {
            let path = self.path();
            let bfpath = self.pt.backing_file();

            match (path, bfpath) {
                (Some(p), Some(bfp)) => {
                    if p != bfp {
                        let _ = fs::remove_file(bfp);
                    }
                }
                _ => {}
            }
        }
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Buffer::new()
    }
}

#[derive(Debug, Error)]
pub(crate) enum BufferError {
    #[error("Read only buffer")]
    ReadOnly,

    #[error("Buffer is not modified")]
    Unmodified,

    #[error("No save path set")]
    NoSavePath,

    #[error("Cannot create tmp file")]
    CannotCreateTmpFile,

    #[error("No more redo points")]
    NoMoreRedoPoints,

    #[error("No more undo points")]
    NoMoreUndoPoints,
}

#[derive(Debug)]
pub(crate) struct Saved {
    pub(crate) snapshot: SnapshotId,
}
