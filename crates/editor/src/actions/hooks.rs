use crate::{
    editor::{hooks::Hook, Editor},
    server::ClientId,
};

pub(crate) fn run(editor: &mut Editor, id: ClientId, hook: Hook) {
    let kind = hook.kind();
    editor.hooks.current.push(hook);

    let hooks = editor.hooks.get(kind);
    for action in hooks {
        // log::info!("Exec hook func: {}", action.name());
        action.execute(editor, id);
    }

    editor.hooks.current.pop();
}
