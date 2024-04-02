use crate::{
    editor::{syntax::SyntaxParseResult, Editor},
    server::ClientId,
};

use super::jobs::SyntaxParser;

#[action("Parse buffer syntax")]
pub(crate) fn parse_syntax(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);

    // Clear syntax
    *win.syntax_result() = SyntaxParseResult::default();
    if let Some(jid) = win.syntax_job().clone() {
        editor.job_broker.stop(jid);
    }

    let (win, buf) = editor.win_buf_mut(id);
    let bid = buf.id;
    let range = win.view().range();
    let ropt = buf.read_only_copy();

    if let Some(ft) = buf.filetype.clone() {
        if let Ok(s) = editor.syntaxes.get(&ft) {
            let jid = editor
                .job_broker
                .request_cpu(SyntaxParser::new(id, bid, s, ropt, range));

            let (win, buf) = editor.win_buf_mut(id);
            *win.syntax_job() = jid.into();
        }
    }
}
