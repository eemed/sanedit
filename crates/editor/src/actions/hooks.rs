use crate::editor::{hooks::Hook, Editor};

use sanedit_server::ClientId;

pub(crate) fn run(editor: &mut Editor, id: ClientId, hook: Hook) {
    // Skip hooks if id doesnt exist anymore
    if editor.windows().get(id).is_none() {
        return;
    }

    let kind = hook.kind();
    editor.hooks.current.push(hook);

    let hooks = editor.hooks.get(kind);
    for action in hooks {
        action.execute(editor, id);
    }

    editor.hooks.current.pop();
}
