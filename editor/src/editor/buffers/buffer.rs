mod change;
mod options;
mod snapshots;

use std::{
    borrow::Cow,
    fs, io,
    ops::RangeBounds,
    path::{Path, PathBuf},
};

use sanedit_buffer::piece_tree::{PieceTree, PieceTreeSlice};

use crate::common::file::File;

use self::{change::Change, options::BufferOptions, snapshots::Snapshots};

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
        let mut pt = PieceTree::new();
        // pt.append("Scratch buffer");
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
        // TODO if file is converted create buffer to tmp dir and
        // save to other dir.
        if file.is_big() {
            Self::file_backed(file)
        } else {
            Self::in_memory(file)
        }
    }

    fn file_backed(file: File) -> io::Result<Buffer> {
        let file = fs::File::open(file.path())?;
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

    fn in_memory(file: File) -> io::Result<Buffer> {
        let file = fs::File::open(file.path())?;
        Self::from_reader(file)
    }

    fn from_reader<R: io::Read>(reader: R) -> io::Result<Buffer> {
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

    pub fn name(&self) -> Cow<'_, str> {
        self.path
            .as_ref()
            .map(|p| p.to_string_lossy())
            .unwrap_or(Cow::from("scratch"))
    }

    pub fn len(&self) -> usize {
        self.pt.len()
    }

    pub fn remove<R: RangeBounds<usize>>(&mut self, range: R) {
        self.pt.remove(range)
    }

    pub fn append<B: AsRef<[u8]>>(&mut self, bytes: B) {
        self.pt.append(bytes)
    }

    pub fn insert<B: AsRef<[u8]>>(&mut self, pos: usize, bytes: B) {
        self.pt.insert(pos, bytes)
    }

    pub fn insert_char(&mut self, pos: usize, ch: char) {
        self.pt.insert_char(pos, ch)
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
