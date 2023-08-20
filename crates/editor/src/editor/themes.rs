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

    // OSRS theme
    // BG       #014CC0
    // BG DARK  #B8A282
    // FG       #014CC0
    // BROWN FG #936039
    // BLUE FG  #014CC0
    // RED FG   #C02614
    // GRAY FG  #4C4C4C
    //
    // GREEN BG  #C3E8A3
    // GREEN BG2 #BBEE91
    // GREEN FG #3C780A
    //
    // RARITY
    // BLUE   #AFEEEE
    // GREEN  #56E156
    // YELLOW #FFED4C
    // ORANGE #FF863C

    ins(Default, "#D8CCB4,#000000,");
    ins(Statusline, "#C0A886,,bold");
    ins(EndOfBuffer, ",#4C4C4C,");
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
