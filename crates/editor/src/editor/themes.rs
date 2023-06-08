use std::collections::HashMap;

use sanedit_messages::redraw::{Color, Style, Theme, ThemeField};

pub(crate) fn default_themes() -> HashMap<String, Theme> {
    let mut map = HashMap::new();
    map.insert("gruvbox".into(), gruvbox());
    map.insert("default".into(), default());
    map.insert("rosepine".into(), rosepine());
    map.insert("yui".into(), yui());
    map
}

fn gruvbox() -> Theme {
    let red = Color::from_hex("#cc241d").unwrap();
    let red_light = Color::from_hex("#fb4934").unwrap();

    let green = Color::from_hex("#98971a").unwrap();
    let green_light = Color::from_hex("#b8bb26").unwrap();

    let yellow = Color::from_hex("#d79921").unwrap();
    let yellow_light = Color::from_hex("#fabd2f").unwrap();

    let blue = Color::from_hex("#458588").unwrap();
    let blue_light = Color::from_hex("#83a598").unwrap();

    let purple = Color::from_hex("#b16286").unwrap();
    let purple_light = Color::from_hex("#d3869b").unwrap();

    let aqua = Color::from_hex("#689d6a").unwrap();
    let aqua_light = Color::from_hex("#8ec07c").unwrap();

    let gray = Color::from_hex("#a89984").unwrap();
    let gray_light = Color::from_hex("#928374").unwrap();

    let orange = Color::from_hex("#d65d0e").unwrap();
    let orange_light = Color::from_hex("#fe8019").unwrap();

    let bg_0 = Color::from_hex("#282828").unwrap();
    let bg_1 = Color::from_hex("#3c3836").unwrap();
    let bg_2 = Color::from_hex("#504945").unwrap();
    let bg_3 = Color::from_hex("#665c54").unwrap();
    let bg_4 = Color::from_hex("#7c6f64").unwrap();

    let fg_0 = Color::from_hex("#fbf1c7").unwrap();
    let fg_1 = Color::from_hex("#ebdbb2").unwrap();
    let fg_2 = Color::from_hex("#d5c4a1").unwrap();
    let fg_3 = Color::from_hex("#bdae93").unwrap();
    let fg_4 = Color::from_hex("#a89984").unwrap();

    let mut theme = Theme::new("gruvbox");
    {
        use ThemeField::*;
        theme.insert(
            Default,
            Style {
                text_style: None,
                bg: bg_0.into(),
                fg: fg_0.into(),
            },
        );
        theme.insert(
            Statusline,
            Style {
                text_style: None,
                bg: bg_2.into(),
                fg: fg_2.into(),
            },
        );
        theme.insert(
            EndOfBuffer,
            Style {
                text_style: None,
                bg: None,
                fg: bg_3.into(),
            },
        );
        theme.insert(
            Symbols,
            Style {
                text_style: None,
                bg: None,
                fg: bg_3.into(),
            },
        );
        theme.insert(
            Selection,
            Style {
                text_style: None,
                bg: bg_3.into(),
                fg: None,
            },
        );

        theme.insert(
            Info,
            Style {
                text_style: None,
                bg: blue_light.into(),
                fg: bg_1.into(),
            },
        );
        theme.insert(
            Warn,
            Style {
                text_style: None,
                bg: yellow_light.into(),
                fg: bg_1.into(),
            },
        );
        theme.insert(
            Error,
            Style {
                text_style: None,
                bg: red_light.into(),
                fg: bg_1.into(),
            },
        );

        theme.insert(
            PromptDefault,
            Style {
                text_style: None,
                bg: bg_1.into(),
                fg: fg_1.into(),
            },
        );
        theme.insert(
            PromptMessage,
            Style {
                text_style: None,
                bg: None,
                fg: yellow_light.into(),
            },
        );
        theme.insert(
            PromptUserInput,
            Style {
                text_style: None,
                bg: None,
                fg: fg_1.into(),
            },
        );
        theme.insert(
            PromptCompletion,
            Style {
                text_style: None,
                bg: None,
                fg: None,
            },
        );
        theme.insert(
            PromptCompletionSelected,
            Style {
                text_style: None,
                bg: bg_2.into(),
                fg: orange_light.into(),
            },
        );
    }
    theme
}

fn default() -> Theme {
    use ThemeField::*;

    // orange #dc8052
    // green  #e5df82
    // blue   #86b9b9
    // selection bg blue #3a5c84 fg blue #a5b9d2
    let mut theme = Theme::new("default");
    theme.insert(Default, Style::from_str("#2c2c2c,#cccccc,").unwrap());
    theme.insert(EndOfBuffer, Style::from_str(",#666666,").unwrap());
    theme.insert(Statusline, Style::from_str("#262626,#adadad,").unwrap());
    theme.insert(Selection, Style::from_str("#3a5c84,#a5b9d2,").unwrap());
    theme.insert(Match, Style::from_str("#4c535b,,").unwrap());

    theme.insert(PromptDefault, Style::from_str("#262626,#adadad,").unwrap());
    theme.insert(PromptUserInput, Style::from_str(",,").unwrap());
    theme.insert(PromptMessage, Style::from_str(",#e5df82,").unwrap());
    theme.insert(
        PromptCompletionSelected,
        Style::from_str("#363636,#dc8052,").unwrap(),
    );
    theme.insert(PromptCompletion, Style::from_str(",,").unwrap());

    theme
}

/// Rosepine moon variant https://rosepinetheme.com/palette/ingredients/
fn rosepine() -> Theme {
    use ThemeField::*;

    let base = "#232136";
    let surface = "#2a273f";
    let overlay = "#393552";
    let muted = "#6e6a86";
    let subtle = "#908caa";
    let text = "#e0def4";
    let love = "#eb6f92";
    let gold = "#f6c177";
    let rose = "#ebbcba";
    let pine = "#31748f";
    let foam = "#9ccfd8";
    let iris = "#c4a7e7";
    let hl_low = "#2a283e";
    let hl_med = "#44415a";
    let hl_high = "#56526e";

    let mut theme = Theme::new("rosepine");
    theme.insert(
        Default,
        Style::from_str(&format!("{base},{text},")).unwrap(),
    );
    theme.insert(
        EndOfBuffer,
        Style::from_str(&format!(",{hl_med},")).unwrap(),
    );
    theme.insert(
        Statusline,
        Style::from_str(&format!("{hl_low},{subtle},")).unwrap(),
    );

    theme.insert(
        Selection,
        Style::from_str(&format!("{rose},{base},")).unwrap(),
    );
    theme.insert(
        Cursor,
        Style::from_str(&format!("{subtle},{base},")).unwrap(),
    );
    theme.insert(
        PrimaryCursor,
        Style::from_str(&format!("{text},{base},")).unwrap(),
    );

    theme.insert(
        Match,
        Style::from_str(&format!("{iris},{overlay},")).unwrap(),
    );

    theme.insert(
        PromptDefault,
        Style::from_str(&format!("{surface},{text},")).unwrap(),
    );
    // theme.insert(PromptUserInput, Style::from_str(",,").unwrap());
    theme.insert(
        PromptMessage,
        Style::from_str(&format!(",{gold},")).unwrap(),
    );
    theme.insert(
        PromptCompletionSelected,
        Style::from_str(&format!("{overlay},{iris},")).unwrap(),
    );
    theme.insert(
        PromptCompletion,
        Style::from_str(&format!(",{subtle},")).unwrap(),
    );
    theme
}

fn yui() -> Theme {
    use ThemeField::*;
    let black2 = "#7f726c";
    let black3 = "#b4aba7";
    let black = "#635954";
    let black4 = "#292523";
    let green_background = "#e8ffd1";
    let green_foreground = "#408000";
    let orange = "#E44C22";
    let red_background = "#ffe0e0";
    let red_foreground = "#a7111d";
    let yellow_background = "#f9ffa3";
    let yellow_foreground = "#7b6a3d";
    let purple5 = "#DCD7F9";
    let purple4 = "#cdc5f6";
    let purple3 = "#7864e8";
    let purple = "#5137e1";
    let purple2 = "#371ec8";
    let blue_background = "#e0f3ff";
    let blue_foreground = "#00588f";
    let white5 = "#fbfaf9";
    let white = "#efeae5";
    let white2 = "#e8e0d9";
    let white3 = "#dcd1c6";
    let white4 = "#cfbfb0";
    let cyan = "#007575";

    let mut theme = Theme::new("yui");
    theme.insert(
        Default,
        Style::from_str(&format!("{white},{black},")).unwrap(),
    );
    theme.insert(
        EndOfBuffer,
        Style::from_str(&format!(",{black3},")).unwrap(),
    );
    theme.insert(
        Statusline,
        Style::from_str(&format!("{black},{white},bold")).unwrap(),
    );

    theme.insert(
        Selection,
        Style::from_str(&format!("{purple4},{purple2},")).unwrap(),
    );
    theme.insert(Cursor, Style::from_str(&format!("{white4},,")).unwrap());
    // theme.insert(
    //     PrimaryCursor,
    //     Style::from_str(&format!("{orange},{white},")).unwrap(),
    // );

    theme.insert(
        Match,
        Style::from_str(&format!("{purple5},{purple},")).unwrap(),
    );

    theme.insert(
        PromptDefault,
        Style::from_str(&format!("{black2},{white},")).unwrap(),
    );

    theme.insert(
        PromptCompletionSelected,
        Style::from_str(&format!("{purple4},{purple2},")).unwrap(),
    );
    // theme.insert(PromptUserInput, Style::from_str(",,").unwrap());
    theme.insert(PromptMessage, Style::from_str(&format!(",,bold")).unwrap());
    theme.insert(
        PromptCompletion,
        Style::from_str(&format!("{white3},{black},")).unwrap(),
    );
    theme
}
