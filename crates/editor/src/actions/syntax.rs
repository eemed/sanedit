use crate::{
    editor::{syntax::SyntaxParseResult, Editor},
    server::ClientId,
};

use super::jobs::SyntaxParser;

#[action("Parse buffer syntax")]
pub(crate) fn parse_syntax(editor: &mut Editor, id: ClientId) {
    const JOB_NAME: &str = "parse-syntax";
    let (win, _buf) = editor.win_buf_mut(id);

    // Clear syntax
    *win.syntax_result() = SyntaxParseResult::default();

    let (win, buf) = editor.win_buf_mut(id);
    let bid = buf.id;
    let range = win.view().range();
    let ropt = buf.read_only_copy();

    if let Some(ft) = buf.filetype.clone() {
        match editor.syntaxes.get(&ft) {
            Ok(s) => {
                editor.job_broker.request_slot(
                    id,
                    JOB_NAME,
                    SyntaxParser::new(id, bid, s, ropt, range),
                );
            }
            Err(e) => log::error!("Failed to load syntax for {}: {e}", ft.as_str()),
        }
    }
}
