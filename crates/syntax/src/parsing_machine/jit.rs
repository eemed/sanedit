mod code;

use code::AssemblyCode;

struct Jit {
    code: AssemblyCode,
}

impl Jit {
    pub fn new() -> Jit {
        let code = AssemblyCode::new();

        Jit { code }
    }

    // Compile routines here
    // fn compile_step() { code.mov(a, b); }
}
