use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::redraw::Style;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Default, Hash)]
pub struct Theme {
    name: String,
    node: ThemeNode,
}

impl Theme {
    pub const SEPARATOR: &'static str = ".";

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
                    let mut nodes = BTreeMap::default();
                    nodes.insert(comp.to_string(), ThemeNode::Leaf(Style::default()));
                    *node = ThemeNode::Node {
                        style: *style,
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

    pub fn get_from_arr<S: AsRef<str>>(&self, paths: &[S]) -> Style {
        let mut node = &self.node;
        let mut cur = *node.style();

        for path in paths {
            for comp in path.as_ref().split(Self::SEPARATOR) {
                match node {
                    ThemeNode::Node { nodes, .. } => {
                        if let Some(n) = nodes.get(comp) {
                            node = n;
                        } else {
                            break;
                        }
                    }
                    ThemeNode::Leaf(_) => break,
                }
                cur.merge(node.style());
            }
        }

        cur
    }

    pub fn get<S: AsRef<str>>(&self, path: S) -> Style {
        let mut node = &self.node;
        let mut cur = *node.style();

        for comp in path.as_ref().split(Self::SEPARATOR) {
            match node {
                ThemeNode::Node { nodes, .. } => {
                    if let Some(n) = nodes.get(comp) {
                        node = n;
                    } else {
                        break;
                    }
                }
                ThemeNode::Leaf(_) => break,
            }
            cur.merge(node.style());
        }

        cur
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub enum ThemeNode {
    Node {
        style: Style,
        nodes: BTreeMap<String, ThemeNode>,
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
    StatuslineNoFocus,
    Selection,
    EndOfBuffer,
    TrailingWhitespace,
    Cursor,
    Completion,
    CompletionDescription,
    CompletionMatch,
    CompletionSelected,
    CompletionSelectedDescription,
    CompletionSelectedMatch,
    Virtual,

    Match,

    Special,
    Constant,
    String,
    Identifier,

    Comment,
    Operator,
    Type,
    Keyword,
    Preproc,

    Hint,
    Info,
    Warn,
    Error,

    Added,
    Deleted,
    Modified,

    PromptDefault,
    PromptMessage,
    PromptUserInput,
    PromptCompletion,
    PromptCompletionDescription,
    PromptCompletionMatch,
    PromptCompletionSelected,
    PromptCompletionSelectedDescription,
    PromptCompletionSelectedMatch,

    PromptOverlayInput,
    PromptOverlayTitle,
    PromptOverlayMessage,

    FiletreeDefault,
    FiletreeFile,
    FiletreeDir,
    FiletreeSelected,
    FiletreeSelectedFile,
    FiletreeSelectedDir,
    FiletreeSelectedMarkers,
    FiletreeMarkers,

    LocationsDefault,
    LocationsTitle,
    LocationsGroup,
    LocationsEntry,
    LocationsSelected,
    LocationsSelectedMatch,
    LocationsSelectedMarkers,
    LocationsSelectedEntry,
    LocationsSelectedGroup,
    LocationsMarkers,
    LocationsMatch,

    PopupDefault,
    PopupHint,
    PopupInfo,
    PopupWarn,
    PopupError,
}

impl AsRef<str> for ThemeField {
    fn as_ref(&self) -> &str {
        use ThemeField::*;

        match self {
            Default => "window",
            Statusline => "window.statusline",
            StatuslineNoFocus => "window.statusline_no_focus",
            Selection => "cursor.selection",
            Cursor => "cursor.normal",
            EndOfBuffer => "window.end_of_buffer",
            TrailingWhitespace => "window.trailing_whitespace",
            Match => "window.match",
            Virtual => "window.virtual",

            Hint => "window.view.hint",
            Info => "window.view.info",
            Warn => "window.view.warn",
            Error => "window.view.error",

            Added => "window.view.added",
            Deleted => "window.view.deleted",
            Modified => "window.view.modified",

            Special => "window.view.special",
            Constant => "window.view.constant",
            String => "window.view.string",

            Identifier => "window.view.identifier",
            Comment => "window.view.comment",
            Operator => "window.view.operator",
            Type => "window.view.type",
            Keyword => "window.view.keyword",
            Preproc => "window.view.preproc",

            Completion => "window.completion",
            CompletionDescription => "window.completion.description",
            CompletionMatch => "window.completion.match",
            CompletionSelected => "window.completion.selected",
            CompletionSelectedDescription => "window.completion.selected.description",
            CompletionSelectedMatch => "window.completion.selected.match",

            PromptDefault => "prompt",
            PromptMessage => "prompt.message",
            PromptUserInput => "prompt.userinput",
            PromptCompletion => "prompt.completion",
            PromptCompletionDescription => "prompt.completion.description",
            PromptCompletionMatch => "prompt.completion.match",
            PromptCompletionSelected => "prompt.completion.selected",
            PromptCompletionSelectedDescription => "prompt.completion.selected.description",
            PromptCompletionSelectedMatch => "prompt.completion.selected.match",

            PromptOverlayInput => "prompt.overlay",
            PromptOverlayTitle => "prompt.overlay.title",
            PromptOverlayMessage => "prompt.overlay.message",

            FiletreeDefault => "filetree",
            FiletreeFile => "filetree.file",
            FiletreeDir => "filetree.directory",
            FiletreeMarkers => "filetree.markers",
            FiletreeSelected => "filetree.selected",
            FiletreeSelectedFile => "filetree.selected.file",
            FiletreeSelectedDir => "filetree.selected.directory",
            FiletreeSelectedMarkers => "filetree.selected.markers",

            LocationsDefault => "locations",
            LocationsTitle => "locations.title",
            LocationsGroup => "locations.group",
            LocationsEntry => "locations.entry",
            LocationsMarkers => "locations.markers",
            LocationsMatch => "locations.match",
            LocationsSelected => "locations.selected",
            LocationsSelectedMatch => "locations.selected.match",
            LocationsSelectedMarkers => "locations.selected.markers",
            LocationsSelectedEntry => "locations.selected.entry",
            LocationsSelectedGroup => "locations.selected.group",

            PopupDefault => "popup",
            PopupHint => "popup.hint",
            PopupInfo => "popup.info",
            PopupWarn => "popup.warn",
            PopupError => "popup.error",
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
