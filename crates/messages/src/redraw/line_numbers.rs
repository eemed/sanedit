use super::Redraw;

pub type LineNumbers = Vec<usize>;

impl From<Vec<usize>> for Redraw {
    fn from(value: Vec<usize>) -> Self {
        Redraw::LineNumbers(value)
    }
}
