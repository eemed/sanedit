use crate::{
    editor::{hooks::Hook, Editor},
    server::ClientId,
};

pub(crate) fn run(editor: &mut Editor, id: ClientId, hook: Hook) {
    let hooks = editor.hooks.get(hook);
    for action in hooks {
        action.execute(editor, id);
    }
}
