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

/// Find matching pair for char at pos
pub(crate) fn matching_pair(slice: &PieceTreeSlice, pos: usize) -> Option<usize> {
    let mut bytes = slice.bytes_at(pos);
    let byte = bytes.at(pos);
    let (dir, pair) = direction_and_pair(byte)?;
    let mut cpos = pos;
    let mut opening_count = 0;

    loop {
        match dir {
            SearchDirection::Backward => {
                if cpos == 0 {
                    break;
                }

                cpos -= 1;
            }
            SearchDirection::Forward => {
                cpos += 1;

                if cpos >= slice.len() {
                    break;
                }
            }
        }

        let b = bytes.at(cpos);

        if b == pair && opening_count == 0 {
            return Some(cpos);
        } else if b == pair {
            opening_count -= 1;
        } else if b == byte {
            opening_count += 1;
        }
    }

    None
}
