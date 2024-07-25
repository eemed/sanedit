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
    locations: Vec<Location>,
}

impl Locations {
    pub fn push(&mut self, loc: Location) {
        self.locations.push(loc);

        if self.selection.is_none() {
            self.selection = Some(0);
        }
    }

    pub fn clear(&mut self) {
        self.locations.clear();
    }

    pub fn iter(&self) -> LocationIter {
        LocationIter::new(&self.locations)
    }

    pub fn selection_index(&self) -> Option<usize> {
        self.selection
    }

    pub fn selected(&self) -> Option<LocationEntry> {
        let n = self.selection?;
        self.iter().nth(n)
    }

    pub fn selected_mut(&mut self) -> Option<&mut Location> {
        let n = self.selection?;
        let mut stack = vec![];

        for loc in self.locations.iter_mut() {
            stack.push(loc);
        }

        let mut curn = 0;
        while let Some(loc) = stack.pop() {
            if curn == n {
                return Some(loc);
            }

            if let Location::Group {
                expanded,
                locations,
                ..
            } = loc
            {
                if *expanded {
                    for ll in locations.iter_mut() {
                        stack.push(ll);
                    }
                }
            }

            curn += 1;
        }

        None
    }

    pub fn parent_of_selected(&self) -> Option<&Location> {
        let n = self.selection?;
        let mut stack = vec![];

        for loc in self.locations.iter() {
            stack.push((None, loc));
        }

        let mut curn = 0;
        while let Some((parent, loc)) = stack.pop() {
            if curn == n {
                return parent;
            }

            if let Location::Group {
                expanded,
                locations,
                ..
            } = loc
            {
                if *expanded {
                    for ll in locations.iter() {
                        stack.push((Some(loc), ll));
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
        if self.locations.is_empty() {
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
        if self.locations.is_empty() {
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
pub(crate) enum Location {
    Group {
        data: Either<PathBuf, String>,
        expanded: bool,
        locations: Vec<Location>,
    },
    Item {
        data: Either<PathBuf, String>,
        line: Option<u64>,
        column: Option<u64>,
        /// Absolute offset where data starts
        absolute_offset: Option<u64>,
        highlights: Vec<Range<usize>>,
    },
}

impl Location {
    pub fn data_as_str(&self) -> Cow<str> {
        match self {
            Location::Group { data, .. } | Location::Item { data, .. } => match data {
                Either::Left(path) => path.to_string_lossy(),
                Either::Right(string) => string.into(),
            },
        }
    }

    pub fn data(&self) -> &Either<PathBuf, String> {
        match self {
            Location::Group { data, .. } => data,
            Location::Item { data, .. } => data,
        }
    }

    pub fn line(&self) -> Option<u64> {
        match self {
            Location::Group { .. } => None,
            Location::Item { line, .. } => *line,
        }
    }
}

impl PartialEq for Location {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Location::Group { data, .. }, Location::Group { data: odata, .. }) => data.eq(odata),
            (Location::Item { data, .. }, Location::Item { data: odata, .. }) => data.eq(odata),
            _ => false,
        }
    }
}

impl Eq for Location {}

impl PartialOrd for Location {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use Location::*;
        match (self, other) {
            (Group { data, .. }, Group { data: odata, .. }) => data.partial_cmp(odata),
            (Group { .. }, Item { .. }) => Some(Ordering::Greater),
            (Item { .. }, Group { .. }) => Some(Ordering::Less),
            (Item { data, .. }, Item { data: odata, .. }) => data.partial_cmp(odata),
        }
    }
}

impl Ord for Location {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[derive(Debug)]
pub(crate) struct LocationEntry<'a> {
    pub(crate) loc: &'a Location,
    pub(crate) level: usize,
}

#[derive(Debug)]
pub(crate) struct LocationIter<'a> {
    stack: Vec<LocationEntry<'a>>,
}

impl<'a> LocationIter<'a> {
    fn new(locs: &'a [Location]) -> Self {
        let mut stack = Vec::with_capacity(locs.len());

        for loc in locs {
            stack.push(LocationEntry { loc, level: 0 });
        }

        LocationIter { stack }
    }
}

impl<'a> Iterator for LocationIter<'a> {
    type Item = LocationEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        use Location::*;

        let next = self.stack.pop()?;
        if let LocationEntry {
            loc:
                Group {
                    expanded,
                    locations,
                    ..
                },
            level,
        } = next
        {
            if *expanded {
                for loc in locations.iter() {
                    self.stack.push(LocationEntry {
                        loc,
                        level: level + 1,
                    });
                }
            }
        }

        Some(next)
    }
}
