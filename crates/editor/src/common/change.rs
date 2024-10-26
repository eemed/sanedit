use sanedit_buffer::PieceTreeSlice;
use sanedit_core::{indent_at_line, movement::start_of_line, Change};

use crate::editor::buffers::Buffer;

use super::text::{is_eol_or_eof_at, only_whitespace_before};

/// Handle indenting newline insertion on an whitepace only line
///
/// Moves the current indentation of the current line
/// to the next line, by inserting the newline at linestart instead.
///
pub(crate) fn newline_empty_line(buf: &Buffer, pos: u64) -> Option<Change> {
    let was_eol = buf
        .last_edit()
        .map(|edit| edit.changes.has_insert_eol())
        .unwrap_or(false);
    let slice = buf.slice(..);
    let at_eol = is_eol_or_eof_at(&slice, pos);
    if !was_eol || !at_eol {
        return None;
    }
    let start = only_whitespace_before(&slice, pos)?;
    Some(Change::insert(start, buf.config.eol.as_str().as_bytes()))
}

/// Handle newline with indenting
pub(crate) fn newline_indent(buf: &Buffer, pos: u64) -> Change {
    let slice = buf.slice(..);

    // Indent next line if previous was indented
    let indent_line = indent_at_line(&slice, pos);
    let indent = {
        match indent_line {
            Some((k, n)) => k.repeat(n as usize),
            None => String::new(),
        }
    };

    let text = format!("{}{}", buf.config.eol.as_str(), indent);
    Change::insert(pos, text.as_bytes())
}

/// Handle newline with autopair closing and indenting
pub(crate) fn newline_autopair(buf: &Buffer, pos: u64) -> Option<Change> {
    let slice = buf.slice(..);
    let at_eol = is_eol_or_eof_at(&slice, pos);
    // Not at eol or eof
    if !at_eol {
        return None;
    }
    let close = close_pairs_before(&slice, pos);
    let iamount = buf.config.indent_amount;
    let indent_line = indent_at_line(&slice, pos);
    let indent = {
        match indent_line {
            Some((k, n)) => k.repeat(n as usize),
            None => String::new(),
        }
    };

    // Nothing to close
    if close.is_empty() {
        return None;
    }
    let next_indent_level = match indent_line {
        Some((k, n)) => k.repeat(n as usize + iamount as usize),
        None => buf.config.indent_kind.repeat(iamount as usize),
    };
    let eol = buf.config.eol;
    let block = format!(
        "{}{}{}{}{}",
        eol.as_str(),
        next_indent_level,
        eol.as_str(),
        indent,
        close
    );
    let mut change = Change::insert(pos, block.as_bytes());
    // Cursor to middle line
    change.cursor_offset = Some(eol.len() + next_indent_level.len() as u64);
    Some(change)
}

/// Returns string to close pairs on line before pos
fn close_pairs_before(slice: &PieceTreeSlice, pos: u64) -> String {
    // TODO tags
    // configurable pairs, case esac in bash etc
    // regex on line + ending

    let start = start_of_line(slice, pos);
    let line = slice.slice(start..pos);
    let mut chars = line.chars();
    let mut result = vec![];
    let mut in_quote: Option<char> = None;
    let mut escaped = false;

    while let Some((_, _, ch)) = chars.next() {
        match ch {
            '{' => {
                if in_quote.is_none() {
                    result.push('}');
                }
            }
            '[' => {
                if in_quote.is_none() {
                    result.push(']');
                }
            }
            '(' => {
                if in_quote.is_none() {
                    result.push(')');
                }
            }
            '\\' => {
                escaped = true;
            }
            '"' | '\'' | '`' => {
                if escaped {
                    continue;
                }
                match in_quote {
                    Some(quot) => {
                        if quot == ch {
                            in_quote = None;
                        }
                    }
                    None => {
                        in_quote = Some(ch);
                    }
                }
            }
            ch => {
                if result.last() == Some(&ch) {
                    result.pop();
                }
            }
        }
    }

    result.into_iter().rev().fold(String::new(), |mut acc, ch| {
        acc.push(ch);
        acc
    })
}
