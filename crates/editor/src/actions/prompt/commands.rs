use crate::actions::*;
use crate::common::Choice;
use sanedit_messages::key::keyevents_to_string;

pub(crate) fn command_palette(editor: &Editor, id: ClientId) -> Vec<Arc<Choice>> {
    // Display descriptions in command palette
    COMMANDS
        .iter()
        .map(|action| {
            let (win, _buf) = editor.win_buf(id);
            let mut description = String::new();
            if let Some(bind) = editor.keymaps.find_bound_key(&win.layer(), action.name()) {
                description = keyevents_to_string(&bind);
            }
            let value: String = action.description().into();
            Choice::from_text_with_description(value, description)
        })
        .collect()
}
