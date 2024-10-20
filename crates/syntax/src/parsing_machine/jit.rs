mod code;

use code::MachineCode;

struct Jit {
    code: MachineCode,
}

impl Jit {
    pub fn new() -> Jit {
        let code = MachineCode::new();

        Jit { code }
    }

    // Compile routines here
    // fn compile_step() { code.mov(a, b); }
}
