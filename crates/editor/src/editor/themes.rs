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
    ins(Statusline, "#896b60,#2a130d,");
    ins(EndOfBuffer, ",#7f755f,");
    ins(Selection, "#fa9e64,,");
    ins(Match, "#C0A886,,");
    ins(Symbols, ",#606060,");
    ins(Cursor, "#a4ac86,,");
    ins(PrimaryCursor, "#016a61,,");

    ins(PromptDefault, "#656d4a,#000000,");
    ins(PromptMessage, ",,bold");
    ins(PromptUserInput, ",#000000,");

    ins(Gutter, ",#4C4C4C,bold");
    ins(PromptCompletionSelected, "#fa9e64,,");
    ins(PromptCompletionSelectedDescription, ",,");
    ins(PromptCompletionSelectedMatch, ",#b0463a,");
    ins(PromptCompletion, "#d8ccb4,#000000,");
    ins(PromptCompletionDescription, ",#7f755f,");
    ins(PromptCompletionMatch, ",#b0463a,");

    ins(PromptOlayDefault, ",#000000,");
    ins(PromptOlayTitle, ",#232d19,bold");
    ins(PromptOlayUserInput, ",,");
    ins(PromptOlayMessage, ",,bold");

    ins(Info, "#00917e,#000000,");
    ins(Warn, "#ff8c5a,#000000,");
    ins(Error, "#c0564a,#000000,");

    ins(String, ",#936039,");
    ins(Constant, ",#014CC0,");
    ins(Identifier, ",#014CC0,");
    ins(Number, ",#014CC0,");

    theme
}
