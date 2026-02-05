mod chooser;
mod completion;
mod config;
mod cursors;
mod filetree;
mod focus;
pub(crate) mod games;
mod jumps;
mod locations;
mod macro_record;
mod mode;
mod mouse;
mod prompt;
mod search;
mod snapshot;
mod view;

#[cfg(test)]
mod test;

use std::{
    cmp::{max, min},
    collections::BTreeMap,
    mem,
    sync::Arc,
};

use anyhow::{bail, Result};
use games::Game;
pub(crate) use mouse::{Mouse, MouseClick};
use rustc_hash::FxHashSet as Set;
use sanedit_buffer::{utf8::next_eol, Mark, MarkResult};
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
    redraw::{Popup, PopupKind, PopupMessage, Severity, Size, StatusMessage},
};
use sanedit_server::ClientId;
use sanedit_utils::{ring::Ref, sorted_vec::SortedVec};

use crate::{
    actions::ActionResult,
    common::{
        change::{newline_autopair, newline_empty_line, newline_indent},
        text::{
            trim_comment_on_line, trim_comment_on_line_back, trim_whitespace, trim_whitespace_back,
        },
    },
    editor::{
        buffers::{Buffer, BufferId, SavedWindowState, SnapshotId},
        keymap::LayerKey,
        Editor, Map,
    },
};

use self::filetree::FiletreeView;
pub(crate) use cursors::Cursors;
pub(crate) use locations::LocationsView;

pub(crate) use self::{
    completion::*, config::*, focus::*, jumps::*, macro_record::*, mode::*, prompt::*, search::*,
    snapshot::*, view::*,
};

#[derive(Debug)]
pub(crate) struct Window {
    bid: BufferId,
    visited_buffers: Map<BufferId, SavedWindowState>,
    message: Option<StatusMessage>,
    view: View,
    keys: Vec<KeyEvent>,
    popup: Option<Popup>,

    pub last_selection: Option<Cursors>,
    pub last_buffer: Option<BufferId>,
    /// Focus determines where to direct input
    pub focus: Focus,
    pub mode: Mode,
    pub completion: Completion,
    pub cursors: Cursors,
    pub search: Search,
    pub prompt: Prompt,
    pub config: WindowConfig,
    pub ft_view: FiletreeView,
    pub snapshot_view: SnapshotView,
    pub locations: Locations<LocationsView>,
    pub snippets: Vec<Jumps<32>>,
    pub macro_record: MacroRecord,
    pub macro_replay: MacroReplay,
    pub mouse: Mouse,
    /// Cursor jumps across files
    pub cursor_jumps: Jumps<512>,
    /// Last edit jumped to in buffer
    pub last_edit_jump: Option<SnapshotId>,
    /// Handles next keypress, before anything else
    pub next_key_handler: Option<NextKeyFunction>,
    /// Delete indent when insert mode is left. Auto indenting changes should set this
    pub delete_indent_on_insert_leave: bool,
    pub game: Option<Box<dyn Game>>,
}

impl Window {
    pub fn new(bid: BufferId, width: usize, height: usize, config: WindowConfig) -> Window {
        Window {
            bid,
            keys: vec![],
            last_buffer: None,
            visited_buffers: Map::default(),
            last_selection: None,
            view: View::new(width, height),
            message: None,
            completion: Completion::default(),
            cursors: Cursors::default(),
            config,
            mode: Mode::Normal,
            search: Search::default(),
            prompt: Prompt::default(),
            focus: Focus::Window,
            snapshot_view: SnapshotView::default(),
            ft_view: FiletreeView::default(),
            locations: Locations::default(),
            popup: None,
            snippets: vec![],
            cursor_jumps: Jumps::default(),
            last_edit_jump: None,
            next_key_handler: None,
            delete_indent_on_insert_leave: false,
            mouse: Mouse::default(),
            game: None,
            macro_record: Default::default(),
            macro_replay: Default::default(),
        }
    }

    pub fn layer(&self) -> LayerKey {
        if self.focus == Focus::Window {
            LayerKey {
                focus: self.focus,
                mode: self.mode,
            }
        } else {
            LayerKey {
                focus: self.focus,
                mode: Mode::Normal,
            }
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
    pub fn push_popup(&mut self, msg: PopupMessage, kind: PopupKind) {
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
                    kind,
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

    pub fn full_reload(&mut self, buf: &Buffer) {
        self.view.invalidate();
        self.view.align_view_to_line(buf);
        self.ensure_cursor_on_grapheme_boundary(buf);
        self.reload();
    }

    pub fn reset(&mut self) {
        let width = self.view.width();
        let height = self.view.height();
        self.view = View::new(width, height);
        self.cursors = Cursors::default();
        self.last_selection = None;
        self.reload();
    }

    pub fn reload(&mut self) {
        self.search.reset_highlighting();
        self.focus = Focus::Window;
        self.view.invalidate();
        self.prompt = Prompt::default();
        self.message = None;
        self.completion = Completion::default();
        self.view.syntax = ViewSyntax::default();
    }

    pub fn display_options(&self) -> &DisplayOptions {
        &self.view.options
    }

    pub fn open_buffer(&mut self, buf: &Buffer) -> BufferId {
        if buf.id == self.bid {
            return self.bid;
        }

        let old = self.bid;
        self.swap_bid(buf.id);
        self.reset();
        self.cursor_jumps.goto_start();
        self.view.options.tabstop = buf.config.tabstop;
        if let Some(data) = self.visited_buffers.get(&self.bid).cloned() {
            self.restore(&data, buf);
        }
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
        let width = self
            .view
            .point_at_pos(cursor)
            .map(|point| point.x)
            .unwrap_or(0);
        self.view.set_offset(cursor);
        self.view.align_start(width, buf);

        match zone {
            Zone::Top => {}
            Zone::Middle => {
                let lines = (self.view.height() / 2) as u64;
                self.view.scroll_up_n(buf, lines);
            }
            Zone::Bottom => {
                let lines = self.view.height().saturating_sub(1) as u64;
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
        self.jump_to_offset(offset, buf);
    }

    pub fn push_new_cursor_jump(&mut self, buf: &Buffer) {
        let primary = self.cursors.primary().pos();
        let mark = buf.mark(primary);
        let jump = Jump::new(mark, None);
        let group = JumpGroup::new(self.bid, vec![jump]);
        self.cursor_jumps.push(group);
        self.cursor_jumps.goto_start();
    }

    /// Move primary cursor to offset and create a jump point at start and end position
    pub fn jump_to_offset(&mut self, offset: u64, buf: &Buffer) {
        self.push_new_cursor_jump(buf);
        self.goto_offset(offset, buf);
    }

    /// Move primary cursor to offset and create a jump to it.
    /// Will not create the jump point at current position
    pub fn goto_offset(&mut self, offset: u64, buf: &Buffer) {
        {
            let offset = min(offset, buf.len());
            let mut cursors = self.cursors.cursors_mut();
            let primary = cursors.primary();
            primary.goto(offset);
        }

        self.ensure_cursor_on_grapheme_boundary(buf);
        self.view_to_around_cursor_zone(buf, Zone::Middle);
        self.push_new_cursor_jump(buf);
    }

    pub fn ensure_cursor_on_grapheme_boundary(&mut self, buf: &Buffer) {
        // Ensure cursor in buf range
        self.cursors.contain_to(0..buf.len());

        // Ensure cursor in buf grapheme boundary
        let mut cursors = self.cursors.cursors_mut();
        let primary = cursors.primary();
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

    fn on_buffer_changed_undo_redo(&mut self, buf: &Buffer) {
        self.view.syntax = ViewSyntax::default();

        let Some(edit) = buf.last_edit() else {
            return;
        };
        let old = &edit.buf;
        let vstart = self.view.start();
        let mark = old.mark(vstart);
        let pos = buf.mark_to_pos(&mark).pos();
        self.view.set_offset(pos);
        self.view.align_view_to_line(buf);

        {
            let mut cursors = self.cursors.cursors_mut();
            for cursor in cursors.iter_mut() {
                let pos = cursor.pos();
                match cursor.selection() {
                    Some(range) => {
                        let mark = old.mark(range.start);
                        let start_nmark_pos = buf.mark_to_pos(&mark);

                        let mark = old.mark(range.end);
                        let end_nmark_pos = buf.mark_to_pos(&mark);

                        if start_nmark_pos.is_found() && end_nmark_pos.is_found() {
                            cursor.select(start_nmark_pos.pos()..end_nmark_pos.pos());
                            if range.end != pos {
                                cursor.swap_selection_dir();
                            }
                        } else {
                            cursor.stop_selection();
                            cursor.goto(start_nmark_pos.pos());
                        }
                    }
                    _ => {
                        let mark = old.mark(pos);
                        let nmark_pos = buf.mark_to_pos(&mark);
                        cursor.goto(nmark_pos.pos());
                    }
                }
            }
        }

        self.ensure_cursor_on_grapheme_boundary(buf);
        self.view.invalidate();
        self.view.redraw(buf);
    }

    /// Called when buffer is changed by another client and we should correct
    /// this window.
    pub fn on_buffer_changed(&mut self, buf: &Buffer) {
        let Some(edit) = buf.last_edit() else {
            return;
        };
        let changes = &edit.changes;
        if changes.is_undo() || changes.is_redo() {
            self.on_buffer_changed_undo_redo(buf);
            return;
        }

        {
            let mut cursors = self.cursors.cursors_mut();
            changes.move_cursors(&mut cursors, self.mode == Mode::Select);
        }

        self.ensure_cursor_on_grapheme_boundary(buf);

        // Redraw view
        let offset = changes.move_offset(self.view().start());
        if offset != self.view().start() {
            self.view.set_offset(offset);
            self.view.align_view_to_line(buf);
        }

        self.view.invalidate();
        self.view.redraw(buf);
    }

    pub fn buffer_id(&self) -> BufferId {
        self.bid
    }

    fn swap_bid(&mut self, new: BufferId) {
        if new == self.bid {
            return;
        }

        let old = self.bid;
        // Store old buffer data
        let odata = self.save_window_state(None);
        self.visited_buffers.insert(old, odata);
        self.last_buffer = Some(old);
        self.bid = new;

        self.view.syntax = ViewSyntax::default();
        self.search.reset_highlighting();
    }

    pub fn goto_view_offset(&mut self, offset: u64, buf: &Buffer) {
        self.view.set_offset(offset);
        self.goto_offset(offset, buf);
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
        self.game = None;
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

        self.ensure_cursor_on_grapheme_boundary(buf);
        self.view.redraw(buf);
    }

    /// Create snapshot additinal data for window
    /// Provide mark to store in aux
    fn save_window_state(&self, mark: Option<Mark>) -> SavedWindowState {
        SavedWindowState {
            cursors: self.cursors.clone(),
            view_offset: self.view.start(),
            change_start: mark,
            last_selection: self.last_selection.clone(),
        }
    }

    fn remove(&mut self, buf: &mut Buffer, ranges: &[BufferRange]) -> Result<()> {
        if ranges.is_empty() {
            return Ok(());
        }

        let changes = Changes::multi_remove(ranges);
        self.change(buf, &changes)
    }

    pub fn change(&mut self, buf: &mut Buffer, changes: &Changes) -> Result<()> {
        self.delete_indent_on_insert_leave = false;
        self.last_edit_jump = None;
        self.cursor_jumps.goto_start();

        let mark = self.cursors.mark_first(buf);
        let aux = self.save_window_state(mark.into());
        let result = buf.apply_changes(changes)?;

        {
            let mut cursors = self.cursors.cursors_mut();
            changes.move_cursors(&mut cursors, self.mode == Mode::Select);
        }

        let offset = changes.move_offset(self.view().start());
        if offset != self.view().start() {
            self.view.set_offset(offset);
        }

        if let Some(id) = result.created_snapshot {
            *buf.snapshot_additional_mut(id).unwrap() = aux;
        } else if let Some(id) = result.forked_snapshot {
            *buf.snapshot_additional_mut(id).unwrap() = aux;
        }

        self.view.invalidate();

        Ok(())
    }

    pub fn remove_cursor_selections(&mut self, buf: &mut Buffer) -> Result<bool> {
        let selections: Vec<BufferRange> = (&self.cursors).into();
        if selections.is_empty() {
            return Ok(false);
        }

        self.stop_selection();
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
                Range::from(pos..next)
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
        {
            let mut all = cursors.cursors_mut();

            let mut diff: i128 = 0;
            for change in changes.iter() {
                let total = change.text().len() as i128 - change.range().len() as i128;
                diff += total;

                let mut pos = change.range().end;
                if total < 0 {
                    pos -= diff.unsigned_abs() as u64;
                } else {
                    pos += diff.unsigned_abs() as u64;
                }

                all.push(Cursor::new(pos));
            }

            if all.len() != 1 {
                all.remove_primary();
            }
        }

        cursors
    }

    pub fn undo_jump(&mut self, buf: &mut Buffer, snapshot: SnapshotId) -> Result<()> {
        self.last_edit_jump = None;
        let aux = {
            let cursors = buf
                .last_edit()
                .map(|edit| Self::cursors_from_changes(&edit.changes))
                .unwrap_or_default();
            let mark = cursors.mark_first(buf);

            SavedWindowState {
                cursors,
                view_offset: self.view.start(),
                change_start: mark.into(),
                last_selection: self.last_selection.clone(),
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
            *buf.snapshot_additional_mut(id).unwrap() = aux;
        }

        if let Some(restored) = restored {
            if let Some(data) = buf.snapshot_additional(restored) {
                self.restore(data, buf);
            } else {
                self.full_reload(buf);
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
                .unwrap_or_default();
            let mark = cursors.mark_first(buf);

            SavedWindowState {
                cursors,
                view_offset: self.view.start(),
                change_start: mark.into(),
                last_selection: self.last_selection.clone(),
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
            *buf.snapshot_additional_mut(id).unwrap() = aux;
        }

        if let Some(restored) = restored {
            if let Some(data) = buf.snapshot_additional(restored) {
                self.restore(data, buf);
            } else {
                self.full_reload(buf);
            }
        }

        self.invalidate();
        self.view_to_cursor(buf);
        Ok(())
    }

    // Restore aux data, if buffer is provided try to scroll to view position
    // otherwise hard set it
    fn restore(&mut self, aux: &SavedWindowState, buf: &Buffer) {
        *self.view_syntax() = ViewSyntax::default();
        self.search.reset_highlighting();
        self.cursors = aux.cursors.clone();
        self.ensure_cursor_on_grapheme_boundary(buf);
        self.invalidate();

        self.view.set_offset(aux.view_offset);
        self.view.align_view_to_line(buf);
        self.view_to_around_cursor_zone(buf, Zone::Middle);
        self.invalidate();

        self.last_selection = aux.last_selection.clone();
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
            if let Some(data) = buf.snapshot_additional(restored) {
                self.restore(data, buf);
            } else {
                self.full_reload(buf)
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
                ranges.push(Range::from(pos..cpos));
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
        let result = self.change(buf, &changes);
        self.delete_indent_on_insert_leave = true;
        result
    }

    pub fn cursors_to_eol(&self, buf: &Buffer) -> Vec<BufferRange> {
        let mut lines = vec![];
        let slice = buf.slice(..);
        let mut starts: Vec<u64> = self
            .cursors()
            .cursors()
            .iter()
            .map(|cursor| cursor.pos())
            .collect();
        starts.sort();

        let mut high = 0;

        for start in starts {
            if high >= start {
                continue;
            }

            let end = end_of_line(&slice, start);
            lines.push(Range::from(start..end));
            high = end;
        }

        lines
    }

    pub fn cursor_lines(&self, buf: &Buffer) -> Vec<BufferRange> {
        let mut lines = vec![];
        let slice = buf.slice(..);
        let starts = self.cursor_line_starts(buf);
        for start in starts {
            let next = end_of_line(&slice, start);
            if next != start {
                lines.push(Range::from(start..next));
            }
        }

        lines
    }

    pub fn cursor_line_starts(&self, buf: &Buffer) -> Vec<u64> {
        let slice = buf.slice(..);
        let mut starts = Set::default();

        for cursor in self.cursors.iter() {
            let cpos = cursor.pos();
            let sel = cursor.selection().unwrap_or(Range::from(cpos..cpos));
            let cstarts = selection_line_starts(&slice, sel);
            starts.extend(cstarts);
        }
        let mut vstarts: Vec<u64> = starts.into_iter().collect();
        vstarts.sort();
        vstarts
    }

    pub fn cursor_line_ends(&self, buf: &Buffer) -> Vec<u64> {
        let slice = buf.slice(..);
        let mut endset = Set::default();

        for cursor in self.cursors.iter() {
            let cpos = cursor.pos();
            let sel = cursor.selection().unwrap_or(Range::from(cpos..cpos));
            let ends = selection_line_ends(&slice, sel);
            endset.extend(ends);
        }
        let mut ends: Vec<u64> = endset.into_iter().collect();
        ends.sort();
        ends
    }

    fn cursor_line_first_chars_of_lines_aligned(&self, buf: &Buffer) -> Vec<u64> {
        let slice = buf.slice(..);
        let mut starts = Set::default();
        let mut dist = u64::MAX;

        for cursor in self.cursors.iter() {
            let cpos = cursor.pos();
            let sel = cursor.selection().unwrap_or(Range::from(cpos..cpos));
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
                    ranges.push(Range::from(pos..pos + off));
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
        if indent.is_empty() {
            bail!("No lines to indent");
        }
        let changes = Changes::multi_insert(&starts, indent.as_bytes());
        self.change(buf, &changes)?;
        Ok(())
    }

    pub fn comment_cursor_lines(
        &mut self,
        buf: &mut Buffer,
        comment: &str,
        comment_end: &str,
    ) -> Result<()> {
        if comment.is_empty() {
            return Ok(());
        }

        let starts = self.cursor_line_first_chars_of_lines_aligned(buf);
        let ends = self.cursor_line_ends(buf);

        self.stop_selection();
        if starts.is_empty() {
            bail!("No lines to comment");
        }

        if comment_end.is_empty() {
            let changes = Changes::multi_insert(&starts, comment.as_bytes());
            return self.change(buf, &changes);
        }

        let mut changes = Vec::with_capacity(starts.len() * 2);
        for start in starts {
            let change = Change::insert(start, comment.as_bytes());
            changes.push(change);
        }

        for end in ends {
            let change = Change::insert(end, comment_end.as_bytes());
            changes.push(change);
        }

        let changes = Changes::from(changes);
        self.change(buf, &changes)
    }

    pub fn uncomment_cursor_lines(
        &mut self,
        buf: &mut Buffer,
        comment: &str,
        comment_end: &str,
    ) -> Result<()> {
        if comment.is_empty() {
            return Ok(());
        }

        let starts = self.cursor_line_starts(buf);
        let slice = buf.slice(..);

        let mut changes = vec![];

        for start in starts {
            let end = end_of_line(&slice, start);
            let line = buf.slice(start..end);
            let line = trim_whitespace(&line);
            let Some(start) = trim_comment_on_line(&line, comment) else {
                continue;
            };
            let start_change = Change::remove(line.start()..start.start());

            if comment_end.is_empty() {
                changes.push(start_change);
                continue;
            }

            let Some(end) = trim_comment_on_line_back(&line, comment_end) else {
                continue;
            };
            let end_change = Change::remove(end.end()..line.end());
            changes.push(start_change);
            changes.push(end_change);
        }

        self.stop_selection();
        if changes.is_empty() {
            bail!("No lines to uncomment");
        }
        let changes = Changes::new(&changes);
        self.change(buf, &changes)
    }

    pub fn stop_selection(&mut self) {
        if self.cursors.has_selections() {
            self.last_selection = Some(self.cursors.clone());
            let mut cursors = self.cursors.cursors_mut();
            for cursor in cursors.iter_mut() {
                cursor.stop_selection();
            }
        }
    }

    fn has_comment_on_line(&self, buf: &Buffer, comment: &str, start_of_line: u64) -> bool {
        let slice = buf.slice(..);
        let end = end_of_line(&slice, start_of_line);
        let line = buf.slice(start_of_line..end);
        trim_comment_on_line(&line, comment).is_some()
    }

    pub fn toggle_comment_cursor_lines(
        &mut self,
        buf: &mut Buffer,
        comment: &str,
        comment_end: &str,
    ) -> Result<()> {
        let starts = self.cursor_line_starts(buf);
        let has_uncommented_line = starts
            .iter()
            .any(|start| !self.has_comment_on_line(buf, comment, *start));

        if has_uncommented_line {
            self.comment_cursor_lines(buf, comment, comment_end)
        } else {
            self.uncomment_cursor_lines(buf, comment, comment_end)
        }
    }

    pub fn cursors_to_lines_start(&mut self, buf: &Buffer) {
        let starts = self.cursor_line_starts(buf);
        let mut cursors = self.cursors.cursors_mut();
        cursors.remove_except_primary();
        for (i, start) in starts.iter().enumerate() {
            if i == 0 {
                cursors.replace_primary(Cursor::new(*start));
            } else {
                cursors.push(Cursor::new(*start));
            }
        }
    }

    pub fn cursors_to_lines_end(&mut self, buf: &Buffer) {
        let ends = self.cursor_line_ends(buf);
        let mut cursors = self.cursors.cursors_mut();
        cursors.remove_except_primary();
        for (i, end) in ends.iter().enumerate() {
            if i == 0 {
                cursors.replace_primary(Cursor::new(*end));
            } else {
                cursors.push(Cursor::new(*end));
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
        let waux = self.save_window_state(mark.into());
        let aux = buf.snapshot_additional_mut(saved.snapshot).unwrap();
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
                Range::from(pos..npos)
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
                    GraphemeCategory::Eol => {
                        end = g.start();
                    }
                    GraphemeCategory::Whitespace => start = Some(g.start()),
                    _ => break,
                }
            }

            if let (Some(start), end) = (start, end) {
                ranges.push(Range::from(start..end));
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

        let mut sorted_cursors = SortedVec::with_capacity(self.cursors.len());
        for cursor in self.cursors().iter() {
            sorted_cursors.push(cursor.pos());
        }

        for cursor in sorted_cursors.iter() {
            let lstart = start_of_line(&slice, *cursor);
            let entry = align.entry(lstart);
            let line = slice.slice(lstart..*cursor);
            let mut insert = find_prev_whitespace(&line, line.len()).unwrap_or(lstart);
            // must ensure we are not inserting to same position for 2 different cursors
            let value = entry.or_default();
            if value.iter().any(|(_, opos)| opos == &insert) {
                insert = *cursor;
            }
            value.push((*cursor, insert));
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
        let bid = get!(self.cursor_jumps.goto(reference.clone())).buffer_id();

        debug_assert!(
            buf.id == bid,
            "Invalid buffer provided to window got id {:?}, expected {:?}",
            buf.id,
            bid
        );

        self.swap_bid(bid);
        let group = get!(self.cursor_jumps.goto(reference));
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
                    snaps.window_state(prev)?
                }
                None => {
                    let id = snaps.current()?;
                    self.last_edit_jump = Some(id);
                    snaps.window_state(id)?
                }
            };

            if let Some(mark) = aux.change_start {
                if let MarkResult::Found(pos) = buf.mark_to_pos(&mark) {
                    let cursor = Cursor::new(pos);
                    self.cursors = Cursors::new(cursor);
                    self.ensure_cursor_on_grapheme_boundary(buf);
                    self.view_to_around_cursor_zone(buf, Zone::Middle);
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
                    let next = snaps.next_of(id)?;
                    self.last_edit_jump = Some(next);
                    snaps.window_state(next)?
                }
                None => {
                    let id = snaps.current()?;
                    self.last_edit_jump = Some(id);
                    snaps.window_state(id)?
                }
            };

            if let Some(mark) = aux.change_start {
                if let MarkResult::Found(pos) = buf.mark_to_pos(&mark) {
                    let cursor = Cursor::new(pos);
                    self.cursors = Cursors::new(cursor);
                    self.ensure_cursor_on_grapheme_boundary(buf);
                    self.view_to_around_cursor_zone(buf, Zone::Middle);
                    return Some(());
                }
            }
        }
    }

    pub fn cursor_to_view_zone(&mut self, zone: Zone) -> bool {
        let line = match zone {
            Zone::Top => 0,
            Zone::Middle => self.view.height() / 2,
            Zone::Bottom => self.view.height().saturating_sub(1),
        };

        let mut pos = self.view.start();
        for i in 0..line {
            pos += self.view.line_len_in_buffer(i);
        }

        let mut cursors = self.cursors.cursors_mut();
        let primary = cursors.primary();
        if primary.pos() != pos {
            primary.goto(pos);
            true
        } else {
            false
        }
    }

    pub fn join_lines(&mut self, buf: &mut Buffer, comment: &str, comment_end: &str) -> Result<()> {
        let ends = self.cursor_line_ends(buf);
        let slice = buf.slice(..);
        let has_comment_on_line = ends
            .first()
            .map(|end| {
                let start = start_of_line(&slice, *end);
                self.has_comment_on_line(buf, comment, start)
            })
            .unwrap_or(false);
        let mut changes = Vec::with_capacity(ends.len());

        for start in ends {
            let end = next_line_start(&slice, start);
            let eol = end_of_line(&slice, end);
            let mut nline = buf.slice(end..eol);
            nline = trim_whitespace(&nline);
            nline = trim_whitespace_back(&nline);
            if has_comment_on_line {
                if let Some(line) = trim_comment_on_line(&nline, comment) {
                    if comment_end.is_empty() {
                        nline = line;
                    } else if let Some(line) = trim_comment_on_line_back(&line, comment_end) {
                        nline = line;
                    }
                }
            }

            // Replace everything with one space
            changes.push(Change::replace(start..nline.start(), b" "));
            if eol != nline.end() {
                changes.push(Change::remove(nline.end()..eol));
            }
        }

        if changes.is_empty() {
            bail!("No changes");
        }

        let changes = Changes::from(changes);
        self.change(buf, &changes)?;
        Ok(())
    }

    pub fn cursor_trim_whitespace(&mut self, buf: &Buffer) -> bool {
        let changed = self.cursors.trim_whitespace(buf);
        if changed {
            self.invalidate();
        }
        changed
    }

    pub fn cursor_sort(&mut self, buf: &mut Buffer, reverse: bool) -> Result<()> {
        let mut sorted_cursors = SortedVec::with_capacity(self.cursors.len());
        for cursor in self.cursors().iter() {
            if let Some(sel) = cursor.selection() {
                sorted_cursors.push(sel);
            }
        }

        let mut strings = Vec::with_capacity(self.cursors.len());
        for range in sorted_cursors.iter() {
            let slice = buf.slice(range);
            let string = String::from(&slice);
            strings.push(string);
        }

        let is_sorted =
            reverse && strings.is_sorted_by(|a, b| b < a) || !reverse && strings.is_sorted();
        if is_sorted {
            bail!("Already sorted")
        }

        if reverse {
            strings.sort_by(|a, b| b.cmp(a));
        } else {
            strings.sort();
        }

        let changes: Vec<Change> = strings
            .into_iter()
            .enumerate()
            .map(|(i, string)| {
                let range = &sorted_cursors[i];
                Change::replace(range, string.as_bytes())
            })
            .collect();

        if changes.is_empty() {
            bail!("No selections")
        }
        let changes = Changes::new(&changes);
        self.change(buf, &changes)?;
        Ok(())
    }

    pub fn uppercase_selections(&mut self, buf: &mut Buffer) -> Result<()> {
        let mut changes = vec![];

        for cursor in self.cursors.cursors() {
            if let Some(range) = cursor.selection() {
                let mut did_change = false;
                let mut uppercase = String::new();
                let slice = buf.slice(range);
                let mut chars = slice.chars();
                while let Some((_, _, ch)) = chars.next() {
                    did_change |= ch.is_lowercase();
                    for nch in ch.to_uppercase() {
                        uppercase.push(nch);
                    }
                }

                if did_change {
                    changes.push(Change::replace(range, uppercase.as_bytes()));
                }
            }
        }

        if changes.is_empty() {
            bail!("No changes");
        }

        let changes = Changes::from(changes);
        self.change(buf, &changes)?;
        Ok(())
    }

    pub fn lowercase_selections(&mut self, buf: &mut Buffer) -> Result<()> {
        let mut changes = vec![];

        for cursor in self.cursors.cursors() {
            if let Some(range) = cursor.selection() {
                let mut did_change = false;
                let mut lowercase = String::new();
                let slice = buf.slice(range);
                let mut chars = slice.chars();
                while let Some((_, _, ch)) = chars.next() {
                    did_change |= ch.is_uppercase();
                    for nch in ch.to_lowercase() {
                        lowercase.push(nch);
                    }
                }

                if did_change {
                    changes.push(Change::replace(range, lowercase.as_bytes()));
                }
            }
        }

        if changes.is_empty() {
            bail!("No changes");
        }

        let changes = Changes::from(changes);
        self.change(buf, &changes)?;
        Ok(())
    }

    pub fn rotate_selections(&mut self, buf: &mut Buffer, reverse: bool) -> Result<()> {
        let mut changes = vec![];

        let cursors = self.cursors.cursors();
        let selecting: Vec<BufferRange> = cursors
            .iter()
            .filter(|c| c.is_selecting())
            .map(|c| c.selection().unwrap())
            .collect();
        if selecting.len() < 2 {
            bail!("Not enough selections to rotate")
        }

        if reverse {
            let mut last = &selecting[0];
            for range in selecting.iter().rev() {
                let text: Vec<u8> = (&buf.slice(last)).into();
                let change = Change::replace(range, &text);
                changes.push(change);
                last = range;
            }
        } else {
            let mut last = selecting.last().unwrap();
            for range in &selecting {
                let text: Vec<u8> = (&buf.slice(last)).into();
                let change = Change::replace(range, &text);
                changes.push(change);
                last = range;
            }
        }

        if changes.is_empty() {
            bail!("No changes");
        }

        let changes = Changes::from(changes);
        self.change(buf, &changes)?;
        Ok(())
    }

    pub fn reindent(&mut self, buf: &mut Buffer) -> Result<()> {
        let slice = buf.slice(..);
        let mut changes = vec![];
        let mut graphemes = slice.graphemes();
        let mut stack: Vec<(usize, usize)> = vec![(0, 0)];
        let mut level = 0;
        let mut ch_found = false;
        let mut start = 0;

        while let Some(grapheme) = graphemes.next() {
            if grapheme.is_eol() {
                start = grapheme.end();
                level = 0;
                ch_found = false;
                continue;
            }

            if ch_found {
                continue;
            }

            if grapheme == "\t" {
                level += buf.config.tabstop as usize;
            } else if grapheme == " " {
                level += 1;
            } else {
                ch_found = true;

                let (mut plevel, mut pn) = *stack.last().unwrap();
                while plevel > level {
                    (plevel, pn) = stack.pop().unwrap();
                }

                let mut n = pn;
                if level > plevel {
                    n += 1;
                }

                stack.push((level, n));

                let ilevel = n * buf.config.indent_amount as usize;
                let indent = buf.config.indent_kind.repeat(ilevel);
                if start == grapheme.start() && indent.is_empty() {
                    continue;
                }
                let change = Change::replace(start..grapheme.start(), indent.as_bytes());
                changes.push(change);
            }
        }

        if changes.is_empty() {
            bail!("No changes");
        }

        let changes = Changes::from(changes);
        self.change(buf, &changes)
    }

    pub fn set_eols(&mut self, buf: &mut Buffer) -> Result<()> {
        let target_eol = buf.config.eol;
        let slice = buf.slice(..);
        let mut bytes = slice.bytes();
        let mut changes = vec![];

        while let Some(mat) = next_eol(&mut bytes) {
            if mat.eol != target_eol {
                changes.push(Change::replace(mat.range, target_eol.as_ref()))
            }
        }

        if changes.is_empty() {
            bail!("No changes");
        }

        let changes = Changes::from(changes);
        self.change(buf, &changes)?;
        Ok(())
    }
}

pub struct NextKeyFunction(pub Arc<NextKey>);
type NextKey = dyn Fn(&mut Editor, ClientId, KeyEvent) -> ActionResult;

impl std::fmt::Debug for NextKeyFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NextKeyFunction").finish()
    }
}
