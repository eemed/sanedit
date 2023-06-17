use std::rc::Rc;

use sanedit_buffer::{Searcher, SearcherRev};

use crate::{
    editor::{
        windows::{Focus, Search, SearchDirection},
        Editor,
    },
    server::ClientId,
};

pub(crate) fn search_forward(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.search = Search::new("Search");
    win.search.prompt.on_confirm = Some(Rc::new(search));
    win.focus = Focus::Search;
}

pub(crate) fn search_backward(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    win.search = Search::new("BSearch");
    win.search.direction = SearchDirection::Backward;
    win.search.prompt.on_confirm = Some(Rc::new(search));
    win.focus = Focus::Search;
}

pub(crate) fn search_close(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    if let Some(on_abort) = win.search.prompt.on_abort.clone() {
        let input = win.search.prompt.input_or_selected();
        (on_abort)(editor, id, &input)
    }

    let (win, buf) = editor.win_buf_mut(id);
    win.focus = Focus::Window;
}

pub(crate) fn search_confirm_all(editor: &mut Editor, id: ClientId) {
    log::info!("Hello world");
}

pub(crate) fn search_confirm(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    if let Some(on_confirm) = win.search.prompt.on_confirm.clone() {
        let input = win.search.prompt.input_or_selected();
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
        let input = win.search.prompt.input().to_string();
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
    win.search.cmatch = None;
}

pub(crate) fn search_next_match(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    let input = win.search.prompt.input().to_string();
    search(editor, id, &input);
}

pub(crate) fn search_prev_match(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    let input = win.search.prompt.input().to_string();

    // search to opposite direction
    let dir = &mut win.search.direction;
    *dir = dir.opposite();

    search(editor, id, &input);

    let (win, _buf) = editor.win_buf_mut(id);
    let dir = &mut win.search.direction;
    *dir = dir.opposite();
}

fn search(editor: &mut Editor, id: ClientId, input: &str) {
    let (win, buf) = editor.win_buf_mut(id);
    let cpos = win.cursors.primary().pos();

    search_impl(editor, id, input, cpos);
}

fn search_impl(editor: &mut Editor, id: ClientId, input: &str, mut pos: usize) {
    let (win, buf) = editor.win_buf_mut(id);
    if input.is_empty() {
        win.search.cmatch = None;
        return;
    }

    // If previous match move to the appropriate position
    if let Some(ref cmat) = win.search.cmatch {
        if cmat.contains(&pos) {
            match win.search.direction {
                SearchDirection::Backward => pos = cmat.start,
                SearchDirection::Forward => pos = cmat.end,
            }
        }
    }

    let (slice, mat) = match win.search.direction {
        SearchDirection::Forward => {
            let searcher = Searcher::new(input.as_bytes());
            let slice = buf.slice(pos..);
            let mut iter = searcher.find_iter(&slice);
            let mat = iter.next();
            (slice, mat)
        }
        SearchDirection::Backward => {
            let searcher = SearcherRev::new(input.as_bytes());
            let slice = buf.slice(..pos);
            let mut iter = searcher.find_iter(&slice);
            let mat = iter.next();
            (slice, mat)
        }
    };

    match mat {
        Some(mut mat) => {
            mat.start += slice.start();
            mat.end += slice.start();

            log::info!("Match: {mat:?}");
            let cursor = win.primary_cursor_mut();
            cursor.goto(mat.start);
            win.search.cmatch = Some(mat);
        }
        None => {
            win.warn_msg("No match found.");
        }
    }
}
