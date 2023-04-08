mod compiler;
mod inst;
mod program;
mod slots;
mod thread;

pub(crate) use compiler::Compiler;
pub(crate) use program::Program;

use std::mem;

use crate::cursor::Cursor;
use crate::regex::RegexResult;

use self::inst::Inst;
use self::slots::Slots;
use self::thread::ThreadSet;

pub(crate) struct VM;

impl VM {
    /// Run a program on thompsons NFA simulation VM
    pub(crate) fn thompson(program: &Program, input: &mut impl Cursor) -> RegexResult {
        let len = program.len();
        let mut current = ThreadSet::with_capacity(len);
        let mut new = ThreadSet::with_capacity(len);
        let mut slots = Slots::new(program.slot_count(), len);

        current.add_thread(program.start);

        loop {
            let pos = input.pos();
            let byte = input.next();

            let mut i = 0;
            while i < current.len() {
                use Inst::*;

                let pc = current[i];
                match &program[pc] {
                    Match => {
                        let groups = slots.get_as_pairs(pc);
                        return RegexResult::Match(groups);
                    }
                    Byte(inst_byte) => {
                        if byte == Some(*inst_byte) {
                            new.add_thread(pc + 1);
                            slots.copy(pc, pc + 1);
                        }
                    }
                    Jmp(x) => {
                        current.add_thread(*x);
                        slots.copy(pc, *x);
                    }
                    Split(splits) => {
                        for split in splits {
                            current.add_thread(*split);
                            slots.copy(pc, *split);
                        }
                    }
                    Save(slot) => {
                        slots.get(pc)[*slot] = Some(pos);
                        current.add_thread(pc + 1);
                        slots.copy(pc, pc + 1);
                    }
                }

                i += 1;
            }

            if byte.is_none() {
                break;
            }

            mem::swap(&mut current, &mut new);
            new.clear();
        }

        RegexResult::NoMatch
    }
}
