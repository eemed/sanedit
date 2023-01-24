pub(crate) mod editor;

use std::{fmt, sync::Arc};

use crate::{editor::Editor, server::ClientId};

pub(crate) type ActionFunction = Arc<dyn Fn(&mut Editor, ClientId) + Send + Sync>;

#[derive(Clone)]
pub(crate) struct Action {
    name: String,
    fun: ActionFunction,
}

impl Action {
    pub fn new<F>(name: &str, fun: F) -> Action
    where
        F: Fn(&mut Editor, ClientId) + Sync + Send + 'static,
    {
        Action {
            name: name.to_string(),
            fun: Arc::new(fun),
        }
    }

    pub fn execute(&mut self, editor: &mut Editor, id: ClientId) {
        (self.fun)(editor, id)
    }
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
