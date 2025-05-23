use std::{ffi::c_int, mem};

enum Register {
    Rax,
}

#[derive(Debug)]
enum Operation {
    Label(String),
}

#[derive(Debug)]
pub(crate) struct AssemblyCode {
    ops: Vec<Operation>,
}

impl AssemblyCode {
    pub fn new() -> AssemblyCode {
        AssemblyCode { ops: vec![] }
    }

    // ASM things here
    fn mov_reg_memory(&mut self, _reg: Register, _mem: ()) {}

    fn mov_reg_reg(&mut self, _from: Register, _to: Register) {}

    pub fn compile(&mut self) {
        // mov %rdi, %rax
        // self.memory.push(0x48);
        // self.memory.push(0x8b);
        // self.memory.push(0xc7);
        // self.ops.extend_from_slice(&[0x48, 0x8b, 0xc7]);

        // ret
        // self.ops.push(0xc3);
    }

    fn ret(&mut self) {
        // self.ops.push(0xc3);
    }

    pub unsafe fn run(&self, n: c_int) -> c_int {
        // https://github.com/spencertipping/jit-tutorial

        // Create executable memory map
        let page_size = 4096;
        let size = self.ops.len();
        let mut raw_addr: *mut libc::c_void = std::ptr::null_mut();
        libc::posix_memalign(&mut raw_addr, page_size, size);
        libc::mprotect(
            raw_addr,
            size,
            libc::PROT_EXEC | libc::PROT_READ | libc::PROT_WRITE,
        );
        let ptr = mem::transmute(raw_addr);

        // Copy compiled instrcutions to it
        std::ptr::copy(self.ops.as_ptr(), ptr, self.ops.len());

        // Execute
        let func: unsafe extern "C" fn(c_int) -> c_int = mem::transmute(ptr);
        func(n)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn jit() {
        unsafe {
            let mut mc = AssemblyCode::new();
            mc.compile();
            for i in 0..10 {
                println!("i: {i}, f(i) = {}", mc.run(i));
            }
        }
    }
}
