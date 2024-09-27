use std::{
    cmp::{max, min},
    collections::{btree_map::Iter, BTreeMap},
    ops::Range,
};

#[derive(Debug)]
pub struct OverlappingRanges<T: Ord + Copy> {
    ranges: BTreeMap<T, T>,
}

impl<T: Ord + Copy> OverlappingRanges<T> {
    pub fn new() -> OverlappingRanges<T> {
        OverlappingRanges {
            ranges: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, range: Range<T>) {
        use std::ops::Bound::*;

        let Range { mut start, mut end } = range;
        let mut lower = self.ranges.range((Unbounded, Excluded(start)));
        if let Some((s, e)) = lower.next_back() {
            if start <= *e {
                start = min(*s, start);
                end = max(end, *e);
            }
        }

        let mut remove = vec![];
        let higher = self.ranges.range((Included(start), Unbounded));
        for (s, e) in higher {
            if *s <= end {
                start = min(*s, start);
                end = max(end, *e);
                remove.push(*s);
            } else {
                break;
            }
        }

        for rem in remove {
            self.ranges.remove(&rem);
        }

        self.ranges.insert(start, end);
    }

    // pub fn remove(&self) {}

    pub fn invert(&mut self, full: Range<T>) {
        let mut inverted = OverlappingRanges::default();

        if self.ranges.is_empty() {
            self.add(full);
            return;
        }

        let mut cur = full.start;
        for (s, e) in &self.ranges {
            if *e < cur {
                continue;
            }

            if *s > full.end {
                break;
            }

            let next = *s;
            if cur != next {
                inverted.add(cur..next);
            }
            cur = *e;
        }

        if cur != full.end {
            inverted.add(cur..full.end);
        }

        *self = inverted;
    }

    pub fn iter(&self) -> OverlappingRangeIter<T> {
        let iter = self.ranges.iter();
        OverlappingRangeIter { iter }
    }
}

impl<T: Ord + Copy> Default for OverlappingRanges<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct OverlappingRangeIter<'a, T: Ord + Copy> {
    iter: Iter<'a, T, T>,
}

impl<'a, T: Ord + Copy> Iterator for OverlappingRangeIter<'a, T> {
    type Item = Range<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let (s, e) = self.iter.next()?;
        Some(*s..*e)
    }
}

impl<'a, T: Ord + Copy> DoubleEndedIterator for OverlappingRangeIter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let (s, e) = self.iter.next_back()?;
        Some(*s..*e)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn one(r: &OverlappingRanges<usize>) -> Range<usize> {
        assert!(r.ranges.len() == 1, "Found too many ranges");
        let item = r.iter().next();

        item.expect("No first item found")
    }

    #[test]
    fn basic() {
        let mut ranges = OverlappingRanges::default();
        ranges.add(10..30);
        assert_eq!(10..30, one(&ranges));
        ranges.add(12..20);
        assert_eq!(10..30, one(&ranges));
        ranges.add(5..10);
        assert_eq!(5..30, one(&ranges));
        ranges.add(30..33);
        assert_eq!(5..33, one(&ranges));
        ranges.add(1..40);
        assert_eq!(1..40, one(&ranges));
    }

    #[test]
    fn invert() {
        let mut ranges = OverlappingRanges::default();
        ranges.add(10..30);
        ranges.invert(0..40);

        let mut iter = ranges.iter();

        assert_eq!(Some(0..10), iter.next());
        assert_eq!(Some(30..40), iter.next());
    }
}
