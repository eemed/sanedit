use std::{collections::BTreeMap, sync::Arc};

use sanedit_buffer::Mark;
use sanedit_core::{indent_at_line, Change, Changes};
use sanedit_server::ClientId;

use crate::{
    actions::hooks::run,
    editor::{
        hooks::Hook,
        snippets::{Snippet, SnippetAtom},
        windows::{Focus, Jump, JumpGroup, Jumps, Prompt},
        Editor,
    },
};

use super::jobs::MatcherJob;

#[action("Jump to next snippet placeholders")]
pub(crate) fn snippet_jump_next(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.cursors_to_next_snippet_jump(buf);
}

#[action("Insert a snippet")]
pub(crate) fn insert_snippet(editor: &mut Editor, id: ClientId) {
    const MESSAGE: &str = "Insert a snippet";
    let (win, buf) = win_buf!(editor, id);
    let filetype = buf.filetype.clone();
    let snippets: Vec<String> = filetype
        .as_ref()
        .map(|ft| editor.snippets.all(ft))
        .unwrap_or_else(|| editor.snippets.all_global())
        .into_iter()
        .map(|(name, _snip)| name)
        .collect();
    let job = MatcherJob::builder(id)
        .options(Arc::new(snippets))
        .handler(Prompt::matcher_result_handler)
        .build();
    editor.job_broker.request_slot(id, MESSAGE, job);

    win.prompt = Prompt::builder()
        .prompt(MESSAGE)
        .on_confirm(move |editor, id, input| {
            let snippet = filetype
                .as_ref()
                .map(|ft| editor.snippets.get_snippet(ft, input))
                .unwrap_or_else(|| editor.snippets.get_global_snippet(input))
                .cloned();

            match snippet {
                Some(snip) => insert_snippet_impl(editor, id, snip),
                _ => log::error!("No snippet with name {input}"),
            }
        })
        .build();
    win.focus_to(Focus::Prompt);
}

pub(crate) fn expand_snippet(editor: &mut Editor, id: ClientId, name: &str) {
    let (_win, buf) = win_buf!(editor, id);
    let snippet = buf
        .filetype
        .as_ref()
        .map(|ft| editor.snippets.get_snippet(ft, name))
        .unwrap_or_else(|| editor.snippets.get_global_snippet(name))
        .cloned();

    match snippet {
        Some(snip) => insert_snippet_impl(editor, id, snip),
        _ => log::error!("No snippet with name {name}"),
    }
}

pub(crate) fn insert_snippet_impl(editor: &mut Editor, id: ClientId, snippet: Snippet) {
    let (win, buf) = editor.win_buf_mut(id);
    let pos = win.cursors.primary().pos();
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

    let change = Change::insert(pos, text.as_bytes());
    let changes = Changes::new(&[change]);

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
    win.cursors_to_next_snippet_jump(buf);
}
