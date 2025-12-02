use std::path::PathBuf;

use anyhow::{anyhow, bail};
use sanedit_core::{ConfigDirectory, Directory};
use sanedit_messages::redraw::{
    text_style::{self, TextStyle},
    Color, Style, Theme, ThemeField,
};
use toml_edit::{InlineTable, Item, Table, Value};

use super::Map;

pub(crate) const DEFAULT_THEME: &str = "default";

#[derive(Debug)]
pub(crate) struct Themes {
    theme_dir: Directory,
    themes: Map<String, Theme>,
}

impl Themes {
    pub fn new(path: Directory) -> Themes {
        let mut themes = Map::default();
        themes.insert(DEFAULT_THEME.into(), default_theme());

        Themes {
            theme_dir: path,
            themes,
        }
    }

    pub fn get(&mut self, name: &str) -> anyhow::Result<&Theme> {
        match self.themes.contains_key(name) {
            true => Ok(&self.themes[name]),
            false => self.load(name),
        }
    }

    pub fn names(&self) -> Vec<&str> {
        self.themes.keys().map(|s| s.as_str()).collect()
    }

    pub fn load_all(&mut self) {
        for path in self.theme_dir.find_all_distinct_files() {
            let Some(fname) = path.file_name() else {
                continue;
            };
            let name = fname.to_string_lossy().to_string();
            let stripped = name.strip_suffix(".toml").unwrap_or(name.as_str());

            if let Err(e) = self.load(stripped) {
                log::error!("Loading theme '{name}' failed: {e}");
            } else {
                log::debug!("Loaded theme: {name:?}");
            }
        }
    }

    pub fn load(&mut self, theme_name: &str) -> anyhow::Result<&Theme> {
        let path = PathBuf::from(format!("{}.toml", theme_name));
        let theme = self
            .theme_dir
            .find(&path)
            .ok_or(anyhow!("Could not find theme"))?;

        use std::io::Read;
        let mut tomls = String::new();
        let mut toml = std::fs::File::open(theme)?;
        toml.read_to_string(&mut tomls)?;
        let doc = tomls.parse::<toml_edit::DocumentMut>()?;

        if !doc.contains_table("colors") {
            bail!("No colors table found");
        }

        let colors = doc.get("colors").unwrap();
        if !colors.is_table() {
            bail!("No colors table found");
        }
        let colors = colors.as_table().unwrap();

        let variables = doc.get("variables").and_then(|v| v.as_table());

        let mut theme = Theme::new(theme_name);
        fill_theme_colors(variables, colors, &mut theme)?;

        self.themes.insert(theme_name.to_string(), theme);
        Ok(&self.themes[theme_name])
    }
}

impl Default for Themes {
    fn default() -> Self {
        let theme_dir = ConfigDirectory::default().theme_dir();
        let mut themes = Map::default();
        themes.insert(DEFAULT_THEME.into(), default_theme());
        Themes { theme_dir, themes }
    }
}

fn fill_theme_colors(
    variables: Option<&Table>,
    colors: &Table,
    theme: &mut Theme,
) -> anyhow::Result<()> {
    fn rec<'a>(
        prefix: &mut Vec<&'a str>,
        variables: Option<&Table>,
        colors: &'a Table,
        theme: &mut Theme,
    ) -> anyhow::Result<()> {
        for (keys, v) in colors.iter() {
            let plen = prefix.len();
            for key in keys.split('.') {
                // Dont add default
                if key != "default" {
                    prefix.push(key);
                }
            }

            match v {
                Item::Value(Value::InlineTable(table)) => {
                    let key = prefix.join(".");
                    let style = get_style(variables, table)?;
                    theme.insert(key, style);
                }
                Item::Table(table) => rec(prefix, variables, table, theme)?,
                _ => (),
            }

            prefix.truncate(plen);
        }

        Ok(())
    }

    let mut stack = vec![];
    rec(&mut stack, variables, colors, theme)
}

fn get_style(variables: Option<&Table>, table: &InlineTable) -> anyhow::Result<Style> {
    let mut style = Style::default();

    for (name, value) in table.iter() {
        if let toml_edit::Value::String(formatted) = value {
            let value = formatted.value().as_str();

            match name {
                "fg" => {
                    let text = variables
                        .and_then(|vars| vars.get(value))
                        .and_then(|value| {
                            if let Item::Value(toml_edit::Value::String(formatted)) = value {
                                Some(formatted.value().as_str())
                            } else {
                                None
                            }
                        })
                        .unwrap_or(value);
                    style.fg = Some(Color::try_from(text)?);
                }
                "bg" => {
                    let text = variables
                        .and_then(|vars| vars.get(value))
                        .and_then(|value| {
                            if let Item::Value(toml_edit::Value::String(formatted)) = value {
                                Some(formatted.value().as_str())
                            } else {
                                None
                            }
                        })
                        .unwrap_or(value);
                    style.bg = Some(Color::try_from(text)?);
                }
                "text_style" => {
                    style.text_style = Some(text_style::from_str(value));
                }
                _ => bail!("Invalid key {}", name),
            }
        }
    }

    Ok(style)
}

fn default_theme() -> Theme {
    use ThemeField::*;
    let mut theme = Theme::new(DEFAULT_THEME);
    let mut ins = |field: ThemeField, style: &str| {
        // theme.insert(field, Style::parse(style).unwrap());
    };

    ins(Default, "#000000,#ffffff,");
    ins(Statusline, "#222222,#ffffff,");
    ins(PromptDefault, "#222222,#ffffff,");
    ins(PromptOverlayInput, "#222222,#ffffff,");
    ins(PromptCompletionSelectedMatch, ",#ff0000,");
    ins(PromptCompletionMatch, ",#ff0000,");
    ins(PromptCompletionSelected, "#dddddd,#000000,");

    ins(Constant, ",#0000ff,");
    ins(String, ",#00ff00,");

    theme
}
