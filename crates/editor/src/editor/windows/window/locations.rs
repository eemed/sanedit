use std::{
    borrow::Cow,
    cmp::{min, Ordering},
    ops::Range,
    path::PathBuf,
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
}

#[derive(Debug)]
pub(crate) struct Group {
    pub(crate) path: PathBuf,
    pub(crate) expanded: bool,
    pub(crate) items: Vec<Item>,
}

#[derive(Debug)]
pub(crate) struct Item {
    pub(crate) name: String,
    pub(crate) line: Option<u64>,
    pub(crate) column: Option<u64>,
    /// Absolute offset where data starts
    pub(crate) absolute_offset: Option<u64>,
    /// String highlights
    pub(crate) highlights: Vec<Range<usize>>,
}

#[derive(Debug)]
pub(crate) struct LocationEntry<'a> {
    pub(crate) loc: Either<&'a Group, &'a Item>,
    pub(crate) level: usize,
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
