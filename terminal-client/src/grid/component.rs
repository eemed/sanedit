use sanedit_messages::redraw::{Cell, Point, Window};

pub(crate) trait Component {
    fn position(&self) -> Point;
    fn draw(&mut self) -> Vec<Vec<Cell>>;
    fn cursor(&self) -> Option<Point>;
    // fn size(&self) -> Size;
    // fn styles(&mut self) -> Vec<Style>;
}

impl Component for Window {
    fn position(&self) -> Point {
        Point { x: 0, y: 0 }
    }

    fn draw(&mut self) -> Vec<Vec<Cell>> {
        self.cells().clone()
    }

    fn cursor(&self) -> Option<Point> {
        Some(self.primary_cursor())
    }
}
