use core::fmt;
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

impl fmt::Debug for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "-- Begin program -- start {} --", self.start)?;

        for (i, inst) in self.iter().enumerate() {
            writeln!(f, "{i:02}: {inst:?}")?;
        }

        writeln!(f, "-- end program --")
    }
}
