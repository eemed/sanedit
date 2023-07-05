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
    All(Vec<Match>),
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
    pub(crate) fn pike(
        program: &Program,
        input: &mut impl Cursor,
        _stop_at_first_match: bool,
    ) -> VMResult {
        fn add_thread(
            list: &mut ThreadSet,
            pc: usize,
            pos: usize,
            program: &Program,
            slots: &mut Slots,
        ) {
            use Inst::*;

            let mut pcs = Vec::new();
            pcs.push((pc, pc));

            loop {
                let (prev, pc) = match pcs.pop() {
                    Some(pair) => pair,
                    None => return,
                };
                slots.copy(prev, pc);

                match &program[pc] {
                    Jmp(x) => {
                        let x = (pc as isize + *x) as usize;
                        pcs.push((pc, x));
                    }
                    Split(x, y) => {
                        let y = (pc as isize + *y) as usize;
                        pcs.push((pc, y));

                        let x = (pc as isize + *x) as usize;
                        pcs.push((pc, x));
                    }
                    Save(slot) => {
                        slots.get(pc)[*slot] = Some(pos);
                        pcs.push((pc, pc + 1));
                    }
                    _ => list.add_thread(pc),
                }
            }
        }

        let mut saved_pc = 0;
        let mut matched = false;
        let len = program.len();
        let mut current = ThreadSet::with_capacity(len);
        let mut new = ThreadSet::with_capacity(len);
        let mut slots = Slots::new(program.slot_count(), len);

        add_thread(&mut current, 0, 0, program, &mut slots);
        println!("PROG: {program:?}, current: {current:?}");

        loop {
            let pos = input.pos();
            let byte = input.next();

            let mut i = 0;
            while i < current.len() {
                use Inst::*;

                let pc = current[i];
                println!("INS: {:?}, pc: {pc} byte/pos: {byte:?}/{pos}", &program[pc]);
                match &program[pc] {
                    Match => {
                        println!("MATCH: {pos}");
                        saved_pc = pc;
                        matched = true;
                        current.clear();
                    }
                    Byte(inst_byte) => {
                        if byte == Some(*inst_byte) {
                            slots.copy(pc, pc + 1);
                            add_thread(&mut new, pc + 1, pos + 1, program, &mut slots);
                        }
                    }
                    ByteRange(start, end) => {
                        if let Some(ref byte) = byte {
                            if start <= byte && byte <= end {
                                slots.copy(pc, pc + 1);
                                add_thread(&mut new, pc + 1, pos + 1, program, &mut slots);
                            }
                        }
                    }
                    _ => unreachable!(),
                }

                i += 1;
            }

            if byte.is_none() {
                break;
            }

            mem::swap(&mut current, &mut new);
            new.clear();
        }

        if matched {
            let groups = slots.get_as_pairs(saved_pc);
            crate::Match::from_groups(groups).into()
        } else {
            VMResult::NoMatch
        }
    }
}
