use std::sync::atomic::{AtomicUsize, Ordering};

use rustc_hash::FxHashMap;

use crate::actions::{Action, *};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub(crate) enum Hook {
    /// Before a text is inserted into the buffer
    InsertPre,
    /// Before a text is removed from the buffer
    RemovePre,
    CursorMoved,

    /// Before client keyevent is processed
    KeyPressedPre,

    /// After buffer changed
    BufChanged,

    /// After buffer opened in a window
    BufOpened,

    /// After buffer is closed, and will be removed
    BufRemovePre,

    /// Before client message is processed
    OnMessagePre,

    /// After client message is processed
    OnMessagePost,

    OnDrawPre,
    Reload,
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
    hook_types: FxHashMap<Hook, Vec<HookId>>,
    hooks: FxHashMap<HookId, Action>,
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
        use Hook::*;

        let mut hooks = Hooks {
            hook_types: FxHashMap::default(),
            hooks: FxHashMap::default(),
        };

        // Search
        hooks.register(InsertPre, search::clear_matches);
        hooks.register(RemovePre, search::clear_matches);

        // Window
        hooks.register(BufChanged, window::sync_windows);
        hooks.register(CursorMoved, cursors::merge_overlapping_cursors);
        hooks.register(OnMessagePre, window::clear_messages);
        // TODO handle registration only when needed?
        hooks.register(CursorMoved, completion::abort);
        hooks.register(BufChanged, completion::send_word);
        hooks.register(BufOpened, indent::detect_indent);
        hooks.register(CursorMoved, popup::close);

        // Syntax
        hooks.register(OnMessagePost, syntax::reparse_view);
        hooks.register(BufOpened, syntax::parse_syntax);
        hooks.register(BufChanged, syntax::prevent_flicker);

        // LSP
        hooks.register(BufOpened, lsp::start_lsp);
        hooks.register(BufOpened, lsp::open_doc);
        hooks.register(BufChanged, lsp::sync_document);
        hooks.register(BufRemovePre, lsp::close_doc);
        // TODO closed events

        hooks
    }
}
