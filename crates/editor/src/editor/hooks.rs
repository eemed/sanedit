use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use crate::actions::Action;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub(crate) enum Hook {
    /// Before a character is inserted into the buffer
    InsertCharPre,
    /// Before a character is removed from the buffer
    RemoveCharPre,
    CursorMoved,

    /// Before client keyevent is processed
    KeyPressedPre,

    /// Before client message is processed
    OnMessagePre,

    OnDrawPre,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub(crate) struct HookId(usize);

impl HookId {
    pub fn next() -> HookId {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        HookId(id)
    }
}

pub(crate) struct Hooks {
    hook_types: HashMap<Hook, Vec<HookId>>,
    hooks: HashMap<HookId, Action>,
}

impl Hooks {
    /// Register a new hook, returns the hook id.
    pub fn register(&mut self, hook: Hook, fun: Action) -> HookId {
        let id = HookId::next();
        self.hooks.insert(id, fun);

        let entry = self.hook_types.entry(hook);
        let ids = entry.or_default();
        ids.push(id);
        id
    }

    /// Remove a registered hook if it exists
    pub fn remove(&mut self, id: HookId) {
        let removed = self.hooks.remove(&id).is_some();

        if !removed {
            return;
        }

        for (_, ids) in self.hook_types.iter_mut() {
            ids.retain(|i| *i != id)
        }
    }

    /// Get all actions to run for a hook
    pub fn get(&self, hook: Hook) -> Vec<Action> {
        let ids = self.hook_types.get(&hook).cloned().unwrap_or(vec![]);

        let mut result = Vec::with_capacity(ids.len());
        for id in ids {
            let hook = self.hooks[&id].clone();
            result.push(hook);
        }
        result
    }
}

impl Default for Hooks {
    fn default() -> Self {
        let mut hooks = Hooks {
            hook_types: HashMap::new(),
            hooks: HashMap::new(),
        };
        hooks.register(Hook::InsertCharPre, Action::search_clear_matches);
        hooks.register(Hook::RemoveCharPre, Action::search_clear_matches);
        hooks.register(
            Hook::CursorMoved,
            Action::Dynamic {
                name: "merge overlapping cursors".into(),
                fun: Arc::new(|editor, id| {
                    let (win, _buf) = editor.win_buf_mut(id);
                    win.cursors.merge_overlapping();
                }),
            },
        );
        hooks.register(
            Hook::OnMessagePre,
            Action::Dynamic {
                name: "clear messages".into(),
                fun: Arc::new(|editor, id| {
                    let (win, _buf) = editor.win_buf_mut(id);
                    win.clear_msg();
                }),
            },
        );

        hooks
    }
}
