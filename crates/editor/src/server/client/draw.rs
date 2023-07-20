use std::mem;

use sanedit_messages::redraw::{
    Completion, Component, Diffable, Prompt, Redraw, StatusMessage, Statusline, Window,
};

macro_rules! diffable_open {
    ($field:expr, $item:ident) => {{
        let mut old = mem::replace(&mut $field, Some($item));
        let current = $field.as_ref().unwrap();

        match old {
            Some(ref mut old) => {
                let diff = old.diff(current)?;
                return Some(diff.into());
            }
            None => return Some(current.clone().into()),
        }
    }};
}

macro_rules! diffable_close {
    ($field:expr) => {{
        if $field.is_none() {
            return None;
        }

        $field = None;
    }};
}

macro_rules! if_changed {
    ($field:expr, $item:ident) => {{
        let old = mem::replace(&mut $field, Some($item));
        let current = $field.as_ref().unwrap();

        if old.as_ref() != Some(current) {
            return Some(current.clone().into());
        } else {
            return None;
        }
    }};
}

#[derive(Default, Debug)]
pub(crate) struct ClientDrawState {
    pub(crate) prompt: Option<Prompt>,
    pub(crate) completion: Option<Completion>,
    pub(crate) msg: Option<StatusMessage>,
    pub(crate) statusline: Option<Statusline>,
    pub(crate) window: Option<Window>,
}

impl ClientDrawState {
    pub fn handle_redraw(&mut self, redraw: Redraw) -> Option<Redraw> {
        use Component::*;
        use Redraw::*;

        match redraw {
            Completion(Open(compl)) => diffable_open!(self.completion, compl),
            Completion(Close) => diffable_close!(self.completion),
            Prompt(Open(prompt)) => diffable_open!(self.prompt, prompt),
            Prompt(Close) => diffable_close!(self.prompt),
            Window(Open(win)) => diffable_open!(self.window, win),
            Window(Close) => diffable_close!(self.window),
            Statusline(Open(status)) => diffable_open!(self.statusline, status),
            Statusline(Close) => diffable_close!(self.statusline),
            _ => {}
        }

        Some(redraw)
    }
}
