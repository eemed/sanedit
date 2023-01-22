mod change;
mod detect;
mod eol;
mod options;
mod snapshots;

use std::{
    fs::File,
    io,
    ops::RangeBounds,
    path::{Path, PathBuf},
};

use sanedit_buffer::piece_tree::{PieceTree, PieceTreeSlice};

use self::{change::Change, options::BufferOptions, snapshots::Snapshots};
pub(crate) use eol::EOL;

slotmap::new_key_type!(
    pub(crate) struct BufferId;
);

#[derive(Debug)]
pub(crate) struct Buffer {
    pub(crate) id: BufferId,

    pt: PieceTree,
    /// Snapshots of the piecetree, used for undo
    snapshots: Snapshots,
    last_saved_snapshot: usize,

    is_modified: bool,
    options: BufferOptions,
    last_change: Option<Change>,

    /// Path used for saving the file.
    path: Option<PathBuf>,
}

impl Buffer {
    pub fn new() -> Buffer {
        let pt = PieceTree::new();
        let snapshot = pt.snapshot();
        Buffer {
            id: BufferId::default(),
            pt,
            is_modified: false,
            snapshots: Snapshots::new(snapshot),
            options: BufferOptions::default(),
            path: None,
            last_change: None,
            last_saved_snapshot: 0,
        }
    }

    pub fn from_file(file: File) -> io::Result<Buffer> {
        let pt = PieceTree::from_file(file)?;
        let snapshot = pt.snapshot();
        Ok(Buffer {
            id: BufferId::default(),
            pt,
            is_modified: false,
            snapshots: Snapshots::new(snapshot),
            options: BufferOptions::default(),
            path: None,
            last_change: None,
            last_saved_snapshot: 0,
        })
    }

    pub fn from_reader<R: io::Read>(reader: R) -> io::Result<Buffer> {
        let pt = PieceTree::from_reader(reader)?;
        let snapshot = pt.snapshot();
        Ok(Buffer {
            id: BufferId::default(),
            pt,
            is_modified: false,
            snapshots: Snapshots::new(snapshot),
            options: BufferOptions::default(),
            path: None,
            last_change: None,
            last_saved_snapshot: 0,
        })
    }

    pub fn set_path<P: AsRef<Path>>(&mut self, path: P) {
        self.path = Some(path.as_ref().to_owned())
    }

    pub fn slice<R: RangeBounds<usize>>(&self, range: R) -> PieceTreeSlice {
        self.pt.slice(range)
    }

    pub fn options(&self) -> &BufferOptions {
        &self.options
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Buffer::new()
    }
}
