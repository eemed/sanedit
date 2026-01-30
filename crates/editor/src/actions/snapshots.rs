use sanedit_server::ClientId;

use crate::{actions::{window::focus, ActionResult}, editor::{windows::Focus, Editor}};

#[action("Snapshots: Select first entry")]
fn snapshots_select_first(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = win_buf!(editor, id);
    win.snapshot_view.selection = 0;
    ActionResult::Ok
}

#[action("Snapshots: Select last entry")]
fn snapshots_select_last(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = win_buf!(editor, id);
    // let max = editor.filetree.iter().count() - 1;
    // win.ft_view.selection = max;
    ActionResult::Ok
}

#[action("Snapshots: Show")]
fn show_snapshots(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = win_buf!(editor, id);
    if win.snapshot_view.show {
        focus(editor, id, Focus::Snapshots);
        return ActionResult::Ok;
    }

    // let visible = editor.filetree.iter().count();
    win.snapshot_view.selection = 0;
    win.snapshot_view.show = true;
    focus(editor, id, Focus::Snapshots);

    // ft_goto_current_file.execute(editor, id);

    ActionResult::Ok
}

#[action("Snapshots: Focus")]
fn focus_snapshots(editor: &mut Editor, id: ClientId) -> ActionResult {
    // let (win, _buf) = win_buf!(editor, id);
    // if win.snapshot_view.show {
    //     focus(editor, id, Focus::Filetree);
    // }

    ActionResult::Ok
}

#[action("Snapshots: Confirm entry")]
fn goto_snapshot_entry(editor: &mut Editor, id: ClientId) -> ActionResult {
    ActionResult::Ok
}

#[action("Snapshots: Next entry")]
fn next_snapshot_entry(editor: &mut Editor, id: ClientId) -> ActionResult {
    // let visible = editor.filetree.iter().count();
    // let (win, _buf) = editor.win_buf_mut(id);
    // win.ft_view.selection = min(visible - 1, win.ft_view.selection + 1);

    ActionResult::Ok
}

#[action("Snapshots: Previous entry")]
fn prev_snapshot_entry(editor: &mut Editor, id: ClientId) -> ActionResult {
    // let (win, _buf) = editor.win_buf_mut(id);
    // win.ft_view.selection = win.ft_view.selection.saturating_sub(1);
    ActionResult::Ok
}

#[action("Snapshots: Close")]
fn close_snapshots(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.snapshot_view.show = false;
    focus(editor, id, Focus::Window);

    ActionResult::Ok
}
