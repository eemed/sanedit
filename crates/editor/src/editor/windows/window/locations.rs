use std::{cmp::Ordering, ops::Range};

use sanedit_utils::sorted_vec::SortedVec;

#[derive(Debug, Default)]
pub(crate) struct Locations {
    locations: SortedVec<Location>,
}

impl Locations {
    pub fn push(&mut self, loc: Location) {
        self.locations.push(loc);
    }

    pub fn clear(&mut self) {
        self.locations.clear();
    }
}

#[derive(Debug)]
pub(crate) enum Location {
    Group {
        name: String,
        locations: Vec<Location>,
    },
    Item {
        name: String,
        line: Option<u64>,
        column: Option<u64>,
        highlights: Vec<Range<usize>>,
    },
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
        match (self, other) {
            (Location::Group { name, .. }, Location::Group { name: oname, .. }) => {
                name.partial_cmp(oname)
            }
            (Location::Group { .. }, Location::Item { .. }) => Some(Ordering::Greater),
            (Location::Item { .. }, Location::Group { .. }) => Some(Ordering::Less),
            (Location::Item { name, .. }, Location::Item { name: oname, .. }) => {
                name.partial_cmp(oname)
            }
        }
    }
}

impl Ord for Location {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}
