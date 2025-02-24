mod chooser;
mod completion;
mod config;
mod cursors;
mod filetree;
mod focus;
mod jumps;
mod prompt;
mod search;
mod shell;
mod view;

#[cfg(test)]
mod tests;

use std::{
    cmp::{max, min},
    collections::BTreeMap,
    mem,
};

use anyhow::{bail, Result};
use rustc_hash::FxHashSet;
use sanedit_core::{
    grapheme_category, indent_at_line,
    movement::{
        end_of_line, next_grapheme_boundary, next_line_end, prev_grapheme_boundary, start_of_line,
    },
    selection_first_chars_of_lines, selection_line_ends, selection_line_starts, width_at_pos,
    BufferRange, Change, Changes, Cursor, DisplayOptions, GraphemeCategory, Locations, Range,
};
use sanedit_messages::{
    key::KeyEvent,
    redraw::{Popup, PopupMessage, Severity, Size, StatusMessage},
};

use crate::{
    common::change::{newline_autopair, newline_empty_line, newline_indent},
    editor::{
        buffers::{Buffer, BufferId, SnapshotAux, SnapshotId},
        keymap::KeymapKind,
    },
};

use self::filetree::FiletreeView;
pub(crate) use cursors::Cursors;

pub(crate) use self::{
    completion::*, config::*, focus::*, jumps::*, prompt::*, search::*, shell::*, view::*,
};

#[derive(Debug)]
pub(crate) struct Window {
    bid: BufferId,
    last_buffer: Option<(BufferId, SnapshotAux)>,
    message: Option<StatusMessage>,
    view: View,

    keys: Vec<KeyEvent>,

    /// Whether the this client is focused
    pub(crate) client_in_focus: bool,

    /// Focus determines where to direct input
    focus_stack: FocusStack,
    focus: Focus,
    popup: Option<Popup>,

    pub keymap_layer: String,
    pub shell_kind: ShellKind,
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
}

impl Window {
    pub fn new(bid: BufferId, width: usize, height: usize, config: WindowConfig) -> Window {
        Window {
            bid,
            keys: vec![],
            last_buffer: None,
            view: View::new(width, height),
            message: None,
            keymap_layer: KeymapKind::Window.as_ref().into(),
            shell_kind: ShellKind::default(),
            completion: Completion::default(),
            cursors: Cursors::default(),
            config,
            search: Search::default(),
            prompt: Prompt::default(),
            client_in_focus: true,
            focus: Focus::Window,
            focus_stack: FocusStack::default(),
            ft_view: FiletreeView::default(),
            locations: Locations::default(),
            popup: None,
            snippets: vec![],
            cursor_jumps: Jumps::default(),
            last_edit_jump: None,
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

    /// Focuses new element and pushes the old one to focus stack
    pub fn push_focus(&mut self, focus: Focus, layer: Option<String>) {
        let entry = FocusEntry {
            focus: self.focus,
            keymap_layer: self.keymap_layer.clone(),
        };
        self.focus_stack.push(entry);

        self.focus = focus;
        if let Some(layer) = layer {
            self.keymap_layer = layer;
        } else {
            let kind = match self.focus {
                Focus::Search => KeymapKind::Search,
                Focus::Prompt => KeymapKind::Prompt,
                Focus::Window => KeymapKind::Window,
                Focus::Completion => KeymapKind::Completion,
                Focus::Filetree => KeymapKind::Filetree,
                Focus::Locations => KeymapKind::Locations,
            };
            self.keymap_layer = kind.as_ref().into();
        }
    }

    /// Restores previously focused element
    pub fn pop_focus(&mut self) -> bool {
        if let Some(entry) = self.focus_stack.pop() {
            self.focus = entry.focus;
            self.keymap_layer = entry.keymap_layer;
            true
        } else {
            false
        }
    }

    /// Switches focus and changes to appropriate keymap layer,
    /// clears focus stack
    pub fn focus_to(&mut self, focus: Focus) {
        self.focus_stack.clear();

        self.focus = focus;

        let kind = match self.focus {
            Focus::Search => KeymapKind::Search,
            Focus::Prompt => KeymapKind::Prompt,
            Focus::Window => KeymapKind::Window,
            Focus::Completion => KeymapKind::Completion,
            Focus::Filetree => KeymapKind::Filetree,
            Focus::Locations => KeymapKind::Locations,
        };
        self.keymap_layer = kind.as_ref().into();
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
        let old = self.bid;
        // Store old buffer data
        let odata = self.window_aux();
        self.last_buffer = Some((old, odata));

        self.bid = bid;
        self.reload();
        old
    }

    pub fn goto_prev_buffer(&mut self) -> bool {
        match mem::take(&mut self.last_buffer) {
            Some((pbid, pdata)) => {
                let old = self.bid;
                let odata = self.window_aux();
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

    /// Called when buffer is changed and we should correct
    /// this window.
    pub fn on_buffer_changed(&mut self, buf: &Buffer) {
        self.ensure_cursor_on_grapheme_boundary(buf);

        // Redraw view
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
    fn window_aux(&self) -> SnapshotAux {
        SnapshotAux {
            cursors: self.cursors.clone(),
            view_offset: self.view.start(),
        }
    }

    fn remove(&mut self, buf: &mut Buffer, ranges: &[BufferRange]) -> Result<()> {
        let changes = Changes::multi_remove(ranges);
        self.change(buf, &changes)
    }

    pub fn change(&mut self, buf: &mut Buffer, changes: &Changes) -> Result<()> {
        let aux = self.window_aux();
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
        buf.apply_changes(&changes)?;
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
        let cursors = buf
            .last_edit()
            .map(|edit| Self::cursors_from_changes(&edit.changes))
            .unwrap_or(Cursors::default());

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
            let mut aux = self.window_aux();
            aux.cursors = cursors;
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
        Ok(())
    }

    // Restore aux data, if buffer is provided try to scroll to view position
    // otherwise hard set it
    fn restore(&mut self, aux: &SnapshotAux, buf: Option<&Buffer>) {
        // Clear highlights
        self.search.hl_matches = vec![];
        self.cursors = aux.cursors.clone();

        match buf {
            Some(buf) => self.view.view_to(aux.view_offset, buf),
            None => self.view.set_offset(aux.view_offset),
        }
        self.invalidate();
    }

    pub fn redo(&mut self, buf: &mut Buffer) -> Result<()> {
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

        log::info!("Changes {changes:?}");
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

    fn cursor_line_first_chars_of_lines(&self, buf: &Buffer) -> Vec<u64> {
        let slice = buf.slice(..);
        let mut starts = FxHashSet::default();

        for cursor in self.cursors.iter() {
            let cpos = cursor.pos();
            let sel = cursor.selection().unwrap_or(Range::new(cpos, cpos));
            let cstarts = selection_first_chars_of_lines(&slice, sel);
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

        let starts = self.cursor_line_first_chars_of_lines(buf);
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
        let aux = buf.snapshot_aux_mut(saved.snapshot).unwrap();
        *aux = self.window_aux();
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
        let mut align: BTreeMap<u64, Vec<u64>> = BTreeMap::default();

        for cursor in self.cursors().iter() {
            let lstart = start_of_line(&slice, cursor.pos());
            let entry = align.entry(lstart);
            entry.or_default().push(cursor.pos());
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
            for (pline, cursors) in &align {
                if let Some(pos) = cursors.get(i) {
                    let dist = *pos - *pline;
                    furthest = max(furthest, dist);
                }
            }

            for (pline, cursors) in &align {
                if let Some(pos) = cursors.get(i) {
                    let added = align_added.entry(*pline).or_default();

                    let dist = *pos - *pline;
                    let pad = " ".repeat((furthest.saturating_sub(dist + *added)) as usize);
                    let change = Change::insert(*pos, pad.as_bytes());

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
            match last.take_front() {
                Some(jumps) => {
                    let empty = last.is_empty();
                    self.cursors = jumps.to_cursors(buf);
                    self.ensure_cursor_on_grapheme_boundary(buf);

                    // Set keymap to snippet if jumped to next
                    self.push_focus(Focus::Window, Some(KeymapKind::Snippet.as_ref().into()));

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

    pub fn cursors_to_next_jump(&mut self, buf: &Buffer) -> bool {
        if let Some(group) = self.cursor_jumps.next() {
            self.cursors = group.to_cursors(buf);
            self.ensure_cursor_on_grapheme_boundary(buf);
            true
        } else {
            false
        }
    }

    pub fn cursors_to_prev_jump(&mut self, buf: &Buffer) -> bool {
        if let Some(group) = self.cursor_jumps.prev() {
            self.cursors = group.to_cursors(buf);
            self.ensure_cursor_on_grapheme_boundary(buf);
            true
        } else {
            false
        }
    }

    pub fn cursors_to_prev_change(&mut self, buf: &Buffer) -> bool {
        self.cursors_to_prev_change_impl(buf).is_some()
    }

    fn cursors_to_prev_change_impl(&mut self, buf: &Buffer) -> Option<()> {
        // TODO this works like shit, we need to record positions in a different way
        let snaps = buf.snapshots();
        // TODO some kind of X characters difference requirement?
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

        // Just copy first cursor
        let cursor = aux
            .cursors
            .iter()
            .min_by(|a, b| a.start().cmp(&b.start()))
            .cloned()
            .expect("No cursors in aux");
        self.cursors = Cursors::new(cursor);
        self.view.view_to(aux.view_offset, buf);
        self.invalidate();

        Some(())
    }

    pub fn cursors_to_next_change(&mut self, buf: &Buffer) -> bool {
        self.cursors_to_next_change_impl(buf).is_some()
    }

    fn cursors_to_next_change_impl(&mut self, buf: &Buffer) -> Option<()> {
        let snaps = buf.snapshots();
        let id = self.last_edit_jump.unwrap_or(snaps.current()?);
        let next = snaps.next_of(id)?;
        let aux = snaps.aux(next)?;

        let cursor = aux
            .cursors
            .iter()
            .min_by(|a, b| a.start().cmp(&b.start()))
            .cloned()
            .expect("No cursors in aux");
        self.cursors = Cursors::new(cursor);
        self.view.view_to(aux.view_offset, buf);
        self.invalidate();

        Some(())
    }
}
