mod compiler;
mod inst;
mod program;
mod slots;
mod thread;

pub(crate) use program::Program;

use std::mem;

use crate::cursor::Cursor;
use crate::Match;

use self::inst::Inst;
use self::slots::Slots;
use self::thread::ThreadSet;

#[derive(Debug)]
pub(crate) enum VMResult {
    /// The first pair is the whole match, and the rest are capturing groups
    /// used in the regex.
    Match(Match),
    NoMatch,
}

impl From<Match> for VMResult {
    fn from(m: Match) -> Self {
        VMResult::Match(m)
    }
}

pub(crate) struct VM;

impl VM {
    /// Run a program on thompsons NFA simulation VM
    pub(crate) fn thompson(program: &Program, input: &mut impl Cursor) -> VMResult {
        let len = program.len();
        let mut current = ThreadSet::with_capacity(len);
        let mut new = ThreadSet::with_capacity(len);
        let mut slots = Slots::new(program.slot_count(), len);

        current.add_thread(0);

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
                        return crate::Match::from_groups(groups).into();
                    }
                    Byte(inst_byte) => {
                        if byte == Some(*inst_byte) {
                            new.add_thread(pc + 1);
                            slots.copy(pc, pc + 1);
                        }
                    }
                    ByteRange(range) => {
                        if let Some(ref byte) = byte {
                            if range.contains(byte) {
                                new.add_thread(pc + 1);
                                slots.copy(pc, pc + 1);
                            }
                        }
                    }
                    Jmp(x) => {
                        let x = (pc as isize + *x) as usize;
                        current.add_thread(x);
                        slots.copy(pc, x);
                    }
                    Split(x, y) => {
                        let x = (pc as isize + *x) as usize;
                        current.add_thread(x);
                        slots.copy(pc, x);

                        let y = (pc as isize + *y) as usize;
                        current.add_thread(y);
                        slots.copy(pc, y);
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

        VMResult::NoMatch
    }
}
