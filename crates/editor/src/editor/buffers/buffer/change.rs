use sanedit_buffer::utf8::EndOfLine;

use super::{SnapshotData, SnapshotId, SortedRanges};

/// Describes what changed in the buffer when the change was made.
#[derive(Debug, Clone)]
pub(crate) struct Change {
    /// Created snapshot position
    pub(crate) created_snapshot: Option<SnapshotId>,

    /// If kind redo or undo the stored snapshot data.
    pub(crate) restored_snapshot: Option<SnapshotData>,

    pub(crate) positions: SortedRanges,
    pub(crate) kind: ChangeKind,
}

impl Change {
    pub fn new(
        prev: Option<&Change>,
        is_modified: bool,
        allow_undo_point: bool,
        kind: ChangeKind,
        ranges: SortedRanges,
    ) -> (Change, bool) {
        let needs_undo_point =
            allow_undo_point && needs_undo_point(prev, is_modified, &kind, &ranges);
        let change = Change {
            kind,
            positions: ranges,
            created_snapshot: None,
            restored_snapshot: None,
        };
        (change, needs_undo_point)
    }
}

fn needs_undo_point(
    prev: Option<&Change>,
    is_modified: bool,
    kind: &ChangeKind,
    ranges: &SortedRanges,
) -> bool {
    use ChangeKind::*;

    if !is_modified || prev.is_none() {
        return false;
    }

    let prev = prev.unwrap();
    match (&prev.kind, kind) {
        (Insert | InsertEOL, Insert) => {
            if prev.positions.len() != ranges.len() {
                return true;
            }

            let mut ins = 0;
            for i in 0..prev.positions.len() {
                let prang = &prev.positions[i];
                let crang = &ranges[i];

                if prang.end + ins != crang.start {
                    return true;
                }

                ins += prang.len();
            }

            false
        }
        (Remove, Remove) => {
            if prev.positions.len() != ranges.len() {
                return true;
            }

            let mut rem = 0;
            for i in 0..prev.positions.len() {
                let prang = &prev.positions[i];
                let crang = &ranges[i];

                if prang.start != crang.end + rem {
                    return true;
                }

                rem += prang.len();
            }

            false
        }
        (_, Redo) => false,
        (Redo | Undo, _) => false,
        _ => true,
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ChangeKind {
    InsertEOL,
    Insert,
    Remove,
    Undo,
    Redo,
}

impl ChangeKind {
    pub fn insert<B: AsRef<[u8]>>(bytes: B) -> ChangeKind {
        let bytes = bytes.as_ref();
        let eol = EndOfLine::is_eol(bytes);

        if eol {
            ChangeKind::InsertEOL
        } else {
            ChangeKind::Insert
        }
    }
}
