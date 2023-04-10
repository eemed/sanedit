use core::fmt;
use std::ops::Deref;

use super::inst::{Inst, InstIndex};

pub(crate) struct Program {
    pub insts: Vec<Inst>,
}

impl Program {
    pub fn slot_count(&self) -> usize {
        self.insts
            .iter()
            .filter(|inst| matches!(inst, Inst::Save(..)))
            .count()
    }
}

impl Deref for Program {
    type Target = Vec<Inst>;

    fn deref(&self) -> &Self::Target {
        &self.insts
    }
}

impl fmt::Debug for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "-- Begin program --")?;

        for (i, inst) in self.iter().enumerate() {
            writeln!(f, "{i:02}: {inst:?}")?;
        }

        writeln!(f, "-- end program --")
    }
}
