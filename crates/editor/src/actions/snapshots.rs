use std::cmp::min;

use sanedit_server::ClientId;

use crate::{
    actions::{window::focus, ActionResult},
    editor::{hooks::Hook, windows::Focus, Editor},
};

#[action("Undotree: Toggle preview based on focus")]
fn toggle_preview(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    if let Focus::Snapshots = win.focus() {
        win.open_snapshot_preview(buf);

        return ActionResult::Ok;
    }

    let prev_focus = getf!(editor.hooks.running_hook().and_then(Hook::previous_focus));
    if prev_focus != Focus::Snapshots {
        return ActionResult::Skipped;
    }

    win.close_snapshot_preview(buf);
    ActionResult::Ok
}

#[action("Undotree: Select first entry")]
fn snapshots_select_first(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    win.snapshot_view.selection = 0;
    win.update_snapshot_preview(buf);
    ActionResult::Ok
}

#[action("Undotree: Select last entry")]
fn snapshots_select_last(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    let nodes = buf.snapshots().nodes().len();
    win.snapshot_view.selection = nodes - 1;
    win.update_snapshot_preview(buf);
    ActionResult::Ok
}

#[action("Undotree: Show")]
fn show_snapshots(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    if win.snapshot_view.show {
        focus(editor, id, Focus::Snapshots);
        return ActionResult::Ok;
    }

    win.open_snapshot_preview(buf);
    focus(editor, id, Focus::Snapshots);

    ActionResult::Ok
}

#[action("Undotree: Focus")]
fn focus_snapshots(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = win_buf!(editor, id);
    if win.snapshot_view.show {
        focus(editor, id, Focus::Snapshots);
    }

    ActionResult::Ok
}

#[action("Undotree: Confirm entry")]
fn goto_snapshot_entry(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    win.confirm_snapshot_preview(buf);
    focus(editor, id, Focus::Window);
    ActionResult::Ok
}

#[action("Undotree: Next entry")]
fn next_snapshot_entry(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    let nodes = buf.snapshots().nodes().len();
    win.snapshot_view.selection = min(nodes - 1, win.snapshot_view.selection + 1);
    win.update_snapshot_preview(buf);

    ActionResult::Ok
}

#[action("Undotree: Previous entry")]
fn prev_snapshot_entry(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    win.snapshot_view.selection = win.snapshot_view.selection.saturating_sub(1);
    win.update_snapshot_preview(buf);
    ActionResult::Ok
}

#[action("Undotree: Close")]
fn close_snapshots(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, buf) = win_buf!(editor, id);
    win.close_snapshot_preview(buf);
    focus(editor, id, Focus::Window);

    ActionResult::Ok
}

// #[action("Simulate")]
// fn simulate_tree(editor: &mut Editor, id: ClientId) -> ActionResult {
// // Simulate tree
// let mut events = std::collections::VecDeque::new();
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('i'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('a'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('§'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('i'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('a'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('§'),
//     0,
// ));

// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('u'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('i'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('b'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('§'),
//     0,
// ));

// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('u'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('i'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('b'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('§'),
//     0,
// ));

// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('i'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('a'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('§'),
//     0,
// ));

// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('u'),
//     0,
// ));

// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('i'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('b'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('§'),
//     0,
// ));

// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('U'),
//     0,
// ));

// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('u'),
//     0,
// ));

// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('i'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('b'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('§'),
//     0,
// ));

// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('u'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('u'),
//     0,
// ));

// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('i'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('b'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('§'),
//     0,
// ));

// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('u'),
//     0,
// ));

// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('i'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('c'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('§'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('i'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('c'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('§'),
//     0,
// ));

// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('u'),
//     0,
// ));

// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('i'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('d'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('§'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('i'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('d'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('§'),
//     0,
// ));

// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('u'),
//     0,
// ));

// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('i'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('d'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('§'),
//     0,
// ));

// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('u'),
//     0,
// ));

// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('i'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('d'),
//     0,
// ));
// events.push_back(sanedit_messages::key::KeyEvent::new(
//     sanedit_messages::key::Key::Char('§'),
//     0,
// ));
// editor.replay_macro(id, events);
//     ActionResult::Ok
// }
