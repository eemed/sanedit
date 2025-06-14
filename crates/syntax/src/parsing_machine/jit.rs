use crate::{ByteSource, ParseError};

use super::compiler::Program;
use super::CaptureList;
use dynasmrt::{dynasm, AssemblyOffset, DynamicLabel, DynasmApi, DynasmLabelApi, ExecutableBuffer};
use rustc_hash::FxHashMap;

use std::io::Write;
use std::{io, mem, slice};

macro_rules! asm {
    ($ops:ident $($t:tt)*) => {
        dynasm!($ops
            ; .arch x64
            // ; .alias subject_len, rcx
            ; .alias subject_end, rcx
            ; .alias subject_pointer, rbx
            $($t)*
        )
    }
}

#[derive(Debug)]
pub(crate) struct Jit {
    program: ExecutableBuffer,
    start: AssemblyOffset,
}

impl Jit {
    /// Return compiled verions of pattern if required instruction sets are available
    pub fn new(program: &Program) -> Option<Jit> {
        if !Self::is_available() {
            return None;
        }

        println!("Program:\n{program:?}");
        let (program, start) = Self::compile(program);
        Some(Jit { program, start })
    }

    pub fn parse<B: ByteSource>(&self, reader: &mut B) -> Result<CaptureList, ParseError> {
        let peg_program: extern "win64" fn() -> bool = unsafe { mem::transmute(self.program.ptr(self.start)) };
        let ok = peg_program();
        if ok {
            Ok(CaptureList::new())
        } else {
            Err(ParseError::Parse("No match".into()))
        }
    }

    pub fn is_available() -> bool {
        #[cfg(not(target_feature = "sse2"))]
        {
            false
        }
        #[cfg(target_feature = "sse2")]
        {
            #[cfg(target_feature = "avx2")]
            {
                true
            }
            #[cfg(not(target_feature = "avx2"))]
            {
                std::is_x86_feature_detected!("avx2")
            }
        }
    }

    fn compile(program: &Program) -> (ExecutableBuffer, AssemblyOffset)  {
        use super::Operation::*;

        let mut labels: FxHashMap<String, DynamicLabel> = FxHashMap::default();
        let mut ops = dynasmrt::x64::Assembler::new().unwrap();

        let start = ops.offset();
        let mut iter = program.ops.iter().enumerate();
        while let Some((i, op)) = iter.next() {
            println!("------------------");
            println!("{ops:?}");
            // Insert labels where necessary
            if let Some(label) = program.names.get(&i) {
                let entry = labels.entry(label.clone());
                let dyn_label = entry.or_insert_with(|| ops.new_dynamic_label()).clone();
                asm!(ops
                    ;=>dyn_label
                );
            }

            match op {
                Jump(_) => todo!(),
                Byte(b) => {
                    asm!(ops
                        // TODO should this be done here
                        // ; cmp subject_pointer, subject_len
                        // ; jl ->fail

                        ; cmp subject_pointer, BYTE *b as _
                        ; jne ->fail_state
                        ; inc subject_pointer
                    );
                }
                Call(label) => {
                    let call_label = program
                        .names
                        .get(label)
                        .expect("Could not find call label")
                        .clone();
                    let dyn_label = ops.new_dynamic_label();
                    labels.insert(call_label, dyn_label);
                    asm!(ops
                        ; call =>dyn_label
                    );
                }
                Commit(_) => todo!(),
                Choice(_) => todo!(),
                Any(_) => todo!(),
                UTF8Range(_, _) => todo!(),
                Set(set) => todo!(),
                Return => {
                    asm!(ops
                        ; ret
                    );
                }
                Fail => todo!(),
                End => {
                    asm!(ops
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

        asm!(ops
            ; ->fail_state:
        );

        let buf = ops.finalize().unwrap();
        (buf, start)
    }

    fn reference(&mut self) {
        let mut ops = dynasmrt::x64::Assembler::new().unwrap();
        let string = "Hello World!";

        dynasm!(ops
            ; .arch x64
            ; ->hello:
            ; .bytes string.as_bytes()
        );

        let hello = ops.offset();
        dynasm!(ops
            ; .arch x64
            ; lea rcx, [->hello]
            ; xor edx, edx
            ; mov dl, BYTE string.len() as _
            ; mov rax, QWORD print as _
            ; sub rsp, BYTE 0x28
            ; call rax
            ; add rsp, BYTE 0x28
            ; ret
        );

        let buf = ops.finalize().unwrap();

        let hello_fn: extern "win64" fn() -> bool = unsafe { mem::transmute(buf.ptr(hello)) };
    }
}

pub extern "win64" fn print(buffer: *const u8, length: u64) -> bool {
    io::stdout()
        .write_all(unsafe { slice::from_raw_parts(buffer, length as usize) })
        .is_ok()
}

#[cfg(test)]
mod test {
    use crate::{grammar::Rules, parsing_machine::compiler::Compiler};

    use super::*;

    fn make_jit(rules: &str) -> Jit {
        let rules = Rules::parse(std::io::Cursor::new(rules)).unwrap();
        let compiler = Compiler::new(&rules);
        let program = compiler.compile().unwrap();
        Jit::new(&program).unwrap()
    }

    #[test]
    fn jit_match() {
        let rules = r#"document = "abc";"#;
        let jit = make_jit(rules);
    }
}
