use std::cmp::min;

use sanedit_messages::redraw::{Prompt, Source};

use super::{item::GridItem, Rect};

#[derive(Debug, Clone, Copy)]
pub enum PromptStyle {
    /// Simple one line prompt with options on another lines
    Oneline,
    /// An overlay window
    Overlay,
}

#[derive(Debug)]
pub struct CustomPrompt {
    pub style: PromptStyle,
    pub prompt: Prompt,
}

pub(crate) fn open_prompt(width: usize, height: usize, prompt: Prompt) -> GridItem<CustomPrompt> {
    use Source::*;
    // Try to fit overlay prompt
    // magic number: overlay paddings 3 + prompt 1 + options + extra space so we
    // dont attach to window sides 6
    //
    // minimum height to draw overlay
    let olay_min_height = prompt.max_completions + 3 + 1 + 6;
    // height the overlay needs
    let olay_height = prompt.max_completions + 3 + 1;
    let oneline_min_height = prompt.max_completions + 1;
    let style = match prompt.source {
        Search | Simple => PromptStyle::Oneline,
        Prompt => {
            use PromptStyle::*;

            if height < olay_min_height {
                Oneline
            } else {
                Overlay
            }
        }
    };

    match style {
        PromptStyle::Oneline => {
            let rect = Rect::new(0, 0, width, min(height, oneline_min_height));
            GridItem::new(CustomPrompt { prompt, style }, rect)
        }
        PromptStyle::Overlay => {
            let width = width / 2;
            let x = width / 2;
            let extra = height - olay_height;
            let y = extra / 4;
            let rect = Rect::new(x, y, width, olay_height);
            GridItem::new(CustomPrompt { prompt, style }, rect)
        }
    }
}
