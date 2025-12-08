use std::{collections::VecDeque, sync::Arc};

use sanedit_server::ClientId;

use crate::{
    actions::{jobs::MatcherJob, window::focus, ActionResult},
    editor::{
        windows::{Focus, Prompt},
        Editor,
    },
};

#[action("Macro: Record and stop recording")]
fn macro_record_toggle(editor: &mut Editor, id: ClientId) -> ActionResult {
    if editor.macros.is_replaying() {
        return ActionResult::Skipped;
    }

    let (win, _buf) = editor.win_buf_mut(id);
    if win.macro_record.is_recording() {
        macro_stop_record.execute(editor, id)
    } else {
        macro_record.execute(editor, id)
    }
}

#[action("Macro: Record")]
fn macro_record(editor: &mut Editor, id: ClientId) -> ActionResult {
    if editor.macros.is_replaying() {
        return ActionResult::Skipped;
    }
    let (win, _buf) = editor.win_buf_mut(id);
    win.macro_record.record();
    ActionResult::Ok
}

#[action("Macro: Record named macro")]
fn macro_record_named(editor: &mut Editor, id: ClientId) -> ActionResult {
    if editor.macros.is_replaying() {
        return ActionResult::Skipped;
    }
    let (win, _buf) = editor.win_buf_mut(id);

    win.prompt = Prompt::builder()
        .prompt("Macro to record")
        .simple()
        .on_confirm(move |editor, id, out| {
            let text = getf!(out.text());
            let (win, _buf) = editor.win_buf_mut(id);
            win.macro_record.record_named(text);
            ActionResult::Ok
        })
        .build();
    focus(editor, id, Focus::Prompt);

    ActionResult::Ok
}

#[action("Macro: Stop recording")]
fn macro_stop_record(editor: &mut Editor, id: ClientId) -> ActionResult {
    if editor.macros.is_replaying() {
        return ActionResult::Skipped;
    }
    let (win, _buf) = editor.win_buf_mut(id);
    win.macro_record.stop_recording();

    if let Some(name) = win.macro_record.name().map(String::from) {
        let mut que = VecDeque::new();
        for input in win.macro_record.events() {
            que.push_back(input.clone());
        }
        editor.macros.insert(name, que);
    }

    ActionResult::Ok
}

#[action("Macro: Replay")]
fn macro_replay(editor: &mut Editor, id: ClientId) -> ActionResult {
    if editor.macros.is_replaying() {
        return ActionResult::Skipped;
    }
    let (win, _buf) = editor.win_buf_mut(id);
    let macr = win.macro_record.events();
    let mut que = VecDeque::new();
    for input in macr {
        que.push_back(input.clone());
    }
    editor.replay_macro(id, que);
    ActionResult::Ok
}

#[action("Macro: Replay named")]
fn macro_replay_named(editor: &mut Editor, id: ClientId) -> ActionResult {
    if editor.macros.is_replaying() {
        return ActionResult::Skipped;
    }
    let macros: Vec<String> = editor.macros.names().map(String::from).collect();
    let (win, _buf) = editor.win_buf_mut(id);

    let job = MatcherJob::builder(id)
        .options(Arc::new(macros))
        .handler(Prompt::matcher_result_handler)
        .build();

    win.prompt = Prompt::builder()
        .prompt("Macro to replay")
        .on_confirm(move |editor, _id, out| {
            let text = getf!(out.text());
            let macr = getf!(editor.macros.get(text));
            editor.replay_macro(id, macr.clone());
            ActionResult::Ok
        })
        .build();
    editor.job_broker.request(job);
    focus(editor, id, Focus::Prompt);
    ActionResult::Ok
}

#[action("Macro: On character")]
fn macro_on_char(editor: &mut Editor, id: ClientId) -> ActionResult {
    if editor.macros.is_replaying() {
        return ActionResult::Skipped;
    }
    let (win, _buf) = editor.win_buf_mut(id);
    let is_recording = win.macro_record.is_recording();
    if !is_recording {
        return ActionResult::Skipped;
    }

    let event = getf!(win.keys().last()).clone();
    win.macro_record.push_event(event);
    ActionResult::Ok
}
