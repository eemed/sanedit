mod completion;
mod cursors;
mod filetree;
mod focus;
mod locations;
mod options;
mod prompt;
mod search;
mod selector;
mod shell;
mod view;

#[cfg(test)]
mod tests;

use std::{
    cmp::{max, min},
    mem,
};

use anyhow::Result;
use rustc_hash::FxHashSet;
use sanedit_messages::redraw::{Severity, Size, StatusMessage};

use crate::{
    common::{
        char::{grapheme_category, DisplayOptions, GraphemeCategory},
        indent::indent_at_line,
        movement,
        text::{selection_line_starts, width_at_pos},
    },
    editor::{
        buffers::{Buffer, BufferId, Change, Changes, SnapshotData, SortedBufferRanges},
        syntax::SyntaxParseResult,
    },
};

use self::filetree::FiletreeView;

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
    completion::*, cursors::*, focus::*, locations::*, options::*, prompt::*, search::*,
    selector::SelectorOption, shell::*, view::*,
};

#[derive(Debug)]
pub(crate) struct Window {
    bid: BufferId,
    last_buf: Option<(BufferId, SnapshotData)>,
    message: Option<StatusMessage>,
    view: View,

    pub shell_executor: Executor,
    pub completion: Completion,
    pub cursors: Cursors,
    pub focus: Focus,
    pub search: Search,
    pub prompt: Prompt,
    pub options: Options,
    pub ft_view: FiletreeView,
    pub locations: Locations,
    pub popup: Option<StatusMessage>,
}

impl Window {
    pub fn new(bid: BufferId, width: usize, height: usize) -> Window {
        Window {
            bid,
            last_buf: None,
            view: View::new(width, height),
            message: None,
            shell_executor: Executor::default(),
            completion: Completion::default(),
            cursors: Cursors::default(),
            options: Options::default(),
            search: Search::default(),
            prompt: Prompt::default(),
            focus: Focus::Window,
            ft_view: FiletreeView::default(),
            locations: Locations::default(),
            popup: None,
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
        self.view.view_to(cursor, buf);
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

        self.cursors.shrink_cursor_to_range(0..buf.len());
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

    fn remove(&mut self, buf: &mut Buffer, ranges: &SortedBufferRanges) -> Result<()> {
        let changes = Changes::multi_remove(ranges);
        let result = buf.apply_changes(&changes)?;

        if let Some(id) = result.created_snapshot {
            *buf.snapshot_data_mut(id).unwrap() = self.create_snapshot_data();
        }
        Ok(())
    }

    fn insert(&mut self, buf: &mut Buffer, positions: &[u64], text: &str) -> Result<()> {
        let changes = Changes::multi_insert(positions, text.as_bytes());
        let result = buf.apply_changes(&changes)?;

        if let Some(id) = result.created_snapshot {
            *buf.snapshot_data_mut(id).unwrap() = self.create_snapshot_data();
        }
        Ok(())
    }

    pub fn remove_cursor_selections(&mut self, buf: &mut Buffer) -> Result<bool> {
        let selections: SortedBufferRanges = (&self.cursors).into();
        if selections.is_empty() {
            return Ok(false);
        }

        self.remove(buf, &selections)?;

        let mut removed = 0;
        for cursor in self.cursors.cursors_mut() {
            let sel = cursor.take_selection();
            let cpos = sel
                .as_ref()
                .map(|range| range.start)
                .unwrap_or(cursor.pos());
            cursor.goto(cpos - removed);

            if let Some(sel) = sel {
                removed += sel.end - sel.start;
            }
        }

        self.invalidate();
        Ok(true)
    }

    pub fn insert_to_each_cursor(&mut self, buf: &mut Buffer, texts: Vec<String>) -> Result<()> {
        debug_assert!(
            texts.len() == self.cursors.len(),
            "Cursors {} and texts {} count mismatch",
            self.cursors.len(),
            texts.len()
        );
        // TODO make this one change instead of many

        //         self.remove_cursor_selections(buf)?;

        //         let mut inserted = 0;
        //         for (i, cursor) in self.cursors.cursors_mut().iter_mut().enumerate() {
        //             let text = &texts[i];
        //             let cpos = cursor.pos() + inserted;
        //             buf.insert(cpos, text)?;
        //             let tlen = text.len() as u64;
        //             cursor.goto(cpos + tlen);
        //             inserted += tlen;
        //         }

        //         self.invalidate();
        Ok(())
    }

    pub fn insert_at_cursors(&mut self, buf: &mut Buffer, text: &str) -> Result<()> {
        // TODO use a replace operation instead if removing
        self.remove_cursor_selections(buf)?;
        let positions: Vec<u64> = (&self.cursors).into();
        self.insert(buf, &positions, text)?;

        let mut inserted = 0;
        for cursor in self.cursors.cursors_mut() {
            let cpos = cursor.pos() + inserted;
            let tlen = text.len() as u64;
            cursor.goto(cpos + tlen);
            inserted += tlen;
        }

        self.invalidate();
        self.view_to_cursor(buf);
        Ok(())
    }

    pub fn remove_grapheme_after_cursors(&mut self, buf: &mut Buffer) -> Result<()> {
        if self.remove_cursor_selections(buf)? {
            return Ok(());
        }

        // TODO

        // let mut removed = 0;
        // for cursor in self.cursors.cursors_mut() {
        //     let cpos = cursor.pos() - removed;
        //     let pos = movement::next_grapheme_boundary(&buf.slice(..), cpos);

        //     cursor.goto(cpos);
        //     buf.remove(cpos..pos)?;
        //     removed += pos - cpos;
        // }

        self.invalidate();
        self.view_to_cursor(buf);
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

        let ranges: SortedBufferRanges = {
            let mut ranges = vec![];

            for cursor in self.cursors.cursors_mut() {
                let cpos = cursor.pos();
                let pos = movement::prev_grapheme_boundary(&buf.slice(..), cpos);
                ranges.push(pos..cpos);
            }

            ranges.into()
        };

        self.remove(buf, &ranges)?;

        let mut removed = 0;
        for (i, range) in ranges.iter().enumerate() {
            let cursor = &mut self.cursors.cursors_mut()[i];
            cursor.goto(range.start - removed);
            removed += range.end - range.start;
        }

        self.invalidate();
        self.view_to_cursor(buf);
        Ok(())
    }

    pub fn syntax_result(&mut self) -> &mut SyntaxParseResult {
        &mut self.view.syntax
    }

    /// Insert a newline to each cursor
    /// if originating line was indented also preserve the indentation
    pub fn insert_newline(&mut self, buf: &mut Buffer) -> Result<()> {
        // 1. Calculate indents
        // 2. insert newlines + indent combo to each cursor
        let eol = buf.options.eol;
        let slice = buf.slice(..);
        let texts: Vec<String> = self
            .cursors()
            .iter()
            .map(|c| {
                let indent = {
                    match indent_at_line(&slice, c.pos()) {
                        // ??
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

    /// Indent all the lines with cursors or their selections
    pub fn indent_cursor_lines(&mut self, buf: &mut Buffer) -> Result<()> {
        let starts: Vec<u64> = {
            let mut starts = FxHashSet::default();

            for cursor in self.cursors.iter() {
                let range = cursor.selection().unwrap_or(cursor.pos()..cursor.pos() + 1);
                let cstarts = selection_line_starts(buf, range);
                starts.extend(cstarts);
            }
            let mut vstarts: Vec<u64> = starts.into_iter().collect();
            vstarts.sort();
            vstarts
        };

        let indent = buf
            .options
            .indent_kind
            .repeat(buf.options.indent_amount as usize);
        let changes = Changes::multi_insert(&starts, indent.as_bytes());
        buf.apply_changes(&changes)?;

        for cursor in self.cursors.cursors_mut() {
            let mut range = cursor.selection().unwrap_or(cursor.pos()..cursor.pos() + 1);
            let pre = starts.iter().take_while(|cur| **cur < range.start).count();
            let count = starts[pre..]
                .iter()
                .take_while(|cur| range.contains(cur))
                .count();
            let plen = (pre * indent.len()) as u64;
            let len = (count * indent.len()) as u64;
            range.start += plen;
            range.end += plen + len;
            cursor.to_range(&range);
        }

        self.invalidate();
        Ok(())
    }

    /// Dedent all the lines with cursors or their selections
    pub fn dedent_cursor_lines(&mut self, buf: &mut Buffer) -> Result<()> {
        let starts: Vec<u64> = {
            let mut starts = FxHashSet::default();

            for cursor in self.cursors.iter() {
                let range = cursor.selection().unwrap_or(cursor.pos()..cursor.pos() + 1);
                let cstarts = selection_line_starts(buf, range);
                starts.extend(cstarts);
            }
            let mut vstarts: Vec<u64> = starts.into_iter().collect();
            vstarts.sort();
            vstarts
        };

        let slice = buf.slice(..);
        let iamount = buf.options.indent_amount;
        let ranges: SortedBufferRanges = {
            let mut ranges = vec![];
            for pos in starts {
                let Some((_kind, n)) = indent_at_line(&slice, pos) else {
                    continue;
                };

                let mut off = n % iamount as u64;
                if off == 0 {
                    off = min(iamount as u64, n);
                }

                ranges.push(pos..pos + off);
            }
            ranges.into()
        };

        self.remove(buf, &ranges)?;
        self.remove_fix_cursors(&ranges);
        self.invalidate();
        Ok(())
    }

    fn remove_fix_cursors(&mut self, ranges: &SortedBufferRanges) {
        for cursor in self.cursors.cursors_mut() {
            let mut range = cursor.selection().unwrap_or(cursor.pos()..cursor.pos() + 1);
            let pre: u64 = ranges
                .iter()
                .take_while(|cur| cur.end <= range.start)
                .map(|range| range.end - range.start)
                .sum();
            let post: u64 = ranges
                .iter()
                .take_while(|cur| cur.end <= range.end)
                .map(|range| range.end - range.start)
                .sum();
            range.start -= pre;
            range.end -= post;
            cursor.to_range(&range);
        }
    }

    /// Insert a tab character
    /// If cursor is at indentation, add an indentation block instead
    pub fn insert_tab(&mut self, buf: &mut Buffer) -> Result<()> {
        let slice = buf.slice(..);
        let ikind = buf.options.indent_kind;
        let iamount = buf.options.indent_amount;
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

        let mut cposs = vec![];
        let mut to_remove = vec![];

        for cursor in self.cursors.cursors() {
            let cpos = cursor.pos();
            let pos = movement::next_line_end(&buf.slice(..), cpos);
            to_remove.push(cpos..pos);
            cposs.push(cpos);
        }

        let ranges = SortedBufferRanges::from(to_remove);
        self.remove(buf, &ranges)?;

        let mut removed = 0;
        for (i, cursor) in self.cursors.cursors_mut().iter_mut().enumerate() {
            cursor.goto(cursor.pos() - removed);
            let range = &ranges[i];
            removed += range.end - range.start;
        }

        self.invalidate();
        self.view_to_cursor(buf);
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

        let ranges = SortedBufferRanges::from(ranges);
        self.remove(buf, &ranges)?;
        self.remove_fix_cursors(&ranges);
        self.invalidate();

        Ok(())
    }
}
