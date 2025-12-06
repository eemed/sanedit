use std::sync::Arc;

use sanedit_messages::key::KeyEvent;
use sanedit_server::ClientId;

use crate::{
    actions::{jobs::MatcherJob, ActionResult},
    editor::{windows::Prompt, Editor},
};

#[action("Macro: Record and stop recording")]
fn macro_record_toggle(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    if win.macro_record.is_recording() {
        win.macro_record.stop_recording();
    } else {
        win.macro_record.record();
    }
    ActionResult::Ok
}

#[action("Macro: Record")]
fn macro_record(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.macro_record.record();
    ActionResult::Ok
}

#[action("Macro: Record named macro")]
fn macro_record_named(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);

    win.prompt = Prompt::builder()
        .prompt("Macro to record")
        .on_confirm(move |editor, id, out| {
            let text = getf!(out.text());
            let (win, _buf) = editor.win_buf_mut(id);
            win.macro_record.record_named(text);
            ActionResult::Ok
        })
        .build();

    ActionResult::Ok
}

#[action("Macro: Stop recording")]
fn macro_stop_record(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    win.macro_record.stop_recording();
    let is_named = win.macro_record.name().is_some();

    if is_named {
        let name = win.macro_record.name().unwrap().to_string();
        let events: Vec<KeyEvent> = win.macro_record.events().into();
        editor.macros.push(name, events);
    }

    ActionResult::Ok
}

#[action("Macro: Replay")]
fn macro_replay(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    let macr = win.macro_record.events().to_vec();
    editor.replay_macro(macr.to_vec());
    ActionResult::Ok
}

#[action("Macro: Replay")]
fn macro_replay_named(editor: &mut Editor, id: ClientId) -> ActionResult {
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
            editor.replay_macro(macr.to_vec());
            ActionResult::Ok
        })
        .build();

    editor.job_broker.request(job);
    ActionResult::Ok
}

#[action("Macro: On character")]
fn macro_on_char(editor: &mut Editor, id: ClientId) -> ActionResult {
    let (win, _buf) = editor.win_buf_mut(id);
    let is_recording = win.macro_record.is_recording();
    if !is_recording {
        return ActionResult::Skipped;
    }

    let event = getf!(win.keys().last()).clone();
    win.macro_record.push_event(event);
    ActionResult::Ok
}
