use std::collections::BTreeMap;

use sanedit_buffer::Mark;
use sanedit_core::{indent_at_line, Change, Changes};
use sanedit_server::ClientId;

use crate::{
    actions::hooks::run,
    editor::{
        hooks::Hook,
        windows::{Jump, JumpGroup, Jumps, Snippet, SnippetAtom},
        Editor,
    },
};

#[action("Jump to next snippet placeholders")]
pub(crate) fn snippet_jump_next(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.cursors_to_next_snippet_jump(buf);
}

#[action("Test snippets")]
pub(crate) fn test_snippet(editor: &mut Editor, id: ClientId) {
    let text = "line 1\\n\\tline2 $0\\nline3 ${3:shitter}\\nline4 ${3:worse}";
    let snippet = Snippet::new(text);
    if let Err(e) = snippet.as_ref() {
        log::error!("Failed to create snippet: {e}");
    }
    insert_snippet(editor, id, snippet.unwrap());
}

pub(crate) fn insert_snippet(editor: &mut Editor, id: ClientId, snippet: Snippet) {
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

    if win.change(buf, &changes).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
    } else {
        return;
    }

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
