use basic_core::{BasicInterpreter, Program, Statement};
use std::collections::BTreeMap;

pub struct Kernel {
    interpreter: BasicInterpreter,
    program: Program,
}

impl Kernel {
    pub fn new() -> Self {
        Self {
            interpreter: BasicInterpreter::new(),
            program: BTreeMap::new(),
        }
    }

    pub fn load_program(&mut self, program: Program) {
        self.program = program;
    }

    pub fn run(&mut self) -> Result<(), String> {
        self.interpreter.execute_program(&self.program)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn kernel_runs_basic_program() {
        let mut program = BTreeMap::new();
        program.insert(10, Statement::Print { expr: "1 + 2".to_string() });

        let mut kernel = Kernel::new();
        kernel.load_program(program);

        assert!(kernel.run().is_ok());
    }
}
