use super::{inst::Inst, program::Program};

use crate::{
    regex::parser::{Postfix, PF},
    RegexError,
};

impl TryFrom<Postfix> for Program {
    type Error = RegexError;

    fn try_from(postfix: Postfix) -> Result<Self, Self::Error> {
        use PF::*;
        let mut blocks = Vec::new();

        for p in postfix {
            match p {
                Any => {
                    let mut insts = Vec::new();
                    insts.push(Inst::ByteRange(0, u8::MAX));
                    blocks.push(insts);
                }
                Range(start, end) => {
                    let mut insts = Vec::new();
                    insts.push(Inst::ByteRange(start, end));
                    blocks.push(insts);
                }
                Char(ch) => {
                    let mut buf = [0u8; 4];
                    ch.encode_utf8(&mut buf);
                    let mut insts = Vec::new();

                    for i in 0..ch.len_utf8() {
                        insts.push(Inst::Byte(buf[i]));
                    }

                    blocks.push(insts);
                }
                Seq => {
                    // e1e2
                    //     codes for e1
                    //     codes for e2
                    let mut e2 = blocks.pop().unwrap();
                    let mut e1 = blocks.pop().unwrap();
                    e1.append(&mut e2);
                    blocks.push(e1);
                }
                Or => {
                    // split L1, L2
                    // L1: codes for e1
                    //     jmp L3
                    // L2: codes for e2
                    // L3:
                    let mut e2 = blocks.pop().unwrap();
                    let mut e1 = blocks.pop().unwrap();
                    let mut insts = Vec::new();
                    let l1 = 1;
                    let l2 = l1 + e1.len() as isize + 1;

                    insts.push(Inst::Split(l1, l2));
                    insts.append(&mut e1);

                    let l3 = e2.len() + 1;
                    insts.push(Inst::Jmp(l3 as isize));
                    insts.append(&mut e2);

                    blocks.push(insts);
                }
                Repeat(n) => {
                    let e = blocks.pop().unwrap();
                    let mut insts = Vec::with_capacity(e.len() * n as usize);

                    for _ in 0..n {
                        insts.extend_from_slice(&e);
                    }

                    blocks.push(insts);
                }
                Star(lazy) => {
                    // L1: split L2, L3
                    // L2: codes for e
                    // jmp L1
                    // L3:
                    //
                    // lazy => split L3, L2

                    let mut e = blocks.pop().unwrap();
                    let mut insts = Vec::new();

                    let l2 = 1;
                    let l3 = l2 + (e.len() as isize) + 1;
                    if lazy {
                        insts.push(Inst::Split(l3, l2));
                    } else {
                        insts.push(Inst::Split(l2, l3));
                    }
                    insts.append(&mut e);

                    let l1 = -(insts.len() as isize);
                    insts.push(Inst::Jmp(l1));

                    blocks.push(insts);
                }
                Plus(lazy) => {
                    // L1: codes for e
                    // split L1, L3
                    // L3:
                    //
                    // lazy => split L3, L1
                    let mut e = blocks.pop().unwrap();
                    let mut insts = Vec::new();

                    let l1 = -(e.len() as isize);
                    let l3 = 1;
                    insts.append(&mut e);
                    if lazy {
                        insts.push(Inst::Split(l3, l1));
                    } else {
                        insts.push(Inst::Split(l1, l3));
                    }

                    blocks.push(insts);
                }
                Question(lazy) => {
                    // split L1, L2
                    // L1: codes for e
                    // L2:
                    //
                    // lazy => split L2, L1
                    let mut e = blocks.pop().unwrap();
                    let mut insts = Vec::new();

                    let l1 = 1;
                    let l2 = l1 + e.len() as isize;
                    if lazy {
                        insts.push(Inst::Split(l2, l1));
                    } else {
                        insts.push(Inst::Split(l1, l2));
                    }
                    insts.append(&mut e);

                    blocks.push(insts);
                }
                Save(n) => {
                    let mut insts = Vec::new();
                    insts.push(Inst::Save(n + 2));
                    blocks.push(insts);
                }
            }
        }

        let mut insts = Vec::new();

        // Add substring searching by prepending .*? insts to the start
        // 00: Split([3, 1])
        // 01: ByteRange(0..255)
        // 02: Jmp(0)
        insts.push(Inst::Split(3, 1));
        insts.push(Inst::ByteRange(0, u8::MAX));
        insts.push(Inst::Jmp(-2));

        // add whole match extraction
        insts.push(Inst::Save(0));
        insts.append(&mut blocks.into_iter().flatten().collect());
        insts.push(Inst::Save(1));
        insts.push(Inst::Match);

        Ok(Program { insts })
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn simple() {
        // let regex = "ab(.*)";
        // let postfix = regex2postfix(regex);
        // if let Ok(prog) = Program::try_from(postfix) {}
    }

    // #[test]
    // fn alt() {
    //     let regex = "a|(b|c)";
    //     let ast = Parser::parse(regex);
    //     let program = Compiler::compile(&ast);
    //     println!("-------- Begin program '{regex}' ---------");
    //     for (i, inst) in program.iter().enumerate() {
    //         println!("{i:02}: {inst:?}");
    //     }
    //     println!("-------- end program ---------");
    // }
}
