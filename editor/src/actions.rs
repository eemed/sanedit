pub(crate) mod editor;
pub(crate) mod movement;
pub(crate) mod prompt;
pub(crate) mod text;
pub(crate) mod jobs;
pub(crate) mod window;

use std::{fmt, sync::Arc};

use crate::{editor::Editor, server::ClientId};

use self::editor::*;
use self::movement::*;
use self::prompt::*;
use self::text::*;
use self::jobs::*;
use self::window::*;

macro_rules! action_list {
    ( $($name:ident,)*) => {
        $(
            #[allow(non_upper_case_globals)]
            pub const $name: Self = Action::Static {
                name: stringify!($name),
                fun: $name,
            };
        )*

        pub const ACTION_LIST: &'static [Self] = &[
            $( Self::$name, )*
        ];
    }
}

pub(crate) type ActionFunction = Arc<dyn Fn(&mut Editor, ClientId) + Send + Sync>;

#[derive(Clone)]
pub(crate) enum Action {
    Dynamic {
        name: String,
        fun: ActionFunction,
    },
    Static {
        name: &'static str,
        fun: fn(&mut Editor, ClientId),
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

    pub fn execute(&mut self, editor: &mut Editor, id: ClientId) {
        match self {
            Action::Dynamic { name, fun } => (fun)(editor, id),
            Action::Static { name, fun } => (fun)(editor, id),
        }
    }

    #[rustfmt::skip]
    action_list!(
        quit,
        next_grapheme,
        prev_grapheme,
        remove_grapheme_after_cursor,
        remove_grapheme_before_cursor,
        start_of_line,
        end_of_line,
        start_of_buffer,
        end_of_buffer,
        next_visual_line,
        prev_visual_line,
        scroll_up,
        scroll_down,

        prompt_next_grapheme,
        prompt_prev_grapheme,
        prompt_remove_grapheme_after_cursor,
        prompt_confirm,
        prompt_next_completion,
        prompt_prev_completion,
        prompt_close,
        prompt_open_file,
    );
}

impl fmt::Debug for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Dynamic { name, fun } => write!(f, "{}", name),
            Action::Static { name, fun } => write!(f, "{}", name),
        }
    }
}
