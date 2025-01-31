use crate::actions::jobs::{MatchedOptions, MatcherMessage};
use crate::actions::*;
use crate::editor::windows::Focus;
use sanedit_core::Choice;
use sanedit_messages::key::keyevents_to_string;

pub(crate) fn command_palette(editor: &Editor, id: ClientId) -> Vec<Arc<dyn Choice>> {
    // Display descriptions in command palette
    COMMANDS
        .iter()
        .map(|action| {
            let (win, _buf) = editor.win_buf(id);
            let mut description = String::new();
            if let Some(bind) = editor
                .keymap()
                .find_bound_key(&win.keymap_layer, action.name())
            {
                description = keyevents_to_string(&bind);
            }
            let value: String = action.description().into();
            let choice: Arc<dyn Choice> = Arc::new((value, description));
            choice
        })
        .collect()
}

pub(crate) fn matcher_result_handler(editor: &mut Editor, id: ClientId, msg: MatcherMessage) {
    use MatcherMessage::*;

    let draw = editor.draw_state(id);
    draw.no_redraw_window();

    let (win, _buf) = editor.win_buf_mut(id);
    match msg {
        Init(sender) => {
            win.prompt.set_on_input(move |_editor, _id, input| {
                let _ = sender.blocking_send(input.to_string());
            });
            win.prompt.clear_choices();
        }
        Progress(opts) => {
            if let MatchedOptions::Options { matched, clear_old } = opts {
                if clear_old {
                    win.prompt.clear_choices();
                }
                win.focus_to(Focus::Prompt);
                let (win, _buf) = editor.win_buf_mut(id);
                win.prompt.add_choices(matched);
            }
        }
    }
}
