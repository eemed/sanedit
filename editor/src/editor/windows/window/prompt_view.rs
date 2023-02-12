use std::cmp;

use sanedit_messages::redraw::{self, Theme};

use super::{Prompt, Window};

/// Prompt view settings
#[derive(Debug, Default)]
pub(crate) struct PromptView {
    /// How far prev render was scrolled
    prev_scroll_offset: usize,
    prev: Option<redraw::Prompt>,
}

impl PromptView {
    pub fn draw_prompt(&mut self, win: &Window, theme: &Theme) -> Option<redraw::Prompt> {
        let msg = win.prompt.message();
        let input = win.prompt.input();
        let cursor = win.prompt.cursor();
        let scroll_offset = {
            let selected = win.prompt.selected_pos().unwrap_or(0);
            if selected >= self.prev_scroll_offset + win.options.prompt_completions {
                // Make selected the bottom most completion, +1 to actually show
                // the selected completion
                selected - win.options.prompt_completions + 1
            } else {
                cmp::min(self.prev_scroll_offset, selected)
            }
        };
        let selected_relative_pos = win.prompt.selected_pos().map(|pos| pos - scroll_offset);
        let options = win
            .prompt
            .matches_window(win.options.prompt_completions, scroll_offset);
        let prompt = Some(redraw::Prompt::new(
            msg,
            input,
            cursor,
            options,
            selected_relative_pos,
        ));

        self.prev_scroll_offset = scroll_offset;

        if prompt != self.prev {
            self.prev = prompt.clone();
            prompt
        } else {
            None
        }
    }
}
