use std::rc::Rc;

use sanedit_regex::Regex;

use crate::{
    editor::{
        windows::{Focus, PAction, SetPrompt, SetSearch},
        Editor,
    },
    server::ClientId,
};

pub(crate) fn search_open(editor: &mut Editor, id: ClientId) {
    let on_confirm: PAction = Rc::new(move |editor, id, input| {
        let (win, buf) = editor.win_buf_mut(id);
        if win.search.select() {
            let regex = Regex::new(input);
            let mut cursor = buf.cursor();

            if let Some(m) = regex.find(&mut cursor) {
                let cursor = win.primary_cursor_mut();
                cursor.unanchor();
                cursor.goto(m.start());
                cursor.anchor();
                cursor.goto(m.end());
            }
        }

        // if let Some(m) = regex.find(&mut cursor) {
        //     let start = m.start();
        //     let end = m.end();
        //     let slice = buf.slice(start..end);
        //     let mtch = format!("{start}..{end} -- '{}'", String::from(&slice));
        //     let captures: Vec<String> = m
        //         .captures()
        //         .iter()
        //         .map(|cap| {
        //             let start = cap.start();
        //             let end = cap.end();
        //             let slice = buf.slice(start..end);
        //             format!("{start}..{end} -- '{}'", String::from(&slice))
        //         })
        //         .collect();
        //     win.search.matches = vec![m];
        //     log::info!("Search: match {mtch}, captures {captures:?}");
        //     // let cursor = win.primary_cursor_mut();
        //     // cursor.unanchor();
        //     // cursor.goto(m.start());
        //     // cursor.anchor();
        //     // cursor.goto(m.end());
        // } else {
        //     log::info!("Search: no match");
        // }
    });
    let on_abort: PAction = Rc::new(move |editor, id, input| {
        let (win, buf) = editor.win_buf_mut(id);
        win.search.matches.clear();
    });
    let on_input: PAction = Rc::new(move |editor, id, input| {
        let (win, buf) = editor.win_buf_mut(id);
        let regex = Regex::new(input);
        let mut cursor = buf.cursor();

        log::info!("on input");
        if let Some(m) = regex.find(&mut cursor) {
            log::info!("match {m:?}");
            win.search.matches = vec![m];
        }
    });
    let set = SetSearch {
        prompt: SetPrompt {
            message: "Search".into(),
            on_confirm: Some(on_confirm),
            on_abort: Some(on_abort),
            on_input: Some(on_input),
            keymap: None,
        },
        is_regex: true,
        select: false,
        stop_at_first_match: true,
    };
    let (win, buf) = editor.win_buf_mut(id);
    win.search.set(set);
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
