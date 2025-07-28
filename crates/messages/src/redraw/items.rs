use sanedit_core::Range;
use serde::{Deserialize, Serialize};

/// Container to draw filesystem like structure
/// Groups that contain other groups or items directly
///
/// This is used to describe filetree and locations
///
/// Note: The structure is not treelike instead the items contain a level
/// property that tells at which level they should be displayed
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Items {
    pub items: Vec<Item>,
    pub selected: usize,
    pub in_focus: bool,
    pub is_loading: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum ItemKind {
    Group { expanded: bool },
    Item,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum ItemLocation {
    Line(u64),
    ByteOffset(u64),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Item {
    pub location: Option<ItemLocation>,

    pub name: String,
    /// What to highlight in name if any
    pub highlights: Vec<Range<usize>>,
    /// Is this a group/folder or an item/file
    pub kind: ItemKind,
    /// What level this item resides on
    pub level: usize,
}
