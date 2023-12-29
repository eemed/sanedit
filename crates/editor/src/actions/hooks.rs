use crate::{
    editor::{hooks::Hook, Editor},
    server::ClientId,
};

pub(crate) fn run(editor: &mut Editor, id: ClientId, hook: Hook) {
    let hooks = editor.hooks.get(hook);
    for action in hooks {
        // log::info!("Exec hook func: {}", action.name());
        action.execute(editor, id);
    }
}
