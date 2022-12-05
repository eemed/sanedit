mod eol;
mod file_info;
mod options;
mod snapshots;

use sanedit_buffer::piece_tree::PieceTree;

use self::{file_info::FileInfo, options::BufferOptions, snapshots::Snapshots};

#[derive(Debug)]
enum Change {
    Insert { pos: usize, len: usize },
    Remove { pos: usize, len: usize },
    Undo,
    Redo,
}

slotmap::new_key_type!(
    pub(crate) struct BufferId;
);

#[derive(Debug)]
pub(crate) struct Buffer {
    id: BufferId,
    pt: PieceTree,
    is_modified: bool,
    snapshots: Snapshots,
    options: BufferOptions,
    original_file: Option<FileInfo>,
    last_change: Option<Change>,
    last_saved_snapshot: usize,
}
