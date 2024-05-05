use super::op::{Addr, CaptureID};

pub(crate) type SubjectPosition = usize;
pub(crate) type CaptureList = Vec<Capture>;

#[derive(Debug, Clone)]
pub(crate) struct Capture {
    pub(crate) id: CaptureID,
    pub(crate) start: SubjectPosition,
    pub(crate) len: usize,
    pub(crate) captures: CaptureList,
}

#[derive(Debug, Clone)]
pub(crate) enum StackEntry {
    Return {
        addr: Addr,
        captures: CaptureList,
    },
    Backtrack {
        addr: Addr,
        spos: SubjectPosition,
        captures: CaptureList,
    },
    Capture {
        capture: Capture,
    },
}

impl StackEntry {
    pub fn captures_mut(&mut self) -> &mut CaptureList {
        match self {
            StackEntry::Return {
                ref mut captures, ..
            } => captures,
            StackEntry::Backtrack {
                ref mut captures, ..
            } => captures,
            StackEntry::Capture { capture } => &mut capture.captures,
        }
    }
}

pub(crate) struct Stack {
    stack: Vec<StackEntry>,
}

impl Stack {
    pub fn new() -> Stack {
        Stack { stack: vec![] }
    }

    pub fn push(&mut self, entry: StackEntry) {
        self.stack.push(entry);
    }

    pub fn push_capture(&mut self, capture: Capture) {
        self.stack.push(StackEntry::Capture { capture });
    }

    pub fn pop(&mut self) -> Option<StackEntry> {
        self.stack.pop()
    }

    pub fn pop_and_prop(&mut self, global: &mut CaptureList) -> Option<StackEntry> {
        let mut entry = self.stack.pop()?;

        let cap_list = self
            .stack
            .last_mut()
            .map(StackEntry::captures_mut)
            .unwrap_or(global);

        cap_list.append(entry.captures_mut());

        Some(entry)
    }

    pub fn last_mut(&mut self) -> Option<&mut StackEntry> {
        self.stack.last_mut()
    }
}
