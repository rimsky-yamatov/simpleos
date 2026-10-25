use basic_core::{BasicInterpreter, Console, Program, Statement};
use std::collections::BTreeMap;

mod boot;
mod fs;

pub use boot::{boot_kernel, BootConfig, BootImage};
pub use fs::{HostStorage, Storage};

pub struct Kernel {
    interpreter: BasicInterpreter,
    program: Program,
    console: Box<dyn Console>,
}

impl Kernel {
    pub fn new(console: Box<dyn Console>) -> Self {
        Self {
            interpreter: BasicInterpreter::new(),
            program: BTreeMap::new(),
            console,
        }
    }

    pub fn load_program(&mut self, program: Program) {
        self.program = program;
    }

    pub fn run(&mut self) -> Result<(), String> {
        self.interpreter.execute_program(&self.program, self.console.as_mut())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    struct TestConsole;

    impl Console for TestConsole {
        fn read_line(&mut self) -> Option<String> {
            None
        }

        fn write_line(&mut self, _s: &str) {}
    }

    #[test]
    fn kernel_runs_basic_program() {
        let mut program = BTreeMap::new();
        program.insert(10, Statement::Print { expr: "1 + 2".to_string() });

        let mut kernel = Kernel::new(Box::new(TestConsole));
        kernel.load_program(program);

        assert!(kernel.run().is_ok());
    }
}
