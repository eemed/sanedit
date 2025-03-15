use std::{collections::BTreeMap, sync::Arc};

use sanedit_buffer::Mark;
use sanedit_core::{indent_at_line, BufferRange, Change, Changes, Range};
use sanedit_server::ClientId;

use crate::{
    actions::{hooks::run, window::focus},
    editor::{
        hooks::Hook,
        keymap::KeymapKind,
        snippets::{Snippet, SnippetAtom},
        windows::{Focus, Jump, JumpGroup, Jumps, Prompt},
        Editor,
    },
};

use super::{
    jobs::MatcherJob,
    window::{pop_focus, push_focus_with_keymap},
    ActionResult,
};

#[action("Snippet: Jump to next placeholder")]
pub(crate) fn snippet_jump_next(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    if !win.cursors_to_next_snippet_jump(buf) {
        pop_focus(editor, id);
    } else {
        // Set keymap to snippet if jumped to next
        push_focus_with_keymap(
            editor,
            id,
            Focus::Window,
            KeymapKind::Snippet.as_ref().into(),
        );
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
        .on_confirm(move |editor, id, out| {
            let (win, _buf) = editor.win_buf(id);
            let primary = win.cursors.primary().pos();
            let snippet = get!(out.snippet().cloned());
            insert_snippet_impl(editor, id, snippet, Range::new(primary, primary));
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
) {
    let (win, buf) = editor.win_buf_mut(id);
    let pos = replace.start;
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
            SnippetAtom::Placeholder(n, sel) => {
                placeholders.push((
                    n,
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

    let mut changes = vec![];
    if replace.is_empty() {
        changes.push(Change::insert(pos, text.as_bytes()));
    } else {
        changes.push(Change::replace(replace, text.as_bytes()));
    }
    let changes = Changes::new(&changes);

    // Insert snippet to buffer
    if win.change(buf, &changes).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
    } else {
        return;
    }

    // Convert recorded placeholders to jumps
    let (win, buf) = editor.win_buf_mut(id);
    let mut jumps: BTreeMap<u8, Vec<Jump>> = BTreeMap::new();
    for (n, start, end) in placeholders {
        let smark = buf.mark(start);
        let mut emark: Option<Mark> = None;

        if start != end {
            emark = Some(buf.mark(end));
        }

        let entry = jumps.entry(*n);
        let value = entry.or_default();
        value.push(Jump::new(smark, emark));
    }

    let mut groups = vec![];
    for (_, jumps) in jumps.into_iter().rev() {
        let group = JumpGroup::new(buf.id, jumps);
        groups.push(group);
    }

    let jumps = Jumps::new(groups);
    win.snippets.push(jumps);
    if win.cursors_to_next_snippet_jump(buf) {
        push_focus_with_keymap(
            editor,
            id,
            Focus::Window,
            KeymapKind::Snippet.as_ref().into(),
        );
    }
}
