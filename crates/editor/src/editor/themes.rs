use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::bail;
use rustc_hash::FxHashMap;
use sanedit_messages::redraw::{Style, Theme, ThemeField};
use toml::{Table, Value};

use crate::common::dirs::ConfigDirectory;

pub(crate) const DEFAULT_THEME: &str = "default";

#[derive(Debug)]
pub(crate) struct Themes {
    theme_dir: PathBuf,
    themes: FxHashMap<String, Theme>,
}

impl Themes {
    pub fn new(path: &Path) -> Themes {
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
        if let Ok(mut paths) = fs::read_dir(&self.theme_dir) {
            while let Some(Ok(path)) = paths.next() {
                let fname = path.file_name();
                let name = fname.to_string_lossy().to_string();
                let stripped = name.strip_suffix(".toml").unwrap_or(name.as_str());

                if let Err(e) = self.load(&stripped) {
                    log::error!("Loading theme '{name}' failed: {e}");
                }
            }
        }
    }

    pub fn load(&mut self, theme_name: &str) -> anyhow::Result<&Theme> {
        let theme = {
            let mut conf = self.theme_dir.clone();
            conf.push(format!("{}.toml", theme_name));
            conf
        };

        let content = std::fs::read_to_string(theme)?;
        let config = content.parse::<Table>()?;

        let mut theme = Theme::new(theme_name);
        for (key, val) in config.iter() {
            match key.as_str() {
                "colors" => {
                    let map = flatten_colors(val)?;
                    for (k, v) in map {
                        theme.insert(k, v);
                    }
                }
                _ => bail!("Unsupported header {} in theme {}", key, theme_name),
            }
        }

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

fn flatten_colors(table: &Value) -> anyhow::Result<FxHashMap<String, Style>> {
    fn rec(path: &str, cur: &Value, result: &mut FxHashMap<String, Style>) -> anyhow::Result<()> {
        match cur {
            Value::String(s) => match Style::from_str(s) {
                Ok(style) => {
                    result.insert(path.into(), style);
                    Ok(())
                }
                _ => bail!("Invalid style for key {}", path),
            },
            Value::Table(t) => {
                for (k, v) in t {
                    let mut npath = path.to_string();
                    if k != "default" {
                        if !npath.is_empty() {
                            npath.push('.')
                        }

                        npath.push_str(k);
                    }

                    rec(&npath, v, result)?;
                }

                Ok(())
            }
            _ => bail!("Unsupported type in theme"),
        }
    }

    let mut result = FxHashMap::default();
    rec("", table, &mut result)?;
    Ok(result)
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

    ins(Identifier, ",#ff0000,");
    ins(Constant, ",#0000ff,");
    ins(Number, ",#0000ff,");
    ins(String, ",#00ff00,");

    theme
}
