use std::{ffi::c_int, mem};

enum Register {
    Rax,
}

#[derive(Debug)]
pub(crate) struct MachineCode {
    memory: Vec<u8>,
}

impl MachineCode {
    pub fn new() -> MachineCode {
        MachineCode { memory: vec![] }
    }

    // ASM things here
    fn mov_reg_memory(&mut self, reg: Register, mem: ()) {}

    fn mov_reg_reg(&mut self, from: Register, to: Register) {}

    pub fn compile(&mut self) {
        // mov %rdi, %rax
        // self.memory.push(0x48);
        // self.memory.push(0x8b);
        // self.memory.push(0xc7);
        self.memory.extend_from_slice(&[0x48, 0x8b, 0xc7]);

        // ret
        self.memory.push(0xc3);
    }

    fn ret(&mut self) {
        self.memory.push(0xc3);
    }

    pub unsafe fn run(&self, n: c_int) -> c_int {
        // https://github.com/spencertipping/jit-tutorial

        // Create executable memory map
        let page_size = 4096;
        let size = self.memory.len();
        let mut raw_addr: *mut libc::c_void = std::ptr::null_mut();
        libc::posix_memalign(&mut raw_addr, page_size, size);
        libc::mprotect(
            raw_addr,
            size,
            libc::PROT_EXEC | libc::PROT_READ | libc::PROT_WRITE,
        );
        let ptr = mem::transmute(raw_addr);

        // Copy compiled instrcutions to it
        std::ptr::copy(self.memory.as_ptr(), ptr, self.memory.len());

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
            let mut mc = MachineCode::new();
            mc.compile();
            for i in 0..10 {
                println!("i: {i}, f(i) = {}", mc.run(i));
            }
        }
    }
}
