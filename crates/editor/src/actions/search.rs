use std::rc::Rc;

use sanedit_regex::Regex;

use crate::{
    editor::{
        buffers::Buffer,
        windows::{Focus, PAction, Search, SetPrompt, SetSearch, Window},
        Editor,
    },
    server::ClientId,
};

pub(crate) fn search_open(editor: &mut Editor, id: ClientId) {
    let on_confirm: PAction = Rc::new(move |editor, id, input| {});
    let on_abort: PAction = Rc::new(move |editor, id, input| {
        let (win, buf) = editor.win_buf_mut(id);
        win.search.matches.clear();
    });
    let set = SetSearch {
        prompt: SetPrompt {
            message: "Search".into(),
            on_confirm: Some(on_confirm),
            on_abort: Some(on_abort),
            on_input: Some(Rc::new(search)),
            keymap: None,
        },
        is_regex: false,
        select: false,
        stop_at_first_match: true,
    };
    let (win, buf) = editor.win_buf_mut(id);
    win.search.set(set);
    win.search.prompt.message = format_search_msg(&win.search);
    win.focus = Focus::Search;
}

pub(crate) fn search_close(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    if let Some(on_abort) = win.search.prompt.on_abort.clone() {
        let input = win.search.prompt.input();
        (on_abort)(editor, id, &input)
    }

    let (win, buf) = editor.win_buf_mut(id);
    win.focus = Focus::Window;
}

pub(crate) fn search_confirm_all(editor: &mut Editor, id: ClientId) {
}

pub(crate) fn search_confirm(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    if let Some(on_confirm) = win.search.prompt.on_confirm.clone() {
        let input = win.search.prompt.input();
        win.search.prompt.history.push(&input);
        (on_confirm)(editor, id, &input)
    }

    let (win, buf) = editor.win_buf_mut(id);
    win.focus = Focus::Window;
}

pub(crate) fn search_next_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.search.prompt.next_grapheme();
}

pub(crate) fn search_prev_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.search.prompt.prev_grapheme();
}

pub(crate) fn search_remove_grapheme_before_cursor(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.search.prompt.remove_grapheme_before_cursor();

    if let Some(on_input) = win.search.prompt.on_input.clone() {
        let input = win.search.prompt.input();
        (on_input)(editor, id, &input)
    }
}

pub(crate) fn search_history_next(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.search.prompt.history_next();
}

pub(crate) fn search_history_prev(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.search.prompt.history_prev();
}

pub(crate) fn search_clear_matches(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.search.matches.clear();
}

pub(crate) fn search_toggle_regex(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.search.is_regex = !win.search.is_regex;
    win.search.prompt.message = format_search_msg(&win.search);
}

pub(crate) fn search_toggle_select(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.search.select = !win.search.select;
    win.search.prompt.message = format_search_msg(&win.search);
}

pub(crate) fn search_toggle_match_all(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.search.stop_at_first_match = !win.search.stop_at_first_match;
    win.search.prompt.message = format_search_msg(&win.search);
}

fn format_search_msg(search: &Search) -> String {
    let mut flags = Vec::new();
    if search.is_regex {
        flags.push("r");
    }

    if search.select {
        flags.push("s");
    }

    if !search.stop_at_first_match {
        flags.push("a");
    }

    if !flags.is_empty() {
        format!("Search ({})", flags.join(", "))
    } else {
        format!("Search")
    }
}

fn search(editor: &mut Editor, id: ClientId, input: &str) {
    let (win, buf) = editor.win_buf_mut(id);
    search_impl(win, buf, input);
}

fn search_impl(win: &mut Window, buf: &Buffer, input: &str) {
    let regex = if win.search.is_regex {
        if let Ok(regex) = Regex::new(input) {
            regex
        } else {
            log::info!("invalid regex");
            return;
        }
    } else {
        Regex::new_literal(input)
    };

    regex_search(win, buf, regex);
}

fn regex_search(win: &mut Window, buf: &Buffer, regex: Regex) {
    let mut cursor = buf.cursor();

    if let Some(m) = regex.find(&mut cursor) {
        log::info!("match {m:?}");

        if win.search.select {
            let cursor = win.primary_cursor_mut();
            cursor.unanchor();
            cursor.goto(m.start());
            cursor.anchor();
            cursor.goto(m.end());
        }

        win.search.matches = vec![m];
    } else {
        win.search.matches = vec![];
    }
}
