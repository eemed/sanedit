use std::collections::HashSet;
use std::hash::{BuildHasher, Hasher};
use std::{collections::VecDeque, hash::RandomState};

use anyhow::bail;
use crossbeam::channel::Sender;
use sanedit_messages::redraw::window::Window;
use sanedit_messages::redraw::{text_style, Size, Theme, ThemeField};
use sanedit_messages::{
    key::{Key, KeyEvent},
    redraw::{Cell, Point},
};

use super::Game;

const BASE_TICK_RATE: u64 = 100;
const GROWTH_RATE: usize = 3;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn glyph(&self, next: &Direction) -> Option<&str> {
        use Direction::*;

        match (self, next) {
            (Right, Down) | (Up, Left) => "╗".into(),
            (Left, Down) | (Up, Right) => "╔".into(),
            (Right, Up) | (Down, Left) => "╝".into(),
            (Left, Up) | (Down, Right) => "╚".into(),
            (Down, Down) | (Up, Up) => "║".into(),
            (Left, Left) | (Right, Right) => "═".into(),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum State {
    Starting(u8),
    Running,
    Done,
}

#[derive(Debug)]
pub(crate) struct Snake {
    grow: usize,
    snake: VecDeque<Point>,
    apple: Point,
    direction: Direction,
    prev_dir: Option<Direction>,
    prev_pop: Option<Point>,
    tick_sender: Option<Sender<u64>>,
    map: Window,
    state: State,
    score: usize,
    left_pad: bool,
}

impl Snake {
    pub fn new(drawn: &Window) -> anyhow::Result<Snake> {
        let Size { width, height } = Self::size(drawn);
        let Point {
            x: center,
            y: middle,
        } = Self::center(drawn);
        let snake = {
            const BODY_LEN: usize = 3;

            let head = Point {
                x: center,
                y: middle,
            };
            let mut snake = VecDeque::new();
            snake.push_back(head);
            for i in 1..BODY_LEN + 1 {
                let mut point = head;
                point.y += i;
                snake.push_back(point);
            }

            if head.x >= width || head.y + BODY_LEN >= height {
                bail!("Too small screen");
            }

            snake
        };

        let read_width = drawn.width();

        Ok(Snake {
            apple: Self::apple(width, height, &snake),
            grow: 0,
            direction: Direction::Up,
            prev_dir: None,
            prev_pop: None,
            score: 0,
            state: State::Starting(3),
            snake,
            tick_sender: None,
            map: drawn.clone(),
            left_pad: read_width != width,
        })
    }

    fn apple(width: usize, height: usize, snake: &VecDeque<Point>) -> Point {
        'outer: loop {
            let rand = RandomState::new().build_hasher().finish();
            let mut x = rand % width as u64;
            let y = rand % height as u64;
            if x & 1 == 1 {
                x -= 1;
            }
            let point = Point {
                x: x as usize,
                y: y as usize,
            };

            for cell in snake {
                if &point == cell {
                    continue 'outer;
                }
            }

            return point;
        }
    }

    fn center(cells: &Window) -> Point {
        let mut center_x = cells.width() / 2;
        if center_x & 1 == 1 {
            center_x -= 1;
        }
        let middle_y = cells.height() / 2;
        Point {
            x: center_x,
            y: middle_y,
        }
    }

    fn size(cells: &Window) -> Size {
        let mut width = cells.width();
        if width & 1 == 1 {
            width -= 1;
        }
        let height = cells.height();
        Size { width, height }
    }

    fn set_tick_rate(&self, rate: u64) {
        if let Some(tick_sender) = &self.tick_sender {
            let _ = tick_sender.send(rate);
        }
    }

    fn get_direction(tail: &Point, head: &Point) -> Direction {
        use Direction::*;

        if tail.y > head.y {
            if tail.y - head.y > 1 {
                Up
            } else {
                Down
            }
        } else if tail.y < head.y {
            if head.y - tail.y > 1 {
                Down
            } else {
                Up
            }
        } else if tail.x < head.x {
            if head.x - tail.x > 2 {
                Right
            } else {
                Left
            }
        } else if tail.x - head.x > 2 {
            Left
        } else {
            Right
        }
    }

    fn draw_snake(&self, grid: &mut Window, theme: &Theme) {
        let mut snake_style = theme.get(ThemeField::String);
        snake_style.text_style = Some(text_style::BOLD);

        let mut last_direction: Option<Direction> = None;
        let mut last = self.snake.front().unwrap();

        for point in self.snake.iter().skip(1) {
            use Direction::*;
            let to = Self::get_direction(point, last);

            if let Some(from) = last_direction {
                let cell = grid.at(last.y, last.x);
                let last_body = from.glyph(&to).unwrap_or(&cell.text);
                cell.text = last_body.into();
            }

            let Size { width, .. } = Self::size(grid);
            if to == Left {
                let cell = grid.at(point.y, (point.x + 1) % width);
                cell.text = "═".into();
                cell.style = snake_style;
            }
            if to == Right {
                let cell = grid.at(point.y, (width + point.x - 1) % width);
                cell.text = "═".into();
                cell.style = snake_style;
            }

            let cell = grid.at(point.y, point.x);
            cell.style = snake_style;
            cell.text = if matches!(to, Left | Right) {
                "═"
            } else {
                "║"
            }
            .into();

            last_direction = Some(to);
            last = point;
        }

        if let Some(point) = self.prev_pop {
            let to = Self::get_direction(&point, last);

            if let Some(from) = last_direction {
                let cell = grid.at(last.y, last.x);
                let last_body = from.glyph(&to).unwrap_or(&cell.text);
                cell.text = last_body.into();
            }
        }

        let head = self.snake.front().unwrap();
        let cell = grid.at(head.y, head.x);
        cell.style = snake_style;
        cell.text = "O".into();
    }

    fn draw_apple(&self, grid: &mut Window, theme: &Theme) {
        let apple_style = theme.get(ThemeField::Preproc);
        let cell = grid.at(self.apple.y, self.apple.x);
        cell.style = apple_style;
        cell.style.text_style = Some(text_style::BOLD);
        cell.text = "●".into();
    }
}

impl Game for Snake {
    fn handle_input(&mut self, keyevent: KeyEvent) -> bool {
        match keyevent.key() {
            Key::Left | Key::Char('h') => {
                if self.prev_dir != Some(Direction::Right) {
                    self.direction = Direction::Left;
                }
            }
            Key::Down | Key::Char('j') => {
                if self.prev_dir != Some(Direction::Up) {
                    self.direction = Direction::Down;
                }
            }
            Key::Up | Key::Char('k') => {
                if self.prev_dir != Some(Direction::Down) {
                    self.direction = Direction::Up;
                }
            }
            Key::Right | Key::Char('l') => {
                if self.prev_dir != Some(Direction::Left) {
                    self.direction = Direction::Right;
                }
            }
            Key::Esc | Key::Char('q') => return true,
            Key::Enter | Key::Char('r') => {
                let tick_sender = std::mem::take(&mut self.tick_sender).unwrap();
                let _ = tick_sender.send(0);
                *self = Self::new(&self.map).unwrap();
                return false;
            }
            _ => return false,
        };

        false
    }

    fn tick(&mut self) {
        if self.tick_sender.is_none() {
            return;
        }

        match &mut self.state {
            State::Starting(n) => {
                *n -= 1;

                if *n == 1 {
                    if let Some(tick_sender) = &self.tick_sender {
                        let _ = tick_sender.send(BASE_TICK_RATE);
                    }
                }
                if *n == 0 {
                    self.state = State::Running;
                }
            }
            State::Running => {
                // Tick snake forward
                let tick_rate = {
                    if self.score == 0 {
                        BASE_TICK_RATE
                    } else {
                        let decay = 0.94_f64.powf(self.score as f64);
                        (BASE_TICK_RATE as f64 * decay) as u64
                    }
                };
                self.set_tick_rate(tick_rate);
                self.prev_pop = None;

                // Advance snake
                let Size { width, height } = Self::size(&self.map);
                let mut dead = false;
                let mut new_head = *self.snake.front().unwrap();
                match self.direction {
                    Direction::Up => new_head.y = (height + new_head.y - 1) % height,
                    Direction::Down => new_head.y = (new_head.y + 1) % height,
                    Direction::Left => new_head.x = (width + new_head.x - 2) % width,
                    Direction::Right => new_head.x = (new_head.x + 2) % width,
                }

                self.prev_dir = Some(self.direction);
                self.snake.push_front(new_head);

                // Grow if needed
                if self.grow > 0 {
                    self.grow -= 1;
                } else {
                    self.prev_pop = self.snake.pop_back();
                }

                // Check for collisions
                let mut set = HashSet::new();
                for point in &self.snake {
                    if set.contains(point) {
                        dead = true;
                        break;
                    }
                    set.insert(point);
                }

                if dead {
                    self.state = State::Done;
                    return;
                }

                // Respawn apple
                if set.contains(&self.apple) {
                    self.map.at(self.apple.y, self.apple.x).text = " ".into();
                    self.map
                        .at(self.apple.y, (width + self.apple.x + 1) % width)
                        .text = " ".into();
                    self.apple = Self::apple(width, height, &self.snake);
                    self.grow += GROWTH_RATE;
                    self.score += 1;
                }
            }
            State::Done => {}
        }
    }

    fn draw(&self, grid: &mut Window, theme: &Theme) {
        *grid = self.map.clone();

        let Size { width, height } = Self::size(&self.map);
        if self.left_pad {
            let statusline = theme.get(ThemeField::Statusline);
            for y in 0..height {
                grid.draw(y, width, Cell::with_style(statusline));
            }
        }

        let Point {
            x: center_x,
            y: middle_y,
        } = Self::center(&self.map);
        let msg_style = theme.get(ThemeField::Statusline);

        if let State::Starting(n) = &self.state {
            let msg = format!("Starting in {n}...");
            let start = center_x.saturating_sub(msg.chars().count() / 2);
            for (i, ch) in msg.chars().enumerate() {
                grid.draw(middle_y, start + i, Cell::new_char(ch, msg_style));
            }

            return;
        }

        self.draw_snake(grid, theme);
        self.draw_apple(grid, theme);

        if self.state == State::Done {
            let msg = format!("Snake died. Score {}...", self.score);
            let start = center_x - msg.chars().count() / 2;
            for (i, ch) in msg.chars().enumerate() {
                grid.draw(middle_y, start + i, Cell::new_char(ch, msg_style));
            }
        }
    }

    fn set_tick_sender(&mut self, tick_sender: Sender<u64>) {
        let _ = tick_sender.send(1000);
        self.tick_sender = Some(tick_sender);
    }
}
