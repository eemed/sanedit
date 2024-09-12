use std::cmp::min;

use sanedit_core::determine_indent;

use crate::editor::{buffers::Options, Editor};

use sanedit_server::ClientId;

#[action("Detect indentation")]
fn detect_indent(editor: &mut Editor, id: ClientId) {
    const MAX: u64 = 1024 * 64; // 64kb

    let (win, buf) = editor.win_buf_mut(id);
    let len = buf.len();
    let slice = buf.slice(..min(len, MAX));
    let (kind, n) = determine_indent(&slice).unwrap_or_else(|| {
        let opts = Options::default();
        (opts.indent_kind, opts.indent_amount)
    });
    buf.options.indent_kind = kind;
    buf.options.indent_amount = n;
}

#[action("Indent cursor lines")]
fn indent_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.indent_cursor_lines(buf);
}

#[action("Dedent cursor lines")]
fn dedent_line(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.dedent_cursor_lines(buf);
}
