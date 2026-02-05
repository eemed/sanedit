// Stores a set of integers upto a maximum value of 255
#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Bitset256 {
    // 32 * 8 = 256 bits
    inner: [u8; 32],
}

impl Bitset256 {
    pub fn new() -> Bitset256 {
        Bitset256 { inner: [0u8; 32] }
    }

    pub fn insert(&mut self, n: u8) {
        let num = (n / 8) as usize;
        let pos = n % 8;
        let shifted = 1 << pos;
        self.inner[num] |= shifted;
    }

    pub fn remove(&mut self, n: u8) {
        let num = (n / 8) as usize;
        let pos = n % 8;
        let shifted = 0 << pos;
        self.inner[num] &= shifted;
    }

    pub fn contains(&self, n: u8) -> bool {
        let num = (n / 8) as usize;
        let pos = n % 8;
        let shifted = 1 << pos;
        self.inner[num] & shifted != 0
    }

    pub fn max(&self) -> Option<u8> {
        for i in (0u8..=255u8).rev() {
            if self.contains(i) {
                return Some(i)
            }
        }

        None
    }

    pub fn min(&self) -> Option<u8> {
        for i in 0u8..=255u8 {
            if self.contains(i) {
                return Some(i)
            }
        }

        None
    }

    pub fn raw(&self) -> *const u8 {
        self.inner.as_ptr()
    }
}

impl Default for Bitset256 {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Bitset256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[")?;
        let mut first = true;
        for i in 0..=255 {
            if !self.contains(i) {
                continue;
            }

            if first {
                write!(f, "{i}")?;
            } else {
                write!(f, ", {i}")?;
            }

            first = false;
        }
        f.write_str("]")
    }
}

#[derive(Debug)]
pub struct Iter<'a> {
    set: &'a Bitset256,
    n: usize,
}

impl<'a> Iterator for Iter<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        while self.n <= u8::MAX as usize {
            let n = self.n as u8;
            self.n += 1;

            if self.set.contains(n) {
                return Some(n);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains() {
        let mut set = Bitset256::new();
        set.insert(2);
        set.insert(50);

        assert!(set.contains(2));
        assert!(set.contains(50));
    }

    #[test]
    fn remove() {
        let mut set = Bitset256::new();
        set.insert(2);
        set.insert(50);
        set.remove(50);

        assert!(set.contains(2));
        assert!(!set.contains(50));
    }
}
