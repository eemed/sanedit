pub(crate) trait Component {
    fn x(&self) -> usize;
    fn y(&self) -> usize;

    fn width(&self) -> usize;
    fn height(&self) -> usize;

    fn draw(&mut self) -> Vec<Vec<String>>;
    // fn styles(&mut self) -> Vec<Style>;
}
