use std::ops::Range;

use super::op::{Addr, CaptureID};

pub(crate) type SubjectPosition = usize;
pub type CaptureList = Vec<Capture>;

#[derive(Debug, Clone)]
pub struct Capture {
    pub(crate) id: CaptureID,
    pub(crate) start: SubjectPosition,
    pub(crate) len: usize,
    pub(crate) sub_captures: CaptureList,
}

impl Capture {
    pub fn id(&self) -> CaptureID {
        self.id
    }

    pub fn range(&self) -> Range<usize> {
        self.start..self.start + self.len
    }

    pub fn sub_captures(&self) -> &CaptureList {
        &self.sub_captures
    }

    pub fn flatten(&self) -> CaptureList {
        let mut result = Vec::with_capacity(4096);

        result.push(self.clone());

        for cap in &self.sub_captures {
            result.append(&mut cap.flatten());
        }

        result
    }
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
            StackEntry::Capture { capture } => &mut capture.sub_captures,
        }
    }
}

// #[derive(Debug, Clone)]
// pub(crate) struct Stack {
//     stack: Vec<StackEntry>,
// }

// impl Stack {
//     pub fn new() -> Stack {
//         Stack { stack: vec![] }
//     }

//     pub fn push(&mut self, entry: StackEntry) {
//         self.stack.push(entry);
//     }

//     pub fn push_capture(&mut self, capture: Capture) {
//         self.stack.push(StackEntry::Capture { capture });
//     }

//     pub fn pop(&mut self) -> Option<StackEntry> {
//         self.stack.pop()
//     }

//     pub fn pop_and_prop(&mut self, global: &mut CaptureList) -> Option<StackEntry> {
//         let mut entry = self.stack.pop()?;

//         let cap_list = self
//             .stack
//             .last_mut()
//             .map(StackEntry::captures_mut)
//             .unwrap_or(global);

//         cap_list.append(entry.captures_mut());

//         Some(entry)
//     }

//     pub fn last_mut(&mut self) -> Option<&mut StackEntry> {
//         self.stack.last_mut()
//     }

//     pub fn print(&self) {
//         for (i, op) in self.stack.iter().rev().enumerate() {
//             println!("{i}: {op:?}");
//         }
//     }

//     pub fn log(&self) {
//         for (i, op) in self.stack.iter().rev().enumerate() {
//             log::info!("{i}: {op:?}");
//         }
//     }
// }

use std::rc::Rc;

#[derive(Debug, Clone)]
pub(crate) struct Stack {
    top: Option<Rc<Node>>,
}

impl Stack {
    pub fn new() -> Stack {
        Stack { top: None }
    }

    pub fn push(&mut self, entry: StackEntry) {
        let next = std::mem::take(&mut self.top);
        let node = Rc::new(Node { entry, next });
        self.top = Some(node);
    }

    pub fn pop(&mut self) -> Option<StackEntry> {
        let node = std::mem::take(&mut self.top)?;
        let Node { entry, mut next } = Rc::try_unwrap(node).unwrap_or_else(|a| Node::clone(&a));

        self.top = std::mem::take(&mut next);
        Some(entry)
    }

    pub fn push_capture(&mut self, capture: Capture) {
        self.push(StackEntry::Capture { capture });
    }

    pub fn last_mut(&mut self) -> Option<&mut StackEntry> {
        self.top.as_mut().map(Rc::make_mut).map(|n| &mut n.entry)
    }

    pub fn pop_and_prop(&mut self, global: &mut CaptureList) -> Option<StackEntry> {
        let mut entry = self.pop()?;

        let cap_list = self
            .last_mut()
            .map(StackEntry::captures_mut)
            .unwrap_or(global);

        cap_list.append(entry.captures_mut());

        Some(entry)
    }

    pub fn print(&self) {
        let mut i = 0;
        let mut node = &self.top;

        while let Some(n) = node {
            println!("{i}: {:?}", n.entry);
            i += 1;
            node = &n.next;
        }
    }
}

#[derive(Debug, Clone)]
struct Node {
    entry: StackEntry,
    next: Option<Rc<Node>>,
}
