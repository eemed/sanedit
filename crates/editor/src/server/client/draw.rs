use std::mem;

use sanedit_messages::redraw::{
    Completion, Component, Diffable, Prompt, Redraw, StatusMessage, Statusline, Window,
};

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
        match redraw {
            Redraw::Completion(Component::Open(compl)) => {
                let mut old = mem::replace(&mut self.completion, Some(compl));
                let current = self.completion.as_ref().unwrap();

                match old {
                    Some(ref mut old) => {
                        let diff = old.diff(current)?;
                        return Some(diff.into());
                    }
                    None => return Some(current.clone().into()),
                }
            }
            Redraw::Completion(Component::Close) => self.completion = None,
            Redraw::Prompt(Component::Open(prompt)) => {
                let mut old = mem::replace(&mut self.prompt, Some(prompt));
                let current = self.prompt.as_ref().unwrap();

                match old {
                    Some(ref mut old) => {
                        let diff = old.diff(current)?;
                        return Some(diff.into());
                    }
                    None => return Some(current.clone().into()),
                }
            }
            Redraw::Prompt(Component::Close) => self.prompt = None,
            _ => {}
        }

        Some(redraw)
    }
}
