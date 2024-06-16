use std::cmp::min;

use sanedit_buffer::{Searcher, SearcherRev};

use crate::{
    actions::jobs,
    editor::{
        windows::{Focus, HistoryKind, Prompt, SearchDirection},
        Editor,
    },
    server::ClientId,
};

/// setups async job to handle matches within the view range.
fn async_view_matches(editor: &mut Editor, id: ClientId, term: &str) {
    let (win, buf) = editor.win_buf_mut(id);
    let pt = buf.read_only_copy();
    let view = win.view().range();
    let job = jobs::Search::forward(id, term, pt, view);
    editor.job_broker.request(job);
}

#[action("Search forward")]
fn forward(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.search.direction = SearchDirection::Forward;
    win.prompt = Prompt::builder()
        .prompt("Search")
        .history(HistoryKind::Search)
        .on_confirm(search)
        .on_input(async_view_matches)
        .build();
    win.focus = Focus::Search;
}

#[action("Search backwards")]
fn backward(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.search.direction = SearchDirection::Backward;
    win.prompt = Prompt::builder()
        .history(HistoryKind::Search)
        .prompt("Backward search")
        .on_confirm(search)
        .on_input(async_view_matches)
        .build();
    win.focus = Focus::Search;
}

#[action("Find all matches")]
fn confirm_all(_editor: &mut Editor, _id: ClientId) {
    log::info!("Hello world");
}

#[action("Clear match highlighting")]
fn clear_matches(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.search.cmatch = None;
    win.search.hl_matches.clear();
}

#[action("Goto next match")]
fn next_match(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    let input = win.search.last_search.clone();
    search(editor, id, &input);
}

#[action("Goto previous match")]
fn prev_match(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    let input = win.search.last_search.clone();

    // search to opposite direction
    let dir = &mut win.search.direction;
    *dir = dir.reverse();

    search(editor, id, &input);

    let (win, _buf) = editor.win_buf_mut(id);
    let dir = &mut win.search.direction;
    *dir = dir.reverse();
}

fn search(editor: &mut Editor, id: ClientId, input: &str) {
    let (win, _buf) = editor.win_buf_mut(id);
    let cpos = win.cursors.primary().pos();
    win.search.last_search = input.into();

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

            win.view_to_cursor(buf);
        }
        None => {
            win.warn_msg("No match found.");
        }
    }
}
