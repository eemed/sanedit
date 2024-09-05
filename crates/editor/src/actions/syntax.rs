use crate::{common::range::RangeUtils, editor::Editor, server::ClientId};

use super::jobs::SyntaxParser;

#[action("Adjust highlighting to take a buffer change into account")]
pub(crate) fn prevent_flicker(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    let old = win.syntax_result();
    let Some(edit) = buf.last_edit() else {
        return;
    };

    for hl in &mut old.highlights {
        for change in edit.changes.iter() {
            todo!()
        }
    }
}

#[action("Parse buffer syntax for view")]
pub(crate) fn reparse_view(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.redraw_view(buf);
    let bid = buf.id;
    let total = buf.total_changes_made();

    let view = win.view().range();
    let old = win.syntax_result();

    if old.bid != bid || old.total_changes_made != total || !old.buffer_range.includes(&view) {
        parse_syntax.execute(editor, id);
    }
}

#[action("Parse buffer syntax")]
pub(crate) fn parse_syntax(editor: &mut Editor, id: ClientId) {
    const JOB_NAME: &str = "parse-syntax";

    let (win, buf) = editor.win_buf_mut(id);
    let bid = buf.id;
    let total_changes_made = buf.total_changes_made();
    let range = win.view().range();
    let ropt = buf.read_only_copy();

    let Some(ft) = buf.filetype.clone() else {
        return;
    };
    match editor.syntaxes.get(&ft) {
        Ok(s) => {
            editor.job_broker.request_slot(
                id,
                JOB_NAME,
                SyntaxParser::new(id, bid, total_changes_made, s, ropt, range),
            );
        }
        Err(e) => log::error!("Failed to load syntax for {}: {e}", ft.as_str()),
    }
}
