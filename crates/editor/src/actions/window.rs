use crate::{
    actions::shell,
    editor::{
        buffers::Buffer,
        hooks::Hook,
        keymap::KeymapKind,
        windows::{Focus, FocusEntry, Jump, JumpGroup, Zone},
        Editor,
    },
    VERSION,
};

use sanedit_server::ClientId;

use super::{hooks, ActionResult};

/// Pop focus from focus stack and run hooks
pub fn pop_focus(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    let entry = get!(win.pop_focus());

    let same_keymap = win.keymap_layer == entry.keymap_layer;
    if same_keymap && win.focus() == entry.focus {
        return;
    }

    if !same_keymap {
        hooks::run(editor, id, Hook::KeymapLeave);
    }

    let (win, _buf) = editor.win_buf_mut(id);
    win.restore_focus(entry);

    if !same_keymap {
        hooks::run(editor, id, Hook::KeymapEnter);
    }
}

/// Focus new focus and keymap and place old one in the focus stack, and run
/// hooks
pub fn push_focus_with_keymap(editor: &mut Editor, id: ClientId, focus: Focus, keymap: String) {
    let (win, _buf) = editor.win_buf_mut(id);
    let entry = FocusEntry {
        focus: win.focus(),
        keymap_layer: win.keymap_layer.clone(),
    };
    win.push_focus(entry);

    focus_with_keymap(editor, id, focus, keymap)
}

/// Focus new focus and place old one in the focus stack, and run hooks
pub fn push_focus(editor: &mut Editor, id: ClientId, focus: Focus) {
    let kind = match focus {
        Focus::Search => KeymapKind::Search,
        Focus::Prompt => KeymapKind::Prompt,
        Focus::Window => KeymapKind::Window,
        Focus::Completion => KeymapKind::Completion,
        Focus::Filetree => KeymapKind::Filetree,
        Focus::Locations => KeymapKind::Locations,
    };

    push_focus_with_keymap(editor, id, focus, kind.as_ref().into())
}

/// Change keymap and run hooks
pub fn change_keymap(editor: &mut Editor, id: ClientId, keymap: String) {
    let (win, _buf) = editor.win_buf_mut(id);
    if win.keymap_layer == keymap {
        return;
    }

    hooks::run(editor, id, Hook::KeymapLeave);

    let (win, _buf) = editor.win_buf_mut(id);
    win.keymap_layer = keymap;

    hooks::run(editor, id, Hook::KeymapEnter);
}

/// Change focus and keymap and run hooks
pub fn focus_with_keymap(editor: &mut Editor, id: ClientId, focus: Focus, keymap: String) {
    let (win, _buf) = editor.win_buf_mut(id);
    let same_keymap = win.keymap_layer == keymap;
    if same_keymap && win.focus() == focus {
        return;
    }

    if !same_keymap {
        hooks::run(editor, id, Hook::KeymapLeave);
    }

    let (win, _buf) = editor.win_buf_mut(id);
    win.focus_to(focus);
    win.keymap_layer = keymap;

    if !same_keymap {
        hooks::run(editor, id, Hook::KeymapEnter);
    }
}

/// Change focus and run hooks
pub fn focus(editor: &mut Editor, id: ClientId, focus: Focus) {
    let kind = match focus {
        Focus::Search => KeymapKind::Search,
        Focus::Prompt => KeymapKind::Prompt,
        Focus::Window => KeymapKind::Window,
        Focus::Completion => KeymapKind::Completion,
        Focus::Filetree => KeymapKind::Filetree,
        Focus::Locations => KeymapKind::Locations,
    };

    focus_with_keymap(editor, id, focus, kind.as_ref().into())
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

    if win.search.hl_last || win.popup().is_some() {
        // Clear search matches
        win.search.hl_last = false;
        win.search.hl_matches.clear();

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
            let config = editor.config.filetype.get(ft.as_str())?;
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
    let istyle = &buf.config.indent_kind.as_str();
    let iamount = &buf.config.indent_amount;
    let eol = &buf.config.eol.as_str();

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
    let at_start = win.cursor_jumps.current().is_none();

    if !at_start {
        return ActionResult::Skipped;
    }

    log::info!("SAVEING: {bid:?}, Cursors: {:?}", win.cursors);
    let buffer = editor.buffers.get(bid).unwrap();
    let primary = win.cursors.primary().pos();
    let mark = buffer.mark(primary);
    let jump = Jump::new(mark, None);
    let group = JumpGroup::new(bid, vec![jump]);
    win.cursor_jumps.push(group);
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

#[action("Keymap: On keymap enter")]
fn on_keymap_enter(editor: &mut Editor, id: ClientId) -> ActionResult {
    let layer = editor.layer(id);
    if let Some(action) = layer.on_enter.clone() {
        action.execute(editor, id);
    }
    ActionResult::Ok
}

#[action("Keymap: On keymap leave")]
fn on_keymap_leave(editor: &mut Editor, id: ClientId) -> ActionResult {
    let layer = editor.layer(id);
    if let Some(action) = layer.on_leave.clone() {
        action.execute(editor, id);
    }
    ActionResult::Ok
}
