use std::collections::HashMap;

use sanedit_messages::redraw::{Style, Theme, ThemeField};

pub(crate) fn default_themes() -> HashMap<String, Theme> {
    let mut map = HashMap::new();
    map.insert("default".into(), default());
    map
}

fn default() -> Theme {
    use ThemeField::*;

    let mut theme = Theme::new("default");
    let mut ins = |field: ThemeField, style: &str| {
        theme.insert(field, Style::from_str(style).unwrap());
    };

    // DIFF RED #df5f5f
    // DIFF GREEN #5faf5f
    // DIFF YELLOW #dfaf87
    //
    // DIFF BLUE #87B7DE
    //
    // GREEN #008700
    // BLUE #005faf
    // RED #df0000
    // PURPLE #8700df
    // CYAN #005f5f
    // DARK RED #af5f5f
    // DARK GREEN #00875f
    // COMMENT GRAY #606060
    //
    // DARK #8C8C8C
    // dark gray #878787
    // #bcbcbc
    // #4e4e4e

    ins(Default, "#d8d8d8,#080808,");
    ins(Statusline, "#a8a8a8,,bold");
    ins(EndOfBuffer, ",#878787,");
    ins(Selection, ",#8700df,");
    ins(Match, "#5faf5f,,");
    ins(Symbols, ",#606060,");
    ins(Cursor, "#87B7DE,,");
    ins(PrimaryCursor, ",,");

    ins(PromptDefault, "#a8a8a8,#080808,");
    ins(PromptUserInput, ",,");
    ins(PromptMessage, ",,bold");
    ins(PromptCompletionSelected, "#5faf5f,,");
    ins(PromptCompletion, "#bcbcbc,,");

    ins(Info, "#87B7DE,#080808,");
    ins(Warn, "#dfaf87,#080808,");
    ins(Error, "#df5f5f,#080808,");
    theme
}
