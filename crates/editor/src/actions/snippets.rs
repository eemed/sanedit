use std::sync::Arc;

use sanedit_buffer::Mark;
use sanedit_core::{indent_at_line, BufferRange, Change, Changes, Range};
use sanedit_server::ClientId;

use crate::{
    actions::{hooks::run, window::focus},
    editor::{
        hooks::Hook,
        snippets::{Snippet, SnippetAtom},
        windows::{Focus, Jump, JumpGroup, Jumps, Prompt},
        Editor,
    },
};

use super::{jobs::MatcherJob, window::mode_insert, ActionResult};

#[action("Snippet: Jump to next placeholder")]
pub(crate) fn snippet_jump_next(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    if win.cursors_to_next_snippet_jump(buf) {
        mode_insert(editor, id);
    }
    ActionResult::Ok
}

#[action("Snippet: Insert new")]
pub(crate) fn insert_snippet(editor: &mut Editor, id: ClientId) -> ActionResult {
    const MESSAGE: &str = "Insert a snippet";
    let snippets = editor.get_snippets(id);

    let (win, _buf) = win_buf!(editor, id);
    let job = MatcherJob::builder(id)
        .options(Arc::new(snippets))
        .handler(Prompt::matcher_result_handler)
        .build();
    editor.job_broker.request_slot(id, MESSAGE, job);

    win.prompt = Prompt::builder()
        .prompt(MESSAGE)
        .loads_options()
        .on_confirm(move |editor, id, out| {
            let (win, _buf) = win_buf_ref!(editor, id);
            let primary = win.cursors.primary().pos();
            let snippet = getf!(out.snippet().cloned());
            insert_snippet_impl(editor, id, snippet, Range::from(primary..primary), vec![])
        })
        .build();
    focus(editor, id, Focus::Prompt);
    ActionResult::Ok
}

pub(crate) fn insert_snippet_impl(
    editor: &mut Editor,
    id: ClientId,
    snippet: Snippet,
    replace: BufferRange,
    additional_changes: Vec<Change>,
) -> ActionResult {
    let mut pos = replace.start;

    // Apply additional edits first
    if !additional_changes.is_empty() {
        let (win, buf) = win_buf!(editor, id);
        let changes = Changes::new(&additional_changes);
        if win.change(buf, &changes).is_ok() {
            let hook = Hook::BufChanged(buf.id);
            run(editor, id, hook);
        } else {
            return ActionResult::Failed;
        }
        pos = changes.move_offset(pos);
    }

    let (win, buf) = win_buf!(editor, id);
    let slice = buf.slice(..);
    let indent_line = indent_at_line(&slice, pos);
    let preindent = {
        match indent_line {
            Some((k, n)) => k.repeat(n as usize),
            None => String::new(),
        }
    };
    let kind = buf.config.indent_kind;
    let amount = buf.config.indent_amount;
    let bufindent = kind.repeat(amount as usize);

    // Convert snippet to text and record the placeholder positions
    let mut placeholders = vec![];
    let mut text = String::new();
    for atom in snippet.atoms() {
        match atom {
            SnippetAtom::Text(txt) => text.push_str(txt),
            SnippetAtom::Placeholder(_, sel) => {
                placeholders.push((
                    pos + text.len() as u64,
                    pos + text.len() as u64 + sel.len() as u64,
                ));
                text.push_str(sel);
            }
            SnippetAtom::Newline => {
                text.push_str(buf.config.eol.as_str());
                text.push_str(&preindent);
            }
            SnippetAtom::Indent => text.push_str(&bufindent),
        }
    }

    // Insert the snippet into the buffer
    let mut changes = vec![];
    if replace.is_empty() {
        changes.push(Change::insert(pos, text.as_bytes()));
    } else {
        changes.push(Change::replace(replace, text.as_bytes()));
    }
    let mut changes = Changes::new(&changes);
    if !additional_changes.is_empty() {
        changes.disable_undo_point_creation();
    }

    if win.change(buf, &changes).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
    } else {
        return ActionResult::Failed;
    }

    // Convert recorded placeholders to jumps
    let (win, buf) = win_buf!(editor, id);
    let mut groups = vec![];
    for (start, end) in placeholders {
        let smark = buf.mark(start);
        let mut emark: Option<Mark> = None;

        if start != end {
            emark = Some(buf.mark(end));
        }

        let group = JumpGroup::new(buf.id, vec![Jump::new(smark, emark)]);
        groups.push(group);
    }

    let jumps = Jumps::<32>::from_groups(groups);
    win.snippets.push(jumps);
    if win.cursors_to_next_snippet_jump(buf) {
        mode_insert(editor, id);
        return ActionResult::Ok;
    }

    ActionResult::Failed
}
