use std::{cmp::Ordering, ops::Range};

#[derive(Debug, Default)]
pub(crate) struct Locations {
    locations: Vec<Location>,
}

impl Locations {
    pub fn push(&mut self, loc: Location) {
        self.locations.push(loc);
    }

    pub fn clear(&mut self) {
        self.locations.clear();
    }

    pub fn iter(&self) -> LocationIter {
        LocationIter::new(&self.locations)
    }
}

#[derive(Debug)]
pub(crate) enum Location {
    Group {
        name: String,
        expanded: bool,
        locations: Vec<Location>,
    },
    Item {
        name: String,
        line: Option<u64>,
        column: Option<u64>,
        highlights: Vec<Range<usize>>,
    },
}

impl Location {
    pub fn name(&self) -> &str {
        match self {
            Location::Group { name, .. } => name,
            Location::Item { name, .. } => name,
        }
    }
}

impl PartialEq for Location {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Location::Group { name, .. }, Location::Group { name: oname, .. }) => name.eq(oname),
            (Location::Item { name, .. }, Location::Item { name: oname, .. }) => name.eq(oname),
            _ => false,
        }
    }
}

impl Eq for Location {}

impl PartialOrd for Location {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use Location::*;
        match (self, other) {
            (Group { name, .. }, Group { name: oname, .. }) => name.partial_cmp(oname),
            (Group { .. }, Item { .. }) => Some(Ordering::Greater),
            (Item { .. }, Group { .. }) => Some(Ordering::Less),
            (Item { name, .. }, Item { name: oname, .. }) => name.partial_cmp(oname),
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
