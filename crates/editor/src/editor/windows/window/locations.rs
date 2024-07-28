use std::{
    cmp::min,
    ops::Range,
    path::{Path, PathBuf},
};

use sanedit_utils::either::Either;

#[derive(Debug, Default)]
pub(crate) struct Locations {
    pub(crate) show: bool,
    selection: Option<usize>,
    groups: Vec<Group>,
}

impl Locations {
    pub fn push(&mut self, group: Group) {
        self.groups.push(group);

        if self.selection.is_none() {
            self.selection = Some(0);
        }
    }

    pub fn clear(&mut self) {
        self.selection = None;
        self.groups.clear();
    }

    pub fn iter(&self) -> LocationIter {
        LocationIter::new(&self.groups)
    }

    pub fn selection_index(&self) -> Option<usize> {
        self.selection
    }

    pub fn selected(&self) -> Option<LocationEntry> {
        let n = self.selection?;
        self.iter().nth(n)
    }

    pub fn selected_mut(&mut self) -> Option<Either<&mut Group, &mut Item>> {
        let n = self.selection?;
        let mut cur = 0;

        for group in &mut self.groups {
            if cur == n {
                return Some(Either::Left(group));
            }

            cur += 1;

            if group.expanded {
                cur += group.items.len();
                if cur > n {
                    let item = &mut group.items[cur - n - 1];
                    return Some(Either::Right(item));
                }
            }
        }

        None
    }

    pub fn parent_of_selected(&self) -> Option<&Group> {
        let n = self.selection?;
        let mut cur = 0;

        for group in &self.groups {
            if cur == n {
                return None;
            }

            cur += 1;

            if group.expanded {
                cur += group.items.len();
                if cur > n {
                    return Some(group);
                }
            }
        }

        None
    }

    pub fn select_parent(&mut self) {
        let Some(n) = self.selection else {
            return;
        };
        let mut cur = 0;

        for group in &self.groups {
            if cur == n {
                return;
            }

            cur += 1;

            if group.expanded {
                cur += group.items.len();
                if cur > n {
                    self.selection = Some(cur - 1 - group.items.len());
                    return;
                }
            }
        }
    }

    fn visible_len(&self) -> usize {
        self.iter().count()
    }

    pub fn select_next(&mut self) {
        if self.groups.is_empty() {
            self.selection = None;
            return;
        }

        self.selection = match self.selection {
            Some(n) => min(n + 1, self.visible_len() - 1),
            None => 0,
        }
        .into();
    }

    pub fn select_prev(&mut self) {
        if self.groups.is_empty() {
            self.selection = None;
            return;
        }

        self.selection = match self.selection {
            Some(n) => n.saturating_sub(1),
            None => self.visible_len() - 1,
        }
        .into();
    }

    fn ensure_selection_in_range(&mut self) {
        if let Some(n) = self.selection {
            let vis = self.visible_len();
            if vis == 0 {
                self.selection = None;
            } else if n > vis {
                self.selection = Some(vis.saturating_sub(1));
            }
        }
    }

    pub fn groups(&self) -> &[Group] {
        &self.groups
    }

    pub fn expand_all(&mut self) {
        for group in &mut self.groups {
            group.expand();
        }
    }

    pub fn collapse_all(&mut self) {
        for group in &mut self.groups {
            group.collapse();
        }

        self.ensure_selection_in_range();
    }
}

#[derive(Debug)]
pub(crate) struct Group {
    path: PathBuf,
    expanded: bool,
    items: Vec<Item>,
}

impl Group {
    pub fn new(path: &Path) -> Group {
        Group {
            path: path.to_path_buf(),
            expanded: true,
            items: vec![],
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    pub fn items(&self) -> &[Item] {
        &self.items
    }

    pub fn collapse(&mut self) {
        self.expanded = false;
    }

    pub fn expand(&mut self) {
        self.expanded = true;
    }

    pub fn push(&mut self, item: Item) {
        self.items.push(item);
    }

    pub fn clear(&mut self) {
        self.items.clear()
    }
}

#[derive(Debug)]
pub(crate) struct Item {
    name: String,
    line: Option<u64>,
    /// Absolute offset where data starts
    absolute_offset: Option<u64>,
    /// String highlights
    highlights: Vec<Range<usize>>,
}

impl Item {
    pub fn new(
        name: &str,
        line: Option<u64>,
        absolute_offset: Option<u64>,
        highlights: Vec<Range<usize>>,
    ) -> Item {
        Item {
            name: name.into(),
            line,
            absolute_offset,
            highlights,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn line(&self) -> Option<u64> {
        self.line
    }

    pub fn absolute_offset(&self) -> Option<u64> {
        self.absolute_offset
    }

    pub fn highlights(&self) -> &[Range<usize>] {
        &self.highlights
    }
}

#[derive(Debug)]
pub(crate) struct LocationEntry<'a> {
    loc: Either<&'a Group, &'a Item>,
    level: usize,
}

impl<'a> LocationEntry<'a> {
    pub fn group(&self) -> Option<&'a Group> {
        match self.loc {
            Either::Left(g) => Some(g),
            Either::Right(_) => None,
        }
    }

    pub fn item(&self) -> Option<&'a Item> {
        match self.loc {
            Either::Left(_) => None,
            Either::Right(i) => Some(i),
        }
    }

    pub fn either(&self) -> &Either<&'a Group, &'a Item> {
        &self.loc
    }

    pub fn level(&self) -> usize {
        self.level
    }
}

#[derive(Debug)]
pub(crate) struct LocationIter<'a> {
    stack: Vec<LocationEntry<'a>>,
}

impl<'a> LocationIter<'a> {
    fn new(locs: &'a [Group]) -> Self {
        let mut stack = Vec::with_capacity(locs.len());

        for loc in locs.iter().rev() {
            stack.push(LocationEntry {
                loc: Either::Left(loc),
                level: 0,
            });
        }

        LocationIter { stack }
    }
}

impl<'a> Iterator for LocationIter<'a> {
    type Item = LocationEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.stack.pop()?;

        if let LocationEntry {
            loc: Either::Left(Group {
                expanded, items, ..
            }),
            level,
        } = next
        {
            if *expanded {
                for loc in items.iter() {
                    self.stack.push(LocationEntry {
                        loc: Either::Right(loc),
                        level: level + 1,
                    });
                }
            }
        }

        Some(next)
    }
}
