use sanedit_buffer::PieceTreeSlice;

use crate::editor::buffers::{Buffer, BufferRange};

/// Get a range of buffer from start - end,
///
/// Params:
///     slice   - piece tree slice
///     pos     - position to start in buffer
///     start   - starting character to find
///     end     - ending character to find
///     include - whether to include starting and ending chars in the range
///     nested  - whether to look for nested structures eg. | { { } } would
///     include the inner brackets in the resulting range or not
///
/// Contains special logic when ch is a bracket
pub(crate) fn find_range(
    slice: &PieceTreeSlice,
    pos: usize,
    start: char,
    end: char,
    include: bool,
    nested: bool,
) -> Option<BufferRange> {
    let mut nest = 0;
    let mut cpos = pos;
    let mut bytes = slice.bytes_at(cpos);

    // Search forward for a start or end
    // if end is found => search backwards from pos for a forward
    // if start is found => continue forward to search for an end

    None
}
