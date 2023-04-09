use sanedit_regex::Regex;

use crate::{
    editor::{
        windows::{Prompt, PromptAction, Search},
        Editor,
    },
    server::ClientId,
};

pub(crate) fn search_open(editor: &mut Editor, id: ClientId) {
    let on_confirm: PromptAction = Box::new(move |editor, id, input| {
        let (win, buf) = editor.get_win_buf_mut(id);
        let regex = Regex::new(input);
        let mut cursor = buf.cursor();

        if let Some(m) = regex.find(&mut cursor) {
            let start = m.start();
            let end = m.end();
            let slice = buf.slice(start..end);
            let mtch = format!("{start}..{end} -- '{}'", String::from(&slice));
            let captures: Vec<String> = m
                .captures()
                .iter()
                .map(|cap| {
                    let start = cap.start();
                    let end = cap.end();
                    let slice = buf.slice(start..end);
                    format!("{start}..{end} -- '{}'", String::from(&slice))
                })
                .collect();
            log::info!("Search: match {mtch}, captures {captures:?}");
        } else {
            log::info!("Search: no match");
        }
    });
    let on_abort: PromptAction = Box::new(move |editor, id, input| {});
    let search = Search::new("Search")
        .on_confirm(on_confirm)
        .on_abort(on_abort);
    let (win, buf) = editor.get_win_buf_mut(id);
    win.open_search(search);
}

pub(crate) fn search_close(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    if let Some(search) = win.close_search() {
        let prompt: Prompt = search.into();
        prompt.abort(editor, id);
    }
}

pub(crate) fn search_confirm(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    if let Some(search) = win.close_search() {
        let prompt: Prompt = search.into();
        prompt.confirm(editor, id);
    }
}

pub(crate) fn search_next_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    if let Some(search) = win.search() {
        search.prompt_mut().next_grapheme();
    }
}

pub(crate) fn search_prev_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    if let Some(search) = win.search() {
        search.prompt_mut().prev_grapheme();
    }
}

pub(crate) fn search_remove_grapheme_after_cursor(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.get_win_buf_mut(id);
    if let Some(search) = win.search() {
        search.prompt_mut().remove_grapheme_after_cursor();
    }
}
