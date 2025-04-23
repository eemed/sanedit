#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub(crate) struct Set {
    // 32 * 8 = 256 bits
    inner: [u32; 8],
}

impl Set {
    pub fn any() -> Set {
        Set {
            inner: [u32::MAX; 8],
        }
    }

    pub fn new() -> Set {
        Set { inner: [0u32; 8] }
    }

    pub fn add(&mut self, n: u8) {
        let num = (n / 32) as usize;
        let pos = n % 32;
        let shifted = 1 << pos;
        self.inner[num] |= shifted;
    }

    pub fn has(&self, n: u8) -> bool {
        let num = (n / 32) as usize;
        let pos = n % 32;
        let shifted = 1 << pos;
        self.inner[num] & shifted != 0
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
    fn has_any() {
        let mut set = Set::new();
        set.add(2);
        set.add(50);
        println!("Set: {set:?}");
    }
}
