use crate::SubjectPosition;

use super::op::Addr;

pub(crate) type Stack = Vec<StackEntry>;

#[derive(Debug, Clone)]
pub(crate) enum StackEntry {
    Return {
        addr: Addr,
    },
    Backtrack {
        addr: Addr,
        spos: SubjectPosition,
        caplevel: usize,
    },
}
