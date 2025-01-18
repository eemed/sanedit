mod change;
mod config;
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
use sanedit_core::Edit;
use sanedit_core::{tmp_file, Changes, FileDescription, Filetype};
use sanedit_utils::key_type;
use thiserror::Error;

use self::snapshots::Snapshots;

pub(crate) use change::ChangeResult;
pub(crate) use config::BufferConfig;
pub(crate) use snapshots::{SnapshotId, SnapshotMetadata};

key_type!(pub(crate) BufferId);

#[derive(Debug)]
pub(crate) struct Buffer {
    pub(crate) id: BufferId,
    pub(crate) filetype: Option<Filetype>,
    pub(crate) config: BufferConfig,
    pub(crate) read_only: bool,

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
        Buffer {
            id: BufferId::default(),
            filetype: None,
            read_only: false,
            pt,
            is_modified: false,
            snapshots: Snapshots::new(),
            config: BufferConfig::default(),
            path: None,
            last_edit: None,
            last_saved_snapshot: 0,
            total_changes_made: 0,
        }
    }

    pub fn from_file(file: FileDescription, options: BufferConfig) -> Result<Buffer> {
        if file.is_big() {
            Self::file_backed(file, options)
        } else {
            Self::in_memory(file, options)
        }
    }

    fn file_backed(file: FileDescription, options: BufferConfig) -> Result<Buffer> {
        log::debug!("creating file backed buffer");
        let path = file.path();
        let pt = PieceTree::from_path(path)?;
        Ok(Buffer {
            id: BufferId::default(),
            read_only: file.read_only(),
            pt,
            filetype: file.filetype().cloned(),
            is_modified: false,
            snapshots: Snapshots::new(),
            config: options,
            path: Some(path.into()),
            last_edit: None,
            last_saved_snapshot: 0,
            total_changes_made: 0,
        })
    }

    fn in_memory(file: FileDescription, options: BufferConfig) -> Result<Buffer> {
        log::debug!("creating in memory buffer");
        let path = file.path();
        let ffile = fs::File::open(path)?;
        let mut buf = Self::from_reader(ffile)?;
        buf.filetype = file.filetype().cloned();
        buf.path = Some(path.into());
        buf.read_only = file.read_only();
        buf.config = options;
        Ok(buf)
    }

    fn from_reader<R: io::Read>(reader: R) -> Result<Buffer> {
        let pt = PieceTree::from_reader(reader)?;
        Ok(Buffer {
            id: BufferId::default(),
            pt,
            filetype: None,
            read_only: false,
            is_modified: false,
            snapshots: Snapshots::new(),
            config: BufferConfig::default(),
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
    pub fn snapshot_data_mut(&mut self, id: SnapshotId) -> Option<&mut SnapshotMetadata> {
        self.snapshots.data_mut(id)
    }

    /// Get access to extra data for a snapshot
    pub fn snapshot_data(&self, id: SnapshotId) -> Option<&SnapshotMetadata> {
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
            && (self.is_modified || self.total_changes_made == 0)
            && change.needs_undo_point(last.as_ref().map(|edit| &edit.changes))
    }

    pub fn apply_changes(&mut self, changes: &Changes) -> Result<ChangeResult> {
        ensure!(!self.read_only, BufferError::ReadOnly);

        let mut result = ChangeResult::default();
        let rollback = self.ro_view();
        let snapshot = self.snapshots.current();
        let needs_undo = self.needs_undo_point(changes);
        let did_undo = self
            .last_edit
            .as_ref()
            .map(|edit| edit.changes.is_undo())
            .unwrap_or(false);
        let forks = did_undo && !changes.is_undo() && !changes.is_redo();

        if forks {
            result.forked_snapshot = snapshot;
        }

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
                    self.snapshots.remove_current_and_set(snapshot);
                }
                return Err(e);
            }
        }

        Ok(result)
    }

    fn apply_changes_impl(&mut self, changes: &Changes) -> Result<Option<SnapshotId>> {
        if changes.is_undo() {
            let snapshot = self.undo()?;
            Ok(Some(snapshot))
        } else if changes.is_redo() {
            let snapshot = self.redo()?;
            Ok(Some(snapshot))
        } else {
            changes.apply(&mut self.pt);
            self.is_modified = true;
            Ok(None)
        }
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

    pub fn set_path<P: AsRef<Path>>(&mut self, path: P) {
        self.path = Some(path.as_ref().to_owned());
        self.is_modified = true;
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
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
                self.pt.rename_backing_file(path)?;
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
            file.write_all(bytes)?;
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

            if let (Some(p), Some(bfp)) = (path, bfpath) {
                if p != bfp {
                    let _ = fs::remove_file(bfp);
                }
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
