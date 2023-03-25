use crate::{properties, table_contains};

#[derive(Debug)]
#[repr(u8)]
pub enum GraphemeBreak {
    CR = 0,
    Control,
    Extend,
    L,
    LF,
    LV,
    LVT,
    Prepend,
    RegionalIndicator,
    SpacingMark,
    T,
    V,
    ZWJ,
    Any,
}

#[derive(Debug)]
pub enum Property {
    ExtendedPictographic,
}

impl Property {
    /// check if a char has this property
    pub fn check(&self, ch: char) -> bool {
        use Property::*;
        let table = match self {
            ExtendedPictographic => properties::EXTENDED_PICTOGRAPHIC,
        };

        table_contains(ch, table)
    }
}
