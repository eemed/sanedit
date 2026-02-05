use sanedit_server::ClientId;

use crate::{actions::{window::focus, ActionResult}, editor::{windows::Focus, Editor}};

#[action("Undotree: Select first entry")]
fn snapshots_select_first(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = win_buf!(editor, id);
    win.snapshot_view.selection = 0;
    ActionResult::Ok
}

#[action("Undotree: Select last entry")]
fn snapshots_select_last(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (_win, _buf) = win_buf!(editor, id);
    // let max = editor.filetree.iter().count() - 1;
    // win.ft_view.selection = max;
    ActionResult::Ok
}

#[action("Undotree: Show")]
fn show_snapshots(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = win_buf!(editor, id);
    if win.snapshot_view.show {
        focus(editor, id, Focus::Snapshots);
        return ActionResult::Ok;
    }

    // Simulate tree
    let mut events = std::collections::VecDeque::new();
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('i'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('a'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('§'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('i'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('a'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('§'), 0));

    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('u'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('i'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('b'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('§'), 0));

    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('u'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('i'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('b'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('§'), 0));


    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('i'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('a'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('§'), 0));


    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('u'), 0));

    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('i'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('b'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('§'), 0));

    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('U'), 0));

    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('u'), 0));

    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('i'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('b'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('§'), 0));

    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('u'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('u'), 0));

    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('i'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('b'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('§'), 0));

    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('u'), 0));

    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('i'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('c'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('§'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('i'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('c'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('§'), 0));

    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('u'), 0));

    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('i'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('d'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('§'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('i'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('d'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('§'), 0));

    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('u'), 0));

    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('i'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('d'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('§'), 0));

    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('u'), 0));

    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('i'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('d'), 0));
    events.push_back(sanedit_messages::key::KeyEvent::new(sanedit_messages::key::Key::Char('§'), 0));

    editor.replay_macro(id, events);

    let (win, _buf) = win_buf!(editor, id);
    win.snapshot_view.selection = 0;
    win.snapshot_view.show = true;
    focus(editor, id, Focus::Snapshots);

    ActionResult::Ok
}

#[action("Undotree: Focus")]
fn focus_snapshots(_editor: &mut Editor, _id: ClientId) -> ActionResult {
    // let (win, _buf) = win_buf!(editor, id);
    // if win.snapshot_view.show {
    //     focus(editor, id, Focus::Filetree);
    // }

    ActionResult::Ok
}

#[action("Undotree: Confirm entry")]
fn goto_snapshot_entry(_editor: &mut Editor, _id: ClientId) -> ActionResult {
    ActionResult::Ok
}

#[action("Undotree: Next entry")]
fn next_snapshot_entry(_editor: &mut Editor, _id: ClientId) -> ActionResult {
    // let visible = editor.filetree.iter().count();
    // let (win, _buf) = editor.win_buf_mut(id);
    // win.ft_view.selection = min(visible - 1, win.ft_view.selection + 1);

    ActionResult::Ok
}

#[action("Undotree: Previous entry")]
fn prev_snapshot_entry(_editor: &mut Editor, _id: ClientId) -> ActionResult {
    // let (win, _buf) = editor.win_buf_mut(id);
    // win.ft_view.selection = win.ft_view.selection.saturating_sub(1);
    ActionResult::Ok
}

#[action("Undotree: Close")]
fn close_snapshots(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.snapshot_view.show = false;
    focus(editor, id, Focus::Window);

    ActionResult::Ok
}
