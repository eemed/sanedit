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

use self::cursors::*;
use self::editor::*;
use self::jobs::*;
use self::movement::*;
use self::prompt::*;
use self::search::*;
use self::text::*;
use self::view::*;

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
        next_line,
        prev_line,
        next_word_start,
        prev_word_start,
        next_word_end,
        prev_word_end,
        next_paragraph,
        prev_paragraph,
        scroll_up,
        scroll_down,

        prompt_next_grapheme,
        prompt_prev_grapheme,
        prompt_remove_grapheme_before_cursor,
        prompt_confirm,
        prompt_next_completion,
        prompt_prev_completion,
        prompt_close,
        prompt_open_file,
        prompt_history_next,
        prompt_history_prev,

        search_forward,
        search_backward,
        search_next_grapheme,
        search_prev_grapheme,
        search_remove_grapheme_before_cursor,
        search_confirm,
        search_close,
        search_history_next,
        search_history_prev,
        search_clear_matches,
        search_next_match,
        search_prev_match,

        cursor_remove_secondary,
        cursor_new_next_line,
        cursor_new_prev_line,
        cursor_new_to_next_search_match,
        cursor_new_to_all_search_matches,
        cursor_swap_selection_dir,
        cursor_next,
        cursor_prev,
        cursor_remove,
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
