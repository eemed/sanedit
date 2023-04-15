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
        let mut iter = path.iter().peekable();

        while let Some(comp) = iter.next() {
            if iter.peek().is_none() {
                node.set_style(nstyle);
                return;
            }

            match node {
                ThemeNode::Node { nodes, .. } => {
                    let entry = nodes.entry(comp.to_string());
                    node = entry.or_insert_with(|| ThemeNode::Node {
                        style: Style::default(),
                        nodes: HashMap::default(),
                    });
                }
                ThemeNode::Leaf(style) => {
                    let mut nodes = HashMap::default();
                    nodes.insert(comp.to_string(), ThemeNode::Leaf(Style::default()));
                    *node = ThemeNode::Node {
                        style: style.clone(),
                        nodes,
                    };

                    if let ThemeNode::Node { nodes, .. } = node {
                        node = nodes.get_mut(*comp).unwrap();
                    }
                }
            }
        }
    }

    pub fn get<S: AsRef<str>>(&self, path: S) -> Option<Style> {
        let path: Vec<&str> = path.as_ref().split(Self::SEPARATOR).collect();
        let mut node = &self.node;
        let mut iter = path.iter().peekable();
        let mut cur = Style::default();

        log::info!("Get: {path:?}");
        while let Some(comp) = iter.next() {
            log::info!("Override with {comp}: {:?}", node.style());
            cur.override_with(node.style());

            if iter.peek().is_none() {
                return Some(cur);
            }

            match node {
                ThemeNode::Node { nodes, .. } => node = nodes.get(&comp.to_string())?,
                ThemeNode::Leaf(_) => return None,
            }
        }

        None
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

    Info,
    Warn,
    Error,

    PromptDefault,
    PromptMessage,
    PromptUserInput,
    PromptCompletion,
    PromptCompletionSelected,
}

impl AsRef<str> for ThemeField {
    fn as_ref(&self) -> &str {
        use ThemeField::*;

        match self {
            Default => "window",
            Statusline => "window.statusline",
            Selection => "window.selection",
            EndOfBuffer => "window.eob",
            Symbols => "window.symbols",

            Info => "info",
            Warn => "warn",
            Error => "error",

            PromptDefault => "prompt",
            PromptMessage => "prompt.message",
            PromptUserInput => "prompt.userinput",
            PromptCompletion => "prompt.completion",
            PromptCompletionSelected => "prompt.completion.selected",
        }
    }
}

#[cfg(test)]
mod test {
    use crate::redraw::{Color, Rgb};

    use super::*;

    #[test]
    fn insert_get() {
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
            Some(Style {
                text_style: None,
                bg: Some(Color::Black),
                fg: None,
            })
        );

        let style = theme.get("window.cursor");
        assert_eq!(
            style,
            Some(Style {
                text_style: None,
                bg: Some(Color::Black),
                fg: Some(Color::White),
            })
        );
    }

    #[test]
    fn insert_get2() {
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
            Some(Style {
                text_style: None,
                bg: Some(Color::Rgb(Rgb {
                    red: 1,
                    green: 1,
                    blue: 1
                })),
                fg: None,
            })
        );
        let style = theme.get("window.cursor");
        assert_eq!(
            style,
            Some(Style {
                text_style: None,
                bg: Some(Color::Black),
                fg: Some(Color::Rgb(Rgb {
                    red: 1,
                    green: 1,
                    blue: 1
                })),
            })
        );
    }
}
