use std::sync::atomic::{AtomicUsize, Ordering};

use rustc_hash::FxHashMap;
use strum_macros::EnumDiscriminants;

use crate::actions::{Action, *};

use super::buffers::BufferId;

#[derive(Debug, EnumDiscriminants)]
#[strum_discriminants(derive(Hash))]
#[strum_discriminants(name(HookKind))]
pub(crate) enum Hook {
    /// Before a text is inserted into the buffer
    InsertPre,
    /// Before a text is removed from the buffer
    RemovePre,
    CursorMoved,

    /// Before client keyevent is processed
    KeyPressedPre,

    /// A new buffer is created
    BufCreated(BufferId),

    /// After buffer entered
    BufEnter(BufferId),

    /// When a buffer is left
    BufLeave(BufferId),

    /// After text is inserted or removed from buffer
    BufChanged(BufferId),

    /// After buffer is closed, and will be removed
    BufDeletedPre(BufferId),

    /// Before client message is processed
    OnMessagePre,

    /// After client message is processed
    OnMessagePost,

    /// Before buffer is saved
    BufSavedPre,
    /// After buffer has been saved
    BufSavedPost,

    OnDrawPre,
    Reload,

    ModeEnter,
    ModeLeave,
}

impl Hook {
    pub fn kind(&self) -> HookKind {
        HookKind::from(self)
    }

    pub fn buffer_id(&self) -> Option<BufferId> {
        match self {
            Hook::BufCreated(id)
            | Hook::BufLeave(id)
            | Hook::BufEnter(id)
            | Hook::BufChanged(id)
            | Hook::BufDeletedPre(id) => Some(*id),
            _ => None,
        }
    }
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
    hook_types: FxHashMap<HookKind, Vec<HookId>>,
    hooks: FxHashMap<HookId, Action>,

    /// Set if a hook is currently being run
    pub(crate) current: Vec<Hook>,
}

impl Hooks {
    /// Register a new hook, returns the hook id.
    pub fn register(&mut self, hook: HookKind, fun: Action) -> HookId {
        let id = HookId::next();
        self.hooks.insert(id, fun);

        let entry = self.hook_types.entry(hook);
        let ids = entry.or_default();
        ids.push(id);
        id
    }

    #[allow(dead_code)]
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
    pub fn get(&self, hook: HookKind) -> Vec<Action> {
        let ids = self.hook_types.get(&hook).cloned().unwrap_or(vec![]);

        let mut result = Vec::with_capacity(ids.len());
        for id in ids {
            let hook = self.hooks[&id].clone();
            result.push(hook);
        }
        result
    }

    pub fn running_hook(&self) -> Option<&Hook> {
        self.current.last()
    }
}

impl Default for Hooks {
    fn default() -> Self {
        use HookKind::*;

        let mut hooks = Hooks {
            hook_types: FxHashMap::default(),
            hooks: FxHashMap::default(),
            current: vec![],
        };

        // Editor
        hooks.register(BufCreated, editor::load_language);

        // Search
        hooks.register(OnMessagePost, search::highlight_search);
        hooks.register(BufChanged, search::prevent_flicker);

        // Window
        hooks.register(BufChanged, window::sync_windows);
        hooks.register(CursorMoved, cursors::merge_overlapping_cursors);
        hooks.register(OnMessagePre, window::clear_messages);
        hooks.register(ModeEnter, window::on_mode_enter);
        hooks.register(ModeLeave, window::on_mode_leave);
        hooks.register(ModeEnter, window::view_to_cursor);
        hooks.register(ModeLeave, window::on_insert_mode_leave);

        // TODO handle registration only when needed?
        hooks.register(CursorMoved, completion::completion_abort);
        hooks.register(BufChanged, completion::send_word);
        hooks.register(BufCreated, indent::detect_indent);
        hooks.register(CursorMoved, popup::close);

        // Syntax
        hooks.register(OnMessagePost, syntax::reparse_view);
        hooks.register(Reload, syntax::parse_syntax);
        hooks.register(BufEnter, syntax::parse_syntax);
        hooks.register(BufChanged, syntax::prevent_flicker);

        // LSP
        hooks.register(BufCreated, lsp::start_lsp_hook);
        hooks.register(BufCreated, lsp::open_document);
        hooks.register(BufChanged, lsp::sync_document);
        hooks.register(BufDeletedPre, lsp::close_document);
        hooks.register(BufSavedPre, lsp::will_save_document);
        hooks.register(BufSavedPost, lsp::did_save_document);

        // Buffer
        // hooks.register(BufChanged, text::clear_diagnostics);

        hooks
    }
}
