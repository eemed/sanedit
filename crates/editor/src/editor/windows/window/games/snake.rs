use std::collections::HashSet;
use std::hash::{BuildHasher, Hasher};
use std::{collections::VecDeque, hash::RandomState, sync::mpsc::Sender};

use anyhow::bail;
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

#[derive(Debug)]
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
    map: Vec<Vec<Cell>>,
    state: State,
    score: usize,
}

impl Snake {
    pub fn new(drawn: &Vec<Vec<Cell>>) -> anyhow::Result<Snake> {
        let Size { width, height } = Self::size(&drawn);
        let Point {
            x: center,
            y: middle,
        } = Self::center(&drawn);
        let snake = {
            const BODY_LEN: usize = 3;

            let head = Point {
                x: center,
                y: middle,
            };
            let mut snake = VecDeque::new();
            snake.push_back(head);
            for i in 1..BODY_LEN + 1 {
                let mut point = head.clone();
                point.y += i;
                snake.push_back(point);
            }

            if head.x >= width || head.y + BODY_LEN >= height {
                bail!("Too small screen");
            }

            snake
        };

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
        })
    }

    fn apple(width: usize, height: usize, snake: &VecDeque<Point>) -> Point {
        loop {
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
                    continue;
                }
            }

            return point;
        }
    }

    fn center(cells: &Vec<Vec<Cell>>) -> Point {
        let mut center_x = cells.get(0).map(|line| line.len() / 2).unwrap_or(0);
        if center_x & 1 == 1 {
            center_x -= 1;
        }
        let middle_y = cells.len() / 2;
        Point {
            x: center_x,
            y: middle_y,
        }
    }

    fn size(cells: &Vec<Vec<Cell>>) -> Size {
        let width = cells.get(0).map(|line| line.len()).unwrap_or(0);
        let height = cells.len();
        Size { width, height }
    }

    fn set_tick_rate(&self, rate: u64) {
        if let Some(tick_sender) = &self.tick_sender {
            let _ = tick_sender.send(rate);
        }
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
            Key::Char('q') => return true,
            Key::Enter | Key::Char('r') => {
                let tick_sender = std::mem::take(&mut self.tick_sender).unwrap();
                *self = Self::new(&self.map).unwrap();
                self.set_tick_sender(tick_sender);
                return false;
            }
            _ => return false,
        };

        false
    }

    fn tick(&mut self) {
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
                    if self.score <= 0 {
                        BASE_TICK_RATE
                    } else {
                        let decay = 0.94_f64.powf(self.score as f64);
                        (BASE_TICK_RATE as f64 * decay) as u64
                    }
                };
                self.set_tick_rate(tick_rate);
                self.prev_pop = None;

                let Size { width, height } = Self::size(&self.map);
                let mut dead = false;
                let mut new_head = self.snake.front().unwrap().clone();
                match self.direction {
                    Direction::Up => {
                        if new_head.y == 0 {
                            dead = true;
                        } else {
                            new_head.y -= 1;
                        }
                    }
                    Direction::Down => {
                        if new_head.y + 1 == height {
                            dead = true;
                        } else {
                            new_head.y += 1;
                        }
                    }
                    Direction::Left => {
                        if new_head.x == 0 {
                            dead = true;
                        } else {
                            new_head.x -= 2;
                        }
                    }
                    Direction::Right => {
                        if new_head.x + 1 == width {
                            dead = true;
                        } else {
                            new_head.x += 2;
                        }
                    }
                }

                self.prev_dir = Some(self.direction);
                self.snake.push_front(new_head);

                if self.grow > 0 {
                    self.grow -= 1;
                } else {
                    self.prev_pop = self.snake.pop_back();
                }

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

                if set.contains(&self.apple) {
                    self.map[self.apple.y][self.apple.x].text = " ".into();
                    self.apple = Self::apple(width, height, &self.snake);
                    self.grow += GROWTH_RATE;
                    self.score += 1;
                }
            }
            State::Done => {}
        }
    }

    fn draw(&self, cells: &mut Vec<Vec<Cell>>, theme: &Theme) {
        // if cells.len() != self.map.len()
        //     || cells.get(0).map(|line| line.len()).unwrap_or(0)
        //         != self.map.get(0).map(|line| line.len()).unwrap_or(0)
        // {
        // }

        *cells = self.map.clone();
        let Point {
            x: center_x,
            y: middle_y,
        } = Self::center(&self.map);
        let msg_style = theme.get(ThemeField::Statusline);

        match self.state {
            State::Starting(n) => {
                let msg = format!("Starting in {n}...");
                let start = center_x.saturating_sub(msg.chars().count() / 2);
                for (i, ch) in msg.chars().enumerate() {
                    cells[middle_y][start + i] = Cell {
                        text: ch.to_string(),
                        style: msg_style,
                    };
                }
            }

            State::Running => {
                let mut snake_style = theme.get(ThemeField::String);
                snake_style.text_style = Some(text_style::BOLD);

                let mut last_direction = None;
                let mut last = self.snake.front().unwrap();
                cells[last.y][last.x].style = snake_style;
                cells[last.y][last.x].text = "O".into();

                for point in self.snake.iter().skip(1) {
                    use Direction::*;
                    let to = if point.y > last.y {
                        Down
                    } else if point.y < last.y {
                        Up
                    } else if point.x < last.x {
                        Left
                    } else {
                        Right
                    };

                    if let Some(from) = last_direction {
                        let last_body = match (from, to) {
                            (Right, Down) | (Up, Left) => "╗",
                            (Left, Down) | (Up, Right) => "╔",
                            (Right, Up) | (Down, Left) => "╝",
                            (Left, Up) | (Down, Right) => "╚",
                            (Down, Down) | (Up, Up) => "║",
                            (Left, Left) | (Right, Right) => "═",
                            _ => unreachable!("Wrong directions"),
                        };
                        cells[last.y][last.x].text = last_body.into();
                    }

                    if to == Left {
                        cells[point.y][point.x + 1].text = "═".into();
                        cells[point.y][point.x + 1].style = snake_style;
                    }
                    if to == Right {
                        cells[point.y][point.x - 1].text = "═".into();
                        cells[point.y][point.x - 1].style = snake_style;
                    }

                    cells[point.y][point.x].style = snake_style;
                    cells[point.y][point.x].text = if matches!(to, Left | Right) {
                        "═"
                    } else {
                        "║"
                    }
                    .into();

                    last_direction = Some(to);
                    last = point;
                }

                if let Some(point) = self.prev_pop {
                    use Direction::*;
                    let to = if point.y > last.y {
                        Down
                    } else if point.y < last.y {
                        Up
                    } else if point.x < last.x {
                        Left
                    } else {
                        Right
                    };

                    if let Some(from) = last_direction {
                        let last_body = match (from, to) {
                            (Right, Down) | (Up, Left) => "╗",
                            (Left, Down) | (Up, Right) => "╔",
                            (Right, Up) | (Down, Left) => "╝",
                            (Left, Up) | (Down, Right) => "╚",
                            (Down, Down) | (Up, Up) => "║",
                            (Left, Left) | (Right, Right) => "═",
                            _ => unreachable!("Wrong directions"),
                        };
                        cells[last.y][last.x].text = last_body.into();
                    }
                }

                let apple_style = theme.get(ThemeField::Comment);
                cells[self.apple.y][self.apple.x].style = apple_style;
                cells[self.apple.y][self.apple.x].text = "🍎".into()
            }
            State::Done => {
                let msg = format!("Snake died. Score {}...", self.score);
                let start = center_x - msg.chars().count() / 2;
                for (i, ch) in msg.chars().enumerate() {
                    cells[middle_y][start + i] = Cell {
                        text: ch.to_string(),
                        style: msg_style,
                    };
                }
            }
        }
    }

    fn set_tick_sender(&mut self, tick_sender: std::sync::mpsc::Sender<u64>) {
        let _ = tick_sender.send(1000);
        self.tick_sender = Some(tick_sender);
    }
}
