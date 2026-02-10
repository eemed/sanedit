mod change;
mod config;
mod snapshots;

use std::{
    borrow::Cow,
    ffi::OsString,
    fs::{self, File},
    io::{self, Write},
    ops::RangeBounds,
    os::unix::fs::MetadataExt as _,
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::ensure;
use anyhow::Result;
use sanedit_buffer::{Mark, MarkResult, PieceTree, PieceTreeSlice};
use sanedit_core::Edit;
use sanedit_core::{tmp_file, Changes, Language};
use sanedit_utils::key_type;
use thiserror::Error;

use crate::editor::file_description::FileDescription;

use self::snapshots::Snapshots;

pub(crate) use change::ChangeResult;
pub(crate) use config::BufferConfig;
pub(crate) use snapshots::{SavedWindowState, SnapshotId};

key_type!(pub(crate) BufferId);

#[derive(Debug)]
pub(crate) struct Buffer {
    pub(crate) id: BufferId,
    pub(crate) language: Option<Language>,
    pub(crate) config: BufferConfig,
    pub(crate) read_only: bool,
    /// Whether the buffer should be removed when it is no longer shown
    /// Useful for example status buffers that should always be destroyed once left
    pub(crate) remove_on_exit: bool,

    pt: PieceTree,
    /// Snapshots of the piecetree, used for undo
    snapshots: Snapshots,
    last_saved_snapshot: SnapshotId,
    pub(crate) last_saved_modified: Option<SystemTime>,

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
            language: None,
            read_only: false,
            pt,
            is_modified: false,
            remove_on_exit: false,
            snapshots: Snapshots::new(),
            config: BufferConfig::default(),
            path: None,
            last_edit: None,
            last_saved_snapshot: 0,
            last_saved_modified: None,
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
        log::debug!("Creating file backed buffer: {file:?}");
        let path = file.path();
        let pt = PieceTree::from_path(path)?;
        let modified = path.metadata()?.modified()?;
        Ok(Buffer {
            id: BufferId::default(),
            read_only: file.read_only(),
            pt,
            language: file.language().cloned(),
            is_modified: false,
            remove_on_exit: false,
            snapshots: Snapshots::new(),
            config: options,
            path: Some(path.into()),
            last_edit: None,
            last_saved_snapshot: 0,
            last_saved_modified: Some(modified),
            total_changes_made: 0,
        })
    }

    fn in_memory(file: FileDescription, options: BufferConfig) -> Result<Buffer> {
        log::debug!(
            "Creating in memory buffer: {file:?}, exists: {}",
            file.path().exists()
        );
        let path = file.path();
        let mut buf = if !path.exists() {
            Self::new()
        } else {
            let ffile = fs::File::open(path)?;
            Self::from_reader(ffile)?
        };

        let modified = {
            let mut modif = None;
            if let Ok(mdata) = path.metadata() {
                if let Ok(modified) = mdata.modified() {
                    modif = Some(modified);
                }
            }

            modif
        };
        buf.is_modified = !path.exists();
        buf.last_saved_modified = modified;
        buf.language = file.language().cloned();
        buf.path = Some(path.into());
        buf.read_only = file.read_only();
        buf.config = options;
        Ok(buf)
    }

    pub fn from_reader<R: io::Read>(reader: R) -> Result<Buffer> {
        let pt = PieceTree::from_reader(reader)?;
        Ok(Buffer {
            id: BufferId::default(),
            pt,
            language: None,
            read_only: false,
            is_modified: false,
            remove_on_exit: false,
            snapshots: Snapshots::new(),
            config: BufferConfig::default(),
            path: None,
            last_edit: None,
            last_saved_snapshot: 0,
            last_saved_modified: None,
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

    pub fn snapshots(&self) -> &Snapshots {
        &self.snapshots
    }

    /// Get mutable access to auxilary data for a snapshot
    pub fn snapshot_additional_mut(&mut self, id: SnapshotId) -> Option<&mut SavedWindowState> {
        self.snapshots.window_state_mut(id)
    }

    /// Get access to auxilary data for a snapshot
    pub fn snapshot_additional(&self, id: SnapshotId) -> Option<&SavedWindowState> {
        self.snapshots.window_state(id)
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

    pub fn on_undopoint(&self) -> bool {
        let on_save = !self.is_modified && self.total_changes_made != 0;
        let on_previous_point = self
            .last_edit
            .as_ref()
            .map(|edit| edit.changes.is_undo() || edit.changes.is_redo())
            .unwrap_or(false);
        on_save || on_previous_point
    }

    pub fn create_undopoint(&mut self, wstate: SavedWindowState) {
        if !self.is_modified() && Some(self.last_saved_snapshot) == self.snapshots().current() {
            return;
        }

        let slice = self.pt.slice(..);
        let id = self.snapshots.insert(slice);
        if let Some(state) = self.snapshots.window_state_mut(id) {
            *state = wstate;
        }
    }

    pub fn apply_changes(&mut self, changes: &Changes) -> Result<ChangeResult> {
        ensure!(!self.read_only, BufferError::ReadOnly);

        let mut result = ChangeResult::default();
        let rollback = self.slice(..);
        let snapshot = self.snapshots.current();
        let needs_undo = self.needs_undo_point(changes);
        let forks = self.on_undopoint() && !changes.is_undo() && !changes.is_redo();

        if forks {
            result.forked_snapshot = snapshot;
        }

        if needs_undo {
            let snapshot = self.slice(..);
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
        if changes.is_undo_jump() {
            let index = changes.undo_jump_index();
            self.snapshots
                .goto_get(index)
                .ok_or(BufferError::NoSuchSnapshot)?;
            self.restore_snapshot(index)?;
            Ok(Some(index))
        } else if changes.is_undo() {
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

    fn restore_snapshot(&mut self, snapshot: SnapshotId) -> Result<()> {
        let node = self
            .snapshots
            .get(snapshot)
            .ok_or(BufferError::NoSuchSnapshot)?;
        let restored = node.id;
        self.is_modified =
            self.last_saved_modified.is_none() || restored != self.last_saved_snapshot;
        self.pt.restore(node.snapshot.clone());
        Ok(())
    }

    fn undo(&mut self) -> Result<SnapshotId> {
        let node = self.snapshots.undo().ok_or(BufferError::NoMoreUndoPoints)?;
        let id = node.id;
        self.restore_snapshot(id)?;
        Ok(id)
    }

    fn redo(&mut self) -> Result<SnapshotId> {
        let node = self.snapshots.redo().ok_or(BufferError::NoMoreRedoPoints)?;
        let id = node.id;
        self.restore_snapshot(id)?;
        Ok(id)
    }

    pub fn set_path<P: AsRef<Path>>(&mut self, path: P) {
        self.path = Some(path.as_ref().to_owned());
        self.is_modified = true;
        self.last_saved_modified = None;
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn slice<R: RangeBounds<u64>>(&self, range: R) -> PieceTreeSlice {
        self.pt.slice(range)
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

        // Create directiories upto file
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let copy_view = self.slice(..);
        let copy = Self::save_copy(&copy_view)?;
        let saved = self.save_rename_copy(copy_view, &copy);
        let _ = fs::remove_file(&copy);
        saved
    }

    fn save_rename_copy(&mut self, copy_view: PieceTreeSlice, copy: &Path) -> Result<Saved> {
        let path = self.path().ok_or(BufferError::NoSavePath)?;
        let metadata = path.metadata().ok();
        let xattrs = {
            let mut attrs = Vec::new();
            if let Ok(attr_names) = xattr::list(path) {
                for attr in attr_names {
                    if let Ok(Some(value)) = xattr::get(path, &attr) {
                        attrs.push((attr, value));
                    }
                }
            }
            attrs
        };

        // Rename backing file if it is the same as our path
        if self.pt.is_file_backed() {
            let backing = self.pt.backing_file().unwrap();
            if backing == path {
                let (path, _file) = tmp_file().ok_or(BufferError::CannotCreateTmpFile)?;
                self.pt.rename_backing_file(path)?;
            }
        }

        if let Err(e) = fs::rename(copy, path) {
            log::error!("Rename failed while saving {path:?}: {e}");
            fs::copy(copy, path)?;
        }

        if let Some(metadata) = metadata {
            copy_metadata(metadata, xattrs, path)?;
        }

        let modified = path.metadata()?.modified()?;
        self.last_saved_modified = Some(modified);

        self.is_modified = false;
        let snap = self.snapshots.insert(copy_view);
        self.last_saved_snapshot = snap;
        Ok(Saved { snapshot: snap })
    }

    pub fn last_saved_snapshot(&self) -> usize {
        self.last_saved_snapshot
    }

    fn save_copy(buf: &PieceTreeSlice) -> Result<PathBuf> {
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

    pub fn mark(&self, pos: u64) -> Mark {
        self.pt.mark(pos)
    }

    pub fn mark_to_pos(&self, mark: &Mark) -> MarkResult {
        self.pt.mark_to_pos(mark)
    }

    pub fn set_unsaved(&mut self) {
        self.is_modified = true;
    }

    pub fn reload_from_disk(&mut self) -> bool {
        let Some(path) = self.path().map(PathBuf::from) else {
            return false;
        };
        self.reload_from_disk_impl(&path).is_ok()
    }

    pub fn is_file_backed(&self) -> bool {
        self.pt.is_file_backed()
    }

    fn reload_from_disk_impl(&mut self, path: &Path) -> io::Result<()> {
        let file_backed = self.pt.is_file_backed();
        self.pt = if file_backed {
            PieceTree::from_path(path)?
        } else {
            let file = File::open(path)?;
            PieceTree::from_reader(file)?
        };

        let modified = path.metadata()?.modified()?;
        self.last_saved_modified = Some(modified);
        self.is_modified = false;
        self.snapshots = Snapshots::new();
        self.last_edit = None;
        self.last_saved_snapshot = 0;
        self.total_changes_made = 0;

        Ok(())
    }

    /// Rename or move the buffer to a different location
    pub fn rename(&mut self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if self.pt.is_file_backed() {
            self.pt.rename_backing_file(path)?;
        } else {
            self.set_path(path);
            self.save_rename()?;
        }

        Ok(())
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

fn copy_metadata(
    metadata: std::fs::Metadata,
    xattrs: Vec<(OsString, Vec<u8>)>,
    to: &Path,
) -> Result<()> {
    // Permissions
    std::fs::set_permissions(to, metadata.permissions())?;

    // User and group
    let usergroup = (metadata.uid(), metadata.gid());
    let current = to.metadata()?;
    let current_user_group = (current.uid(), current.gid());
    if usergroup != current_user_group {
        // Dont care if this fails, we have succesfully written to this already
        let _ = std::os::unix::fs::chown(to, Some(usergroup.0), Some(usergroup.1));
    }

    // User and group
    for (attr, value) in &xattrs {
        xattr::set(to, attr, value)?;
    }

    Ok(())
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

    #[error("Snapshot does not exist")]
    NoSuchSnapshot,
}

#[derive(Debug)]
pub(crate) struct Saved {
    pub(crate) snapshot: SnapshotId,
}
