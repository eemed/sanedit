mod change;
mod options;
mod snapshots;

use std::{
    borrow::Cow,
    fs,
    io::{self, Write},
    ops::RangeBounds,
    path::{Path, PathBuf},
};

use anyhow::ensure;
use anyhow::Result;
use sanedit_buffer::{PieceTree, PieceTreeSlice, PieceTreeView};
use sanedit_utils::key_type;
use thiserror::Error;

use crate::common::dirs::tmp_file;

use self::snapshots::Snapshots;
pub(crate) use change::ChangeResult;
pub(crate) use options::Options;
use sanedit_core::{BufferRange, Change, Changes, Diagnostic, FileDescription, Filetype};
use sanedit_core::{BufferRangeExt as _, Edit};
pub(crate) use snapshots::{SnapshotData, SnapshotId};

key_type!(pub(crate) BufferId);

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
    last_edit: Option<Edit>,

    /// Total changes made to the buffer, used as a identifier for LSP
    total_changes_made: u32,

    /// Path used for saving the file.
    path: Option<PathBuf>,
}

impl Buffer {
    pub fn new() -> Buffer {
        let pt = PieceTree::new();
        let snapshot = pt.view();
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
            last_edit: None,
            last_saved_snapshot: 0,
            total_changes_made: 0,
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
        let snapshot = pt.view();
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
            last_edit: None,
            last_saved_snapshot: 0,
            total_changes_made: 0,
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
        let snapshot = pt.view();
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
            last_edit: None,
            last_saved_snapshot: 0,
            total_changes_made: 0,
        })
    }

    pub fn name(&self) -> Cow<'_, str> {
        self.path
            .as_ref()
            .map(|p| p.to_string_lossy())
            .unwrap_or_else(|| Cow::from(format!("scratch-{}", self.id.0)))
    }

    pub fn len(&self) -> u64 {
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
    pub fn last_edit(&self) -> Option<&Edit> {
        self.last_edit.as_ref()
    }

    pub fn total_changes_made(&self) -> u32 {
        self.total_changes_made
    }

    /// Creates undo point if it is needed
    fn needs_undo_point(&mut self, change: &Changes) -> bool {
        let last = self.last_edit.as_ref();
        change.allows_undo_point_creation()
            && self.is_modified
            && change.needs_undo_point(last.as_ref().map(|edit| &edit.changes))
    }

    pub fn apply_changes(&mut self, changes: &Changes) -> Result<ChangeResult> {
        let mut result = ChangeResult::default();

        if changes.is_empty() {
            return Ok(result);
        }

        ensure!(!self.read_only, BufferError::ReadOnly);

        let rollback = self.ro_view();
        let needs_undo = self.needs_undo_point(changes);
        let rollback_snapshot_id = self.snapshots.current();

        if needs_undo {
            let snapshot = self.ro_view();
            let id = self.snapshots.insert(snapshot);
            result.created_snapshot = Some(id);
        }

        match self.apply_changes_impl(changes) {
            Ok(restored) => {
                result.restored_snapshot = restored;
                self.last_edit = Some(Edit {
                    buf: rollback,
                    changes: changes.clone(),
                });
                self.total_changes_made += 1;
            }
            Err(e) => {
                self.pt.restore(rollback);
                if needs_undo {
                    self.snapshots.remove_current_and_set(rollback_snapshot_id);
                }
                return Err(e);
            }
        }

        Ok(result)
    }

    fn apply_changes_impl(&mut self, changes: &Changes) -> Result<Option<SnapshotId>> {
        if changes.is_multi_insert() {
            let text = changes.iter().next().unwrap().text();
            let positions: Vec<u64> = changes.iter().map(Change::start).collect();
            self.insert_multi(&positions, text)?;
            return Ok(None);
        }

        if changes.is_remove() {
            let ranges = changes.before_ranges();
            self.remove_multi(&ranges)?;
            return Ok(None);
        }

        let mut result = None;
        for change in changes.iter() {
            if let Some(res) = self.apply_change(change)? {
                result = Some(res);
            }
        }

        Ok(result)
    }

    fn apply_change(&mut self, change: &Change) -> Result<Option<SnapshotId>> {
        let mut result = None;

        if change.is_undo() {
            let snapshot = self.undo()?;
            result = Some(snapshot);
        } else if change.is_redo() {
            let snapshot = self.redo()?;
            result = Some(snapshot);
        } else {
            let range = change.range();
            if range.len() != 0 {
                self.remove_multi(&[range])?;
            }

            if !change.text().is_empty() {
                self.insert_multi(&[change.start()], change.text())?;
            }
        }

        Ok(result)
    }

    fn undo(&mut self) -> Result<SnapshotId> {
        let node = self.snapshots.undo().ok_or(BufferError::NoMoreUndoPoints)?;
        let restored = node.id;
        self.is_modified = restored != self.last_saved_snapshot;
        self.pt.restore(node.snapshot);
        Ok(restored)
    }

    pub fn redo(&mut self) -> Result<SnapshotId> {
        let node = self.snapshots.redo().ok_or(BufferError::NoMoreRedoPoints)?;
        let restored = node.id;
        self.is_modified = restored != self.last_saved_snapshot;
        self.pt.restore(node.snapshot);
        Ok(restored)
    }

    fn insert_multi<B: AsRef<[u8]>>(&mut self, pos: &[u64], bytes: B) -> Result<()> {
        let bytes = bytes.as_ref();
        self.pt.insert_multi(pos, bytes);
        self.is_modified = true;
        Ok(())
    }

    fn remove_multi(&mut self, ranges: &[BufferRange]) -> Result<()> {
        fn is_sorted(ranges: &[BufferRange]) -> bool {
            let mut last = 0;
            for range in ranges.iter() {
                if range.start < last {
                    return false;
                }

                last = range.end;
            }

            true
        }

        let ranges: Cow<[BufferRange]> = if is_sorted(ranges) {
            ranges.into()
        } else {
            let mut ranges = ranges.to_vec();
            ranges.sort_by(|a, b| a.start.cmp(&b.start));
            ranges.into()
        };

        for range in ranges.iter().rev() {
            self.pt.remove(range.clone());
        }
        self.is_modified = true;
        Ok(())
    }

    pub fn set_path<P: AsRef<Path>>(&mut self, path: P) {
        self.path = Some(path.as_ref().to_owned());
        self.is_modified = true;
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_ref().map(|p| p.as_path())
    }

    pub fn slice<R: RangeBounds<u64>>(&self, range: R) -> PieceTreeSlice {
        self.pt.slice(range)
    }

    pub fn ro_view(&self) -> PieceTreeView {
        self.pt.view()
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

        let cur = self.ro_view();
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

    fn save_copy(buf: &PieceTreeView) -> Result<PathBuf> {
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
