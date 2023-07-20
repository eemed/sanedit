mod change;
mod cursor;
mod options;
mod snapshots;
mod sorted;

use std::{
    borrow::Cow,
    fs, io,
    ops::{Range, RangeBounds},
    path::{Path, PathBuf},
};

use sanedit_buffer::{PieceTree, PieceTreeSlice, ReadOnlyPieceTree, SortedPositions};
use sanedit_regex::Cursor;

use crate::common::file::File;

use self::{
    change::{Change, ChangeKind},
    cursor::BufferCursor,
    options::Options,
    snapshots::Snapshots,
};
pub(crate) use snapshots::SnapshotData;
pub(crate) use sorted::SortedRanges;

slotmap::new_key_type!(
    pub(crate) struct BufferId;
);

pub(crate) type BufferRange = Range<usize>;

#[derive(Debug)]
pub(crate) struct Buffer {
    pub(crate) id: BufferId,

    pt: PieceTree,
    /// Snapshots of the piecetree, used for undo
    snapshots: Snapshots,
    last_saved_snapshot: usize,

    is_modified: bool,
    options: Options,
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
            pt,
            is_modified: false,
            snapshots: Snapshots::new(snapshot),
            options: Options::default(),
            path: None,
            last_change: None,
            last_saved_snapshot: 0,
        }
    }

    pub fn from_file(file: File) -> io::Result<Buffer> {
        if file.is_big() {
            Self::file_backed(file)
        } else {
            Self::in_memory(file)
        }
    }

    fn file_backed(file: File) -> io::Result<Buffer> {
        log::debug!("New file backed buf");
        let path = file.path().canonicalize()?;
        let file = fs::File::open(&path)?;
        // let pt = PieceTree::mmap(file)?;
        let pt = PieceTree::from_file(file);
        let snapshot = pt.read_only_copy();
        Ok(Buffer {
            id: BufferId::default(),
            pt,
            is_modified: false,
            snapshots: Snapshots::new(snapshot),
            options: Options::default(),
            path: Some(path),
            last_change: None,
            last_saved_snapshot: 0,
        })
    }

    fn in_memory(file: File) -> io::Result<Buffer> {
        log::debug!("New buf");
        let path = file.path().canonicalize()?;
        let file = fs::File::open(&path)?;
        let mut buf = Self::from_reader(file)?;
        buf.path = Some(path);
        Ok(buf)
    }

    fn from_reader<R: io::Read>(reader: R) -> io::Result<Buffer> {
        let pt = PieceTree::from_reader(reader)?;
        let snapshot = pt.read_only_copy();
        Ok(Buffer {
            id: BufferId::default(),
            pt,
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

    /// Stores snapshot data to the snapshot that we are currently on
    pub fn store_snapshot_data(&mut self, sdata: SnapshotData) {
        self.snapshots.set_current_data(sdata)
    }

    /// Get the last change done to buffer
    pub fn last_change(&self) -> Option<&Change> {
        self.last_change.as_ref()
    }

    fn prepare_change(&mut self, kind: ChangeKind, ranges: SortedRanges) -> Change {
        self.prepare_change_impl(kind, ranges, true)
    }

    fn prepare_change_impl(
        &mut self,
        kind: ChangeKind,
        ranges: SortedRanges,
        allow_undo_point: bool,
    ) -> Change {
        let last = self.last_change.as_ref();
        let change = Change::new(last, self.is_modified, allow_undo_point, kind, ranges);

        if change.needs_undo_point {
            let snap = self.pt.read_only_copy();
            self.snapshots.insert(snap);
        }

        change
    }

    pub fn undo(&mut self) -> Result<Option<SnapshotData>, &str> {
        let change = self.prepare_change(ChangeKind::Undo, vec![].into());
        if let Some(node) = self.snapshots.undo() {
            self.is_modified = node.idx != self.last_saved_snapshot;
            self.last_change = change.into();
            self.pt.restore(node.snapshot);
            Ok(node.data)
        } else {
            Err("No more undo points")
        }
    }

    pub fn redo(&mut self) -> Result<Option<SnapshotData>, &str> {
        let change = self.prepare_change(ChangeKind::Redo, vec![].into());
        if let Some(node) = self.snapshots.redo() {
            self.is_modified = node.idx != self.last_saved_snapshot;
            self.last_change = change.into();
            self.pt.restore(node.snapshot);
            Ok(node.data)
        } else {
            Err("No more redo points")
        }
    }

    pub fn remove(&mut self, range: Range<usize>) {
        let change = self.prepare_change(ChangeKind::Remove, vec![range.clone()].into());

        self.pt.remove(range);
        self.is_modified = true;
        self.last_change = change.into();
    }

    pub fn append<B: AsRef<[u8]>>(&mut self, bytes: B) {
        self.insert_multi(self.pt.len().into(), bytes)
    }

    pub fn insert<B: AsRef<[u8]>>(&mut self, pos: usize, bytes: B) {
        self.insert_multi(pos.into(), bytes)
    }

    pub fn insert_multi<B: AsRef<[u8]>>(&mut self, pos: SortedPositions, bytes: B) {
        let bytes = bytes.as_ref();
        let kind = ChangeKind::insert(bytes);
        let ranges: Vec<BufferRange> = pos.iter().map(|pos| *pos..pos + bytes.len()).collect();
        let change = self.prepare_change(kind, ranges.into());

        self.pt.insert_multi(pos, bytes);
        self.is_modified = true;
        self.last_change = change.into();
    }

    pub fn remove_multi(&mut self, ranges: SortedRanges) {
        let change = self.prepare_change(ChangeKind::Remove, ranges);

        for change in change.positions.iter().rev() {
            self.pt.remove(change.clone());
        }

        self.is_modified = true;
        self.last_change = change.into();
    }

    pub fn set_path<P: AsRef<Path>>(&mut self, path: P) {
        self.path = Some(path.as_ref().to_owned());
        self.is_modified = true;
    }

    pub fn slice<R: RangeBounds<usize>>(&self, range: R) -> PieceTreeSlice {
        self.pt.slice(range)
    }

    pub fn options(&self) -> &Options {
        &self.options
    }

    pub fn cursor<'a>(&'a self) -> impl Cursor + 'a {
        BufferCursor::new(&self.pt)
    }

    pub fn save(&self) -> Result<(), io::Error> {
        log::info!("SAVE");
        debug_assert!(!self.pt.is_file_backed());

        let path = self
            .path
            .as_ref()
            .ok_or(io::Error::from(io::ErrorKind::NotFound))?;
        log::info!("SAVING to {:?}", path);
        let file = fs::File::create(&path)?;
        self.pt.write_to(file)?;
        Ok(())
    }

    pub fn read_only_copy(&self) -> ReadOnlyPieceTree {
        self.pt.read_only_copy()
    }

    pub fn is_modified(&self) -> bool {
        self.is_modified
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Buffer::new()
    }
}
