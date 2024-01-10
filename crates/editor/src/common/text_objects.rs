use sanedit_buffer::{utf8::Graphemes, Bytes, PieceTreeSlice, Searcher, SearcherRev};

use crate::editor::buffers::{Buffer, BufferRange};

/// Get a range of buffer from start - end,
///
/// Params:
///     slice   - piece tree slice
///     pos     - position to start in buffer
///     start   - starting character to find
///     end     - ending character to find
///     include - whether to include starting and ending chars in the range
///
/// Contains special logic when ch is a bracket
pub(crate) fn find_range(
    slice: &PieceTreeSlice,
    pos: usize,
    start: &str,
    end: &str,
    include: bool,
) -> Option<BufferRange> {
    // Search forward for a start or end
    // if end is found => search backwards from pos for a forward
    // if start is found => continue forward to search for an end

    None
}
