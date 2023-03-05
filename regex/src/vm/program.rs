use std::ops::Deref;

use super::inst::{Inst, InstPtr};

pub(crate) struct Program {
    start: InstPtr,
    insts: Vec<Inst>,
}

impl Deref for Program {
    type Target = Vec<Inst>;

    fn deref(&self) -> &Self::Target {
        &self.insts
    }
}
