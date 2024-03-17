use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::redraw::Style;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default)]
pub struct Theme {
    name: String,
    node: ThemeNode,
}

impl Theme {
    pub const SEPARATOR: &str = ".";

    pub fn new(name: &str) -> Theme {
        Theme {
            name: name.into(),
            node: ThemeNode::default(),
        }
    }

    pub fn insert<S: AsRef<str>>(&mut self, path: S, nstyle: Style) {
        let path: Vec<&str> = path.as_ref().split(Self::SEPARATOR).collect();
        let mut node = &mut self.node;

        for comp in path {
            match node {
                ThemeNode::Node { nodes, .. } => {
                    let entry = nodes.entry(comp.to_string());
                    node = entry.or_insert_with(|| ThemeNode::Leaf(Style::default()));
                }
                ThemeNode::Leaf(style) => {
                    let mut nodes = HashMap::default();
                    nodes.insert(comp.to_string(), ThemeNode::Leaf(Style::default()));
                    *node = ThemeNode::Node {
                        style: style.clone(),
                        nodes,
                    };

                    if let ThemeNode::Node { nodes, .. } = node {
                        node = nodes.get_mut(comp).unwrap();
                    }
                }
            }
        }

        node.set_style(nstyle);
    }

    pub fn get<S: AsRef<str>>(&self, path: S) -> Style {
        let path: Vec<&str> = path.as_ref().split(Self::SEPARATOR).collect();
        let mut node = &self.node;
        let mut cur = node.style().clone();

        for comp in path {
            match node {
                ThemeNode::Node { nodes, .. } => {
                    if let Some(n) = nodes.get(&comp.to_string()) {
                        node = n;
                    } else {
                        break;
                    }
                }
                ThemeNode::Leaf(_) => break,
            }
            cur.override_with(node.style());
        }

        cur
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum ThemeNode {
    Node {
        style: Style,
        nodes: HashMap<String, ThemeNode>,
    },
    Leaf(Style),
}

impl ThemeNode {
    fn style(&self) -> &Style {
        match self {
            ThemeNode::Node { style, .. } => style,
            ThemeNode::Leaf(style) => style,
        }
    }

    fn set_style(&mut self, nstyle: Style) {
        match self {
            ThemeNode::Node { style, .. } => *style = nstyle,
            ThemeNode::Leaf(style) => *style = nstyle,
        }
    }
}

impl Default for ThemeNode {
    fn default() -> Self {
        ThemeNode::Leaf(Style::default())
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ThemeField {
    Default,
    Statusline,
    Selection,
    EndOfBuffer,
    Symbols,
    Cursor,
    PrimaryCursor,
    Gutter,

    Match,

    String,
    Constant,
    Identifier,
    Number,

    Info,
    Warn,
    Error,

    PromptDefault,
    PromptMessage,
    PromptUserInput,
    PromptCompletion,
    PromptCompletionDescription,
    PromptCompletionMatch,
    PromptCompletionSelected,
    PromptCompletionSelectedDescription,
    PromptCompletionSelectedMatch,

    PromptOlayDefault,
    PromptOlayTitle,
    PromptOlayUserInput,
    PromptOlayMessage,
}

impl AsRef<str> for ThemeField {
    fn as_ref(&self) -> &str {
        use ThemeField::*;

        match self {
            Default => "window",
            Statusline => "window.statusline",
            Selection => "window.cursor.selection",
            Cursor => "window.cursor",
            PrimaryCursor => "window.cursor.primary",
            Gutter => "window.gutter",
            EndOfBuffer => "window.eob",
            Symbols => "window.symbols",
            Match => "window.match",

            Info => "window.view.info",
            Warn => "window.view.warn",
            Error => "window.view.error",

            String => "window.view.string",
            Constant => "window.view.constant",
            Identifier => "window.view.identifier",
            Number => "window.view.number",

            PromptDefault => "prompt",
            PromptMessage => "prompt.message",
            PromptUserInput => "prompt.userinput",
            PromptCompletion => "prompt.completion",
            PromptCompletionDescription => "prompt.completion.description",
            PromptCompletionMatch => "prompt.completion.match",
            PromptCompletionSelected => "prompt.completion.selected",
            PromptCompletionSelectedDescription => "prompt.completion.selected.description",
            PromptCompletionSelectedMatch => "prompt.completion.selected.match",

            PromptOlayDefault => "prompt.overlay",
            PromptOlayTitle => "prompt.overlay.title",
            PromptOlayUserInput => "prompt.overlay.userinput",
            PromptOlayMessage => "prompt.overlay.message",
        }
    }
}

#[cfg(test)]
mod test {
    use crate::redraw::{Color, Rgb};

    use super::*;

    #[test]
    fn merge() {
        let mut theme = Theme::default();
        theme.insert(
            "window.cursor",
            Style {
                text_style: None,
                bg: None,
                fg: Some(Color::White),
            },
        );
        theme.insert(
            "window",
            Style {
                text_style: None,
                bg: Some(Color::Black),
                fg: None,
            },
        );

        let style = theme.get("window");
        assert_eq!(
            style,
            Style {
                text_style: None,
                bg: Some(Color::Black),
                fg: None,
            }
        );

        let style = theme.get("window.cursor");
        assert_eq!(
            style,
            Style {
                text_style: None,
                bg: Some(Color::Black),
                fg: Some(Color::White),
            }
        );
    }

    #[test]
    fn latest_matched() {
        let mut theme = Theme::default();
        theme.insert(
            "window",
            Style {
                text_style: None,
                bg: Some(Color::Black),
                fg: Some(Color::White),
            },
        );
        let style = theme.get("window.cursor");
        assert_eq!(
            style,
            Style {
                text_style: None,
                bg: Some(Color::Black),
                fg: Some(Color::White),
            }
        );
    }

    #[test]
    fn merge_override() {
        let mut theme = Theme::default();
        theme.insert(
            "window",
            Style {
                text_style: None,
                bg: Some(Color::Black),
                fg: Some(Color::White),
            },
        );

        theme.insert(
            "window.cursor",
            Style {
                text_style: None,
                fg: Some(Color::Rgb(Rgb {
                    red: 1,
                    green: 1,
                    blue: 1,
                })),
                bg: None,
            },
        );

        let style = theme.get("window");
        assert_eq!(
            style,
            Style {
                text_style: None,
                bg: Some(Color::Black),
                fg: Some(Color::White),
            }
        );
        let style = theme.get("window.cursor");
        assert_eq!(
            style,
            Style {
                text_style: None,
                bg: Some(Color::Black),
                fg: Some(Color::Rgb(Rgb {
                    red: 1,
                    green: 1,
                    blue: 1
                })),
            }
        );
    }
}
