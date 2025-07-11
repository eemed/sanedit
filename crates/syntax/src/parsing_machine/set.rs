#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub(crate) struct Set {
    // 32 * 8 = 256 bits
    inner: [u8; 32],
}

impl Set {
    pub fn new() -> Set {
        Set { inner: [0u8; 32] }
    }

    pub fn add(&mut self, n: u8) {
        let num = (n / 8) as usize;
        let pos = n % 8;
        let shifted = 1 << pos;
        self.inner[num] |= shifted;
    }

    pub fn has(&self, n: u8) -> bool {
        let num = (n / 8) as usize;
        let pos = n % 8;
        let shifted = 1 << pos;
        self.inner[num] & shifted != 0
    }

    pub fn raw(&self) -> *const u8 {
        self.inner.as_ptr()
    }
}

impl std::fmt::Debug for Set {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[")?;
        let mut first = true;
        for i in 0..=255 {
            if !self.has(i) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains() {
        let mut set = Set::new();
        set.add(2);
        set.add(50);

        assert!(set.has(2));
        assert!(set.has(50));
    }
}
