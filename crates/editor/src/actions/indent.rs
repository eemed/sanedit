use std::cmp::min;

use sanedit_core::determine_indent;

use crate::editor::{buffers::BufferConfig, Editor};

use sanedit_server::ClientId;

use super::ActionResult;

#[action("Detect indentation")]
fn detect_indent(editor: &mut Editor, id: ClientId) -> ActionResult {
    const MAX: u64 = 1024 * 64; // 64kb

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

#[action("Buffer: Indent lines")]
fn indent_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    win.indent_cursor_lines(buf).into()
}

#[action("Buffer: Dedent lines")]
fn dedent_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    win.dedent_cursor_lines(buf).into()
}
