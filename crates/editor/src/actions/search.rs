use std::{cmp::min, rc::Rc};

use sanedit_buffer::{Searcher, SearcherRev};
use tokio::sync::mpsc::channel;

use crate::{
    // actions::jobs,
    editor::{
        windows::{Focus, Search, SearchDirection},
        Editor,
    },
    server::ClientId,
};

/// setups async job to handle matches within the view range.
fn async_view_matches(editor: &mut Editor, id: ClientId) {
    const CHANNEL_SIZE: usize = 64;
    // let (tx, rx) = channel(CHANNEL_SIZE);

    // let (win, _buf) = editor.win_buf_mut(id);
    // win.search.prompt.on_input = Some(Rc::new(move |editor, id, input| {
    //     let _ = tx.blocking_send(input.into());
    // }));
    // let job = jobs::search(editor, id, rx);
    // editor.jobs.request(job);
}

#[action("Search forward")]
fn forward(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.search = Search::new("Search");
    win.search.prompt.on_confirm = Some(Rc::new(search));
    win.focus = Focus::Search;

    async_view_matches(editor, id);
}

#[action("Search backwards")]
fn backward(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.search = Search::new("BSearch");
    win.search.direction = SearchDirection::Backward;
    win.search.prompt.on_confirm = Some(Rc::new(search));
    win.focus = Focus::Search;

    async_view_matches(editor, id);
}

#[action("Close search")]
fn close(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    if let Some(on_abort) = win.search.prompt.on_abort.clone() {
        let input = win.search.prompt.input_or_selected();
        (on_abort)(editor, id, &input)
    }

    let (win, _buf) = editor.win_buf_mut(id);
    win.focus = Focus::Window;
}

#[action("Find all matches")]
fn confirm_all(_editor: &mut Editor, _id: ClientId) {
    log::info!("Hello world");
}

#[action("Find next match")]
fn confirm(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    if let Some(on_confirm) = win.search.prompt.on_confirm.clone() {
        let input = win.search.prompt.input_or_selected();
        win.search.prompt.history.push(&input);
        (on_confirm)(editor, id, &input)
    }

    let (win, _buf) = editor.win_buf_mut(id);
    win.focus = Focus::Window;
}

#[action("Move cursor one character right")]
fn next_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.search.prompt.next_grapheme();
}

#[action("Move cursor one character left")]
fn prev_grapheme(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.search.prompt.prev_grapheme();
}

#[action("Remove character before cursor")]
fn remove_grapheme_before_cursor(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.search.prompt.remove_grapheme_before_cursor();

    if let Some(on_input) = win.search.prompt.on_input.clone() {
        let input = win.search.prompt.input().to_string();
        (on_input)(editor, id, &input)
    }
}

#[action("Select next history entry")]
fn history_next(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.search.prompt.history_next();
}

#[action("Select previous history entry")]
fn history_prev(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.search.prompt.history_prev();
}

#[action("Clear match highlighting")]
fn clear_matches(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.search.cmatch = None;
}

#[action("Goto next match")]
fn next_match(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    let input = win.search.prompt.input().to_string();
    search(editor, id, &input);
}

#[action("Goto previous match")]
fn prev_match(editor: &mut Editor, id: ClientId) {
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
    let (win, _buf) = editor.win_buf_mut(id);
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

    let (slice, mat, wrap) = match win.search.direction {
        SearchDirection::Forward => {
            let searcher = Searcher::new(input.as_bytes());
            let blen = buf.len();
            let slice = buf.slice(pos..);
            let mut iter = searcher.find_iter(&slice);
            let mat = iter.next();

            // Wrap if no match
            if mat.is_none() {
                let last = min(blen, pos + searcher.pattern_len() - 1);
                let slice = buf.slice(..last);
                let mut iter = searcher.find_iter(&slice);
                let mat = iter.next();
                (slice, mat, true)
            } else {
                (slice, mat, false)
            }
        }
        SearchDirection::Backward => {
            let searcher = SearcherRev::new(input.as_bytes());
            let slice = buf.slice(..pos);
            let mut iter = searcher.find_iter(&slice);
            let mat = iter.next();

            // Wrap if no match
            if mat.is_none() {
                let first = pos.saturating_sub(searcher.pattern_len() - 1);
                let slice = buf.slice(first..);
                let mut iter = searcher.find_iter(&slice);
                let mat = iter.next();
                (slice, mat, true)
            } else {
                (slice, mat, false)
            }
        }
    };

    match mat {
        Some(mut mat) => {
            mat.start += slice.start();
            mat.end += slice.start();

            let cursor = win.primary_cursor_mut();
            cursor.goto(mat.start);
            win.search.cmatch = Some(mat);

            if wrap {
                if win.search.direction == SearchDirection::Forward {
                    win.info_msg("Wrapped to beginning");
                } else {
                    win.info_msg("Wrapped to end");
                }
            }
        }
        None => {
            win.warn_msg("No match found.");
        }
    }
}
