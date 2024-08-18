use std::cmp::min;

use crate::{common::indent::determine_indent, editor::Editor, server::ClientId};

#[action("Detect indentation")]
fn detect_indent(editor: &mut Editor, id: ClientId) {
    const MAX: u64 = 1024 * 64; // 64kb

    let (win, buf) = editor.win_buf_mut(id);
    let len = buf.len();
    let slice = buf.slice(..min(len, MAX));
    let (kind, n) = determine_indent(&slice);
    buf.options.indent_kind = kind;
    buf.options.indent_amount = n;
}

#[action("Indent cursor lines")]
fn indent_line(editor: &mut Editor, id: ClientId) {
    unimplemented!()
}

#[action("Dedent cursor lines")]
fn dedent_line(editor: &mut Editor, id: ClientId) {
    unimplemented!()
}
