mod change;
mod detect;
mod eol;
mod options;
mod snapshots;

use std::{fs::File, io, path::PathBuf};

use sanedit_buffer::piece_tree::PieceTree;

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
        // TODO
        // do we need encoding here
        // create an in memory buffer if file size is small
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
}

impl Default for Buffer {
    fn default() -> Self {
        todo!()
    }
}
