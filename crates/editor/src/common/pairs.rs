use sanedit_buffer::{Bytes, PieceTreeSlice};

use crate::editor::windows::SearchDirection;

const PAIRS: [(char, char); 4] = [('(', ')'), ('[', ']'), ('{', '}'), ('<', '>')];

pub(crate) fn is_pair(ch: &char) -> bool {
    PAIRS.iter().any(|(a, b)| a == ch || b == ch)
}

/// Get direction search direction and pair to search for a byte
fn direction_and_pair(byte: u8) -> Option<(SearchDirection, u8)> {
    for (a, b) in PAIRS {
        if a as u8 == byte {
            return Some((SearchDirection::Forward, b as u8));
        }

        if b as u8 == byte {
            return Some((SearchDirection::Backward, a as u8));
        }
    }

    None
}

fn next(dir: SearchDirection, bytes: &mut Bytes) -> Option<u8> {
    match dir {
        SearchDirection::Backward => bytes.prev(),
        SearchDirection::Forward => bytes.next(),
    }
}

/// Find matching pair for char at pos
pub(crate) fn matching_pair(slice: &PieceTreeSlice, pos: usize) -> Option<usize> {
    let bytes = slice.bytes_at(pos);
    let byte = bytes.at(pos);

    let (dir, pair) = direction_and_pair(byte)?;

    let mut opening_count = 0;
    while let Some(b) = next(dir, &mut bytes) {
        if b == pair {
            if opening_count == 0 {
                return Some(bytes.pos());
            } else {
                opening_count -= 1;
            }
        }

        if b == byte {
            opening_count += 1;
        }
    }

    None
}
