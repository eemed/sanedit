mod chooser;
mod completion;
mod config;
mod cursors;
mod filetree;
mod focus;
mod prompt;
mod search;
mod shell;
mod view;

#[cfg(test)]
mod tests;

use std::{
    cmp::{max, min},
    mem,
};

use anyhow::{bail, Result};
use rustc_hash::FxHashSet;
use sanedit_core::{
    grapheme_category, indent_at_line,
    movement::{next_grapheme_boundary, next_line_end, prev_grapheme_boundary},
    selection_line_starts, width_at_pos, BufferRange, BufferRangeExt as _, Change, Changes, Cursor,
    DisplayOptions, GraphemeCategory, Locations,
};
use sanedit_messages::redraw::{Popup, PopupMessage, Severity, Size, StatusMessage};

use crate::editor::buffers::{Buffer, BufferId, SnapshotData};

use self::filetree::FiletreeView;
pub(crate) use cursors::Cursors;

macro_rules! show_error {
    ($self:ident, $result:expr) => {{
        let result = $result;
        if let Err(e) = &result {
            $self.error_msg(&e.to_string());
        }
        result?
    }};
}

macro_rules! show_warn {
    ($self:ident, $result:expr) => {{
        let result = $result;
        if let Err(e) = &result {
            $self.warn_msg(&e.to_string());
        }
        result?
    }};
}

pub(crate) use self::{
    completion::*, config::*, focus::*, prompt::*, search::*, shell::*, view::*,
};

#[derive(Debug)]
pub(crate) struct Window {
    bid: BufferId,
    last_buf: Option<(BufferId, SnapshotData)>,
    message: Option<StatusMessage>,
    view: View,

    /// Jump to primary cursor on next buffer changed event
    jump_to_primary_cursor: bool,

    pub shell_executor: Executor,
    pub completion: Completion,
    pub cursors: Cursors,
    pub focus: Focus,
    pub search: Search,
    pub prompt: Prompt,
    pub config: WindowConfig,
    pub ft_view: FiletreeView,
    pub locations: Locations,
    popup: Option<Popup>,
}

impl Window {
    pub fn new(bid: BufferId, width: usize, height: usize) -> Window {
        Window {
            bid,
            last_buf: None,
            view: View::new(width, height),
            jump_to_primary_cursor: false,
            message: None,
            shell_executor: Executor::default(),
            completion: Completion::default(),
            cursors: Cursors::default(),
            config: WindowConfig::default(),
            search: Search::default(),
            prompt: Prompt::default(),
            focus: Focus::Window,
            ft_view: FiletreeView::default(),
            locations: Locations::default(),
            popup: None,
        }
    }

    pub fn clear_popup(&mut self) {
        self.popup = None;
    }

    /// Push a new popup message
    pub fn push_popup(&mut self, msg: PopupMessage) {
        match self.popup.as_mut() {
            Some(popup) => {
                popup.messages.push(msg);
            }
            None => {
                let pos = self.cursors.primary().pos();
                let point = self.view.point_at_pos(pos).unwrap_or_default();
                self.popup = Some(Popup {
                    point,
                    messages: vec![msg],
                });
            }
        }
    }

    pub fn popup(&self) -> Option<&Popup> {
        self.popup.as_ref()
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
        self.completion = Completion::default();
        self.view.syntax = ViewSyntax::default();
    }

    pub fn display_options_mut(&mut self) -> &mut DisplayOptions {
        &mut self.view.options
    }

    pub fn display_options(&self) -> &DisplayOptions {
        &self.view.options
    }

    pub fn open_buffer(&mut self, bid: BufferId) -> BufferId {
        let old = self.bid;
        // Store old buffer data
        let odata = self.create_snapshot_data();
        self.last_buf = Some((old, odata));

        self.bid = bid;
        self.reload();
        old
    }

    pub fn goto_prev_buffer(&mut self) -> bool {
        match mem::take(&mut self.last_buf) {
            Some((pbid, pdata)) => {
                let old = self.bid;
                let odata = self.create_snapshot_data();
                self.last_buf = Some((old, odata));

                self.bid = pbid;
                self.restore(&pdata);
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

    pub fn scroll_down_n(&mut self, buf: &Buffer, n: u64) {
        debug_assert!(
            buf.id == self.bid,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            self.bid
        );

        self.view.scroll_down_n(buf, n);
        self.view.redraw(buf);
    }

    pub fn scroll_up_n(&mut self, buf: &Buffer, n: u64) {
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

        if !self.view.is_visible(cursor) {
            self.view.view_to(cursor, buf);
        }
        self.jump_to_primary_cursor = false;
    }

    /// Move primary cursor to line and the view
    pub fn goto_line(&mut self, line: u64, buf: &Buffer) {
        let slice = buf.slice(..);
        let mut lines = slice.lines();
        for _ in 1..max(line, 1) {
            lines.next();
        }

        let offset = lines.next().map(|line| line.start()).unwrap_or(buf.len());
        self.goto_offset(offset, buf);
    }

    /// Move primary cursor to offset and the view too
    pub fn goto_offset(&mut self, offset: u64, buf: &Buffer) {
        let offset = min(offset, buf.len());
        let primary = self.cursors.primary_mut();
        primary.goto(offset);

        self.ensure_cursor_on_grapheme_boundary(buf);
        self.view_to_cursor(buf);
    }

    pub fn ensure_cursor_on_grapheme_boundary(&mut self, buf: &Buffer) {
        // Ensure cursor in buf range
        self.cursors.contain_to(0..buf.len());

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
        self.move_cursors_according_to_last_change(buf);
        self.cursors.merge_overlapping();

        if self.jump_to_primary_cursor {
            self.view_to_cursor(buf);
        }

        // Redraw view
        self.view.invalidate();
        self.view.redraw(buf);
    }

    pub fn buffer_id(&self) -> BufferId {
        self.bid
    }

    pub fn prev_buffer_id(&self) -> Option<BufferId> {
        self.last_buf.as_ref().map(|(a, _)| a).copied()
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

        self.cursors.contain_to(0..buf.len());
        // let primary_pos = self.cursors.primary().pos();
        self.view.redraw(buf);
        // self.view.view_to(primary_pos, buf);
    }

    fn create_snapshot_data(&self) -> SnapshotData {
        SnapshotData {
            cursors: self.cursors.clone(),
            view_offset: self.view.start(),
        }
    }

    fn remove(&mut self, buf: &mut Buffer, ranges: &[BufferRange]) -> Result<()> {
        let changes = Changes::multi_remove(ranges);
        self.change(buf, &changes)
    }

    fn move_cursors_according_to_last_change(&mut self, buf: &Buffer) {
        let Some(edit) = buf.last_edit() else {
            return;
        };
        let changes = &edit.changes;

        changes.move_cursors(self.cursors.cursors_mut());
    }

    fn change(&mut self, buf: &mut Buffer, changes: &Changes) -> Result<()> {
        let result = buf.apply_changes(&changes)?;

        if let Some(id) = result.created_snapshot {
            *buf.snapshot_data_mut(id).unwrap() = self.create_snapshot_data();
        }

        self.view.invalidate();

        Ok(())
    }

    pub fn remove_cursor_selections(&mut self, buf: &mut Buffer) -> Result<bool> {
        let selections: Vec<BufferRange> = (&self.cursors).into();
        if selections.is_empty() {
            return Ok(false);
        }

        self.remove(buf, &selections)?;
        self.cursors.stop_selection();

        Ok(true)
    }

    pub fn insert_to_each_cursor(&mut self, buf: &mut Buffer, texts: Vec<String>) -> Result<()> {
        debug_assert!(
            texts.len() == self.cursors.len(),
            "Cursors {} and texts {} count mismatch",
            self.cursors.len(),
            texts.len()
        );

        let changes: Vec<Change> = self
            .cursors
            .iter()
            .enumerate()
            .map(|(i, cursor)| {
                let text = texts[i].as_bytes();
                if let Some(sel) = cursor.selection() {
                    Change::replace(sel, text)
                } else {
                    Change::insert(cursor.pos(), text)
                }
            })
            .collect();
        let changes: Changes = changes.into();
        self.change(buf, &changes)
    }

    pub fn insert_at_cursors(&mut self, buf: &mut Buffer, text: &str) -> Result<()> {
        let changes: Vec<Change> = self
            .cursors
            .iter()
            .map(|cursor| {
                if let Some(sel) = cursor.selection() {
                    Change::replace(sel, text.as_bytes())
                } else {
                    Change::insert(cursor.pos(), text.as_bytes())
                }
            })
            .collect();
        let changes: Changes = changes.into();

        self.change(buf, &changes)?;
        self.jump_to_primary_cursor = true;
        Ok(())
    }

    pub fn remove_grapheme_after_cursors(&mut self, buf: &mut Buffer) -> Result<()> {
        if self.remove_cursor_selections(buf)? {
            return Ok(());
        }

        let slice = buf.slice(..);
        let ranges: Vec<BufferRange> = self
            .cursors
            .cursors()
            .iter()
            .map(Cursor::pos)
            .map(|pos| {
                let next = next_grapheme_boundary(&slice, pos);
                pos..next
            })
            .collect();
        let changes = Changes::multi_remove(&ranges);
        buf.apply_changes(&changes)?;
        self.jump_to_primary_cursor = true;
        Ok(())
    }

    pub fn undo(&mut self, buf: &mut Buffer) -> Result<()> {
        let change = show_warn!(self, buf.apply_changes(&Changes::undo()));
        let created = change.created_snapshot;
        let restored = change.restored_snapshot;

        if let Some(id) = created {
            *buf.snapshot_data_mut(id).unwrap() = self.create_snapshot_data();
        }

        if let Some(restored) = restored {
            if let Some(data) = buf.snapshot_data(restored) {
                self.restore(data);
            } else {
                self.reload();
            }
        }

        self.invalidate();
        Ok(())
    }

    fn restore(&mut self, sdata: &SnapshotData) {
        self.cursors = sdata.cursors.clone();
        self.view.set_offset(sdata.view_offset);
        self.invalidate();
    }

    pub fn redo(&mut self, buf: &mut Buffer) -> Result<()> {
        let change = show_warn!(self, buf.apply_changes(&Changes::redo()));
        let created = change.created_snapshot;
        let restored = change.restored_snapshot;
        if let Some(id) = created {
            *buf.snapshot_data_mut(id).unwrap() = self.create_snapshot_data();
        }

        if let Some(restored) = restored {
            if let Some(data) = buf.snapshot_data(restored) {
                self.restore(data);
            } else {
                self.reload()
            }
        }

        self.invalidate();
        Ok(())
    }

    /// Remove a grapheme before the cursor, if at indentation
    /// remove a block of it
    pub fn remove_grapheme_before_cursors(&mut self, buf: &mut Buffer) -> Result<()> {
        if self.remove_cursor_selections(buf)? {
            return Ok(());
        }

        let ranges: Vec<BufferRange> = {
            let mut ranges = vec![];

            for cursor in self.cursors.cursors() {
                let cpos = cursor.pos();
                let pos = prev_grapheme_boundary(&buf.slice(..), cpos);
                ranges.push(pos..cpos);
            }

            ranges
        };

        self.remove(buf, &ranges)?;

        self.jump_to_primary_cursor = true;
        Ok(())
    }

    pub fn view_syntax(&mut self) -> &mut ViewSyntax {
        &mut self.view.syntax
    }

    /// Insert a newline to each cursor
    /// if originating line was indented also preserve the indentation
    pub fn insert_newline(&mut self, buf: &mut Buffer) -> Result<()> {
        // 1. Calculate indents
        // 2. insert newlines + indent combo to each cursor
        let eol = buf.config.eol;
        let slice = buf.slice(..);
        let texts: Vec<String> = self
            .cursors()
            .iter()
            .map(|c| {
                let indent = {
                    match indent_at_line(&slice, c.pos()) {
                        Some((k, n)) => k.repeat(n as usize),
                        None => String::new(),
                    }
                };

                format!("{}{}", eol.as_str(), indent)
            })
            .collect();

        self.insert_to_each_cursor(buf, texts)?;
        Ok(())
    }

    fn cursor_line_starts(&self, buf: &Buffer) -> Vec<u64> {
        let slice = buf.slice(..);
        let mut starts = FxHashSet::default();

        for cursor in self.cursors.iter() {
            let cpos = cursor.pos();
            let sel = cursor.selection().unwrap_or(cpos..cpos);
            let cstarts = selection_line_starts(&slice, sel);
            starts.extend(cstarts);
        }
        let mut vstarts: Vec<u64> = starts.into_iter().collect();
        vstarts.sort();
        vstarts
    }

    /// Dedent all the lines with cursors or their selections
    pub fn dedent_cursor_lines(&mut self, buf: &mut Buffer) -> Result<()> {
        let starts = self.cursor_line_starts(buf);
        let slice = buf.slice(..);
        let iamount = buf.config.indent_amount;
        let ranges: Vec<BufferRange> = {
            let mut ranges = vec![];
            for pos in starts {
                let Some((_kind, n)) = indent_at_line(&slice, pos) else {
                    continue;
                };

                let mut off = n % iamount as u64;
                if off == 0 && n != 0 {
                    off = iamount as u64;
                }

                if off != 0 {
                    ranges.push(pos..pos + off);
                }
            }
            ranges
        };

        if ranges.is_empty() {
            bail!("No lines to dedent");
        }

        self.remove(buf, &ranges)?;
        Ok(())
    }

    /// Indent all the lines with cursors or their selections
    pub fn indent_cursor_lines(&mut self, buf: &mut Buffer) -> Result<()> {
        let starts = self.cursor_line_starts(buf);
        let indent = buf
            .config
            .indent_kind
            .repeat(buf.config.indent_amount as usize);
        let changes = Changes::multi_insert(&starts, indent.as_bytes());
        self.change(buf, &changes)?;
        Ok(())
    }

    /// Add indentation at cursors
    pub fn indent(&mut self, buf: &mut Buffer) -> Result<()> {
        let slice = buf.slice(..);
        let ikind = buf.config.indent_kind;
        let iamount = buf.config.indent_amount;
        let texts: Vec<String> = self
            .cursors()
            .iter()
            .map(|c| {
                let col = width_at_pos(&slice, c.pos(), &self.view.options);
                let mut to_add = col % iamount as usize;
                if to_add == 0 {
                    to_add = iamount as usize;
                }
                ikind.repeat(to_add)
            })
            .collect();
        self.insert_to_each_cursor(buf, texts)?;
        Ok(())
    }

    /// Synchronously saves the buffer
    pub fn save_buffer(&mut self, buf: &mut Buffer) -> Result<()> {
        let saved = show_error!(self, buf.save_rename());
        let sdata = buf.snapshot_data_mut(saved.snapshot).unwrap();
        *sdata = self.create_snapshot_data();
        Ok(())
    }

    pub fn remove_line_after_cursor(&mut self, buf: &mut Buffer) -> Result<()> {
        if self.remove_cursor_selections(buf)? {
            return Ok(());
        }

        let slice = buf.slice(..);
        let ranges: Vec<BufferRange> = self
            .cursors
            .cursors()
            .iter()
            .map(Cursor::pos)
            .map(|pos| {
                let npos = next_line_end(&slice, pos);
                pos..npos
            })
            .collect();

        self.remove(buf, &ranges)?;
        self.jump_to_primary_cursor = true;
        Ok(())
    }

    pub fn strip_trailing_whitespace(&mut self, buf: &mut Buffer) -> Result<()> {
        let mut ranges = vec![];
        let slice = buf.slice(..);
        let mut lines = slice.lines();

        while let Some(line) = lines.next() {
            let mut start = None;
            let mut end = line.end();

            let mut graphemes = line.graphemes_at(line.len());
            while let Some(g) = graphemes.prev() {
                let cat = grapheme_category(&g);
                match cat {
                    GraphemeCategory::EOL => {
                        end = g.start();
                    }
                    GraphemeCategory::Whitespace => start = Some(g.start()),
                    _ => break,
                }
            }

            if let (Some(start), end) = (start, end) {
                ranges.push(start..end);
            }
        }

        self.remove(buf, &ranges)?;

        Ok(())
    }
}
