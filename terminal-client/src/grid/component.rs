use sanedit_messages::redraw::{Cell, Point, Prompt, Statusline, Window};

pub(crate) trait Component {
    fn position(&self) -> Point;
    fn draw(&mut self) -> Vec<Vec<Cell>>;
    fn cursor(&self) -> Option<Point>;
    // fn size(&self) -> Size;
    // fn styles(&mut self) -> Vec<Style>;
}

impl Component for Window {
    fn position(&self) -> Point {
        Point { x: 0, y: 1 }
    }

    fn draw(&mut self) -> Vec<Vec<Cell>> {
        self.cells().clone()
    }

    fn cursor(&self) -> Option<Point> {
        let cursor = self.primary_cursor();
        let pos = self.position();
        Some(cursor + pos)
    }
}

impl Component for Statusline {
    fn position(&self) -> Point {
        Point { x: 0, y: 0 }
    }

    fn draw(&mut self) -> Vec<Vec<Cell>> {
        let line = self.line().clone();
        vec![line]
    }

    fn cursor(&self) -> Option<Point> {
        None
    }
}

impl Component for Prompt {
    fn position(&self) -> Point {
        Point { x: 0, y: 0 }
    }

    fn draw(&mut self) -> Vec<Vec<Cell>> {
        let prompt_line = self.prompt().clone();
        let mut prompt = vec![prompt_line];
        let opts = self.options().clone();
        prompt.extend(opts);
        prompt
    }

    fn cursor(&self) -> Option<Point> {
        let point = self.position();
        Some(Point {
            x: point.x + self.cursor_x(),
            y: point.y,
        })
    }
}
