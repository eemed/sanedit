use sanedit_buffer::utf8::EndOfLine;

use super::{SnapshotId, SortedRanges};

#[derive(Debug, Clone)]
pub(crate) enum ChangeKind {
    Insert {
        inserted: SortedRanges,
        is_eol: bool,
    },
    Remove {
        removed: SortedRanges,
    },
    Undo,
    Redo,
    NoOp,
}

/// Describes what changed in the buffer when the change was made.
#[derive(Debug, Clone)]
pub(crate) struct Change {
    // data that will be set after the change is created
    /// Created snapshot id before this operation
    pub(crate) created_snapshot: Option<SnapshotId>,
    /// If kind is undo or redo, the restored snapshot id
    pub(crate) restored_snapshot: Option<SnapshotId>,

    // Change kind the primary data in a change
    pub(crate) kind: ChangeKind,
}

impl Change {
    pub fn insert(ranges: &SortedRanges, bytes: &[u8]) -> Change {
        let is_eol = EndOfLine::is_eol(bytes);
        Change {
            created_snapshot: None,
            restored_snapshot: None,
            kind: ChangeKind::Insert {
                inserted: ranges.clone(),
                is_eol,
            },
        }
    }

    pub fn remove(ranges: &SortedRanges) -> Change {
        Change {
            created_snapshot: None,
            restored_snapshot: None,
            kind: ChangeKind::Remove {
                removed: ranges.clone(),
            },
        }
    }

    pub fn undo() -> Change {
        Change {
            created_snapshot: None,
            restored_snapshot: None,
            kind: ChangeKind::Undo,
        }
    }

    pub fn redo() -> Change {
        Change {
            created_snapshot: None,
            restored_snapshot: None,
            kind: ChangeKind::Redo,
        }
    }

    pub fn no_op() -> Change {
        Change {
            created_snapshot: None,
            restored_snapshot: None,
            kind: ChangeKind::NoOp,
        }
    }

    /// Check wether this and previous change need an undopoint between them
    ///
    /// If true the undopoint should be created at set to this change
    pub fn needs_undo_point(&self, previous: Option<&Change>) -> bool {
        use ChangeKind::*;

        // no previous edits, undo point should be created automatically
        if previous.is_none() {
            return false;
        }

        let pchange = previous.unwrap();
        match (&pchange.kind, &self.kind) {
            (
                Insert {
                    inserted: pranges, ..
                },
                Insert {
                    is_eol: false,
                    inserted: ranges,
                },
            ) => {
                if pranges.len() != ranges.len() {
                    return true;
                }

                let mut ins = 0;
                for i in 0..pranges.len() {
                    let prang = &pranges[i];
                    let crang = &ranges[i];

                    if prang.end + ins != crang.start {
                        return true;
                    }

                    ins += prang.len();
                }

                false
            }
            (Remove { removed: pranges }, Remove { removed: ranges }) => {
                if pranges.len() != ranges.len() {
                    return true;
                }

                let mut rem = 0;
                for i in 0..pranges.len() {
                    let prang = &pranges[i];
                    let crang = &ranges[i];

                    if prang.start != crang.end + rem {
                        return true;
                    }

                    rem += prang.len();
                }

                false
            }
            (_, Redo { .. }) => false,
            (Redo { .. } | Undo { .. }, _) => false,
            _ => true,
        }
    }
}
