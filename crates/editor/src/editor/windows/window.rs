mod chooser;
mod completion;
mod config;
mod cursors;
mod filetree;
mod focus;
mod jumps;
mod mode;
mod prompt;
mod search;
mod view;
mod window_manager;

#[cfg(test)]
mod tests;

use std::{
    cmp::{max, min},
    collections::BTreeMap,
    mem,
    sync::Arc,
};

use anyhow::{bail, Result};
use rustc_hash::FxHashSet;
use sanedit_buffer::{Mark, MarkResult};
use sanedit_core::{
    grapheme_category, indent_at_line,
    movement::{
        end_of_line, find_prev_whitespace, next_grapheme_boundary, next_line_end, next_line_start,
        prev_grapheme_boundary, start_of_line,
    },
    selection_first_chars_of_lines, selection_line_ends, selection_line_starts, width_at_pos,
    BufferRange, Change, Changes, Cursor, DisplayOptions, GraphemeCategory, Locations, Range,
};
use sanedit_messages::{
    key::KeyEvent,
    redraw::{Popup, PopupMessage, Severity, Size, StatusMessage},
};
use sanedit_server::ClientId;
use sanedit_utils::{ring::Ref, sorted_vec::SortedVec};

use crate::{
    actions::ActionResult,
    common::change::{newline_autopair, newline_empty_line, newline_indent},
    editor::{
        buffers::{Buffer, BufferId, SnapshotAux, SnapshotId},
        keymap::LayerKey,
        Editor,
    },
};

use self::filetree::FiletreeView;
pub(crate) use cursors::Cursors;

pub(crate) use self::{
    completion::*, config::*, focus::*, jumps::*, mode::*, prompt::*, search::*, view::*,
    window_manager::*,
};

#[derive(Debug)]
pub(crate) struct Window {
    bid: BufferId,
    last_buffer: Option<(BufferId, SnapshotAux)>,
    message: Option<StatusMessage>,
    view: View,
    keys: Vec<KeyEvent>,
    popup: Option<Popup>,

    /// Focus determines where to direct input
    pub focus: Focus,
    pub mode: Mode,
    pub window_manager: WindowManager,
    pub completion: Completion,
    pub cursors: Cursors,
    pub search: Search,
    pub prompt: Prompt,
    pub config: WindowConfig,
    pub ft_view: FiletreeView,
    pub locations: Locations,
    pub snippets: Vec<Jumps>,

    /// Cursor jumps across files
    pub cursor_jumps: Jumps,

    /// Last edit jumped to in buffer
    pub last_edit_jump: Option<SnapshotId>,

    /// Handles next keypress, before anything else
    pub next_key_handler: Option<NextKeyFunction>,
}

impl Window {
    pub fn new(bid: BufferId, width: usize, height: usize, config: WindowConfig) -> Window {
        Window {
            bid,
            keys: vec![],
            last_buffer: None,
            view: View::new(width, height),
            message: None,
            window_manager: config.window_manager.get(),
            completion: Completion::default(),
            cursors: Cursors::default(),
            config,
            mode: Mode::Normal,
            search: Search::default(),
            prompt: Prompt::default(),
            focus: Focus::Window,
            ft_view: FiletreeView::default(),
            locations: Locations::default(),
            popup: None,
            snippets: vec![],
            cursor_jumps: Jumps::with_capacity(512),
            last_edit_jump: None,
            next_key_handler: None,
        }
    }

    pub fn layer(&self) -> LayerKey {
        LayerKey {
            focus: self.focus,
            mode: self.mode,
        }
    }

    pub fn keys(&self) -> &[KeyEvent] {
        &self.keys
    }

    pub fn push_key(&mut self, event: KeyEvent) {
        self.keys.push(event)
    }

    pub fn clear_keys(&mut self) -> Vec<KeyEvent> {
        mem::take(&mut self.keys)
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
                    line_offset: 0,
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

    pub fn display_options(&self) -> &DisplayOptions {
        &self.view.options
    }

    pub fn open_buffer(&mut self, bid: BufferId) -> BufferId {
        self.cursor_jumps.goto_start();

        let old = self.bid;
        // Store old buffer data
        let odata = self.window_aux(None);
        self.last_buffer = Some((old, odata));

        self.bid = bid;
        self.reload();
        old
    }

    pub fn goto_prev_buffer(&mut self) -> bool {
        match mem::take(&mut self.last_buffer) {
            Some((pbid, pdata)) => {
                let old = self.bid;
                let odata = self.window_aux(None);
                self.last_buffer = Some((old, odata));

                self.bid = pbid;
                self.restore(&pdata, None);
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

        match &mut self.popup {
            Some(popup) => {
                popup.line_offset += n as usize;
            }
            None => {
                self.view.scroll_down_n(buf, n);
                self.view.redraw(buf);
            }
        }
    }

    pub fn scroll_up_n(&mut self, buf: &Buffer, n: u64) {
        debug_assert!(
            buf.id == self.bid,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            self.bid
        );
        match &mut self.popup {
            Some(popup) => {
                popup.line_offset = popup.line_offset.saturating_sub(n as usize);
            }
            None => {
                self.view.scroll_up_n(buf, n);
                self.view.redraw(buf);
            }
        }
    }

    /// Put View to cursor if cursor is not visible otherwise
    pub fn view_to_around_cursor_zone(&mut self, buf: &Buffer, zone: Zone) {
        let cursor = self.primary_cursor().pos();
        self.view.redraw(buf);

        if !self.view.is_visible(cursor) {
            self.view_to_cursor_zone(buf, zone);
        }
    }

    pub fn view_to_cursor_zone(&mut self, buf: &Buffer, zone: Zone) {
        debug_assert!(
            buf.id == self.bid,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            self.bid
        );
        let cursor = self.primary_cursor().pos();

        self.view.set_offset(cursor);

        match zone {
            Zone::Top => self.view.scroll_up_n(buf, 1),
            Zone::Middle => {
                let lines = (self.view.height() / 2) as u64;
                self.view.scroll_up_n(buf, lines);
            }
            Zone::Bottom => {
                let lines = self.view.height().saturating_sub(2) as u64;
                self.view.scroll_up_n(buf, lines);
            }
        }
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

        self.view.redraw(buf);

        if !self.view.is_visible(cursor) {
        log::info!("view to: {cursor}, len: {}", buf.len());
            self.view.view_to(cursor, buf);
        }
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

    pub fn push_new_cursor_jump(&mut self, buf: &Buffer) {
        let primary = self.cursors.primary().pos();
        let mark = buf.mark(primary);
        let jump = Jump::new(mark, None);
        let group = JumpGroup::new(self.bid, vec![jump]);
        self.cursor_jumps.push(group);
    }

    /// Move primary cursor to offset and the view too
    pub fn goto_offset(&mut self, offset: u64, buf: &Buffer) {
        let offset = min(offset, buf.len());
        let primary = self.cursors.primary_mut();
        primary.goto(offset);

        self.ensure_cursor_on_grapheme_boundary(buf);
        self.view_to_cursor_zone(buf, Zone::Middle);
        self.push_new_cursor_jump(buf);
    }

    pub fn ensure_cursor_on_grapheme_boundary(&mut self, buf: &Buffer) {
        // Ensure cursor in buf range
        self.cursors.contain_to(Range::new(0, buf.len()));

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

    /// Called when buffer is changed by another client and we should correct
    /// this window.
    pub fn on_buffer_changed(&mut self, buf: &Buffer) {
        let Some(edit) = buf.last_edit() else {
            return;
        };
        let changes = &edit.changes;
        changes.move_cursors(self.cursors.cursors_mut());

        self.cursors.merge_overlapping();
        self.ensure_cursor_on_grapheme_boundary(buf);

        // Redraw view
        let offset = changes.move_offset(self.view().start());
        if offset != self.view().start() {
            self.view.set_offset(offset);
        }

        self.view.invalidate();
        self.view.redraw(buf);
    }

    pub fn buffer_id(&self) -> BufferId {
        self.bid
    }

    pub fn prev_buffer_id(&self) -> Option<BufferId> {
        self.last_buffer.as_ref().map(|(a, _)| a).copied()
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

        self.cursors.contain_to(Range::new(0, buf.len()));
        // let primary_pos = self.cursors.primary().pos();
        self.view.redraw(buf);
        // self.view.view_to(primary_pos, buf);
    }

    /// Create snapshot auxilary data for window
    /// Provide mark to store in aux
    fn window_aux(&self, mark: Option<Mark>) -> SnapshotAux {
        SnapshotAux {
            cursors: self.cursors.clone(),
            view_offset: self.view.start(),
            change_start: mark,
        }
    }

    fn remove(&mut self, buf: &mut Buffer, ranges: &[BufferRange]) -> Result<()> {
        let changes = Changes::multi_remove(ranges);
        self.change(buf, &changes)
    }

    pub fn change(&mut self, buf: &mut Buffer, changes: &Changes) -> Result<()> {
        self.last_edit_jump = None;
        self.cursor_jumps.goto_start();

        let mark = self.cursors.mark_first(buf);
        let aux = self.window_aux(mark.into());
        let result = buf.apply_changes(changes)?;

        changes.move_cursors(self.cursors.cursors_mut());
        self.cursors.merge_overlapping();

        if let Some(id) = result.created_snapshot {
            *buf.snapshot_aux_mut(id).unwrap() = aux;
        } else if let Some(id) = result.forked_snapshot {
            *buf.snapshot_aux_mut(id).unwrap() = aux;
        }

        self.view.invalidate();

        Ok(())
    }

    pub fn remove_cursor_selections(&mut self, buf: &mut Buffer) -> Result<bool> {
        let selections: Vec<BufferRange> = (&self.cursors).into();
        if selections.is_empty() {
            return Ok(false);
        }

        self.cursors.stop_selection();
        self.remove(buf, &selections)?;
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

    pub fn insert_at_cursors_next_line(&mut self, buf: &mut Buffer, text: &str) -> Result<()> {
        let slice = buf.slice(..);
        let changes: Vec<Change> = self
            .cursors
            .iter()
            .map(|cursor| {
                let pos = cursor.pos();
                let next_line = next_line_start(&slice, pos);
                Change::insert(next_line, text.as_bytes())
            })
            .collect();
        let changes: Changes = changes.into();

        self.change(buf, &changes)?;
        Ok(())
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
                Range::new(pos, next)
            })
            .collect();
        let changes = Changes::multi_remove(&ranges);
        self.change(buf, &changes)?;
        // buf.apply_changes(&changes)?;
        // self.invalidate();
        Ok(())
    }

    fn cursors_from_changes(changes: &Changes) -> Cursors {
        let mut cursors = Cursors::default();

        let mut diff: i128 = 0;
        for change in changes.iter() {
            let total = change.text().len() as i128 - change.range().len() as i128;
            diff += total;

            let mut pos = change.range().end;
            if total < 0 {
                pos -= diff.abs() as u64;
            } else {
                pos += diff.abs() as u64;
            }

            cursors.push(Cursor::new(pos));
        }

        if cursors.len() != 1 {
            cursors.remove_primary();
        }
        cursors
    }

    pub fn undo_jump(&mut self, buf: &mut Buffer, snapshot: SnapshotId) -> Result<()> {
        self.last_edit_jump = None;
        let aux = {
            let cursors = buf
                .last_edit()
                .map(|edit| Self::cursors_from_changes(&edit.changes))
                .unwrap_or(Cursors::default());
            let mark = cursors.mark_first(buf);

            SnapshotAux {
                cursors,
                view_offset: self.view.start(),
                change_start: mark.into(),
            }
        };

        let change = match buf.apply_changes(&Changes::undo_jump(snapshot)) {
            Ok(res) => res,
            Err(e) => {
                self.warn_msg(&format!("{e}"));
                return Err(e);
            }
        };
        let created = change.created_snapshot;
        let restored = change.restored_snapshot;

        if let Some(id) = created {
            *buf.snapshot_aux_mut(id).unwrap() = aux;
        }

        if let Some(restored) = restored {
            if let Some(data) = buf.snapshot_aux(restored) {
                self.restore(data, Some(buf));
            } else {
                self.reload();
            }
        }

        self.invalidate();
        self.view_to_cursor(buf);
        Ok(())
    }

    pub fn undo(&mut self, buf: &mut Buffer) -> Result<()> {
        // Nothing -> insert h -> SNAP A -> insert ello
        //
        // undo -> SNAP B hello| after/berfore
        //      -> restore SNAP A before cursors
        //
        // redo -> restore SNAP B hello| after/before cursors
        //
        // remove o -> SNAP B before hello| after hell|
        //     undo -> SNAP C after before hell|
        //          -> restore SNAP B hello| before
        //     redo -> restore SNAP C hell| before
        //
        // FORK:
        // remove o -> SNAP B before hello| after hell|
        //     undo -> SNAP C after before hell|
        //          -> restore SNAP B hello| before
        //  insert line 2 w
        //          -> No snap
        //          -> UPDATE SNAP B
        //          -> FORKS the SNAPSHOTS HERE
        //     undo -> SNAP D after before w|
        //          -> restore SNAP B line2 |
        //
        // When undoing stuff create cursors from last edit, instead of current
        // position
        self.last_edit_jump = None;
        let aux = {
            let cursors = buf
                .last_edit()
                .map(|edit| Self::cursors_from_changes(&edit.changes))
                .unwrap_or(Cursors::default());
            let mark = cursors.mark_first(buf);

            SnapshotAux {
                cursors,
                view_offset: self.view.start(),
                change_start: mark.into(),
            }
        };

        let change = match buf.apply_changes(&Changes::undo()) {
            Ok(res) => res,
            Err(e) => {
                self.warn_msg(&format!("{e}"));
                return Err(e);
            }
        };
        let created = change.created_snapshot;
        let restored = change.restored_snapshot;

        if let Some(id) = created {
            *buf.snapshot_aux_mut(id).unwrap() = aux;
        }

        if let Some(restored) = restored {
            if let Some(data) = buf.snapshot_aux(restored) {
                self.restore(data, Some(buf));
            } else {
                self.reload();
            }
        }

        self.invalidate();
        self.view_to_cursor(buf);
        Ok(())
    }

    // Restore aux data, if buffer is provided try to scroll to view position
    // otherwise hard set it
    fn restore(&mut self, aux: &SnapshotAux, buf: Option<&Buffer>) {
        self.cursors = aux.cursors.clone();

        match buf {
            Some(buf) => self.view.view_to(aux.view_offset, buf),
            None => self.view.set_offset(aux.view_offset),
        }
        self.invalidate();
    }

    pub fn redo(&mut self, buf: &mut Buffer) -> Result<()> {
        self.last_edit_jump = None;
        let change = match buf.apply_changes(&Changes::redo()) {
            Ok(res) => res,
            Err(e) => {
                self.warn_msg(&format!("{e}"));
                return Err(e);
            }
        };
        let restored = change.restored_snapshot;

        if let Some(restored) = restored {
            if let Some(data) = buf.snapshot_aux(restored) {
                self.restore(data, Some(buf));
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
                ranges.push(Range::new(pos, cpos));
            }

            ranges
        };

        self.remove(buf, &ranges)?;

        Ok(())
    }

    pub fn view_syntax(&mut self) -> &mut ViewSyntax {
        &mut self.view.syntax
    }

    /// Insert a newline to each cursor
    /// Tries to preserve indentation
    pub fn insert_newline(&mut self, buf: &mut Buffer) -> Result<()> {
        let eol = buf.config.eol.as_str();
        let mut changes: Vec<Change> = vec![];

        for c in self.cursors().iter() {
            if !self.config.autoindent {
                changes.push(Change::insert(c.pos(), eol.as_bytes()));
                continue;
            }

            // Delete empty lines indentation and indent current instead
            if let Some(change) = newline_empty_line(buf, c.pos()) {
                changes.push(change);
                continue;
            }

            // Add autopairs if necessary
            if self.config.autopair {
                if let Some(change) = newline_autopair(buf, c.pos()) {
                    changes.push(change);
                    continue;
                }
            }

            // Otherwise just insert normal eol + indent
            changes.push(newline_indent(buf, c.pos()));
        }

        let changes: Changes = changes.into();
        self.change(buf, &changes)
    }

    fn cursor_line_starts(&self, buf: &Buffer) -> Vec<u64> {
        let slice = buf.slice(..);
        let mut starts = FxHashSet::default();

        for cursor in self.cursors.iter() {
            let cpos = cursor.pos();
            let sel = cursor.selection().unwrap_or(Range::new(cpos, cpos));
            let cstarts = selection_line_starts(&slice, sel);
            starts.extend(cstarts);
        }
        let mut vstarts: Vec<u64> = starts.into_iter().collect();
        vstarts.sort();
        vstarts
    }

    fn cursor_line_ends(&self, buf: &Buffer) -> Vec<u64> {
        let slice = buf.slice(..);
        let mut endset = FxHashSet::default();

        for cursor in self.cursors.iter() {
            let cpos = cursor.pos();
            let sel = cursor.selection().unwrap_or(Range::new(cpos, cpos));
            let ends = selection_line_ends(&slice, sel);
            endset.extend(ends);
        }
        let mut ends: Vec<u64> = endset.into_iter().collect();
        ends.sort();
        ends
    }

    fn cursor_line_first_chars_of_lines_aligned(&self, buf: &Buffer) -> Vec<u64> {
        let slice = buf.slice(..);
        let mut starts = FxHashSet::default();
        let mut dist = u64::MAX;

        for cursor in self.cursors.iter() {
            let cpos = cursor.pos();
            let sel = cursor.selection().unwrap_or(Range::new(cpos, cpos));
            let cstarts = selection_first_chars_of_lines(&slice, sel);
            for (sol, fch) in cstarts {
                starts.insert(sol);
                dist = std::cmp::min(dist, fch - sol);
            }
        }
        let mut vstarts: Vec<u64> = starts.into_iter().map(|s| s + dist).collect();
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
                    ranges.push(Range::new(pos, pos + off));
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

    pub fn comment_cursor_lines(&mut self, buf: &mut Buffer, comment: &str) -> Result<()> {
        if comment.is_empty() {
            return Ok(());
        }

        let starts = self.cursor_line_first_chars_of_lines_aligned(buf);
        self.cursors.stop_selection();
        if starts.is_empty() {
            return Ok(());
        }
        let changes = Changes::multi_insert(&starts, comment.as_bytes());
        self.change(buf, &changes)?;
        Ok(())
    }

    pub fn uncomment_cursor_lines(&mut self, buf: &mut Buffer, comment: &str) -> Result<()> {
        if comment.is_empty() {
            return Ok(());
        }

        let starts = self.cursor_line_starts(buf);
        let slice = buf.slice(..);
        let patt: Vec<char> = comment.chars().collect();

        let mut changes = vec![];

        'outer: for start in starts {
            let mut npatt = 0;
            let end = end_of_line(&slice, start);
            let line = buf.slice(start..end);
            let mut chars = line.chars();

            while let Some((_, e, ch)) = chars.next() {
                if ch == patt[npatt] {
                    npatt += 1;
                    if npatt == patt.len() {
                        let end = e + line.start();
                        let start = end - comment.len() as u64;
                        changes.push(Change::remove(Range::new(start, end)));
                        break;
                    }
                } else if !ch.is_whitespace() {
                    continue 'outer;
                }
            }
        }

        self.cursors.stop_selection();
        let changes = Changes::new(&changes);
        self.change(buf, &changes)?;
        Ok(())
    }

    fn has_comment_on_line(&self, buf: &Buffer, comment: &str, start_of_line: u64) -> bool {
        let patt: Vec<char> = comment.chars().collect();
        let mut npatt = 0;
        let slice = buf.slice(..);
        let end = end_of_line(&slice, start_of_line);
        let line = buf.slice(start_of_line..end);
        let mut chars = line.chars();

        while let Some((_, _, ch)) = chars.next() {
            if ch == patt[npatt] {
                npatt += 1;
                if npatt == patt.len() {
                    return true;
                }
            } else if !ch.is_whitespace() {
                return false;
            }
        }

        false
    }

    pub fn toggle_comment_cursor_lines(&mut self, buf: &mut Buffer, comment: &str) -> Result<()> {
        let starts = self.cursor_line_starts(buf);
        let start = starts[0];

        if self.has_comment_on_line(buf, comment, start) {
            self.uncomment_cursor_lines(buf, comment)
        } else {
            self.comment_cursor_lines(buf, comment)
        }
    }

    pub fn cursors_to_lines_start(&mut self, buf: &Buffer) {
        let starts = self.cursor_line_starts(buf);
        self.cursors.remove_except_primary();
        for (i, start) in starts.iter().enumerate() {
            if i == 0 {
                self.cursors.replace_primary(Cursor::new(*start));
            } else {
                self.cursors.push(Cursor::new(*start));
            }
        }
    }

    pub fn cursors_to_lines_end(&mut self, buf: &Buffer) {
        let ends = self.cursor_line_ends(buf);
        self.cursors.remove_except_primary();
        for (i, end) in ends.iter().enumerate() {
            if i == 0 {
                self.cursors.replace_primary(Cursor::new(*end));
            } else {
                self.cursors.push(Cursor::new(*end));
            }
        }
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
        let saved = match buf.save_rename() {
            Ok(res) => res,
            Err(e) => {
                self.error_msg(&format!("{e}"));
                return Err(e);
            }
        };
        let mark = self.cursors.mark_first(buf);
        let waux = self.window_aux(mark.into());
        let aux = buf.snapshot_aux_mut(saved.snapshot).unwrap();
        *aux = waux;
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
                Range::new(pos, npos)
            })
            .collect();

        self.remove(buf, &ranges)?;
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
                ranges.push(Range::new(start, end));
            }
        }

        self.remove(buf, &ranges)?;

        Ok(())
    }

    pub fn align_cursors(&mut self, buf: &mut Buffer) -> Result<()> {
        let slice = buf.slice(..);
        // Cursor positions per line,
        // entries are line index: (cursor position, whitespace start position)
        //
        // We want to add whitespaces next to other whitespaces. So this
        // can align right or left depending on the cursor position
        //
        // ||abba                  |    |abba
        // |  |babba        =>     |    |babba
        // |    |chabba            |    |chabba
        //
        // |abba|                  |           abba|
        // |  babbatwo|        =>  |       babbatwo|
        // |    chabbathree|       |    chabbathree|
        //
        let mut align: BTreeMap<u64, SortedVec<(u64, u64)>> = BTreeMap::default();

        for cursor in self.cursors().iter() {
            let lstart = start_of_line(&slice, cursor.pos());
            let entry = align.entry(lstart);
            let line = slice.slice(lstart..cursor.pos());
            let insert = find_prev_whitespace(&line, line.len())
                .map(|pos| pos)
                .unwrap_or(lstart);
            entry.or_default().push((cursor.pos(), insert));
        }

        let most_on_one_line = align
            .values()
            .map(|positions| positions.len())
            .max()
            .unwrap();
        // How much we have added to a line already
        let mut align_added: BTreeMap<u64, u64> = BTreeMap::default();
        let mut changes = vec![];

        // For each cursor on a line
        for i in 0..most_on_one_line {
            // Find the furthest cursor
            let mut furthest = 0;
            for (line_start, cursors) in &align {
                if let Some((pos, _)) = cursors.get(i) {
                    let added = align_added.entry(*line_start).or_default();

                    let dist = *pos - *line_start + *added;
                    furthest = max(furthest, dist);
                }
            }

            for (line_start, cursors) in &align {
                if let Some((pos, insert)) = cursors.get(i) {
                    let added = align_added.entry(*line_start).or_default();
                    let dist = *pos - *line_start + *added;
                    let pad = " ".repeat((furthest - dist) as usize);
                    let change = Change::insert(*insert, pad.as_bytes());

                    *added += pad.len() as u64;

                    changes.push(change);
                }
            }
        }

        let changes = Changes::new(&changes);
        self.change(buf, &changes)
    }

    pub fn cursors_to_next_snippet_jump(&mut self, buf: &Buffer) -> bool {
        while let Some(last) = self.snippets.last_mut() {
            match last.take() {
                Some(jumps) => {
                    let empty = last.is_empty();
                    self.cursors = jumps.to_cursors(buf);
                    self.ensure_cursor_on_grapheme_boundary(buf);

                    // Clear jumps if empty
                    if empty {
                        self.snippets.pop();
                    }
                    return true;
                }
                None => {
                    self.snippets.pop();
                }
            }
        }

        false
    }

    // Goto current jump, buffer provided should be the one in current jump
    pub fn goto_cursor_jump(&mut self, reference: Ref, buf: &Buffer) {
        let group = get!(self.cursor_jumps.goto(reference));

        debug_assert!(
            buf.id == group.buffer_id(),
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            group.buffer_id()
        );

        self.bid = group.buffer_id();
        self.cursors = group.to_cursors(buf);
        self.ensure_cursor_on_grapheme_boundary(buf);
        self.invalidate();

        self.view_to_around_cursor_zone(buf, Zone::Middle);
        self.invalidate();
    }

    pub fn cursors_to_prev_change(&mut self, buf: &Buffer) -> bool {
        self.cursors_to_prev_change_impl(buf).is_some()
    }

    fn cursors_to_prev_change_impl(&mut self, buf: &Buffer) -> Option<()> {
        let snaps = buf.snapshots();
        // Loop until we find a mark that has not been deleted
        loop {
            let aux = match self.last_edit_jump {
                Some(id) => {
                    let prev = snaps.prev_of(id)?;
                    self.last_edit_jump = Some(prev);
                    snaps.aux(prev)?
                }
                None => {
                    let id = snaps.current()?;
                    self.last_edit_jump = Some(id);
                    snaps.aux(id)?
                }
            };

            if let Some(mark) = aux.change_start {
                if let MarkResult::Found(pos) = buf.mark_to_pos(&mark) {
                    let cursor = Cursor::new(pos);
                    self.cursors = Cursors::new(cursor);
                    self.view.view_to(aux.view_offset, buf);
                    self.ensure_cursor_on_grapheme_boundary(buf);
                    self.invalidate();
                    return Some(());
                }
            }
        }
    }

    pub fn cursors_to_next_change(&mut self, buf: &Buffer) -> bool {
        self.cursors_to_next_change_impl(buf).is_some()
    }

    fn cursors_to_next_change_impl(&mut self, buf: &Buffer) -> Option<()> {
        let snaps = buf.snapshots();

        loop {
            let aux = match self.last_edit_jump {
                Some(id) => {
                    let prev = snaps.next_of(id)?;
                    self.last_edit_jump = Some(prev);
                    snaps.aux(prev)?
                }
                None => {
                    let id = snaps.current()?;
                    self.last_edit_jump = Some(id);
                    snaps.aux(id)?
                }
            };

            if let Some(mark) = aux.change_start {
                if let MarkResult::Found(pos) = buf.mark_to_pos(&mark) {
                    let cursor = Cursor::new(pos);
                    self.cursors = Cursors::new(cursor);
                    self.view.view_to(aux.view_offset, buf);
                    self.ensure_cursor_on_grapheme_boundary(buf);
                    self.invalidate();
                    return Some(());
                }
            }
        }
    }

    pub fn cursor_to_view_zone(&mut self, zone: Zone) -> bool {
        let line = match zone {
            Zone::Top => 1,
            Zone::Middle => self.view.height() / 2,
            Zone::Bottom => self.view.height().saturating_sub(2),
        };

        let mut pos = self.view.start();
        for i in 0..line {
            pos += self.view.line_len_in_buffer(i);
        }

        let primary = self.cursors.primary_mut();
        if primary.pos() != pos {
            primary.goto(pos);
            true
        } else {
            false
        }
    }

    pub fn join_lines(&mut self, buf: &mut Buffer, comment: &str) -> Result<()> {
        let ends = self.cursor_line_ends(buf);
        let slice = buf.slice(..);
        let mut changes = Vec::with_capacity(ends.len());

        for start in ends {
            let mut end = next_line_start(&slice, start);

            // Strip whitespace
            let mut graphemes = slice.graphemes_at(end);
            while let Some(g) = graphemes.next() {
                let cat = grapheme_category(&g);
                if cat != GraphemeCategory::Whitespace {
                    end = g.start();
                    break;
                }
            }

            // Strip comment
            let mut bytes = slice.bytes_at(end);
            let has_comment = comment
                .as_bytes()
                .iter()
                .all(|comment_byte| Some(*comment_byte) == bytes.next());
            if has_comment {
                end += comment.as_bytes().len() as u64;
            }

            // Strip whitespace
            let mut graphemes = slice.graphemes_at(end);
            while let Some(g) = graphemes.next() {
                let cat = grapheme_category(&g);
                if cat != GraphemeCategory::Whitespace {
                    end = g.start();
                    break;
                }
            }

            // Replace everything with one space
            changes.push(Change::replace(Range::new(start, end), b" "));
        }

        if changes.is_empty() {
            return Ok(());
        }

        let changes = Changes::from(changes);
        self.change(buf, &changes)?;
        Ok(())
    }
}

pub struct NextKeyFunction(pub Arc<dyn Fn(&mut Editor, ClientId, KeyEvent) -> ActionResult>);

impl std::fmt::Debug for NextKeyFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NextKeyFunction").finish()
    }
}
