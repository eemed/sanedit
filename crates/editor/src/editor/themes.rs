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
    // BG       #E2DBC8
    // BG DARK  #D8CCB4
    // BG DARK  #D0BD97
    // BG DARK2 #C0A886
    // BG DARK3 #B8A282
    //
    // BROWN FG #936039
    // BLUE FG  #014CC0
    // RED FG   #C02614
    // GRAY FG  #4C4C4C
    //
    // BROWN BORDER #94866D
    //
    // GREEN BG  #C3E8A3
    // GREEN BG2 #BBEE91
    // GREEN FG  #3C780A
    //
    // RARITY
    // BLUE   #AFEEEE
    // GREEN  #56E156
    // YELLOW #FFED4C
    // ORANGE #FF863C
    // RED    #FF6262

    ins(Default, "#E2DBC8,#000000,");
    ins(Statusline, "#B8A282,,bold");
    ins(EndOfBuffer, ",#4C4C4C,");
    ins(Selection, ",#8700df,");
    ins(Match, "#91bbee,,");
    ins(Symbols, ",#606060,");
    ins(Cursor, "#9691ee,,");
    ins(PrimaryCursor, ",,");

    ins(PromptDefault, "#B8A282,#000000,");
    ins(Gutter, ",#4C4C4C,bold");
    ins(PromptUserInput, ",,");
    ins(PromptTitle, ",#000000,,bold");
    ins(PromptMessage, ",#4C4C4C,bold");
    ins(PromptCompletionSelected, "#BBEE91,,");
    ins(PromptCompletion, "#D8CCB4,,");

    ins(Info, "#9aeaea,#000000,");
    ins(Warn, "#ebb87b,#000000,");
    ins(Error, "#eb817b,#000000,");
    theme
}
