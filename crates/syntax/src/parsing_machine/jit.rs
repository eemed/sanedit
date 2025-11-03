use std::alloc::Layout;

use crate::grammar::Rules;
use crate::source::Source;
use crate::{Annotation, Capture, ParseError};

use super::captures::ParserRef;
use super::compiler::Program;
use super::{CaptureID, CaptureIter, CaptureList, Compiler};
use dynasmrt::{dynasm, AssemblyOffset, DynamicLabel, DynasmApi, DynasmLabelApi, ExecutableBuffer};
use sanedit_buffer::utf8::{ACCEPT, REJECT, UTF8_CHAR_CLASSES, UTF8_TRANSITIONS};

macro_rules! offset_i32 {
    ($struct:path, $field:tt) => {
        core::mem::offset_of!($struct, $field) as i32
    };
}

#[allow(dead_code)]
#[repr(u8)]
#[derive(Debug)]
enum Kind {
    Open = 0,
    Close = 1,
}

#[repr(C)]
#[derive(Debug)]
struct PartialCapture {
    id: u32,
    kind: Kind,
    ptr: *mut u8,
}

impl Drop for PartialCapture {
    fn drop(&mut self) {
        // println!("Drop: {self:?}");
    }
}

macro_rules! asm {
    ($ops:ident $($t:tt)*) => {
        dynasm!($ops
            ; .arch x64
            ; .alias arg1, rdi
            ; .alias arg2, rsi
            ; .alias arg3, rdx
            ; .alias arg4, rcx
            ; .alias arg5, r8
            ; .alias arg6, r9

            // Just for easier handling
            // available: rbx, rcx, rdx, rsi, rdi
            ; .alias tmp, r8
            ; .alias tmp2, r11
            ; .alias tmp3, rdi
            ; .alias trash, r9
            ; .alias label, r10
            ; .alias capture_pointer, r11

            // These are mandatory
            ; .alias state, r12
            ; .alias subject_end, r13
            ; .alias subject_pointer, r14
            ; .alias captop, r15
            $($t)*
        )
    }
}

macro_rules! double_cap_size {
    ($ops:ident, $ptr:expr) => {
        #[allow(clippy::fn_to_numeric_cast)] {
            asm!($ops
                ; mov rdi, state
                ; mov rax, QWORD $ptr as _

                ; call rax
            )
        }
    }
}

macro_rules! check_subject {
    ($ops:ident) => {
        asm!($ops
            ; cmp subject_pointer, subject_end
            ; jge ->fail
        )
    }
}

/// Provide return addr at tmp3
/// Returns length of codepoint at tmp2
/// Returns codepoint at rax
/// If invalid returns -1 at rax
macro_rules! validate_utf8 {
    ($ops:ident, $classes:expr, $transitions:expr) => {
        asm!($ops
            ; .alias cp, rbx
            ; .alias dstate, r9 // decode state
            ; .alias class, rcx
            ; .alias class_byte, cl
            ; .alias byte, r10

            ;->validate_utf8:
            ; mov dstate, ACCEPT as _
            ; xor cp, cp // zero out
            ; xor tmp2, tmp2

            ;start:
            // let byte = byte as u32;
            // Ensure subject_pointer is ok
            ; lea tmp, [subject_pointer + tmp2]
            ; cmp tmp, subject_end
            ; jge >fail
            ; movzx byte, BYTE [tmp]


            // let class = UTF8_CHAR_CLASSES[byte as usize];
            ; mov tmp, QWORD $classes as _
            ; add tmp, byte
            ; movzx class, [tmp]
            // ;  movzx class, [$classes as _ + byte]


            ; cmp dstate, ACCEPT as _
            ; je >ok
            // dstate != ACCEPT
            //      *cp = (byte & 0x3f) | (*cp << 6);
            ; shl cp, 6
            ; and byte, 0x3f
            ; or cp, byte
            ; jmp >next

            // dstate == ACCEPT
            //    *cp = (0xff >> class) & byte;
            ;ok:
            ; mov cp, 0xff
            ; shr cp, class_byte
            ; and cp, byte

            ;next:
            // *dstate = UTF8_TRANSITIONS[(*dstate + (class as u32)) as usize] as u32;
            ; mov tmp, QWORD $transitions as _
            ; add tmp, class
            ; add tmp, dstate
            ; movzx dstate, [tmp]


            ; cmp dstate, ACCEPT as _
            ; je >done

            ; cmp dstate, REJECT as _
            ; je >fail

            ; inc tmp2
            ; jmp <start

            ;done:
            ; inc tmp2

            ;codepoint:
            ; cmp tmp2, 1
            ; je >onebyte
            ; cmp tmp2, 2
            ; je >twobyte
            ; cmp tmp2, 3
            ; je >threebyte
            ; jmp >fourbyte

            // 1-byte
            ; onebyte:
            ; movzx rax, BYTE [subject_pointer]
            ; jmp >end

            // 2-byte
            ;twobyte:
            ; xor rax, rax

            ; mov tmp, 0b0001_1111
            ; and tmp, [subject_pointer]
            ; shl tmp, 6
            ; add rax, tmp

            ; mov tmp, 0b0011_1111
            ; and tmp, [subject_pointer + 1]
            ; add rax, tmp
            ; jmp >end

            // 3-byte
            ;threebyte:
            ; xor rax, rax

            ; mov tmp, 0b0000_1111
            ; and tmp, [subject_pointer]
            ; shl tmp, 12
            ; add rax, tmp

            ; mov tmp, 0b0011_1111
            ; and tmp, [subject_pointer + 1]
            ; shl tmp, 6
            ; add rax, tmp

            ; mov tmp, 0b0011_1111
            ; and tmp, [subject_pointer + 2]
            ; add rax, tmp

            ; jmp >end

            // 4-byte
            ;fourbyte:
            ; xor rax, rax

            ; mov tmp, 0b0000_0111
            ; and tmp, [subject_pointer]
            ; shl tmp, 16
            ; add rax, tmp

            ; mov tmp, 0b0011_1111
            ; and tmp, [subject_pointer + 1]
            ; shl tmp, 12
            ; add rax, tmp

            ; mov tmp, 0b0011_1111
            ; and tmp, [subject_pointer + 2]
            ; shl tmp, 6
            ; add rax, tmp

            ; mov tmp, 0b0011_1111
            ; and tmp, [subject_pointer + 3]
            ; add rax, tmp
            ; jmp >end

            ;fail:
            ;  mov rax, -1

            ;end:
            ; jmp tmp3
        );
    };
}

#[derive(Debug)]
struct State {
    cap: usize, // These are always u64
    len: usize,
    sp: *mut u8,
    ptr: *mut PartialCapture,
}

impl State {
    unsafe extern "C" fn double_cap_size(state_ptr: *mut State) {
        let state = &mut *state_ptr;
        // println!("double_cap_size: {state_ptr:p} {:?}", state);
        let olayout = Layout::array::<PartialCapture>(state.cap).unwrap();
        state.cap *= 2;
        let nlayout = Layout::array::<PartialCapture>(state.cap).unwrap();

        // println!("realloc: {:?} -> {:?}", olayout.size(), nlayout.size());
        let nptr = std::alloc::realloc(state.ptr as *mut u8, olayout, nlayout.size())
            as *mut PartialCapture;
        if nptr.is_null() {
            panic!("Realloc failed")
        }

        state.ptr = nptr;
        // println!("double_cap_size done: {state:?}");
        // unsafe {
        //     let slice = std::slice::from_raw_parts(state.ptr, state.len);
        //     for (i, cap) in slice.iter().enumerate() {
        //         println!("Capture {}: {:?}", i, cap);
        //     }
        // }
    }
}

impl Drop for State {
    fn drop(&mut self) {
        // println!(
        //     "Drop: {self:?}, => {}",
        //     self.ptr as usize % align_of::<PartialCapture>()
        // );

        unsafe {
            let layout = Layout::array::<PartialCapture>(self.cap).unwrap();

            // Drop each element
            for i in 0..self.len {
                std::ptr::drop_in_place(self.ptr.add(i));
            }

            // Free memory
            std::alloc::dealloc(self.ptr as *mut u8, layout);
        }
    }
}

impl Default for State {
    fn default() -> Self {
        let cap = 2;
        let layout = Layout::array::<PartialCapture>(cap).expect("Invalid layout");
        let ptr = unsafe { std::alloc::alloc(layout) as *mut PartialCapture };
        if ptr.is_null() {
            panic!("No space")
        }
        State {
            cap,
            len: 0,
            sp: std::ptr::null_mut::<u8>(),
            ptr,
        }
    }
}

#[derive(Debug)]
pub struct Jit {
    rules: Rules,
    ops: Program,
    program: ExecutableBuffer,
    start: AssemblyOffset,
}

impl Jit {
    pub(crate) fn new(rules: Rules, ops: Program) -> Jit {
        let (program, start) = Self::compile(&ops);
        Jit {
            rules,
            ops,
            program,
            start,
        }
    }

    pub fn from_read<R: std::io::Read>(rules: R) -> Result<Jit, ParseError> {
        let rules = Rules::parse(rules).unwrap();
        let compiler = Compiler::new(&rules);
        let program = compiler.compile().unwrap();
        Jit::from_program(rules, program)
    }

    pub fn program(&self) -> &Program {
        &self.ops
    }

    pub(crate) fn rules(&self) -> &Rules {
        &self.rules
    }

    /// Return compiled verions of pattern if required instruction sets are available
    fn from_program(rules: Rules, ops: Program) -> Result<Jit, ParseError> {
        if !Self::is_available() {
            return Err(ParseError::JitUnsupported);
        }

        //         println!("Program:\n{program:?}");
        let (program, start) = Self::compile(&ops);
        Ok(Jit {
            rules,
            ops,
            program,
            start,
        })
    }

    /// Try to match text multiple times. Skips errors and yields an element only when part of the text matches
    pub fn captures<'a, 'b, S: Source>(&'a self, reader: &'b mut S) -> CaptureIter<'a, 'b, S> {
        CaptureIter {
            parser: ParserRef::Jit(self),
            sp: 0,
            sp_rev: reader.len(),
            source: reader,
        }
    }

    pub fn parse<S: Source>(&self, bytes: &mut S) -> Result<CaptureList, ParseError> {
        let (caps, _) = self.do_parse(bytes, 0, false)?;
        Ok(caps)
    }

    pub(crate) fn do_parse<S: Source>(
        &self,
        source: &mut S,
        offset: u64,
        stop_on_match: bool,
    ) -> Result<(CaptureList, u64), ParseError> {
        // 2kb overlap between windows to not miss matches on boundaries
        // This means matches that would have been larger than 2kb may not be found
        const OVERLAP: usize = 1024 * 2;

        source.refill_buffer(offset)?;

        let mut succesful_parse = false;
        let mut captures = CaptureList::new();
        let mut sp = 0;
        while sp != source.len() {
            let (buf_pos, buf) = source.buffer();
            if let Ok((mut caps, ssp)) = self.parse_chunk(buf) {
                succesful_parse = true;
                sp = buf_pos + ssp as u64;
                // Adjust indices
                caps.iter_mut().for_each(|cap| {
                    cap.start += buf_pos;
                    cap.end += buf_pos;
                });

                if stop_on_match {
                    return Ok((caps, sp));
                }

                if captures.is_empty() {
                    captures = caps;
                } else {
                    // We already have captures thus this is already an overlapping run
                    captures.retain_mut(|cap| cap.start < buf_pos);
                    captures.extend(caps);
                }
            }

            if source.stop() {
                return Err(ParseError::UserStop);
            }

            let overlapping_start = buf_pos + buf.len().saturating_sub(OVERLAP) as u64;
            if !source.refill_buffer(overlapping_start)? {
                break;
            }
        }

        if !succesful_parse {
            return Err(ParseError::ParsingFailed);
        }

        Ok((captures, sp))
    }

    fn parse_chunk<B: AsRef<[u8]>>(&self, bytes: B) -> Result<(CaptureList, usize), ParseError> {
        let mut state = State::default();
        let state_ref = &mut state as *mut State;
        let bytes = bytes.as_ref();
        let peg_program: extern "C" fn(*mut State, *const u8, *const u8) -> i64 =
            unsafe { std::mem::transmute(self.program.ptr(self.start)) };

        let start = bytes.as_ptr();
        let end = unsafe { start.add(bytes.len()) };
        let res = peg_program(state_ref, start, end);

        if res != 0 {
            return Err(ParseError::ParsingFailed);
        }

        let mut captures = Vec::with_capacity(state.len / 2);

        let mut stack = vec![];
        let slice = unsafe { std::slice::from_raw_parts(state.ptr, state.len) };
        for cap in slice {
            match cap.kind {
                Kind::Open => {
                    stack.push(cap);
                }
                Kind::Close => {
                    let start_cap = stack.pop().unwrap();
                    let start_pos = start_cap.ptr as usize - start as usize;
                    let end_pos = start_pos + (cap.ptr as usize - start_cap.ptr as usize);
                    let capture = Capture {
                        id: start_cap.id as usize,
                        start: start_pos as u64,
                        end: end_pos as u64,
                    };
                    // println!("Capture: {capture:?}: {:?}", std::str::from_utf8_unchecked(&bytes[start_pos..end_pos]));
                    captures.push(capture);
                }
            }
        }
        let sp = state.sp as usize - start as usize;

        Ok((captures, sp))
    }

    pub fn is_available() -> bool {
        #[cfg(not(target_arch = "x86_64"))]
        {
            false
        }
        #[cfg(target_arch = "x86_64")]
        {
            true
        }
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

    pub(crate) fn compile(program: &Program) -> (ExecutableBuffer, AssemblyOffset) {
        use super::Operation::*;
        let mut ops = dynasmrt::x64::Assembler::new().unwrap();
        // let mut data: Vec<(DynamicLabel, Vec<u8>)> = vec![];
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
            ; mov subject_pointer, arg2
            ; mov subject_end, arg3

            ; xor captop, captop

            // Push stack entry to indicate no match
            ; lea label, [->nomatch]
            ; push label
            ; push subject_pointer
            ; push captop
        );

        let iter = program.ops.iter().enumerate();
        for (i, op) in iter {
            let ilabel = inst_labels[i];
            asm!(ops
                ;=>ilabel
            );

            match op {
                Jump(l) => {
                    let jump_label = inst_labels[*l];
                    asm!(ops
                        ; jmp =>jump_label
                    );
                }
                Byte(b) => {
                    check_subject!(ops);
                    asm!(ops
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
                Any(n) => {
                    asm!(ops
                        ; add subject_pointer, *n as _
                        ; cmp subject_pointer, subject_end
                        ; jg ->fail
                    );
                }
                UTF8Range(a, b) => {
                    let label = ops.new_dynamic_label();
                    let a = *a as u32;
                    let b = *b as u32;
                    asm!(ops
                        // Prepare validate_utf8
                        ; lea tmp3, [=>label]
                        ; jmp ->validate_utf8

                        // Check result and advance subject_pointer, if between a and b
                        ;=>label
                        // ; cmp rax, -1
                        // ; je ->fail

                        ; cmp rax, a as _
                        ; jl ->fail
                        ; cmp rax, b as _
                        ; jg ->fail

                        ; add subject_pointer, tmp2
                    );
                }
                Set(set) => {
                    check_subject!(ops);
                    let ptr = set.raw();
                    asm!(ops
                        // Compare if byte is found at bitset
                        ; mov tmp2, QWORD ptr as _
                        ; movzx tmp, BYTE [subject_pointer]
                        ; bt [tmp2], tmp
                        ; jnc ->fail
                        ; inc subject_pointer
                    );
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
                Fail => {
                    asm!(ops
                        ; jmp ->fail
                    );
                }
                End => {
                    asm!(ops
                        ; pop trash // captop
                        ; pop trash // subject_pointer
                        ; pop trash // label

                        ; xor rax, rax
                        ; jmp ->epilogue
                    );
                }
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
                FailTwice => {
                    asm!(ops
                        ; pop trash
                        ; pop trash
                        ; pop trash

                        ; jmp ->fail
                    );
                }
                BackCommit(l) => {
                    let jump_label = inst_labels[*l];
                    asm!(ops
                        ; pop captop
                        ; pop subject_pointer
                        ; pop label

                        ; jmp =>jump_label
                    );
                }
                CaptureBegin(id) => {
                    asm!(ops
                        // Check if needs to grow, jump past if not needed
                        ; cmp captop, [state + offset_i32!(State, cap)]
                        ; jb >next
                    );

                    double_cap_size!(ops, State::double_cap_size);

                    asm!(ops
                        ;next:
                        ; mov capture_pointer, captop
                        ; shl capture_pointer, 4 // size_of(PartialCapture) = 16
                        ; add capture_pointer, [state + offset_i32!(State, ptr)]

                        // Save capture to the capture pointer and advance it
                        ; mov DWORD [capture_pointer + offset_i32!(PartialCapture, id)], *id as i32
                        ; mov BYTE [capture_pointer + offset_i32!(PartialCapture, kind)], 0
                        ; mov QWORD [capture_pointer + offset_i32!(PartialCapture, ptr)], subject_pointer

                        // Increase captop and point to the top
                        ; inc captop
                        ; mov [state + offset_i32!(State, len)], captop
                    );
                }
                CaptureLate(id, diff) => {
                    asm!(ops
                        // Check if needs to grow, jump past if not needed
                        ; cmp captop, [state + offset_i32!(State, cap)]
                        ; jb >next
                    );

                    double_cap_size!(ops, State::double_cap_size);

                    asm!(ops
                        ;next:
                        ; mov capture_pointer, captop
                        ; shl capture_pointer, 4 // size_of(PartialCapture) = 16
                        ; add capture_pointer, [state + offset_i32!(State, ptr)]

                        // Save capture to the capture pointer and advance it
                        ; mov DWORD [capture_pointer + offset_i32!(PartialCapture, id)], *id as i32
                        ; mov BYTE [capture_pointer + offset_i32!(PartialCapture, kind)], 0
                        ; mov tmp, subject_pointer
                        ; sub tmp, *diff as _
                        ; mov QWORD [capture_pointer + offset_i32!(PartialCapture, ptr)], tmp

                        // Increase captop and point to the top
                        ; inc captop
                        ; mov [state + offset_i32!(State, len)], captop
                    );
                }
                CaptureEnd => {
                    asm!(ops
                        // Check if needs to grow, jump past if not needed
                        ; cmp captop, [state + offset_i32!(State, cap)]
                        ; jb >next
                    );

                    double_cap_size!(ops, State::double_cap_size);

                    asm!(ops
                        ;next:
                        ; mov capture_pointer, captop
                        ; shl capture_pointer, 4 // size_of(PartialCapture) = 16
                        ; add capture_pointer, [state + offset_i32!(State, ptr)]

                        // Save capture to the capture pointer and advance it
                        ; mov DWORD [capture_pointer + offset_i32!(PartialCapture, id)], 0
                        ; mov BYTE [capture_pointer + offset_i32!(PartialCapture, kind)], 1
                        ; mov QWORD [capture_pointer + offset_i32!(PartialCapture, ptr)], subject_pointer

                        // Increase captop and point to the top
                        ; inc captop
                        ; mov [state + offset_i32!(State, len)], captop
                    );
                }
                TestByte(b, l) => {
                    let jump_label = inst_labels[*l];
                    asm!(ops
                        // Check subject manually to jump to label
                        ; cmp subject_pointer, subject_end
                        ; jge =>jump_label
                        // Compare subject pointer value if not equal jump to label
                        ; cmp [subject_pointer], BYTE *b as _
                        ; jne =>jump_label

                        // Ok
                        // Push a backtrack entry
                        ; lea label, [=>jump_label]
                        ; push label
                        ; push subject_pointer
                        ; push captop

                        ; inc subject_pointer
                    );
                }
                TestSet(set, l) => {
                    let jump_label = inst_labels[*l];
                    let ptr = set.raw();
                    asm!(ops
                        // Check subject manually to jump to label
                        ; cmp subject_pointer, subject_end
                        ; jge =>jump_label

                        // Compare if byte is found at bitset
                        ; mov tmp2, QWORD ptr as _
                        ; movzx tmp, BYTE [subject_pointer]
                        ; bt [tmp2], tmp
                        ; jnc =>jump_label

                        // Ok
                        // Push a backtrack entry
                        ; lea label, [=>jump_label]
                        ; push label
                        ; push subject_pointer
                        ; push captop

                        ; inc subject_pointer
                    );
                }
                Span(set) => {
                    let ptr = set.raw();
                    asm!(ops
                        ;again:
                        // Check subject manually to jump to label
                        ; cmp subject_pointer, subject_end
                        ; jge >next

                        // Compare if byte is found at bitset
                        ; mov tmp2, QWORD ptr as _
                        ; movzx tmp, BYTE [subject_pointer]
                        ; bt [tmp2], tmp
                        // Jump to next if no match
                        ; jnc >next

                        // Go again if ok
                        ; inc subject_pointer
                        ; jmp <again

                        ;next:
                    );
                }
            }
        }

        // Return without a match
        asm!(ops
            ;->nomatch:
            ; mov rax, 1
            ;->epilogue:
            ; mov [state + offset_i32!(State, len)], captop
            ; mov [state + offset_i32!(State, sp)], subject_pointer
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

        let classes = UTF8_CHAR_CLASSES.as_ptr();
        let transitions = UTF8_TRANSITIONS.as_ptr();
        validate_utf8!(ops, classes, transitions);

        // Write needed data
        // for (label, bytes) in data {
        //     asm!(ops
        //         ;=>label
        //         ; .bytes bytes
        //     );
        // }

        // println!("ops: {ops:?}");
        let buf = ops.finalize().unwrap();
        (buf, start)
    }

    pub fn label_for(&self, id: CaptureID) -> &str {
        if let Some(rule) = self.rules.get(id) {
            return &rule.name;
        }

        // If the capture was not from a rule should be from an embedded
        // operation
        "embed"
    }

    pub fn annotations_for(&self, id: CaptureID) -> &[Annotation] {
        self.rules
            .get(id)
            .map(|info| info.annotations.as_slice())
            .unwrap_or(&[])
    }
}

#[cfg(test)]
mod test {
    use crate::{ParsingMachine, Regex};

    use super::*;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn jit_validate() {
        let mut ops = dynasmrt::x64::Assembler::new().unwrap();
        let haystack = "\u{0400}".as_bytes();
        // let haystack = b"\xFF";
        let hs_start = haystack.as_ptr();
        let hs_end = unsafe { hs_start.add(haystack.len()) };
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
            ; mov subject_pointer, QWORD hs_start as _
            ; mov subject_end, QWORD hs_end as _

            ; lea tmp3, [>prologue]
        );

        let classes = UTF8_CHAR_CLASSES.as_ptr();
        let transitions = UTF8_TRANSITIONS.as_ptr();
        validate_utf8!(ops, classes, transitions);

        asm!(ops
            ;prologue:
            // Prologue: callee-saved registers
            ; pop r15
            ; pop r14
            ; pop r13
            ; pop r12
            ; pop rbp
            ; pop rbx
            // ; mov rax, tmp2
            ; ret
        );

        let program = ops.finalize().unwrap();
        let peg_program: extern "C" fn() -> i64 =
            unsafe { std::mem::transmute(program.ptr(start)) };

        let res = peg_program();
        // println!("res: {haystack:?} {res:#02x}");
        assert_eq!(res, 0x400);
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn jit_rust() {
        let peg = include_str!("../../../../runtime/language/rust/syntax.peg");
        let jit = Jit::from_read(std::io::Cursor::new(peg)).expect("Failed to create JIT");
        // println!("{:?}", jit.ops);
        let mut rust = r#"
            use crate::editor::snippets::{Snippet, SNIPPET_DESCRIPTION};

            #[derive(Debug, Hash, PartialEq, Eq, Ord, PartialOrd, Clone)]
            pub(crate) enum Choice {
                Snippet {
                    snippet: Snippet,
                    display: String,
                },
                Path {
                    path: PathBuf,
                    display: String,
                },
                Text {
                    text: String,
                    description: String,
                },
                Numbered {
                    n: u32,
                    text: String,
                    display: String,
                },
                LSPCompletion {
                    item: Box<CompletionItem>,
                },
            }
            impl Choice {
                pub fn from_completion_item(completion: CompletionItem) -> Arc<Choice> {
                    Arc::new(Choice::LSPCompletion {
                        item: Box::new(completion),
                    })
                }
            }
        "#;
        let _captures = jit.parse(&mut rust).expect("Parsing failed");
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn jit_json() {
        let rules = include_str!("../../pegs/json.peg");
        let jit = Jit::from_read(std::io::Cursor::new(rules)).expect("Failed to create JIT");
        // let json = r#"{ "nimi": "perkele", "ika": 42, lapset: ["matti", "teppo"]}"#;
        // println!("{:?}", jit.ops);
        let mut json = include_str!("../../benches/large.json");
        let _captures = jit.parse(&mut json).expect("Parsing failed");
        // for cap in captures {
        //     println!("{cap:?}: {}", &json[cap.start as usize..cap.end as usize]);
        // }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn jit_toml() {
        let rules = include_str!("../../pegs/toml.peg");
        let jit = Jit::from_read(std::io::Cursor::new(rules)).expect("Failed to create JIT");
        let mut json = r#"
        [hello]
        number = 42
        array = ["bob", "alice"]
        debug = true

        [another.section]
        setter = true
        "#;
        let _captures = jit.parse(&mut json).unwrap();
        // for cap in captures {
        //     println!("{cap:?}: {}", &json[cap.start as usize..cap.end as usize]);
        // }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn jit_match_1() {
        let rules = r#"document = "abc";"#;
        let jit = Jit::from_read(std::io::Cursor::new(rules)).unwrap();
        let mut haystack = "abc";
        assert!(jit.parse(&mut haystack).is_ok())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn jit_match_2() {
        let rules = r#"document = ("amet" / .)*;"#;
        let jit = Jit::from_read(std::io::Cursor::new(rules)).unwrap();
        let haystack = LOREM.repeat(10);
        assert!(jit.parse(&mut haystack.as_str()).is_ok())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn jit_match_3() {
        let rules = r#"
            document = (amet / .)*;

            @show
            amet = "amet";
        "#;
        let jit = Jit::from_read(std::io::Cursor::new(rules)).unwrap();
        let haystack = LOREM.repeat(10);
        assert!(jit.parse(&mut haystack.as_str()).is_ok())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn jit_match_4() {
        let rules = r#"@show document = ("amet" / .)*;"#;
        let jit = Jit::from_read(std::io::Cursor::new(rules)).unwrap();
        let haystack = LOREM.repeat(10);
        assert!(jit.parse(&mut haystack.as_str()).is_ok())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn jit_match_5() {
        let rules = Regex::parse_rules("(a|ab)*c").unwrap();
        let parser = ParsingMachine::from_rules_unanchored(rules.0).unwrap();
        // println!("{:?}", parser.program);
        let jit = Jit::from_program(parser.rules, parser.program).unwrap();
        let mut haystack = "c";
        assert!(jit.parse(&mut haystack).is_ok())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn jit_match_6() {
        let rules = Regex::parse_rules("a").unwrap();
        let parser = ParsingMachine::from_rules_unanchored(rules.0).unwrap();
        // println!("{:?}", parser.program);
        let jit = Jit::from_program(parser.rules, parser.program).unwrap();
        let mut haystack = "a";
        assert!(jit.parse(&mut haystack).is_ok())
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn jit_no_match_1() {
        let rules = r#"document = "abc";"#;
        let jit = Jit::from_read(std::io::Cursor::new(rules)).unwrap();
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
