use std::ops::{Range, RangeBounds};

use sanedit_buffer::utf8::EndOfLine;

#[derive(Debug, Clone)]
pub(crate) enum Change {
    Insert { pos: usize, len: usize, eol: bool },
    Remove { pos: usize, len: usize },
    Undo,
    Redo,
}

impl Change {
    pub fn remove<R: RangeBounds<usize>>(range: R, buf: Range<usize>) -> Change {
        use std::ops::Bound::*;
        let start = match range.start_bound() {
            Included(n) => *n,
            Excluded(n) => *n + 1,
            Unbounded => 0,
        };
        let end = match range.end_bound() {
            Included(n) => *n + 1,
            Excluded(n) => *n,
            Unbounded => buf.end,
        };

        Change::Remove {
            pos: start,
            len: end - start,
        }
    }

    pub fn insert<B: AsRef<[u8]>>(pos: usize, bytes: B) -> Change {
        let bytes = bytes.as_ref();
        let eol = EndOfLine::is_eol(bytes);
        let len = bytes.len();
        Change::Insert { pos, len, eol }
    }
}
