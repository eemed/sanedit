#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Color {
    Red,
    Black,
    BlackBlack,
    NegativeBlack,
}

impl Color {
    #[inline]
    pub fn blacken(&mut self) {
        match *self {
            Color::Red => {
                *self = Color::Black;
            }
            Color::Black => {
                *self = Color::BlackBlack;
            }
            Color::BlackBlack => {
                unreachable!();
            }
            Color::NegativeBlack => {
                *self = Color::Red;
            }
        }
    }

    #[inline]
    pub fn redden(&mut self) {
        match *self {
            Color::Red => {
                *self = Color::NegativeBlack;
            }
            Color::Black => {
                *self = Color::Red;
            }
            Color::BlackBlack => {
                *self = Color::Black;
            }
            Color::NegativeBlack => {
                unreachable!();
            }
        }
    }
}
