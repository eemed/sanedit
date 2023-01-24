mod editor;

use std::{fmt, sync::Arc};

use crate::editor::Editor;

pub(crate) type ActionFunction = Arc<dyn Fn(&mut Editor) + Send + Sync>;

// Actions operate on editor
#[derive(Clone)]
pub(crate) struct Action {
    name: String,
    fun: ActionFunction,
}

impl Action {
    pub fn execute(&mut self, editor: &mut Editor) {
        (self.fun)(editor)
    }
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
