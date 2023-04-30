use std::{
    collections::HashMap,
    mem,
    sync::atomic::{Ordering, AtomicUsize},
};

use crate::actions::{Action, ActionFunction};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub(crate) enum Hook {
    InsertChar,
    RemoveChar,
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
        hooks.register(Hook::InsertChar, Action::search_clear_matches);
        hooks.register(Hook::RemoveChar, Action::search_clear_matches);

        hooks
    }
}
