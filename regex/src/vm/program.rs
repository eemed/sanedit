use std::ops::Deref;

use super::inst::{Inst, InstIndex};

pub(crate) struct Program {
    pub start: InstIndex,
    pub insts: Vec<Inst>,
}

impl Deref for Program {
    type Target = Vec<Inst>;

    fn deref(&self) -> &Self::Target {
        &self.insts
    }
}
