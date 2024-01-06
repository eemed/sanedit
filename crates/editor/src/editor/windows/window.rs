mod completion;
mod cursors;
mod focus;
mod locations;
mod options;
mod prompt;
mod search;
mod selector;
mod view;

use std::ops::Range;

use sanedit_buffer::SortedPositions;
use sanedit_messages::redraw::{Severity, Size, StatusMessage};

use crate::{
    common::{
        char::DisplayOptions,
        movement::{self, prev_grapheme_boundary},
        text::{as_lines, to_line},
    },
    editor::{
        buffers::{Buffer, BufferId, ChangeKind, SnapshotData, SortedRanges},
        clipboard::{Clipboard, DefaultClipboard},
        keymap::{DefaultKeyMappings, KeyMappings, Keymap},
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
    buf: BufferId,
    view: View,
    message: Option<StatusMessage>,
    keymap: Keymap,
    pub clipboard: Box<dyn Clipboard>,

    pub completion: Completion,
    pub cursors: Cursors,
    pub focus: Focus,
    pub search: Search,
    pub prompt: Prompt,
    pub options: Options,
}

impl Window {
    pub fn new(buf: BufferId, width: usize, height: usize) -> Window {
        Window {
            buf,
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

    pub fn display_options(&self) -> &DisplayOptions {
        &self.view.options
    }

    pub fn open_buffer(&mut self, bid: BufferId) -> BufferId {
        let old = self.buf;
        let width = self.view.width();
        let height = self.view.height();
        *self = Window::new(bid, width, height);
        old
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
            buf.id == self.buf,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            self.buf
        );

        self.view.scroll_down_n(buf, n);
        self.view.redraw(buf);

        let primary = self.cursors.primary_mut();
        let Range { start, .. } = self.view.range();
        if primary.pos() < start {
            primary.goto(start);
        }
    }

    pub fn scroll_up_n(&mut self, buf: &Buffer, n: usize) {
        debug_assert!(
            buf.id == self.buf,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            self.buf
        );
        self.view.scroll_up_n(buf, n);
        self.view.redraw(buf);

        let primary = self.cursors.primary_mut();
        let pos = primary.pos();
        if !self.view.is_visible(pos) {
            let prev = prev_grapheme_boundary(&buf.slice(..), self.view.end());
            primary.goto(prev);
        }
    }

    /// sets window offset so that primary cursor is visible in the drawn view.
    pub fn view_to_cursor(&mut self, buf: &Buffer) {
        debug_assert!(
            buf.id == self.buf,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            self.buf
        );
        let cursor = self.primary_cursor().pos();
        self.view.view_to(cursor, buf);
    }

    /// Called when buffer is changed in the background and we should correct
    /// this window.
    pub fn on_buffer_changed(&mut self, buf: &Buffer) {
        // Remove cursors
        self.cursors.remove_secondary_cursors();
        self.cursors.primary_mut().unanchor();

        // Ensure cursor in buf range
        self.cursors.ensure_in_range(0..buf.len());

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
        }

        // Redraw view
        self.view.invalidate();
        self.view.redraw(buf);
    }

    pub fn buffer_id(&self) -> BufferId {
        self.buf
    }

    pub fn view(&self) -> &View {
        &self.view
    }

    pub fn invalidate(&mut self) {
        self.view.invalidate();
    }

    pub fn resize(&mut self, size: Size, buf: &Buffer) {
        debug_assert!(
            buf.id == self.buf,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            self.buf
        );
        self.view.resize(size);
        self.view_to_cursor(buf);
    }

    pub fn message(&self) -> Option<&StatusMessage> {
        self.message.as_ref()
    }

    pub fn redraw_view(&mut self, buf: &Buffer) {
        debug_assert!(
            buf.id == self.buf,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            self.buf
        );

        self.cursors.ensure_in_range(0..buf.len());
        let primary_pos = self.cursors.primary().pos();
        self.view.redraw(buf);
        self.view.view_to(primary_pos, buf);
    }

    pub fn keymap(&self) -> &Keymap {
        match self.focus {
            Focus::Search => &self.search.prompt.keymap,
            Focus::Prompt => &self.prompt.keymap,
            Focus::Window => &self.keymap,
            Focus::Completion => &self.keymap,
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

    fn insert(&mut self, buf: &mut Buffer, positions: &SortedPositions, text: &str) {
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
            if let Some(mut sel) = cursor.take_selection() {
                sel.start -= removed;
                sel.end -= removed;

                removed += sel.len();
                cursor.goto(sel.start);
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

        let line = to_line(lines, buf.options().eol);
        self.clipboard.copy(&line);
    }

    fn insert_to_each_cursor(&mut self, buf: &mut Buffer, texts: Vec<String>) {
        debug_assert!(
            texts.len() == self.cursors.len(),
            "Cursors {} and texts {} count mismatch",
            self.cursors.len(),
            texts.len()
        );

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
        self.insert(buf, &(&self.cursors).into(), text);

        let mut inserted = 0;
        for cursor in self.cursors.cursors_mut() {
            let cpos = cursor.pos() + inserted;
            cursor.goto(cpos + text.len());
            inserted += text.len();
        }

        self.invalidate();
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
    }

    pub fn undo(&mut self, buf: &mut Buffer) {
        match buf.undo() {
            Ok(change) => {
                let created = change.created_snapshot;
                let restored = change.restored_snapshot;

                if let Some(id) = created {
                    buf.store_snapshot_data(id, self.create_snapshot_data());
                }

                if let Some(restored) = restored {
                    let data = buf.snapshot_data(restored);
                    self.restore(data);
                }

                self.invalidate();
            }
            Err(msg) => self.warn_msg(msg),
        }
    }

    fn restore(&mut self, restored: Option<SnapshotData>) {
        if let Some(ref sdata) = restored {
            self.cursors = sdata.cursors.clone();
            self.view.set_offset(sdata.view_offset);
        } else {
            self.cursors = Cursors::default();
            self.view.set_offset(0);
        }
    }

    pub fn redo(&mut self, buf: &mut Buffer) {
        match buf.redo() {
            Ok(change) => {
                let created = change.created_snapshot;
                let restored = change.restored_snapshot;
                if let Some(id) = created {
                    buf.store_snapshot_data(id, self.create_snapshot_data());
                }

                if let Some(restored) = restored {
                    let data = buf.snapshot_data(restored);
                    self.restore(data);
                }

                self.invalidate();
            }
            Err(msg) => self.warn_msg(msg),
        }
    }

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
    }

    pub fn set_keymap(mappings: impl KeyMappings) {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn to_str(row: &Vec<Cell>) -> String {
        let mut string = String::new();
        for cell in row {
            if let Some(ch) = cell.char() {
                string.push_str(ch.grapheme());
            }
        }
        string
    }

    fn view_lines(win: &Window) -> Vec<String> {
        win.view().cells().iter().map(to_str).collect()
    }

    fn wrapped_line_view() -> (Window, Buffer) {
        let mut buf = Buffer::new();
        buf.insert(
            0,
            "this is a long line that will not fit\nthis is another long line that will not fit into the view\nthis is the third line that is longer than the view",
        );
        let mut win = Window::new(buf.id, 10, 3);
        win.redraw_view(&buf);
        (win, buf)
    }

    fn ten_line() -> (Window, Buffer) {
        let mut buf = Buffer::new();
        buf.insert(
            0,
            "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten",
        );
        let mut win = Window::new(buf.id, 80, 3);
        win.redraw_view(&buf);
        (win, buf)
    }

    #[test]
    fn scroll_up() {
        let (mut win, buf) = ten_line();
        win.view.set_offset(14);
        win.view.redraw(&buf);
        assert_eq!(vec!["four\n", "five\n", "six\n"], view_lines(&win));
        win.scroll_up_n(&buf, 2);
        assert_eq!(vec!["two\n", "three\n", "four\n"], view_lines(&win));
    }

    #[test]
    fn scroll_up_wrapped() {
        let (mut win, buf) = wrapped_line_view();
        win.view.set_offset(52);
        win.view.redraw(&buf);
        assert_eq!(
            vec!["r long lin", "e that wil", "l not fit "],
            view_lines(&win)
        );

        win.scroll_up_n(&buf, 1);
        assert_eq!(
            vec!["this is an", "other long", " line that"],
            view_lines(&win)
        );

        win.scroll_up_n(&buf, 1);
        assert_eq!(
            vec!["a long lin", "e that wil", "l not fit\n"],
            view_lines(&win)
        );
    }

    #[test]
    fn scroll_down() {
        let (mut win, buf) = ten_line();
        win.scroll_down_n(&buf, 2);
        assert_eq!(vec!["three\n", "four\n", "five\n"], view_lines(&win));
    }

    #[test]
    fn scroll_down_wrapped() {
        let (mut win, buf) = wrapped_line_view();
        win.scroll_down_n(&buf, 2);
        assert_eq!(
            vec!["that will ", "not fit\n", "this is an"],
            view_lines(&win)
        );
    }

    #[test]
    fn view_to_after() {
        // let (mut win, buf) = wrapped_line_view();
        // assert_eq!(vec!["", "", ""], view_lines(&win));
    }

    #[test]
    fn view_to_after_small() {}

    #[test]
    fn view_to_before() {}

    #[test]
    fn view_to_before_small() {}
}
