use std::cmp::min;

use crate::{
    actions::jobs,
    common::search::PTSearcher,
    editor::{
        keymap::KeymapKind,
        windows::{Focus, HistoryKind, Prompt, SearchDirection, SearchKind},
        Editor,
    },
    server::ClientId,
};

/// setups async job to handle matches within the view range.
fn async_view_matches(editor: &mut Editor, id: ClientId, pattern: &str) {
    let (win, buf) = editor.win_buf_mut(id);
    let pt = buf.read_only_copy();
    let view = win.view().range();
    let dir = win.search.direction;
    let kind = win.search.kind;

    let job = jobs::Search::new(id, pattern, pt, view, dir, kind);
    editor.job_broker.request(job);
}

#[action("Search forward")]
fn forward(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.search.direction = SearchDirection::Forward;
    win.prompt = Prompt::builder()
        .prompt("Search")
        .history(HistoryKind::Search)
        .keymap(KeymapKind::Search)
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
        .keymap(KeymapKind::Search)
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
    win.search.current_match = None;
    win.search.hl_matches.clear();
}

#[action("Goto next match")]
fn next_match(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    let Some(input) = win.search.last_search_pattern().map(String::from) else {
        return;
    };
    search(editor, id, &input);
}

#[action("Goto previous match")]
fn prev_match(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);

    let Some(input) = win.search.last_search_pattern().map(String::from) else {
        return;
    };
    // search to opposite direction
    let dir = &mut win.search.direction;
    *dir = dir.reverse();

    search(editor, id, &input);

    let (win, _buf) = editor.win_buf_mut(id);
    let dir = &mut win.search.direction;
    *dir = dir.reverse();
}

#[action("Toggle regex search")]
fn toggle_regex(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    win.search.kind = match win.search.kind {
        SearchKind::Regex => SearchKind::Smart,
        _ => SearchKind::Regex,
    };
}

/// Execute a search from primary cursor position
fn search(editor: &mut Editor, id: ClientId, input: &str) {
    let (win, _buf) = editor.win_buf_mut(id);
    let cpos = win.cursors.primary().pos();
    win.search.save_last_search(input.into());

    search_impl(editor, id, input, cpos);
}

/// Execute the search for input at position pos
///
/// The potential match is reported to win.search.current_match
/// If no match is found current match is set to None
///
fn search_impl(editor: &mut Editor, id: ClientId, input: &str, mut pos: usize) {
    let (win, buf) = editor.win_buf_mut(id);
    if input.is_empty() {
        return;
    }

    // If previous match move to the appropriate position
    if let Some(last_match) = win
        .search
        .last_search()
        .map(|ls| ls.current_match.as_ref())
        .flatten()
    {
        if last_match.contains(&pos) {
            match win.search.direction {
                SearchDirection::Backward => pos = last_match.start,
                SearchDirection::Forward => pos = last_match.end,
            }
        }
    }

    let Ok(searcher) = PTSearcher::new(input, win.search.direction, win.search.kind) else {
        return;
    };

    let (start, mat, wrap) = match win.search.direction {
        SearchDirection::Forward => {
            let blen = buf.len();
            let slice = buf.slice(pos..);
            let mut iter = searcher.find_iter(&slice);
            let mat = iter.next();

            // Wrap if no match
            if mat.is_none() {
                let last = min(blen, pos + input.len() - 1);
                let slice = buf.slice(..last);
                let mut iter = searcher.find_iter(&slice);
                let mat = iter.next();
                (slice.start(), mat, true)
            } else {
                (slice.start(), mat, false)
            }
        }
        SearchDirection::Backward => {
            let slice = buf.slice(..pos);
            let mut iter = searcher.find_iter(&slice);
            let mat = iter.next();

            // Wrap if no match
            if mat.is_none() {
                let first = pos.saturating_sub(input.len() - 1);
                let slice = buf.slice(first..);
                let mut iter = searcher.find_iter(&slice);
                let mat = iter.next();
                (slice.start(), mat, true)
            } else {
                (slice.start(), mat, false)
            }
        }
    };

    match mat {
        Some(mut mat) => {
            mat.range.start += start;
            mat.range.end += start;

            let cursor = win.primary_cursor_mut();
            cursor.goto(mat.range.start);

            win.search.current_match = Some(mat.range);

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
            win.search.current_match = None;
            win.warn_msg("No match found.");
        }
    }
}
