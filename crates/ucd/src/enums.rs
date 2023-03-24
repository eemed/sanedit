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
    Regional,
    SpacingMark,
    T,
    V,
    ZWJ,
    Any,
}
