mod completion;
mod cursors;
mod focus;
mod locations;
mod options;
mod prompt;
mod search;
mod selector;
mod view;

#[cfg(test)]
mod tests;

use std::{
    cmp::{max, min},
    mem,
};

use rustc_hash::FxHashSet;
use sanedit_messages::redraw::{Severity, Size, StatusMessage};

use crate::{
    common::{
        char::DisplayOptions,
        indent::{indent_at_line, indent_at_pos},
        movement,
        text::{as_lines, selection_line_starts, to_line},
    },
    editor::{
        buffers::{Buffer, BufferId, SnapshotData, SortedRanges},
        clipboard::{Clipboard, DefaultClipboard},
        keymap::{DefaultKeyMappings, KeyMappings, Keymap},
        syntax::SyntaxParseResult,
    },
};

pub(crate) use self::{
    completion::Completion,
    cursors::{Cursor, Cursors},
    focus::Focus,
    options::Options,
    prompt::Prompt,
    search::Search,
    search::SearchDirection,
    selector::*,
    view::{Cell, View},
};

#[derive(Debug)]
pub(crate) struct Window {
    bid: BufferId,
    prev_buf_data: Option<(BufferId, SnapshotData)>,
    message: Option<StatusMessage>,
    keymap: Keymap,
    view: View,
    pub clipboard: Box<dyn Clipboard>,

    pub completion: Completion,
    pub cursors: Cursors,
    pub focus: Focus,
    pub search: Search,
    pub prompt: Prompt,
    pub options: Options,
}

impl Window {
    pub fn new(bid: BufferId, width: usize, height: usize) -> Window {
        Window {
            bid,
            prev_buf_data: None,
            view: View::new(width, height),
            message: None,
            clipboard: DefaultClipboard::new(),
            completion: Completion::default(),
            cursors: Cursors::default(),
            keymap: DefaultKeyMappings::window(),
            options: Options::default(),
            search: Search::default(),
            prompt: Prompt::default(),
            focus: Focus::Window,
        }
    }

    pub fn focus(&self) -> Focus {
        self.focus
    }

    pub fn reload(&mut self) {
        self.focus = Focus::Window;
        self.view.set_offset(0);
        self.view.invalidate();
        self.cursors = Cursors::default();
        self.search = Search::default();
        self.prompt = Prompt::default();
        self.message = None;
        self.clipboard = DefaultClipboard::new();
        self.completion = Completion::default();
    }

    pub fn display_options_mut(&mut self) -> &mut DisplayOptions {
        &mut self.view.options
    }

    pub fn display_options(&self) -> &DisplayOptions {
        &self.view.options
    }

    pub fn open_buffer(&mut self, bid: BufferId) -> BufferId {
        let old = self.bid;
        let odata = self.create_snapshot_data();
        self.prev_buf_data = Some((old, odata));
        self.bid = bid;
        self.reload();
        old
    }

    pub fn goto_prev_buffer(&mut self) -> bool {
        match mem::take(&mut self.prev_buf_data) {
            Some((pbid, pdata)) => {
                let old = self.bid;
                let odata = self.create_snapshot_data();
                self.prev_buf_data = Some((old, odata));

                self.bid = pbid;
                self.restore(pdata);
                true
            }
            None => false,
        }
    }

    pub fn info_msg(&mut self, message: &str) {
        self.message = Some(StatusMessage {
            severity: Severity::Info,
            message: message.into(),
        });
    }

    pub fn warn_msg(&mut self, message: &str) {
        self.message = Some(StatusMessage {
            severity: Severity::Warn,
            message: message.into(),
        });
    }

    pub fn error_msg(&mut self, message: &str) {
        self.message = Some(StatusMessage {
            severity: Severity::Error,
            message: message.into(),
        });
    }

    pub fn clear_msg(&mut self) {
        self.message = None;
    }

    pub fn primary_cursor(&self) -> &Cursor {
        self.cursors.primary()
    }

    pub fn primary_cursor_mut(&mut self) -> &mut Cursor {
        self.cursors.primary_mut()
    }

    pub fn cursors(&self) -> &Cursors {
        &self.cursors
    }

    pub fn scroll_down_n(&mut self, buf: &Buffer, n: usize) {
        debug_assert!(
            buf.id == self.bid,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            self.bid
        );

        self.view.scroll_down_n(buf, n);
        self.view.redraw(buf);
    }

    pub fn scroll_up_n(&mut self, buf: &Buffer, n: usize) {
        debug_assert!(
            buf.id == self.bid,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            self.bid
        );
        self.view.scroll_up_n(buf, n);
        self.view.redraw(buf);
    }

    /// sets window offset so that primary cursor is visible in the drawn view.
    pub fn view_to_cursor(&mut self, buf: &Buffer) {
        debug_assert!(
            buf.id == self.bid,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            self.bid
        );
        let cursor = self.primary_cursor().pos();
        self.view.view_to(cursor, buf);
    }

    /// Move primary cursor to line and the view
    pub fn goto_line(&mut self, line: usize, buf: &Buffer) {
        let slice = buf.slice(..);
        let mut lines = slice.lines();
        for _ in 1..max(line, 1) {
            lines.next();
        }

        let offset = lines.next().map(|line| line.start()).unwrap_or(buf.len());
        self.goto_offset(offset, buf);
    }

    /// Move primary cursor to offset and the view too
    pub fn goto_offset(&mut self, offset: usize, buf: &Buffer) {
        let offset = min(offset, buf.len());
        let primary = self.cursors.primary_mut();
        primary.goto(offset);

        self.ensure_cursor_on_grapheme_boundary(buf);
        self.view_to_cursor(buf);
    }

    pub fn ensure_cursor_on_grapheme_boundary(&mut self, buf: &Buffer) {
        // Ensure cursor in buf range
        self.cursors.shrink_cursor_to_range(0..buf.len());

        // Ensure cursor in buf grapheme boundary
        let primary = self.cursors.primary_mut();
        let ppos = primary.pos();
        let slice = buf.slice(..);
        let mut graphemes = slice.graphemes_at(ppos);
        let npos = graphemes
            .next()
            .map(|slice| slice.start())
            .unwrap_or(buf.len());
        if ppos != npos {
            primary.goto(npos);
            self.view.invalidate();
        }
    }

    /// Called when buffer is changed in the background and we should correct
    /// this window.
    pub fn on_buffer_changed(&mut self, buf: &Buffer) {
        // Remove cursors
        self.cursors.remove_secondary_cursors();
        self.cursors.primary_mut().unanchor();

        self.ensure_cursor_on_grapheme_boundary(buf);

        // Redraw view
        self.view.invalidate();
        self.view.redraw(buf);
    }

    pub fn buffer_id(&self) -> BufferId {
        self.bid
    }

    pub fn prev_buffer_id(&self) -> Option<BufferId> {
        self.prev_buf_data.as_ref().map(|(a, _)| a).copied()
    }

    pub fn view(&self) -> &View {
        &self.view
    }

    pub fn invalidate(&mut self) {
        self.view.invalidate();
    }

    pub fn resize(&mut self, size: Size, buf: &Buffer) {
        debug_assert!(
            buf.id == self.bid,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            self.bid
        );
        self.view.resize(size);
        self.view_to_cursor(buf);
    }

    pub fn message(&self) -> Option<&StatusMessage> {
        self.message.as_ref()
    }

    pub fn redraw_view(&mut self, buf: &Buffer) {
        debug_assert!(
            buf.id == self.bid,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            self.bid
        );

        self.cursors.shrink_cursor_to_range(0..buf.len());
        // let primary_pos = self.cursors.primary().pos();
        self.view.redraw(buf);
        // self.view.view_to(primary_pos, buf);
    }

    pub fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    /// Return the currently focused elements keymap
    pub fn focus_keymap(&self) -> &Keymap {
        match self.focus {
            Focus::Search => &self.search.prompt.keymap,
            Focus::Prompt => &self.prompt.keymap,
            Focus::Window => &self.keymap,
            Focus::Completion => &self.completion.keymap,
        }
    }

    fn create_snapshot_data(&self) -> SnapshotData {
        SnapshotData {
            cursors: self.cursors.clone(),
            view_offset: self.view.start(),
        }
    }

    fn remove(&mut self, buf: &mut Buffer, ranges: &SortedRanges) {
        let change = buf.remove_multi(ranges);
        if let Some(id) = change.created_snapshot {
            buf.store_snapshot_data(id, self.create_snapshot_data());
        }
    }

    fn insert(&mut self, buf: &mut Buffer, positions: &[usize], text: &str) {
        let change = buf.insert_multi(positions, text);
        if let Some(id) = change.created_snapshot {
            buf.store_snapshot_data(id, self.create_snapshot_data());
        }
    }

    fn remove_cursor_selections(&mut self, buf: &mut Buffer) -> bool {
        let selections: SortedRanges = (&self.cursors).into();
        if selections.is_empty() {
            return false;
        }

        self.remove(buf, &selections);

        let mut removed = 0;
        for cursor in self.cursors.cursors_mut() {
            let sel = cursor.take_selection();
            let cpos = sel
                .as_ref()
                .map(|range| range.start)
                .unwrap_or(cursor.pos());
            cursor.goto(cpos - removed);

            if let Some(sel) = sel {
                // sel.start -= removed;
                // sel.end -= removed;

                removed += sel.len();
            }
        }

        self.invalidate();
        true
    }

    pub fn copy_to_clipboard(&mut self, buf: &Buffer) {
        let mut lines = vec![];
        for cursor in self.cursors.cursors_mut() {
            if let Some(sel) = cursor.selection() {
                let text = String::from(&buf.slice(sel));
                lines.push(text);
                cursor.unanchor();
            }
        }

        let line = to_line(lines, buf.options.eol);
        self.clipboard.copy(&line);
    }

    pub fn insert_to_each_cursor(&mut self, buf: &mut Buffer, texts: Vec<String>) {
        debug_assert!(
            texts.len() == self.cursors.len(),
            "Cursors {} and texts {} count mismatch",
            self.cursors.len(),
            texts.len()
        );
        // TODO make this one change instead of many

        self.remove_cursor_selections(buf);

        let mut inserted = 0;
        for (i, cursor) in self.cursors.cursors_mut().iter_mut().enumerate() {
            let text = &texts[i];
            let cpos = cursor.pos() + inserted;
            buf.insert(cpos, text);
            cursor.goto(cpos + text.len());
            inserted += text.len();
        }

        self.invalidate();
    }

    pub fn paste_from_clipboard(&mut self, buf: &mut Buffer) {
        if let Ok(text) = self.clipboard.paste() {
            let lines = as_lines(text.as_str());
            let clen = self.cursors.cursors().len();
            let llen = lines.len();

            if clen == llen {
                self.insert_to_each_cursor(buf, lines);
            } else {
                self.insert_at_cursors(buf, &text);
            }
        }
    }

    pub fn insert_at_cursors(&mut self, buf: &mut Buffer, text: &str) {
        // TODO use a replace operation instead if removing
        self.remove_cursor_selections(buf);
        let positions: Vec<usize> = (&self.cursors).into();
        self.insert(buf, &positions, text);

        let mut inserted = 0;
        for cursor in self.cursors.cursors_mut() {
            let cpos = cursor.pos() + inserted;
            cursor.goto(cpos + text.len());
            inserted += text.len();
        }

        self.invalidate();
        self.view_to_cursor(buf);
    }

    pub fn remove_grapheme_after_cursors(&mut self, buf: &mut Buffer) {
        if self.remove_cursor_selections(buf) {
            return;
        }

        let mut removed = 0;
        for cursor in self.cursors.cursors_mut() {
            let cpos = cursor.pos() - removed;
            let pos = movement::next_grapheme_boundary(&buf.slice(..), cpos);

            cursor.goto(cpos);
            buf.remove(cpos..pos);
            removed += pos - cpos;
        }

        self.invalidate();
        self.view_to_cursor(buf);
    }

    pub fn undo(&mut self, buf: &mut Buffer) -> bool {
        match buf.undo() {
            Ok(change) => {
                let created = change.created_snapshot;
                let restored = change.restored_snapshot;

                if let Some(id) = created {
                    buf.store_snapshot_data(id, self.create_snapshot_data());
                }

                if let Some(restored) = restored {
                    if let Some(data) = buf.snapshot_data(restored) {
                        self.restore(data);
                    } else {
                        self.reload();
                    }
                }

                self.invalidate();
                true
            }
            Err(msg) => {
                self.warn_msg(msg);
                false
            }
        }
    }

    fn restore(&mut self, sdata: SnapshotData) {
        self.cursors = sdata.cursors.clone();
        self.view.set_offset(sdata.view_offset);
        self.invalidate();
    }

    pub fn redo(&mut self, buf: &mut Buffer) -> bool {
        match buf.redo() {
            Ok(change) => {
                let created = change.created_snapshot;
                let restored = change.restored_snapshot;
                if let Some(id) = created {
                    buf.store_snapshot_data(id, self.create_snapshot_data());
                }

                if let Some(restored) = restored {
                    if let Some(data) = buf.snapshot_data(restored) {
                        self.restore(data);
                    } else {
                        self.reload()
                    }
                }

                self.invalidate();
                true
            }
            Err(msg) => {
                self.warn_msg(msg);
                false
            }
        }
    }

    /// Remove a grapheme before the cursor, if at indentation
    /// remove a block of it
    pub fn remove_grapheme_before_cursors(&mut self, buf: &mut Buffer) {
        if self.remove_cursor_selections(buf) {
            return;
        }

        let ranges: SortedRanges = {
            let mut ranges = vec![];

            for cursor in self.cursors.cursors_mut() {
                let cpos = cursor.pos();
                let pos = movement::prev_grapheme_boundary(&buf.slice(..), cpos);
                ranges.push(pos..cpos);
            }

            ranges.into()
        };

        self.remove(buf, &ranges);

        let mut removed = 0;
        for (i, range) in ranges.iter().enumerate() {
            let cursor = &mut self.cursors.cursors_mut()[i];
            cursor.goto(range.start - removed);
            removed += range.len();
        }

        self.invalidate();
        self.view_to_cursor(buf);
    }

    pub fn set_keymap(mappings: impl KeyMappings) {
        todo!()
    }

    pub fn syntax_result(&mut self) -> &mut SyntaxParseResult {
        &mut self.view.syntax
    }

    /// Insert a newline to each cursor
    /// if originating line was indented also preserve the indentation
    pub fn insert_newline(&mut self, buf: &mut Buffer) {
        // 1. Calculate indents
        // 2. insert newlines + indent combo to each cursor
        let eol = buf.options.eol;
        let slice = buf.slice(..);
        let texts: Vec<String> = self
            .cursors()
            .iter()
            .map(|c| {
                let indent = indent_at_line(&slice, c.pos());
                format!("{}{}", eol.as_str(), indent.to_string())
            })
            .collect();

        self.insert_to_each_cursor(buf, texts);
    }

    /// Indent all the lines with cursors or their selections
    pub fn indent_cursor_lines(&mut self, buf: &mut Buffer) {
        let starts: Vec<usize> = {
            let mut starts = FxHashSet::default();

            for cursor in self.cursors.iter() {
                let range = cursor.selection().unwrap_or(cursor.pos()..cursor.pos() + 1);
                let cstarts = selection_line_starts(buf, range);
                starts.extend(cstarts);
            }
            let mut vstarts: Vec<usize> = starts.into_iter().collect();
            vstarts.sort();
            vstarts
        };

        let indent = buf.options.indent.to_string();
        buf.insert_multi(&starts, &indent);

        for cursor in self.cursors.cursors_mut() {
            let mut range = cursor.selection().unwrap_or(cursor.pos()..cursor.pos() + 1);
            let pre = starts.iter().take_while(|cur| **cur < range.start).count();
            let count = starts[pre..]
                .iter()
                .take_while(|cur| range.contains(cur))
                .count();
            let plen = pre * indent.len();
            let len = count * indent.len();
            range.start += plen;
            range.end += plen + len;
            log::info!("cursor: {cursor:?}, start: {starts:?}, pre: {pre}, plen: {plen}, count: {count}, len: {len}, range: {range:?}");
            cursor.to_range(&range);
        }

        self.invalidate();
    }

    /// Dedent all the lines with cursors or their selections
    pub fn dedent_cursor_lines(&mut self, buf: &mut Buffer) {
        todo!()
    }

    /// Insert a tab character
    /// If cursor is at indentation, add an indentation block instead
    pub fn insert_tab(&mut self, buf: &mut Buffer) {
        let slice = buf.slice(..);
        let texts: Vec<String> = self
            .cursors()
            .iter()
            .map(|c| {
                let indmul = buf.options.indent.n;

                match indent_at_pos(&slice, c.pos()) {
                    Some(mut indent) => {
                        indent.n = indent.indent_to_multiple_of(indmul);
                        indent.to_string()
                    }
                    None => String::from("\t"),
                }
            })
            .collect();
        self.insert_to_each_cursor(buf, texts);
    }

    /// If cursor is at indentation, try to dedent the line
    pub fn backtab(&mut self, buf: &mut Buffer) {
        let slice = buf.slice(..);
        let indmul = buf.options.indent.n;
        let ranges: SortedRanges = {
            let mut ranges = vec![];
            for cursor in self.cursors.iter() {
                let pos = cursor.pos();
                if let Some(at) = indent_at_pos(&slice, pos) {
                    let n = at.dedent_to_multiple_of(indmul);
                    // At start of line
                    if n == 0 {
                        continue;
                    }

                    let small = pos.saturating_sub(n);
                    ranges.push(small..pos);
                }
            }
            ranges.into()
        };

        self.remove(buf, &ranges);

        let mut removed = 0;
        for (i, range) in ranges.iter().enumerate() {
            let cursor = &mut self.cursors.cursors_mut()[i];
            cursor.goto(range.start - removed);
            removed += range.len();
        }

        self.invalidate();
    }
}
