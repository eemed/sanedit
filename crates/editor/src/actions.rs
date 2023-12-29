pub(crate) mod completion;
pub(crate) mod cursors;
pub(crate) mod editor;
pub(crate) mod hooks;
pub(crate) mod jobs;
pub(crate) mod movement;
pub(crate) mod prompt;
pub(crate) mod search;
pub(crate) mod text;
pub(crate) mod view;

#[cfg(test)]
pub(crate) mod tests;

use std::{fmt, sync::Arc};

use crate::{editor::Editor, server::ClientId};

pub(crate) type ActionFunction = Arc<dyn Fn(&mut Editor, ClientId) + Send + Sync>;

#[derive(Clone)]
pub(crate) enum Action {
    Dynamic {
        name: String,
        fun: ActionFunction,
    },
    Static {
        name: &'static str,
        module: &'static str,
        fun: fn(&mut Editor, ClientId),
        desc: &'static str,
    },
}

impl Action {
    pub fn new<F>(name: &str, fun: F) -> Action
    where
        F: Fn(&mut Editor, ClientId) + Sync + Send + 'static,
    {
        Action::Dynamic {
            name: name.to_string(),
            fun: Arc::new(fun),
        }
    }

    pub fn execute(&self, editor: &mut Editor, id: ClientId) {
        match self {
            Action::Dynamic { name: _, fun } => (fun)(editor, id),
            Action::Static {
                name: _,
                module: _,
                fun,
                desc: _,
            } => (fun)(editor, id),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Action::Dynamic { name, .. } => name,
            Action::Static { name, .. } => name,
        }
    }
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Dynamic { name, fun: _ } => write!(f, "{}", name),
            Action::Static {
                name,
                module: _,
                fun: _,
                desc: _,
            } => write!(f, "{}", name),
        }
    }
}
