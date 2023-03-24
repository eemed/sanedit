use std::collections::HashMap;

use sanedit_messages::redraw::{Color, Style, Theme, ThemeField};

pub(crate) fn default_themes() -> HashMap<String, Theme> {
    let mut map = HashMap::new();
    map.insert("gruvbox".into(), gruvbox());
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
        theme.set(
            Default.into(),
            Style {
                text_style: None,
                bg: bg_0.into(),
                fg: fg_0.into(),
            },
        );
        theme.set(
            Statusline.into(),
            Style {
                text_style: None,
                bg: bg_2.into(),
                fg: fg_2.into(),
            },
        );
        theme.set(
            EndOfBuffer.into(),
            Style {
                text_style: None,
                bg: None,
                fg: bg_3.into(),
            },
        );
        theme.set(
            Symbols.into(),
            Style {
                text_style: None,
                bg: None,
                fg: bg_3.into(),
            },
        );
        theme.set(
            Selection.into(),
            Style {
                text_style: None,
                bg: bg_3.into(),
                fg: None,
            },
        );

        theme.set(
            Info.into(),
            Style {
                text_style: None,
                bg: blue_light.into(),
                fg: bg_1.into(),
            },
        );
        theme.set(
            Warn.into(),
            Style {
                text_style: None,
                bg: yellow_light.into(),
                fg: bg_1.into(),
            },
        );
        theme.set(
            Error.into(),
            Style {
                text_style: None,
                bg: red_light.into(),
                fg: bg_1.into(),
            },
        );

        theme.set(
            PromptDefault.into(),
            Style {
                text_style: None,
                bg: bg_1.into(),
                fg: fg_1.into(),
            },
        );
        theme.set(
            PromptMessage.into(),
            Style {
                text_style: None,
                bg: None,
                fg: yellow_light.into(),
            },
        );
        theme.set(
            PromptUserInput.into(),
            Style {
                text_style: None,
                bg: None,
                fg: fg_1.into(),
            },
        );
        theme.set(
            PromptCompletion.into(),
            Style {
                text_style: None,
                bg: None,
                fg: None,
            },
        );
        theme.set(
            PromptCompletionSelected.into(),
            Style {
                text_style: None,
                bg: bg_2.into(),
                fg: orange_light.into(),
            },
        );
    }
    theme
}
