use crate::{
    editor::{syntax::SyntaxParseResult, Editor},
    server::ClientId,
};

use super::jobs::SyntaxParser;

#[action("Parse buffer syntax")]
pub(crate) fn parse_syntax(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    // Clear syntax
    win.on_syntax_parsed(SyntaxParseResult::default());

    let bid = buf.id;
    let range = win.view().range();
    let ropt = buf.read_only_copy();

    if let Some(ft) = buf.filetype.clone() {
        if let Ok(s) = editor.syntaxes.get(&ft) {
            editor
                .job_broker
                .request(SyntaxParser::new(id, bid, s, ropt, range));
        }
    }
}
