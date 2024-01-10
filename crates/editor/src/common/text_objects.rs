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
    let slen = start.len();
    // Search forward for a start or end
    // if end is found => search backwards from pos for a forward
    // if start is found => continue forward to search for an end

    let mut graphemes = slice.graphemes_at(pos);
    let (adv, is_start) = find_start_or_end(&mut graphemes, start, end)?;
    if is_start {
        // TODO nesting eg. [ a [ b ] ] with start [ and end ]
        let start = pos + adv + slen;
        let adv = find_next(&mut graphemes, end)?;
        return Some(start..start + adv);
    } else {
        log::info!("no impl");
    }
    None
}

/// Find next item in graphemes and return how much we advanced the iterator
fn find_next(graphemes: &mut Graphemes, item: &str) -> Option<usize> {
    let mut adv = 0;
    while let Some(g) = graphemes.next() {
        if g == item {
            return Some(adv);
        }
        adv += g.len();
    }

    None
}

/// Find next start or end in graphemes and return how much we advanced the iterator
fn find_start_or_end(graphemes: &mut Graphemes, start: &str, end: &str) -> Option<(usize, bool)> {
    let mut advanced = 0;
    while let Some(g) = graphemes.next() {
        let gs = String::from(&g);

        if gs == start {
            return Some((advanced, true));
        } else if gs == end {
            return Some((advanced, true));
        }

        advanced += g.len();
    }

    None
}
