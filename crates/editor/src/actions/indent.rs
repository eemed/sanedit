use std::cmp::min;

use crate::{common::indent::Indent, editor::Editor, server::ClientId};

#[action("Detect indentation")]
fn detect_indent(editor: &mut Editor, id: ClientId) {
    const MAX: usize = 1024 * 64; // 64kb

    let (win, buf) = editor.win_buf_mut(id);
    let len = buf.len();
    let slice = buf.slice(..min(len, MAX));
    let indent = Indent::determine(&slice);
    buf.options.indent = indent;
}
