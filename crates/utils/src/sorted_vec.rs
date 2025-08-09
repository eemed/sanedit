use std::{
    mem,
    ops::{Deref, Index},
};

/// A vector that is always sorted
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SortedVec<T: Ord> {
    items: Vec<T>,
}

impl<T: Ord + Clone> SortedVec<T> {
    pub fn from_unsorted(items: &[T]) -> SortedVec<T> {
        let mut items = items.to_vec();
        items.sort();
        SortedVec { items }
    }
}

impl<T: Ord> SortedVec<T> {
    pub fn new() -> SortedVec<T> {
        SortedVec { items: Vec::new() }
    }

    pub fn from_unsorted_owned(mut items: Vec<T>) -> SortedVec<T> {
        items.sort();
        SortedVec { items }
    }

    pub unsafe fn from_sorted_unchecked(items: Vec<T>) -> SortedVec<T> {
        SortedVec { items }
    }

    pub fn with_capacity(cap: usize) -> SortedVec<T> {
        SortedVec {
            items: Vec::with_capacity(cap),
        }
    }

    pub fn get(&self, i: usize) -> Option<&T> {
        self.items.get(i)
    }

    /// # SAFETY
    ///
    /// Can be changed in a way that items Ord placement is not changed
    pub unsafe fn get_mut(&mut self, i: usize) -> Option<&mut T> {
        self.items.get_mut(i)
    }

    /// # SAFETY
    ///
    /// Can be changed in a way that items Ord placement is not changed
    pub unsafe fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.items.iter_mut()
    }

    pub fn retain<F: Fn(&T) -> bool>(&mut self, f: F) {
        self.items.retain(f)
    }

    /// # SAFETY
    ///
    /// Can be changed in a way that items Ord placement is not changed
    pub unsafe fn retain_mut<F: FnMut(&mut T) -> bool>(&mut self, f: F) {
        self.items.retain_mut(f)
    }

    pub fn iter(&self) -> std::slice::Iter<T> {
        self.items.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn push(&mut self, item: T) {
        let pos = self.items.binary_search(&item).unwrap_or_else(|e| e);
        self.items.insert(pos, item);
    }

    pub fn into_iter(self) -> std::vec::IntoIter<T> {
        self.items.into_iter()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn remove(&mut self, idx: usize) -> T {
        self.items.remove(idx)
    }

    pub fn merge(&mut self, other: SortedVec<T>) {
        let cap = other.len() + self.items.len();
        let items = mem::replace(&mut self.items, Vec::with_capacity(cap));

        let mut iiter = items.into_iter();
        let mut oiter = other.into_iter();

        let mut iitem = iiter.next();
        let mut oitem = oiter.next();

        loop {
            match (iitem, oitem) {
                (Some(ii), Some(oi)) => {
                    if ii < oi {
                        self.items.push(ii);
                        iitem = iiter.next();
                        oitem = Some(oi);
                    } else {
                        self.items.push(oi);
                        iitem = Some(ii);
                        oitem = oiter.next();
                    }
                }
                (Some(ii), _) => {
                    self.items.push(ii);
                    break;
                }
                (_, Some(oi)) => {
                    self.items.push(oi);
                    break;
                }
                _ => break,
            }
        }

        self.items.extend(iiter);
        self.items.extend(oiter);
    }
}

impl<T: Ord> Index<usize> for SortedVec<T> {
    type Output = T;

    fn index(&self, i: usize) -> &Self::Output {
        &self.items[i]
    }
}

impl<T: Ord + Clone> From<&[T]> for SortedVec<T> {
    fn from(arr: &[T]) -> Self {
        Self::from_unsorted(arr)
    }
}

impl<T: Ord> From<Vec<T>> for SortedVec<T> {
    fn from(mut items: Vec<T>) -> Self {
        items.sort();
        SortedVec { items }
    }
}

impl<T: Ord> From<T> for SortedVec<T> {
    fn from(value: T) -> Self {
        SortedVec { items: vec![value] }
    }
}

impl<T: Ord> Default for SortedVec<T> {
    fn default() -> Self {
        SortedVec { items: vec![] }
    }
}

impl<T: Ord> Deref for SortedVec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl<T: Ord> From<SortedVec<T>> for Vec<T> {
    fn from(value: SortedVec<T>) -> Self {
        value.items
    }
}

impl<T: Ord> FromIterator<T> for SortedVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let vec: Vec<T> = iter.into_iter().collect();
        SortedVec::from_unsorted_owned(vec)
    }
}
