use crate::{
    actions::hooks::run,
    common::to_human_readable_size,
    editor::{
        buffers::Buffer,
        hooks::Hook,
        windows::{games::snake::Snake, Focus, Mode, Zone},
        Editor,
    },
    VERSION,
};

use sanedit_core::{grapheme_category, Change, Changes, GraphemeCategory, Range};
use sanedit_messages::{
    redraw::{window::WindowUpdate, Redraw},
    ClientMessage,
};
use sanedit_server::{ClientId, FromEditor};

use super::{
    editor::open_new_scratch_buffer,
    hooks,
    jobs::GameTick,
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
    let old = std::mem::replace(&mut win.focus, focus);

    if win.focus != old {
        hooks::run(editor, id, Hook::OnFocusChanged(old));
    }

    let (win, _buf) = editor.win_buf_mut(id);
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
    let (win, _buf) = editor.win_buf_mut(id);
    match win.last_buffer {
        Some(bid) => {
            if editor.buffers.get(bid).is_none() {
                return ActionResult::Failed;
            }

            let (win, buf) = editor.win_buf_mut(id);
            win.push_new_cursor_jump(buf);
            editor.open_buffer(id, bid);
            ActionResult::Ok
        }
        _ => {
            win.warn_msg("No previous buffer");
            ActionResult::Failed
        }
    }
}

pub fn goto_other_buffer(editor: &mut Editor, id: ClientId) {
    let (win, _buf) = editor.win_buf_mut(id);
    let bid = win
        .last_buffer
        .filter(|bid| editor.buffers.get(*bid).is_some());

    match bid {
        Some(bid) => editor.open_buffer(id, bid),
        _ => {
            open_new_scratch_buffer.execute(editor, id);
        }
    }
}

#[action("Window: Cancel")]
fn cancel(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.clear_popup();

    if win.cursors.cursors().iter().any(|c| c.is_selecting()) {
        win.stop_selection();
    } else if win.cursors().len() > 1 {
        win.cursors.cursors_mut().remove_except_primary();

        let (win, _buf) = editor.win_buf_mut(id);
        win.stop_selection();
    }

    let (win, _buf) = editor.win_buf_mut(id);
    if win.search.is_highlighting_enabled() {
        // Clear search matches
        win.search.disable_highlighting();

        return ActionResult::Ok;
    }

    ActionResult::Ok
}

#[action("Window: New horizontal")]
fn new_window_horizontal(editor: &mut Editor, id: ClientId) -> ActionResult {
    editor.send_to_client(
        id,
        sanedit_server::FromEditorSharedMessage::Owned {
            message: FromEditor::Message(ClientMessage::SplitHorizontal),
        },
    );
    ActionResult::Ok
}

#[action("Window: New vertical")]
fn new_window_vertical(editor: &mut Editor, id: ClientId) -> ActionResult {
    editor.send_to_client(
        id,
        sanedit_server::FromEditorSharedMessage::Owned {
            message: FromEditor::Message(ClientMessage::SplitVertical),
        },
    );
    ActionResult::Ok
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
    let bufsize = to_human_readable_size(buf.len() as f64);
    let edits = buf.total_changes_made();
    let (lsp, command, args) = buf
        .language
        .as_ref()
        .and_then(|ft| {
            let lsp = editor.language_servers.get(ft)?;
            let name = lsp.server_name();
            let config = editor.languages.get(ft)?;
            let command = config.language_server.command.as_str();
            let args = config.language_server.args.clone();
            Some((name, command, args))
        })
        .unwrap_or(("no", "-", vec![]));
    let options = &win.view().options;
    let width = options.width;
    let height = options.height;
    let client_id = id.as_usize();
    let working_dir = editor.working_dir();
    let config_dir = editor.config_dir.root();
    let istyle = format!("{:?}", buf.config.indent_kind);
    let iamount = &buf.config.indent_amount;
    let eol = format!("{:?}", buf.config.eol);
    let listen_address = format!("{}", editor.listen_address());

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
        Listen address: {listen_address}\n\
    "
    );

    let mut buf = Buffer::from_reader(std::io::Cursor::new(text)).unwrap();
    buf.read_only = true;
    buf.remove_on_exit = true;
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
fn run_keymap_mode_enter(editor: &mut Editor, id: ClientId) -> ActionResult {
    let Some(layer) = editor.layer(id) else {
        return ActionResult::Failed;
    };
    if let Some(action) = layer.on_enter.clone() {
        action.execute(editor, id);
    }
    ActionResult::Ok
}

#[action("Mode: On mode leave")]
fn run_keymap_mode_leave(editor: &mut Editor, id: ClientId) -> ActionResult {
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
    win.stop_selection();
    win.snippets.clear();
    remove_indent(editor, id);
    prev_grapheme_on_line.execute(editor, id);

    focus_with_mode(editor, id, Focus::Window, Mode::Normal);
    ActionResult::Ok
}

/// Removes auto inserted indentation if it was the last change
fn remove_indent(editor: &mut Editor, id: ClientId) {
    let (win, buf) = editor.win_buf_mut(id);
    if !win.delete_indent_on_insert_leave {
        return;
    }

    let slice = buf.slice(..);
    let mut indents = vec![];
    // Every change needs to be eol + indent otherwise do nothing
    for cursor in win.cursors().cursors() {
        let cpos = cursor.pos();
        let mut graphemes = slice.graphemes_at(cpos);
        if let Some(g) = graphemes.next() {
            if !g.is_eol() {
                continue;
            }
            graphemes.prev();
        }

        let mut len = 0;
        while let Some(g) = graphemes.prev() {
            if g.is_eol() {
                break;
            }

            match grapheme_category(&g) {
                GraphemeCategory::Whitespace => len += g.len(),
                _ => {
                    len = 0;
                    break;
                }
            }
        }

        if len == 0 {
            continue;
        }

        let range = Range::from(cpos - len..cpos);
        indents.push(Change::remove(range));
    }

    if indents.is_empty() {
        return;
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
    win.delete_indent_on_insert_leave = false;
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

#[action("Games: Snake")]
fn snake(editor: &mut Editor, id: ClientId) -> ActionResult {
    let snake_game = {
        let dstate = editor.draw_state(id);

        let window_buffer = getf!(dstate.window_buffer.recv().ok());
        let grid = if let FromEditor::Message(ClientMessage::Redraw(Redraw::Window(
            WindowUpdate::Full(win),
        ))) = window_buffer.as_ref()
        {
            win
        } else {
            unreachable!()
        };
        match Snake::new(grid) {
            Ok(game) => {
                let _ = dstate.window_buffer_sender.send(window_buffer);
                game
            }
            Err(e) => {
                log::error!("Cannot launch game: {e}");
                return ActionResult::Failed;
            }
        }
    };

    let (win, _) = win_buf!(editor, id);
    win.game = Some(Box::new(snake_game));
    let job = GameTick::new(id);
    editor.job_broker.request(job);
    ActionResult::Ok
}
