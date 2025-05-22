use crate::{
    actions::shell,
    editor::{
        buffers::Buffer,
        hooks::Hook,
        windows::{Focus, Mode, Zone},
        Editor,
    },
    VERSION,
};

use sanedit_server::ClientId;

use super::{hooks, ActionResult};

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

    if win.search.highlights.is_some() || win.popup().is_some() {
        // Clear search matches
        win.search.highlights = None;

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
    let ft = buf
        .filetype
        .as_ref()
        .map(|ft| ft.as_str())
        .unwrap_or("no filetype");
    let bufsize = buf.len();
    let edits = buf.total_changes_made();
    let (lsp, command, args) = buf
        .filetype
        .as_ref()
        .map(|ft| {
            let lsp = editor.language_servers.get(ft)?;
            let name = lsp.server_name();
            let config = editor.filetypes.get(&ft)?;
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
        Sanedit v{VERSION}\n\
        --------------\n\
        \n\
        Buffer:\n  \
        File: {file}\n  \
        Filetype: {ft}\n  \
        Indent style: {istyle}\n\
        Indent amount: {iamount}\n\
        End of line: {eol}\n\
        Size: {bufsize}\n  \
        Edits made: {edits}\n\
        \n\
        Language server: ({ft})\n  \
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

    let buf = Buffer::from_reader(std::io::Cursor::new(text)).unwrap();
    let bid = editor.buffers_mut().insert(buf);
    editor.open_buffer(id, bid);

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

#[action("Save cursor jump")]
fn save_cursor_jump(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (_win, buf) = editor.win_buf_mut(id);
    let bid = buf.id;
    let bid = editor
        .hooks
        .running_hook()
        .and_then(Hook::buffer_id)
        .unwrap_or(bid);

    let (win, _buf) = win_buf!(editor, id);
    // TODO if jumping not at start?
    let at_start = win.cursor_jumps.current().is_none();

    if !at_start {
        return ActionResult::Skipped;
    }

    let buffer = editor.buffers.get(bid).unwrap();
    win.push_new_cursor_jump(buffer);
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
fn mode_normal(editor: &mut Editor, id: ClientId) -> ActionResult {
    focus_with_mode(editor, id, Focus::Window, Mode::Normal);
    ActionResult::Ok
}

#[action("Mode: Insert")]
fn mode_insert(editor: &mut Editor, id: ClientId) -> ActionResult {
    focus_with_mode(editor, id, Focus::Window, Mode::Insert);
    ActionResult::Ok
}

#[action("Mode: Visual")]
fn mode_visual(editor: &mut Editor, id: ClientId) -> ActionResult {
    focus_with_mode(editor, id, Focus::Window, Mode::Visual);
    ActionResult::Ok
}
