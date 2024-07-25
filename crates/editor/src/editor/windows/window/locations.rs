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
        let mut stack = vec![];

        for loc in self.groups.iter_mut() {
            stack.push(Either::Left(loc));
        }

        let mut curn = 0;
        while let Some(loc) = stack.pop() {
            if curn == n {
                return Some(loc);
            }

            if let Either::Left(Group {
                expanded, items, ..
            }) = loc
            {
                if *expanded {
                    for ll in items.iter_mut() {
                        stack.push(Either::Right(ll));
                    }
                }
            }

            curn += 1;
        }

        None
    }

    pub fn parent_of_selected(&self) -> Option<&Group> {
        let n = self.selection?;
        let mut stack = vec![];

        for loc in self.groups.iter() {
            stack.push((None, Either::Left(loc)));
        }

        let mut curn = 0;
        while let Some((parent, loc)) = stack.pop() {
            if curn == n {
                return parent;
            }

            if let Either::Left(group) = loc {
                let Group {
                    expanded, items, ..
                } = group;

                if *expanded {
                    for ll in items.iter() {
                        stack.push((Some(group), Either::Right(ll)));
                    }
                }
            }

            curn += 1;
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

// /// Location that may be just text or a path with various offsets into the file
// #[derive(Debug)]
// pub(crate) enum Location {
//     Group {},
//     Item {},
// }

// impl Location {
//     pub fn name(&self) -> Cow<str> {
//         match self {
//             Location::Group { path, .. } => path.to_string_lossy(),
//             Location::Item { name, .. } => name.into(),
//         }
//     }

//     pub fn data(&self) -> &Either<PathBuf, String> {
//         match self {
//             Location::Group { data, .. } => data,
//             Location::Item { data, .. } => data,
//         }
//     }

//     pub fn line(&self) -> Option<u64> {
//         match self {
//             Location::Group { .. } => None,
//             Location::Item { line, .. } => *line,
//         }
//     }
// }

// impl PartialEq for Location {
//     fn eq(&self, other: &Self) -> bool {
//         match (self, other) {
//             (Location::Group { data, .. }, Location::Group { data: odata, .. }) => data.eq(odata),
//             (Location::Item { data, .. }, Location::Item { data: odata, .. }) => data.eq(odata),
//             _ => false,
//         }
//     }
// }

// impl Eq for Location {}

// impl PartialOrd for Location {
//     fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
//         use Location::*;
//         match (self, other) {
//             (Group { data, .. }, Group { data: odata, .. }) => data.partial_cmp(odata),
//             (Group { .. }, Item { .. }) => Some(Ordering::Greater),
//             (Item { .. }, Group { .. }) => Some(Ordering::Less),
//             (Item { data, .. }, Item { data: odata, .. }) => data.partial_cmp(odata),
//         }
//     }
// }

// impl Ord for Location {
//     fn cmp(&self, other: &Self) -> Ordering {
//         self.partial_cmp(other).unwrap()
//     }
// }

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

        for loc in locs {
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
