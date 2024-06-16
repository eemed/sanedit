use std::collections::VecDeque;

#[derive(Debug, Clone, Eq, PartialEq)]
#[repr(u8)]
pub(crate) enum HistoryKind {
    Command = 0,
    Search = 1,
}

impl HistoryKind {
    pub const fn variant_count() -> usize {
        2
    }
}

#[derive(Debug, Clone)]
pub(crate) struct History {
    items: VecDeque<String>,
    limit: usize,
    pos: Pos,
}

impl Default for History {
    fn default() -> Self {
        History::new(100)
    }
}

impl History {
    pub fn new(limit: usize) -> History {
        History {
            items: VecDeque::with_capacity(limit),
            limit,
            pos: Pos::First,
        }
    }

    pub fn reset(&mut self) {
        self.pos = Pos::First;
    }

    pub fn get(&self) -> Option<&str> {
        match self.pos {
            Pos::Index(n) => self.items.get(n).map(|s| s.as_str()),
            _ => None,
        }
    }

    pub fn push(&mut self, item: &str) {
        self.items.retain(|i| i != item);

        while self.items.len() >= self.limit {
            self.items.pop_back();
        }

        self.items.push_front(item.into());
    }

    pub fn next(&mut self) -> Option<&str> {
        match self.pos {
            Pos::Last => {
                if !self.items.is_empty() {
                    self.pos = Pos::Index(self.items.len() - 1);
                }
            }
            Pos::Index(n) => {
                self.pos = if n > 0 { Pos::Index(n - 1) } else { Pos::First };
            }
            _ => {}
        }

        self.get()
    }

    pub fn prev(&mut self) -> Option<&str> {
        match self.pos {
            Pos::First => {
                if !self.items.is_empty() {
                    self.pos = Pos::Index(0);
                }
            }
            Pos::Index(n) => {
                let pos = n + 1;
                self.pos = if pos < self.items.len() {
                    Pos::Index(pos)
                } else {
                    Pos::Last
                };
            }
            _ => {}
        }

        self.get()
    }
}

#[derive(Debug, Clone, Copy)]
enum Pos {
    First,
    Last,
    Index(usize),
}
