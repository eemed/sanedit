use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail};
use config::{File, Value};
use rustc_hash::FxHashMap;
use sanedit_core::{ConfigDirectory, Directory};
use sanedit_messages::redraw::{Style, Theme, ThemeField};

pub(crate) const DEFAULT_THEME: &str = "default";

#[derive(Debug)]
pub(crate) struct Themes {
    theme_dir: Directory,
    themes: FxHashMap<String, Theme>,
}

impl Themes {
    pub fn new(path: Directory) -> Themes {
        let mut themes = FxHashMap::default();
        themes.insert(DEFAULT_THEME.into(), default_theme());

        Themes {
            theme_dir: path.into(),
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
        for path in self.theme_dir.find_all_files() {
            log::info!("Loading theme: {:?}", path);
            let Some(fname) = path.file_name() else {
                continue;
            };
            let name = fname.to_string_lossy().to_string();
            let stripped = name.strip_suffix(".toml").unwrap_or(name.as_str());

            if let Err(e) = self.load(&stripped) {
                log::error!("Loading theme '{name}' failed: {e}");
            }
        }
    }

    pub fn load(&mut self, theme_name: &str) -> anyhow::Result<&Theme> {
        let components = [PathBuf::from(format!("{}.toml", theme_name))];
        let theme = self
            .theme_dir
            .find(&components)
            .ok_or(anyhow!("Could not find theme"))?;

        let theme_file = config::Config::builder()
            .add_source(File::from(theme))
            .build()?;
        let table = theme_file.get_table("colors")?;
        let mut theme = Theme::new(theme_name);
        fill_theme_colors(&table, &mut theme)?;

        self.themes.insert(theme_name.to_string(), theme);
        Ok(&self.themes[theme_name])
    }
}

impl Default for Themes {
    fn default() -> Self {
        let theme_dir = ConfigDirectory::default().theme_dir();
        let mut themes = FxHashMap::default();
        themes.insert(DEFAULT_THEME.into(), default_theme());
        Themes { theme_dir, themes }
    }
}

fn fill_theme_colors(table: &HashMap<String, Value>, theme: &mut Theme) -> anyhow::Result<()> {
    fn rec<'a>(
        prefix: &mut Vec<&'a str>,
        cur: &'a HashMap<String, Value>,
        theme: &mut Theme,
    ) -> anyhow::Result<()> {
        for (k, v) in cur {
            let plen = prefix.len();
            // Add all keys split by .
            for key in k.split(".") {
                // Dont add default
                if k != "default" {
                    prefix.push(key);
                }
            }

            match &v.kind {
                config::ValueKind::String(s) => match Style::from_str(&s) {
                    Ok(style) => {
                        let key = prefix.join(".");
                        theme.insert(key, style);
                    }
                    _ => bail!("Invalid style for key {}", prefix.join(".")),
                },
                config::ValueKind::Table(table) => rec(prefix, table, theme)?,
                _ => unreachable!(),
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
    ins(PromptOlayDefault, "#222222,#ffffff,");
    ins(PromptCompletionSelectedMatch, ",#ff0000,");
    ins(PromptCompletionMatch, ",#ff0000,");
    ins(PromptCompletionSelected, "#dddddd,#000000,");

    ins(Constant, ",#0000ff,");
    ins(Number, ",#0000ff,");
    ins(String, ",#00ff00,");

    theme
}
