use std::{cmp::min, collections::BTreeMap};

use sanedit_buffer::utf8::{next_eol, EndOfLine};
use sanedit_core::determine_indent;

use crate::editor::{buffers::BufferConfig, hooks::Hook, Editor};

use sanedit_server::ClientId;

use super::{hooks, ActionResult};

#[action("Detect indentation")]
fn detect_indent(editor: &mut Editor, id: ClientId) -> ActionResult {
    const MAX: u64 = 1024 * 64; // 64kb

    if !editor.config.editor.detect_indent {
        return ActionResult::Ok;
    }

    let (_win, buf) = editor.win_buf_mut(id);
    let len = buf.len();
    let slice = buf.slice(..min(len, MAX));
    let (kind, n) = determine_indent(&slice).unwrap_or_else(|| {
        let opts = BufferConfig::default();
        (opts.indent_kind, opts.indent_amount)
    });
    buf.config.indent_kind = kind;
    buf.config.indent_amount = n;
    ActionResult::Ok
}

#[action("Detect indentation")]
fn detect_eol(editor: &mut Editor, id: ClientId) -> ActionResult {
    const MAX: u64 = 1024 * 64; // 64kb

    if !editor.config.editor.detect_eol {
        return ActionResult::Ok;
    }

    let (_win, buf) = editor.win_buf_mut(id);
    let len = buf.len();
    let slice = buf.slice(..min(len, MAX));
    let mut bytes = slice.bytes();

    let mut votes: BTreeMap<EndOfLine, usize> = BTreeMap::default();
    while let Some(eol) = next_eol(&mut bytes) {
        let entry = votes.entry(eol.eol);
        let value = entry.or_default();
        *value += 1;
    }

    // Find eol with most votes
    let mut max = 0;
    let mut eol = None;
    for (e, votes) in votes {
        if votes > max {
            max = votes;
            eol = Some(e);
        }
    }

    buf.config.eol = eol.unwrap_or_default();
    ActionResult::Ok
}

#[action("Buffer: Indent lines")]
fn indent_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    if win.indent_cursor_lines(buf).is_ok() {
        let bid = buf.id;
        hooks::run(editor, id, Hook::BufChanged(bid));
        ActionResult::Ok
    } else {
        ActionResult::Failed
    }
}

#[action("Buffer: Dedent lines")]
fn dedent_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    if win.dedent_cursor_lines(buf).is_ok() {
        let bid = buf.id;
        hooks::run(editor, id, Hook::BufChanged(bid));
        ActionResult::Ok
    } else {
        ActionResult::Failed
    }
}
