use crate::{
    actions::{hooks::run, shell},
    common::to_human_readable,
    editor::{
        buffers::Buffer,
        hooks::Hook,
        windows::{Focus, Mode, Zone},
        Editor,
    },
    VERSION,
};

use sanedit_buffer::utf8::EndOfLine;
use sanedit_core::{Change, Changes, IndentKind, Range};
use sanedit_server::ClientId;

use super::{
    hooks,
    movement::{end_of_line, first_char_of_line, next_grapheme_on_line, prev_grapheme_on_line},
    ActionResult,
};

/// Change focus and keymap and run hooks
pub fn focus_with_mode(editor: &mut Editor, id: ClientId, focus: Focus, mode: Mode) {
    let (win, _buf) = editor.win_buf_mut(id);
    let same_mode = win.mode == mode;
    if same_mode && win.focus() == focus {
        return;
    }

    if !same_mode {
        hooks::run(editor, id, Hook::ModeLeave);
    }

    let (win, _buf) = editor.win_buf_mut(id);
    win.focus = focus;
    win.mode = mode;

    if !same_mode {
        hooks::run(editor, id, Hook::ModeEnter);
    }
}

pub fn mode_normal(editor: &mut Editor, id: ClientId) {
    mode(editor, id, Mode::Normal);
}

pub fn mode_select(editor: &mut Editor, id: ClientId) {
    mode(editor, id, Mode::Select);
}

pub fn mode_insert(editor: &mut Editor, id: ClientId) {
    mode(editor, id, Mode::Insert);
}

pub fn mode(editor: &mut Editor, id: ClientId, mode: Mode) {
    focus_with_mode(editor, id, Focus::Window, mode);
}

/// Change focus and run hooks
pub fn focus(editor: &mut Editor, id: ClientId, focus: Focus) {
    let (win, _buf) = editor.win_buf(id);
    let mode = win.mode;
    focus_with_mode(editor, id, focus, mode)
}

#[action("Window: Focus window")]
fn focus_window(editor: &mut Editor, id: ClientId) -> ActionResult {
    focus(editor, id, Focus::Window);
    ActionResult::Ok
}

#[action("Window: Reload")]
fn reload_window(editor: &mut Editor, id: ClientId) -> ActionResult {
    editor.reload(id);
    ActionResult::Ok
}

#[action("Window: Clear messages")]
fn clear_messages(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.clear_msg();
    ActionResult::Ok
}

#[action("Sync windows if a buffer is changed")]
fn sync_windows(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (_win, buf) = editor.win_buf(id);
    let bid = buf.id;
    let bid = editor
        .hooks
        .running_hook()
        .and_then(Hook::buffer_id)
        .unwrap_or(bid);
    let clients = editor.windows().find_clients_with_buf(bid);

    for client in clients {
        if id != client {
            let (win, buf) = editor.win_buf_mut(client);
            win.on_buffer_changed(buf);
        }
    }

    ActionResult::Ok
}

#[action("Window: Goto previous buffer")]
fn goto_prev_buffer(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    if win.goto_prev_buffer() {
        let hook = Hook::BufEnter(buf.id);
        hooks::run(editor, id, hook);
        ActionResult::Ok
    } else {
        win.warn_msg("No previous buffer");
        ActionResult::Failed
    }
}

#[action("Window: Cancel")]
fn cancel(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);

    if win.search.is_highlighting_enabled() || win.popup().is_some() {
        // Clear search matches
        win.search.disable_highlighting();

        // Close popups
        win.clear_popup();
        return ActionResult::Ok;
    }

    if win.cursors.cursors().iter().any(|c| c.is_selecting()) {
        win.cursors.stop_selection();
    } else if win.cursors().len() > 1 {
        win.cursors.remove_except_primary();

        let (win, _buf) = editor.win_buf_mut(id);
        win.cursors.primary_mut().stop_selection();
    }

    ActionResult::Ok
}

#[action("Window: New")]
fn new_window(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf(id);
    let command = win.window_manager.new_window();
    shell::execute(editor, id, false, &command)
}

#[action("Editor: Status")]
fn status(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf(id);
    let file = buf
        .path()
        .map(|path| {
            path.strip_prefix(editor.working_dir())
                .unwrap_or(path)
                .display()
                .to_string()
                .into()
        })
        .unwrap_or(buf.name());
    let lang = buf
        .language
        .as_ref()
        .map(|ft| ft.as_str())
        .unwrap_or("no language");
    let bufsize = to_human_readable(buf.len() as f64);
    let edits = buf.total_changes_made();
    let (lsp, command, args) = buf
        .language
        .as_ref()
        .map(|ft| {
            let lsp = editor.language_servers.get(ft)?;
            let name = lsp.server_name();
            let config = editor.languages.get(&ft)?;
            let command = config.language_server.command.as_str();
            let args = config.language_server.args.clone();
            Some((name, command, args))
        })
        .flatten()
        .unwrap_or(("no", "-", vec![]));
    let options = &win.view().options;
    let width = options.width;
    let height = options.height;
    let client_id = id.0;
    let working_dir = editor.working_dir();
    let shell = &editor.config.editor.shell;
    let config_dir = editor.config_dir.root();
    let istyle = format!("{:?}", buf.config.indent_kind);
    let iamount = &buf.config.indent_amount;
    let eol = format!("{:?}", buf.config.eol);

    let text = format!(
        "\
        SanEdit v{VERSION}\n\
        --------------\n\
        \n\
        Buffer:\n  \
        File: {file}\n  \
        Language: {lang}\n  \
        Indent style: {istyle}\n  \
        Indent amount: {iamount}\n  \
        End of line: {eol}\n  \
        Size: {bufsize}\n  \
        Edits made: {edits}\n\
        \n\
        Language server: ({lang})\n  \
        Running: {lsp}\n  \
        Start command: {command} {args:?}\n\
        \n\
        Window:\n  \
        Client ID: {client_id}\n  \
        Size: {width}x{height}\n\
        \n\
        Editor:\n  \
        Working directory: {working_dir:?}\n  \
        Config directory: {config_dir:?}\n  \
        Shell: {shell}\n\
    "
    );

    let mut buf = Buffer::from_reader(std::io::Cursor::new(text)).unwrap();
    buf.read_only = true;
    let bid = editor.buffers_mut().insert(buf);
    editor.open_buffer(id, bid);

    ActionResult::Ok
}

#[action("View: Move to cursor")]
fn view_to_cursor(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    win.view_to_cursor(buf);
    ActionResult::Ok
}

#[action("View: Move to cursor high")]
fn view_to_cursor_top(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    win.view_to_cursor_zone(buf, Zone::Top);
    ActionResult::Ok
}

#[action("View: Move to cursor middle")]
fn view_to_cursor_middle(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    win.view_to_cursor_zone(buf, Zone::Middle);
    ActionResult::Ok
}

#[action("View: Move to cursor bottom")]
fn view_to_cursor_bottom(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = editor.win_buf_mut(id);
    win.view_to_cursor_zone(buf, Zone::Bottom);
    ActionResult::Ok
}

#[action("LSP: Show diagnostic highlights")]
fn show_diagnostic_highlights(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.config.highlight_diagnostics = true;
    ActionResult::Ok
}

#[action("LSP: Hide diagnostic higlights")]
fn hide_diagnostic_highlights(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.config.highlight_diagnostics = false;
    ActionResult::Ok
}

#[action("Mode: On mode enter")]
fn on_mode_enter(editor: &mut Editor, id: ClientId) -> ActionResult {
    let Some(layer) = editor.layer(id) else {
        return ActionResult::Failed;
    };
    if let Some(action) = layer.on_enter.clone() {
        action.execute(editor, id);
    }
    ActionResult::Ok
}

#[action("Mode: On mode leave")]
fn on_mode_leave(editor: &mut Editor, id: ClientId) -> ActionResult {
    let Some(layer) = editor.layer(id) else {
        return ActionResult::Failed;
    };
    if let Some(action) = layer.on_leave.clone() {
        action.execute(editor, id);
    }
    ActionResult::Ok
}

#[action("Mode: Normal")]
fn normal_mode(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    if win.mode == Mode::Select {
        win.cursors.stop_selection();
    }

    win.snippets.clear();
    remove_auto_inserted_indent(editor, id);
    focus_with_mode(editor, id, Focus::Window, Mode::Normal);
    prev_grapheme_on_line.execute(editor, id);
    ActionResult::Ok
}

/// Removes auto inserted indentation if it was the last change
fn remove_auto_inserted_indent(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    let win_last = get!(win.last_insert_change.as_ref());
    let buf_last = get!(buf.last_edit());

    // If not last change dont do anything
    if win_last != &buf_last.changes {
        return;
    }

    let mut indents = vec![];
    // Every change needs to be eol + indent otherwise do nothing
    for change in win_last.iter() {
        if !change.is_insert() {
            return;
        }
        let mut text = change.text();
        let len = text.len();

        // Strip spaces or tabs
        loop {
            if let Some(t) = text.strip_suffix(&[IndentKind::Space.as_byte()]) {
                text = t;
                continue;
            }

            if let Some(t) = text.strip_suffix(&[IndentKind::Tab.as_byte()]) {
                text = t;
                continue;
            }

            break;
        }

        if !EndOfLine::is_eol(&text) {
            return;
        }

        let indent = (len - text.len()) as u64;
        let sol = change.start() + text.len() as u64;
        indents.push(Change::remove(Range::new(sol, sol + indent)));
    }

    let mut changes = Changes::from(indents);
    changes.disable_undo_point_creation();

    if win.change(buf, &changes).is_ok() {
        let hook = Hook::BufChanged(buf.id);
        run(editor, id, hook);
    }
}

#[action("On insert mode leave")]
fn on_insert_mode_leave(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.last_insert_change = None;
    ActionResult::Ok
}

#[action("Mode: Insert")]
fn insert_mode(editor: &mut Editor, id: ClientId) -> ActionResult {
    focus_with_mode(editor, id, Focus::Window, Mode::Insert);
    ActionResult::Ok
}

#[action("Mode: Visual")]
fn select_mode(editor: &mut Editor, id: ClientId) -> ActionResult {
    focus_with_mode(editor, id, Focus::Window, Mode::Select);
    ActionResult::Ok
}

#[action("Mode: Insert after cursor")]
fn insert_mode_after(editor: &mut Editor, id: ClientId) -> ActionResult {
    next_grapheme_on_line.execute(editor, id);
    focus_with_mode(editor, id, Focus::Window, Mode::Insert);
    ActionResult::Ok
}

#[action("Mode: Insert at the end of the line")]
fn insert_mode_end_of_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    end_of_line.execute(editor, id);
    focus_with_mode(editor, id, Focus::Window, Mode::Insert);
    ActionResult::Ok
}

#[action("Mode: Insert at first character of line")]
fn insert_mode_first_char_of_line(editor: &mut Editor, id: ClientId) -> ActionResult {
    first_char_of_line.execute(editor, id);
    focus_with_mode(editor, id, Focus::Window, Mode::Insert);
    ActionResult::Ok
}
