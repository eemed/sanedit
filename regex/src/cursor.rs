pub trait CharCursor {
    fn at_start(&self) -> bool;
    fn at_end(&self) -> bool;
    fn next(&mut self) -> Option<char>;
    fn prev(&mut self) -> Option<char>;
}
