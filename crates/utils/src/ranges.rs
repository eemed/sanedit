use std::{
    cmp::{max, min},
    collections::{btree_map::Iter, BTreeMap},
    ops::Range,
};

#[derive(Debug, Default)]
pub struct OverlappingRanges {
    ranges: BTreeMap<usize, usize>,
}

impl OverlappingRanges {
    pub fn add(&mut self, range: Range<usize>) {
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
        let mut higher = self.ranges.range((Included(start), Unbounded));
        while let Some((s, e)) = higher.next() {
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

    pub fn invert(&mut self, full: Range<usize>) {
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

    pub fn iter(&self) -> OverlappingRangeIter {
        let iter = self.ranges.iter();
        OverlappingRangeIter { iter }
    }
}

#[derive(Debug)]
pub struct OverlappingRangeIter<'a> {
    iter: Iter<'a, usize, usize>,
}

impl<'a> Iterator for OverlappingRangeIter<'a> {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let (s, e) = self.iter.next()?;
        Some(*s..*e)
    }
}

impl<'a> DoubleEndedIterator for OverlappingRangeIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let (s, e) = self.iter.next_back()?;
        Some(*s..*e)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn one(r: &OverlappingRanges) -> Range<usize> {
        assert!(r.ranges.len() == 1, "Found too many ranges");
        let item = r.iter().next();
        let item = item.expect("No first item found");
        item
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
