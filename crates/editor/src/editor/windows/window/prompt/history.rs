use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub(crate) enum HistoryKind {
    Search,
    Grep,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum HistoryPosition {
    First,
    Pos(usize),
    Last,
}

#[derive(Debug, Clone)]
pub(crate) struct History {
    items: VecDeque<String>,
    limit: usize,
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
        }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn get(&self, pos: HistoryPosition) -> Option<&str> {
        match pos {
            HistoryPosition::Pos(n) => self.items.get(n).map(|s| s.as_str()),
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
}
