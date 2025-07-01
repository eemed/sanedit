use crate::grammar::Rules;
use crate::ParseError;

use super::compiler::Program;
use super::{CaptureList, Compiler};
use dynasmrt::{dynasm, AssemblyOffset, DynamicLabel, DynasmApi, DynasmLabelApi, ExecutableBuffer};

#[repr(u8)]
enum Kind {
    Open = 0,
    Close = 1,
}

#[repr(C)]
struct PartialCapture {
    id: u32,
    kind: Kind,
    pos: u64,
}

macro_rules! asm {
    ($ops:ident $($t:tt)*) => {
        dynasm!($ops
            ; .arch x64
            // ; .alias subject_len, rcx
            ; .alias arg1, rdi
            ; .alias arg2, rsi
            ; .alias arg3, rdx
            ; .alias arg4, rcx
            ; .alias arg5, r8
            ; .alias arg6, r9

            // Just for easier handling
            // available: rbx, rcx, rdx, rsi, rdi
            ; .alias tmp, r8
            ; .alias trash, r9
            ; .alias state, r10
            ; .alias label, r11

            // These are mandatory
            ; .alias subject_end, r12
            ; .alias subject_pointer, r13
            ; .alias capture_pointer, r14
            ; .alias captop, r15
            $($t)*
        )
    }
}

struct State {
    captures: Vec<PartialCapture>,
}

impl State {
    unsafe extern "C" fn double_cap_size(state: *mut State) -> *mut PartialCapture {
        todo!()
    }

}

impl Default for State {
    fn default() -> Self {
        State {
            captures: Vec::with_capacity(512),
        }
    }
}

#[derive(Debug)]
pub struct Jit {
    program: ExecutableBuffer,
    start: AssemblyOffset,
}

impl Jit {
    pub fn new<R: std::io::Read>(rules: R) -> Result<Jit, ParseError> {
        let rules = Rules::parse(rules).unwrap();
        let compiler = Compiler::new(&rules);
        let program = compiler.compile().unwrap();
        Jit::from_program(&program)
    }

    /// Return compiled verions of pattern if required instruction sets are available
    pub fn from_program(program: &Program) -> Result<Jit, ParseError> {
        if !Self::is_available() {
            return Err(ParseError::JitUnsupported);
        }

        // println!("Program:\n{program:?}");
        let (program, start) = Self::compile(program);
        Ok(Jit { program, start })
    }

    pub fn parse<B: AsRef<[u8]>>(&self, bytes: B) -> Result<CaptureList, ParseError> {
        let mut state = State::default();
        let bytes = bytes.as_ref();
        let peg_program: extern "C" fn(*mut State, *mut PartialCapture, *const u8, *const u8) -> i64 =
            unsafe { std::mem::transmute(self.program.ptr(self.start)) };
        let start = bytes.as_ptr();
        let end = unsafe { start.add(bytes.len()) };
        let res = peg_program(&mut state, state.captures.as_mut_ptr(), start, end);

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
        let mut ops = dynasmrt::x64::Assembler::new().unwrap();
        let mut data: Vec<(DynamicLabel, Vec<u8>)> = vec![];
        // TODO probably a better way of handling this?
        let inst_labels: Vec<DynamicLabel> = (0..program.ops.len())
            .map(|_| ops.new_dynamic_label())
            .collect();


        let start = ops.offset();
        asm!(ops
            // Prologue: callee-saved registers
            ; push rbx
            ; push rbp
            ; push r12
            ; push r13
            ; push r14
            ; push r15

            // Load passed in subject pointer
            ; mov state, arg1
            ; mov capture_pointer, arg2
            ; mov subject_pointer, arg3
            ; mov subject_end, arg4

            ; mov captop, 0

            // Push stack entry to indicate no match
            ; lea label, [->nomatch]
            ; push label
            ; push subject_pointer
            ; push captop
        );

        let mut iter = program.ops.iter().enumerate();
        while let Some((i, op)) = iter.next() {
            let ilabel = inst_labels[i];
            asm!(ops
                ;=>ilabel
            );

            match op {
                Jump(_) => todo!(),
                Byte(b) => {
                    asm!(ops
                        // ensure subject pointer is ok
                        ; cmp subject_pointer, subject_end
                        ; je ->fail

                        // Compare subject pointer value if not equal jump to fail
                        ; cmp [subject_pointer], BYTE *b as _
                        ; jne ->fail
                        ; inc subject_pointer
                    );
                }
                Call(label) => {
                    let jump_label = inst_labels[*label];
                    asm!(ops
                        // Push call continue addr to stack
                        ; lea label, [>next]
                        ; push label
                        ; push 0     // push subject_pointer as 0 or NULL
                        ; push captop

                        // Jump to call
                        ; jmp =>jump_label
                        ;next:
                    );
                }
                Commit(l) => {
                    let jump_label = inst_labels[*l];
                    asm!(ops
                        ; pop trash
                        ; pop trash
                        ; pop trash
                        ; jmp =>jump_label
                    );
                }
                Choice(l) => {
                    let jump_label = inst_labels[*l];
                    asm!(ops
                        // Push a backtrack entry
                        ; lea label, [=>jump_label]
                        ; push label
                        ; push subject_pointer
                        ; push captop
                    );
                }
                Any(_) => todo!(),
                UTF8Range(_, _) => todo!(),
                Set(set) => {
                    let bytes = set.raw().to_vec();
                    let byte_label = ops.new_dynamic_label();
                    asm!(ops
                        // ensure subject pointer is ok
                        ; cmp subject_pointer, subject_end
                        ; je ->fail

                        // Compare if byte is found at bitset
                        ; movzx tmp, BYTE [subject_pointer]
                        ; bt [=>byte_label], tmp
                        ; jnc ->fail
                        ; inc subject_pointer
                    );

                    data.push((byte_label, bytes));
                }
                Return => {
                    asm!(ops
                        // Pop return address and jump to it
                        ; pop trash // captop
                        ; pop trash // subject_pointer
                        ; pop label
                        ; jmp label
                    );
                }
                Fail => todo!(),
                End => {
                    asm!(ops
                        ; pop trash // captop
                        ; pop trash // subject_pointer
                        ; pop trash // label

                        ; mov rax, 0
                        ; jmp ->epilogue
                    );
                }
                EndFail => todo!(),
                PartialCommit(l) => {
                    let jump_label = inst_labels[*l];
                    asm!(ops
                        ; pop trash // captop
                        ; pop trash // subject_pointer
                        ; push subject_pointer
                        ; push captop
                        ; jmp =>jump_label
                    );
                }
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
            }
        }

        // Return without a match
        asm!(ops
            ;->nomatch:
            ; mov rax, 1
            ;->epilogue:
            ; pop r15
            ; pop r14
            ; pop r13
            ; pop r12
            ; pop rbp
            ; pop rbx
            ; ret
        );


        // Backtrack on failure
        asm!(ops
            ;->fail:
            ; pop captop
            ; pop subject_pointer
            ; pop label

            // If subject pointer is 0, this stack entry is from a call
            // Because we are in a failed state this call was failed, so fetch the next one
            ; cmp subject_pointer, 0
            ; je ->fail
            ; jmp label
        );

        // Write needed data
        for (label, bytes) in data {
            asm!(ops
                ;=>label
                ; .bytes bytes
            );
        }

        // println!("ops: {ops:?}");
        let buf = ops.finalize().unwrap();
        (buf, start)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn jit_match_1() {
        let rules = r#"document = "abc";"#;
        let jit = Jit::new(std::io::Cursor::new(rules)).unwrap();
        let mut haystack = "abc";
        assert!(jit.parse(&mut haystack).is_ok())
    }

    #[test]
    fn jit_match_2() {
        let rules = r#"document = ("amet" / .)*;"#;
        let jit = Jit::new(std::io::Cursor::new(rules)).unwrap();
        let mut haystack = LOREM.repeat(10);
        assert!(jit.parse(&mut haystack).is_ok())
    }

    #[test]
    fn jit_no_match_1() {
        let rules = r#"document = "abc";"#;
        let jit = Jit::new(std::io::Cursor::new(rules)).unwrap();
        let mut haystack = "aac";
        assert!(jit.parse(&mut haystack).is_err())
    }

    const LOREM: &str = "
Lorem ipsum dolor sit amet, consectetur adipiscing elit. Maecenas sit amet tellus
nec turpis feugiat semper. Nam at nulla laoreet, finibus eros sit amet, fringilla
mauris. Fusce vestibulum nec ligula efficitur laoreet. Nunc orci leo, varius eget
ligula vulputate, consequat eleifend nisi. Cras justo purus, imperdiet a augue
malesuada, convallis cursus libero. Fusce pretium arcu in elementum laoreet. Duis
mauris nulla, suscipit at est nec, malesuada pellentesque eros. Quisque semper porta
malesuada. Nunc hendrerit est ac faucibus mollis. Nam fermentum id libero sed
egestas. Duis a accumsan sapien. Nam neque diam, congue non erat et, porta sagittis
turpis. Vivamus vitae mauris sit amet massa mollis molestie. Morbi scelerisque,
augue id congue imperdiet, felis lacus euismod dui, vitae facilisis massa dui quis
sapien. Vivamus hendrerit a urna a lobortis.

Donec ut suscipit risus. Vivamus dictum auctor vehicula. Sed lacinia ligula sit amet
urna tristique commodo. Sed sapien risus, egestas ac tempus vel, pellentesque sed
velit. Duis pulvinar blandit suscipit. Curabitur viverra dignissim est quis ornare.
Nam et lectus purus. Integer sed augue vehicula, volutpat est vel, convallis justo.
Suspendisse a convallis nibh, pulvinar rutrum nisi. Fusce ultrices accumsan mauris
vitae ornare. Cras elementum et ante at tincidunt. Sed luctus scelerisque lobortis.
Sed vel dictum enim. Fusce quis arcu euismod, iaculis mi id, placerat nulla.
Pellentesque porttitor felis elementum justo porttitor auctor.

Aliquam finibus metus commodo sem egestas, non mollis odio pretium. Aenean ex
lectus, rutrum nec laoreet at, posuere sit amet lacus. Nulla eros augue, vehicula et
molestie accumsan, dictum vel odio. In quis risus finibus, pellentesque ipsum
blandit, volutpat diam. Etiam suscipit varius mollis. Proin vel luctus nisi, ac
ornare justo. Integer porttitor quam magna. Donec vitae metus tempor, ultricies
risus in, dictum erat. Integer porttitor faucibus vestibulum. Class aptent taciti
sociosqu ad litora torquent per conubia nostra, per inceptos himenaeos. Vestibulum
ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia Curae; Nam
semper congue ante, a ultricies velit venenatis vitae. Proin non neque sit amet ex
commodo congue non nec elit. Nullam vel dignissim ipsum. Duis sed lobortis ante.
Aenean feugiat rutrum magna ac luctus.

Ut imperdiet non ante sit amet rutrum. Cras vel massa eget nisl gravida auctor.
Nulla bibendum ut tellus ut rutrum. Quisque malesuada lacinia felis, vitae semper
elit. Praesent sit amet velit imperdiet, lobortis nunc at, faucibus tellus. Nullam
porttitor augue mauris, a dapibus tellus ultricies et. Fusce aliquet nec velit in
mattis. Sed mi ante, lacinia eget ornare vel, faucibus at metus.

Pellentesque nec viverra metus. Sed aliquet pellentesque scelerisque. Duis efficitur
erat sit amet dui maximus egestas. Nullam blandit ante tortor. Suspendisse vitae
consectetur sem, at sollicitudin neque. Suspendisse sodales faucibus eros vitae
pellentesque. Cras non quam dictum, pellentesque urna in, ornare erat. Praesent leo
est, aliquet et euismod non, hendrerit sed urna. Sed convallis porttitor est, vel
aliquet felis cursus ac. Vivamus feugiat eget nisi eu molestie. Phasellus tincidunt
nisl eget molestie consectetur. Phasellus vitae ex ut odio sollicitudin vulputate.
Sed et nulla accumsan, eleifend arcu eget, gravida neque. Donec sit amet tincidunt
eros. Ut in volutpat ante.
";
}
