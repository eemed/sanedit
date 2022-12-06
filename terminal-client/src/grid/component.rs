pub struct Point {
    x: usize,
    y: usize,
}

pub struct Size {
    width: usize,
    height: usize,
}

pub(crate) trait Component {
    fn position(&self) -> Point;
    fn size(&self) -> Size;
    fn draw(&mut self) -> Vec<Vec<String>>;
    // fn styles(&mut self) -> Vec<Style>;
}
