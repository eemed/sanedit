use sanedit_core::BufferRangeExt as _;

use crate::editor::{hooks::Hook, Editor};

use sanedit_server::ClientId;

use super::jobs::SyntaxParser;
use sanedit_core::RangeUtils;

/// Prevents syntax highlighting flicker on buffer change, simply adjusts
/// higlights to a simple solution, highlights are processed in the
/// background and will override the guesses made here anyway.
#[action("Adjust highlighting to take a buffer change into account")]
pub(crate) fn prevent_flicker(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf(id);
    let bid = buf.id;
    let bid = editor
        .hooks
        .running_hook()
        .map(Hook::buffer_id)
        .flatten()
        .unwrap_or(bid);
    let clients = editor.windows().find_clients_with_buf(bid);

    for client in clients {
        let (win, buf) = editor.win_buf_mut(client);
        let old = win.view_syntax();
        let Some(edit) = buf.last_edit() else {
            return;
        };

        let mut off = 0i128;
        let mut iter = edit.changes.iter().peekable();

        old.spans_mut().retain_mut(|hl| {
            while let Some(next) = iter.peek() {
                if next.end() <= hl.start() {
                    // Before highlight
                    off -= next.range().len() as i128;
                    off += next.text().len() as i128;
                } else if next.start() > hl.end() {
                    // Went past highlight
                    break;
                } else if hl.range().includes(&next.range()) {
                    // Inside a higlight assume the highlight spans this edit too
                    let removed = next.range().len() as i128;
                    let added = next.text().len() as i128;
                    off -= removed;
                    off += added;

                    // counteract this offset
                    hl.add_offset(removed - added);
                    // Extend or shrink instead
                    hl.extend_by(added as u64);
                    hl.shrink_by(removed as u64);
                } else {
                    // When edit is over highlight boundary just remove the higlight
                    return false;
                }

                iter.next();
            }

            hl.add_offset(off);
            true
        });
    }
}

#[action("Parse buffer syntax for view")]
pub(crate) fn reparse_view(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.redraw_view(buf);
    let bid = buf.id;
    let total = buf.total_changes_made();

    let view = win.view().range();
    let old = win.view_syntax();

    if old.buffer_id() != bid
        || old.total_changes_made() != total
        || !old.parsed_range().includes(&view)
    {
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
    let ropt = buf.ro_view();

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
