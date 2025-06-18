use crate::editor::{hooks::Hook, Editor};

use sanedit_server::ClientId;

use super::{jobs::SyntaxParser, ActionResult};

/// Prevents syntax highlighting flicker on buffer change, simply adjusts
/// higlights based on the change, highlights are processed in the
/// background and will override the guesses made here.
#[action("Adjust highlighting to take a buffer change into account")]
pub(crate) fn prevent_flicker(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (_win, buf) = editor.win_buf(id);
    let bid = buf.id;
    let bid = editor
        .hooks
        .running_hook()
        .and_then(Hook::buffer_id)
        .unwrap_or(bid);
    let clients = editor.windows().find_clients_with_buf(bid);

    for client in clients {
        let (win, buf) = editor.win_buf_mut(client);
        let old = win.view_syntax();
        let edit = getf!(buf.last_edit());

        old.spans_mut().retain_mut(|hl| {
            for next in edit.changes.iter() {
                let removed = next.range().len() as i128;
                let added = next.text().len() as i128;
                if next.end() <= hl.start() {
                    // Before highlight
                    hl.add_offset(added - removed);
                } else if next.start() > hl.end() {
                    // Went past highlight
                    break;
                } else if hl.range().includes(&next.range()) {
                    // Inside a higlight assume the highlight spans this edit too
                    // counteract this offset
                    hl.add_offset(removed - added);
                    // Extend or shrink instead
                    hl.extend_by(added as u64);
                    hl.shrink_by(removed as u64);
                } else {
                    // When edit contains highlight just remove the higlight
                    return false;
                }
            }

            true
        });
    }

    ActionResult::Ok
}

#[action("Parse buffer syntax for view")]
pub(crate) fn reparse_view(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (_win, buf) = editor.win_buf_mut(id);
    let bid = buf.id;
    let bid = editor
        .hooks
        .running_hook()
        .and_then(Hook::buffer_id)
        .unwrap_or(bid);
    let clients = editor.windows().find_clients_with_buf(bid);

    for client in clients {
        if !editor.has_syntax(client) {
            continue;
        }

        let (win, buf) = editor.win_buf_mut(client);
        if !win.config.highlight_syntax {
            continue;
        }

        win.redraw_view(buf);
        let bid = buf.id;
        let total = buf.total_changes_made();

        let view = win.view().range();
        let old = win.view_syntax();

        if old.buffer_id() != bid
            || old.total_changes_made() != total
            || !old.parsed_range().includes(&view)
        {
            parse_syntax.execute(editor, client);
        }
    }

    ActionResult::Ok
}

#[action("Parse buffer syntax")]
pub(crate) fn parse_syntax(editor: &mut Editor, id: ClientId) -> ActionResult {
    const JOB_NAME: &str = "parse-syntax";

    let (win, buf) = editor.win_buf_mut(id);
    if !win.config.highlight_syntax {
        return ActionResult::Skipped;
    }
    let bid = buf.id;
    let total_changes_made = buf.total_changes_made();
    let range = win.view().range();
    let ropt = buf.ro_view();

    let lang = getf!(buf.language.clone());
    if let Ok(s) = editor.syntaxes.get(&lang) {
        editor.job_broker.request_slot(
            id,
            JOB_NAME,
            SyntaxParser::new(id, bid, total_changes_made, s, ropt, range),
        );
    }

    ActionResult::Ok
}
