use crate::{BufferRange, Range};
use sanedit_buffer::{
    utf8::{EndOfLine, Graphemes},
    PieceTreeSlice,
};

/// Get a range of buffer from start - end,
///
/// Params:
///     slice   - piece tree slice
///     pos     - position to start in buffer
///     start   - starting character to find
///     end     - ending character to find
///     include - whether to include starting and ending chars in the range
///
pub fn find_range(
    slice: &PieceTreeSlice,
    pos: u64,
    start: &str,
    end: &str,
    include: bool,
) -> Option<BufferRange> {
    let mut range = if start == end {
        find_range_delim_included(slice, pos, start)?
    } else {
        find_range_included(slice, pos, start, end)?
    };

    if !include {
        range.start += start.len() as u64;
        range.end -= end.len() as u64;
    }

    Some(range)
}

/// Find range of delimiters in the current line
fn find_range_delim_included(slice: &PieceTreeSlice, pos: u64, delim: &str) -> Option<BufferRange> {
    let mut ngraphemes = slice.graphemes_at(pos);
    let mut pgraphemes = ngraphemes.clone();

    // Find next delimiter
    let next = find_next_delim(&mut ngraphemes, delim)?;
    // Try to find previous delimiter
    // "abc|e"
    if let Some(prev) = find_prev_delim(&mut pgraphemes, delim) {
        return Some(Range::new(pos - prev, pos + next));
    }

    // Find next of next
    // | "abce"
    let next_next = find_next_delim(&mut ngraphemes, delim)?;
    Some(Range::new(pos + next, pos + next_next))
}

fn find_prev_delim(graphemes: &mut Graphemes, delim: &str) -> Option<u64> {
    let mut adv = 0;

    while let Some(g) = graphemes.prev() {
        if EndOfLine::is_slice_eol(&g) {
            break;
        }

        adv += g.len();

        if &g == &delim {
            return Some(adv);
        }
    }

    None
}

fn find_next_delim(graphemes: &mut Graphemes, delim: &str) -> Option<u64> {
    let mut adv = 0;

    while let Some(g) = graphemes.next() {
        if EndOfLine::is_slice_eol(&g) {
            break;
        }

        adv += g.len();

        if &g == &delim {
            return Some(adv);
        }
    }

    None
}

fn find_range_included(
    slice: &PieceTreeSlice,
    pos: u64,
    start: &str,
    end: &str,
) -> Option<BufferRange> {
    let slen = start.len() as u64;
    // Search forward for a start or end

    let mut graphemes = slice.graphemes_at(pos);
    let (adv, is_start) = find_next_start_or_end(&mut graphemes, start, end)?;
    if !is_start {
        // if end is found => search backwards from pos for a start
        // "[[] | ]" select the whole thing
        let end_pos = pos + adv + slen;
        let mut graphemes = slice.graphemes_at(pos);
        let mut cpos = pos;
        let mut nest = 1;

        while nest != 0 {
            let (adv, is_start) = find_prev_start_or_end(&mut graphemes, start, end)?;
            cpos -= adv;

            if is_start {
                nest -= 1;
            } else {
                nest += 1;
            }
        }

        Some((cpos..end_pos).into())
    } else {
        // if start is found => search backwards for end or start
        let first_start_after = pos + adv + slen;
        let mut g = slice.graphemes_at(pos);
        let (adv, is_start) = find_prev_start_or_end(&mut g, start, end)?;
        if is_start {
            // "[ | [ ] ]" select the whole thing
            // Search an end for the previous start instead
            let start_pos = pos - adv;
            let mut cpos = first_start_after;
            let mut nest = 2;

            while nest != 0 {
                // NOTE: using graphemes, Continue from old position
                let (adv, is_start) = find_next_start_or_end(&mut graphemes, start, end)?;
                cpos += adv + slen;

                if is_start {
                    nest += 1;
                } else {
                    nest -= 1;
                }
            }

            Some((start_pos..cpos).into())
        } else {
            // "] | [ ]"  select next brackets
            // Jump forward to the first starting pos and search an end for that
            let mut cpos = first_start_after;
            let mut nest = 1;

            while nest != 0 {
                // NOTE: using graphemes, Continue from old position
                let (adv, is_start) = find_next_start_or_end(&mut graphemes, start, end)?;
                cpos += adv + slen;

                if is_start {
                    nest += 1;
                } else {
                    nest -= 1;
                }
            }

            Some((first_start_after - slen..cpos).into())
        }
    }
}

/// Find next start or end in graphemes and return how much we advanced the iterator
fn find_prev_start_or_end(
    graphemes: &mut Graphemes,
    start: &str,
    end: &str,
) -> Option<(u64, bool)> {
    let mut advanced = 0;
    while let Some(g) = graphemes.prev() {
        let gs = String::from(&g);
        advanced += g.len();

        if gs == start {
            return Some((advanced, true));
        } else if gs == end {
            return Some((advanced, false));
        }
    }

    None
}

/// Find next start or end in graphemes and return how much we advanced the iterator
fn find_next_start_or_end(
    graphemes: &mut Graphemes,
    start: &str,
    end: &str,
) -> Option<(u64, bool)> {
    let mut advanced = 0;
    while let Some(g) = graphemes.next() {
        let gs = String::from(&g);

        if gs == start {
            return Some((advanced, true));
        } else if gs == end {
            return Some((advanced, false));
        }

        advanced += g.len();
    }

    None
}
