use crate::grammar::Rules;
use crate::source::ChunkSource;
use crate::{ByteSource, ParseError};

use super::compiler::Program;
use super::{CaptureList, Compiler};
use dynasmrt::{dynasm, AssemblyOffset, DynamicLabel, DynasmApi, DynasmLabelApi, ExecutableBuffer};
use rustc_hash::FxHashMap;

use std::io::Write;
use std::{io, mem, slice};

macro_rules! asm {
    ($ops:ident $($t:tt)*) => {
        dynasm!($ops
            ; .arch x64
            // ; .alias subject_len, rcx
            ; .alias label, r13
            ; .alias state, r12
            ; .alias subject_end, rcx
            ; .alias subject_pointer, rbx
            $($t)*
        )
    }
}

macro_rules! prologue {
    ($ops:ident) => {{
        start
    }};
}
struct State<'a, C: ChunkSource> {
    bytes: &'a mut C,
}

impl<'a, C: ChunkSource> State<'a, C> {
    // unsafe extern "C" fn len(state: *mut State<C>) -> u64 {
    //     let state = &mut *state;
    //     state.bytes.len() as u64
    // }

    // unsafe extern "C" fn get(state: *mut State<C>, at: *mut u64) -> u8 {
    //     let state = &mut *state;
    //     state.bytes[*at as usize]
    // }
}

#[derive(Debug)]
pub(crate) struct Jit {
    program: ExecutableBuffer,
    start: AssemblyOffset,
}

impl Jit {
    pub fn new(rules: &str) -> Result<Jit, ParseError> {
        let rules = Rules::parse(std::io::Cursor::new(rules)).unwrap();
        let compiler = Compiler::new(&rules);
        let program = compiler.compile().unwrap();
        Jit::from_program(&program)
    }

    /// Return compiled verions of pattern if required instruction sets are available
    pub fn from_program(program: &Program) -> Result<Jit, ParseError> {
        if !Self::is_available() {
            return Err(ParseError::JitUnsupported);
        }

        println!("Program:\n{program:?}");
        let (program, start) = Self::compile(program);
        Ok(Jit { program, start })
    }

    pub fn parse<C: ChunkSource>(&self, reader: &mut C) -> Result<CaptureList, ParseError> {
        let state = State { bytes: reader };
        let (_start, chunk) = state.bytes.get();
        let chunk = chunk.as_ref();

        let peg_program: extern "C" fn(*const u8, *const u8) -> i64 =
            unsafe { mem::transmute(self.program.ptr(self.start)) };
        let start = chunk.as_ptr();
        let end = unsafe { start.add(chunk.len()) };
        println!("Start: {start:?}, end: {end:?}");
        let res = peg_program(start, end);
        println!("Return code: {res}");
        if res == 0 {
            Ok(CaptureList::new())
        } else {
            Err(ParseError::Parse("No match".into()))
        }
    }

    pub fn is_available() -> bool {
        true
        // #[cfg(not(target_feature = "sse2"))]
        // {
        //     false
        // }
        // #[cfg(target_feature = "sse2")]
        // {
        //     #[cfg(target_feature = "avx2")]
        //     {
        //         true
        //     }
        //     #[cfg(not(target_feature = "avx2"))]
        //     {
        //         std::is_x86_feature_detected!("avx2")
        //     }
        // }
    }

    fn compile(program: &Program) -> (ExecutableBuffer, AssemblyOffset) {
        use super::Operation::*;
        let mut labels: FxHashMap<String, DynamicLabel> = FxHashMap::default();
        let mut ops = dynasmrt::x64::Assembler::new().unwrap();

        // Return without a match
        asm!(ops
            ;->nomatch:
            ; mov rax, 1
            ; ret
        );

        let start = ops.offset();
        asm!(ops
            // Load passed in subject pointer
            ; mov subject_pointer, rdi
            ; mov subject_end, rsi

            // Push stack entry to indicate no match
            ; lea label, [->nomatch]
            ; push label
            ; push subject_pointer
        );

        let mut iter = program.ops.iter().enumerate();
        while let Some((i, op)) = iter.next() {
            // Insert labels where necessary
            if let Some(label) = program.names.get(&i) {
                let entry = labels.entry(label.clone());
                let dyn_label = entry.or_insert_with(|| ops.new_dynamic_label()).clone();
                println!("LABEL: {op:?}");
                asm!(ops
                    ;=>dyn_label
                );
            }

            match op {
                Jump(_) => todo!(),
                Byte(b) => {
                    asm!(ops
                        // TODO should this be done here
                        ; cmp subject_pointer, subject_end
                        ; je ->fail

                        // Compare subject pointer value if not equal jump to fail
                        ; cmp [subject_pointer], BYTE *b as _
                        ; jne ->fail
                        ; inc subject_pointer
                    );
                }
                Call(label) => {
                    let call_label = program
                        .names
                        .get(label)
                        .expect("Could not find call label")
                        .clone();
                    let entry = labels.entry(call_label);
                    let dyn_label = entry.or_insert_with(|| ops.new_dynamic_label()).clone();
                    asm!(ops
                        // Push call continue addr to stack
                        ; lea label, [>next]
                        ; push label
                        ; push 0     // push subject_pointer as 0 or NULL

                        // Jump to call
                        ; jmp =>dyn_label
                        ;next:
                    );
                }
                Commit(_) => todo!(),
                Choice(_) => todo!(),
                Any(_) => todo!(),
                UTF8Range(_, _) => todo!(),
                Set(set) => todo!(),
                Return => {
                    asm!(ops
                        // Pop return address and jump to it
                        ; pop label // Discard label
                        ; pop label
                        ; jmp label
                    );
                }
                Fail => todo!(),
                End => {
                    asm!(ops
                        ; pop label // Discard subject_pointer
                        ; pop label // Discard label
                        ; mov rax, 0
                        ; ret
                    );
                }
                EndFail => todo!(),
                PartialCommit(_) => todo!(),
                FailTwice => todo!(),
                Span(set) => todo!(),
                BackCommit(_) => todo!(),
                TestChar(_, _) => todo!(),
                TestCharNoChoice(_, _) => todo!(),
                TestSet(set, _) => todo!(),
                TestSetNoChoice(set, _) => todo!(),
                TestAny(_, _) => todo!(),
                CaptureBegin(_) => todo!(),
                CaptureBeginMultiEnd(_) => todo!(),
                CaptureEnd => todo!(),
                Checkpoint => todo!(),
            }
        }

        // Backtrack on failure
        asm!(ops
            ;->fail:
            ; pop subject_pointer
            ; pop label

            // If subject pointer is 0, this stack entry is from a call
            // Because we are in a failed state this call was failed, so fetch the next one
            ; cmp subject_pointer, 0
            ; je ->fail
            ; jmp label
        );

        // println!("ops: {ops:?}");
        let buf = ops.finalize().unwrap();
        (buf, start)
    }
}

#[cfg(test)]
mod test {
    use crate::{grammar::Rules, parsing_machine::compiler::Compiler};

    use super::*;

    fn make_jit(rules: &str) -> Jit {
        let rules = Rules::parse(std::io::Cursor::new(rules)).unwrap();
        let compiler = Compiler::new(&rules);
        let program = compiler.compile().unwrap();
        Jit::from_program(&program).unwrap()
    }

    #[test]
    fn jit_match_1() {
        let rules = r#"document = "abc";"#;
        let jit = make_jit(rules);
        let mut haystack = "abc";
        let _ = jit.parse(&mut haystack);
    }

    #[test]
    fn jit_no_match_1() {
        let rules = r#"document = "abc";"#;
        let jit = make_jit(rules);
        let mut haystack = "aac";
        let _ = jit.parse(&mut haystack);
    }
}
