use std::path::PathBuf;

use anyhow::{anyhow, bail};
use sanedit_core::{ConfigDirectory, Directory};
use sanedit_messages::redraw::{Style, Theme, ThemeField};
use toml_edit::{Item, Table};

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
            log::info!("Loading theme: {:?}", path);
            let Some(fname) = path.file_name() else {
                continue;
            };
            let name = fname.to_string_lossy().to_string();
            let stripped = name.strip_suffix(".toml").unwrap_or(name.as_str());

            if let Err(e) = self.load(stripped) {
                log::error!("Loading theme '{name}' failed: {e}");
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

        let mut theme = Theme::new(theme_name);
        fill_theme_colors(colors, &mut theme)?;

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

fn fill_theme_colors(table: &Table, theme: &mut Theme) -> anyhow::Result<()> {
    fn rec<'a>(
        prefix: &mut Vec<&'a str>,
        table: &'a Table,
        theme: &mut Theme,
    ) -> anyhow::Result<()> {
        for (keys, v) in table.iter() {
            let plen = prefix.len();
            for key in keys.split('.') {
                // Dont add default
                if key != "default" {
                    prefix.push(key);
                }
            }
            match v {
                Item::Value(value) => match value {
                    toml_edit::Value::String(formatted) => {
                        let Ok(style) = Style::from_str(formatted.value().as_str()) else {
                            bail!("Invalid style for key {}", prefix.join("."))
                        };
                        let key = prefix.join(".");
                        theme.insert(key, style);
                    }
                    _ => {}
                },
                Item::Table(table) => rec(prefix, table, theme)?,
                _ => {}
            }

            prefix.truncate(plen);
        }

        Ok(())
    }

    let mut stack = vec![];
    rec(&mut stack, table, theme)
}

fn default_theme() -> Theme {
    use ThemeField::*;
    let mut theme = Theme::new(DEFAULT_THEME);
    let mut ins = |field: ThemeField, style: &str| {
        theme.insert(field, Style::from_str(style).unwrap());
    };

    ins(Default, "#000000,#ffffff,");
    ins(Statusline, "#222222,#ffffff,");
    ins(PromptDefault, "#222222,#ffffff,");
    ins(PromptOverlayInput, "#222222,#ffffff,");
    ins(PromptCompletionSelectedMatch, ",#ff0000,");
    ins(PromptCompletionMatch, ",#ff0000,");
    ins(PromptCompletionSelected, "#dddddd,#000000,");

    ins(Constant, ",#0000ff,");
    ins(Number, ",#0000ff,");
    ins(String, ",#00ff00,");

    theme
}
