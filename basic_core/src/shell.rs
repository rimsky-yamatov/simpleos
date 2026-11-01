//! エミュレータとカーネルで共有する対話シェル。
//!
//! 行番号付きの行はプログラムに登録し、それ以外はコマンド/即時実行として処理する。

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use crate::{
    format_statement, parse_program_line, parse_statement, program_from_text, program_to_text,
    sanitize_name, BasicInterpreter, Console, Program, VERSION,
};

/// プログラムの保存先(ホストのディレクトリ、カーネルの RAM ディスクなど)。
pub trait Storage {
    fn list(&self) -> Vec<String>;
    fn save(&mut self, name: &str, text: &str) -> Result<(), String>;
    fn load(&self, name: &str) -> Result<String, String>;
    fn delete(&mut self, name: &str) -> Result<(), String>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellAction {
    Continue,
    Exit,
}

#[derive(Default)]
pub struct Shell {
    interpreter: BasicInterpreter,
    program: Program,
}

fn file_name(arg: &str) -> String {
    let n = sanitize_name(arg).to_ascii_lowercase();
    if let Some(stem) = n.strip_suffix(".bas") {
        if !stem.is_empty() {
            return stem.to_string();
        }
    }
    n
}

impl Shell {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn program(&self) -> &Program {
        &self.program
    }

    /// テキスト形式のプログラムを読み込む。
    pub fn load_text(&mut self, text: &str) {
        self.program = program_from_text(text);
    }

    pub fn banner(&self, console: &mut dyn Console) {
        console.write_line(VERSION);
        console.write_line("Type HELP for a list of commands.");
    }

    /// 1行分の入力を処理する。
    pub fn handle_line(
        &mut self,
        line: &str,
        console: &mut dyn Console,
        storage: &mut dyn Storage,
    ) -> ShellAction {
        let line = line.trim();
        if line.is_empty() {
            return ShellAction::Continue;
        }

        if line.as_bytes()[0].is_ascii_digit() {
            self.edit_line(line, console);
            return ShellAction::Continue;
        }

        let mut parts = line.splitn(2, char::is_whitespace);
        let cmd = parts.next().unwrap_or("").to_ascii_uppercase();
        let arg = parts.next().unwrap_or("").trim();

        match cmd.as_str() {
            "EXIT" | "QUIT" | "SHUTDOWN" | "HALT" | "POWEROFF" => return ShellAction::Exit,
            "HELP" | "?" => print_help(console),
            "VER" | "VERSION" => console.write_line(VERSION),
            "NEW" => {
                self.program.clear();
                self.interpreter.clear();
                console.write_line("Program and variables cleared.");
            }
            "CLS" => console.clear_screen(),
            "LIST" => self.list(console),
            "RUN" => {
                if !arg.is_empty() {
                    match self.load_into(arg, storage) {
                        Ok(_) => {}
                        Err(e) => {
                            console.write_line(&format!("ERROR: {}", e));
                            return ShellAction::Continue;
                        }
                    }
                }
                self.run(console);
            }
            "SAVE" => {
                if arg.is_empty() {
                    console.write_line("Usage: SAVE <name>");
                } else {
                    let name = file_name(arg);
                    match storage.save(&name, &program_to_text(&self.program)) {
                        Ok(()) => console.write_line(&format!("Saved {}", name)),
                        Err(e) => console.write_line(&format!("ERROR: {}", e)),
                    }
                }
            }
            "LOAD" => {
                if arg.is_empty() {
                    console.write_line("Usage: LOAD <name>");
                } else {
                    match self.load_into(arg, storage) {
                        Ok(name) => console.write_line(&format!("Loaded {}", name)),
                        Err(e) => console.write_line(&format!("ERROR: {}", e)),
                    }
                }
            }
            "DIR" | "LS" | "CATALOG" | "TREE" => {
                let names = storage.list();
                if names.is_empty() {
                    console.write_line("<empty>");
                } else {
                    for n in names {
                        console.write_line(&n);
                    }
                }
            }
            "TYPE" | "CAT" => {
                if arg.is_empty() {
                    console.write_line("Usage: TYPE <name>");
                } else {
                    match storage.load(&file_name(arg)) {
                        Ok(text) => {
                            for l in text.lines() {
                                console.write_line(l);
                            }
                        }
                        Err(e) => console.write_line(&format!("ERROR: {}", e)),
                    }
                }
            }
            "DEL" | "DELETE" | "RM" => {
                if arg.is_empty() {
                    console.write_line("Usage: DEL <name>");
                } else {
                    let name = file_name(arg);
                    match storage.delete(&name) {
                        Ok(()) => console.write_line(&format!("Deleted {}", name)),
                        Err(e) => console.write_line(&format!("ERROR: {}", e)),
                    }
                }
            }
            "VARS" => {
                if self.interpreter.variables().is_empty() {
                    console.write_line("<no variables>");
                } else {
                    for (k, v) in self.interpreter.variables() {
                        console.write_line(&format!("{} = {}", k, v));
                    }
                }
            }
            _ => self.direct(line, console),
        }
        ShellAction::Continue
    }

    fn edit_line(&mut self, line: &str, console: &mut dyn Console) {
        let digits = line.bytes().take_while(|b| b.is_ascii_digit()).count();
        let rest = line[digits..].trim();
        if rest.is_empty() {
            // 行番号だけ入力された場合はその行を削除する
            if let Ok(n) = line[..digits].parse::<usize>() {
                self.program.remove(&n);
            }
            return;
        }
        match parse_program_line(line) {
            Some((n, stmt)) => {
                self.program.insert(n, stmt);
            }
            None => console.write_line("ERROR: Syntax error"),
        }
    }

    fn list(&self, console: &mut dyn Console) {
        if self.program.is_empty() {
            console.write_line("<empty program>");
            return;
        }
        for (n, stmt) in &self.program {
            console.write_line(&format!("{} {}", n, format_statement(stmt)));
        }
    }

    fn run(&mut self, console: &mut dyn Console) {
        if self.program.is_empty() {
            console.write_line("ERROR: no program");
            return;
        }
        self.interpreter.clear();
        if let Err(e) = self.interpreter.execute_program(&self.program, console) {
            console.write_line(&format!("ERROR: {}", e));
        }
    }

    fn load_into(&mut self, arg: &str, storage: &mut dyn Storage) -> Result<String, String> {
        let name = file_name(arg);
        let text = storage.load(&name)?;
        self.program = program_from_text(&text);
        Ok(name)
    }

    fn direct(&mut self, line: &str, console: &mut dyn Console) {
        if let Some(stmt) = parse_statement(line) {
            if let Err(e) = self.interpreter.execute_statement(&stmt, console) {
                console.write_line(&format!("ERROR: {}", e));
            }
            return;
        }
        match self.interpreter.eval_expr(line) {
            Ok(v) => console.write_line(&v.to_string()),
            Err(e) => console.write_line(&format!("ERROR: {}", e)),
        }
    }
}

fn print_help(console: &mut dyn Console) {
    const HELP: [&str; 14] = [
        "Commands:",
        "  10 PRINT \"HI\"   add/replace a program line (just '10' deletes it)",
        "  LIST            show the program",
        "  RUN [name]      run the program (or load and run a saved one)",
        "  NEW             clear program and variables",
        "  SAVE <name>     save the program     LOAD <name>  load it",
        "  DIR             list saved programs  TYPE <name>  show one",
        "  DEL <name>      delete a saved program",
        "  VARS / CLS / VER / HELP",
        "  EXIT            halt the system",
        "Statements: LET PRINT INPUT GOTO GOSUB RETURN",
        "  IF..THEN  FOR..TO..STEP  NEXT  REM  CLS  END",
        "Operators: + - * / % ( )   Conditions: = <> < > <= >= AND OR",
        "Ctrl-C or Esc stops a running program.",
    ];
    for l in HELP.iter() {
        console.write_line(l);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeMap;

    struct Cap {
        out: Vec<String>,
    }

    impl Console for Cap {
        fn read_line(&mut self) -> Option<String> {
            None
        }
        fn write_line(&mut self, s: &str) {
            self.out.push(s.to_string());
        }
        fn write_prompt(&mut self, s: &str) {
            self.out.push(s.to_string());
        }
    }

    #[derive(Default)]
    struct Mem {
        files: BTreeMap<String, String>,
    }

    impl Storage for Mem {
        fn list(&self) -> Vec<String> {
            self.files.keys().cloned().collect()
        }
        fn save(&mut self, name: &str, text: &str) -> Result<(), String> {
            self.files.insert(name.to_string(), text.to_string());
            Ok(())
        }
        fn load(&self, name: &str) -> Result<String, String> {
            self.files
                .get(name)
                .cloned()
                .ok_or_else(|| format!("file {} not found", name))
        }
        fn delete(&mut self, name: &str) -> Result<(), String> {
            self.files
                .remove(name)
                .map(|_| ())
                .ok_or_else(|| format!("file {} not found", name))
        }
    }

    fn feed(shell: &mut Shell, cap: &mut Cap, mem: &mut Mem, lines: &[&str]) {
        for l in lines {
            shell.handle_line(l, cap, mem);
        }
    }

    #[test]
    fn enter_run_save_load() {
        let mut shell = Shell::new();
        let mut cap = Cap { out: Vec::new() };
        let mut mem = Mem::default();
        feed(
            &mut shell,
            &mut cap,
            &mut mem,
            &["10 PRINT 1 + 2", "20 PRINT \"x\"", "RUN", "SAVE t", "NEW", "LOAD t", "RUN"],
        );
        assert_eq!(
            cap.out,
            ["3", "x", "Saved t", "Program and variables cleared.", "Loaded t", "3", "x"]
        );
    }

    #[test]
    fn direct_mode_and_delete_line() {
        let mut shell = Shell::new();
        let mut cap = Cap { out: Vec::new() };
        let mut mem = Mem::default();
        feed(&mut shell, &mut cap, &mut mem, &["A = 5", "PRINT A * 2", "(1 + 2)", "10 END", "10", "LIST"]);
        assert_eq!(cap.out, ["10", "3", "<empty program>"]);
    }

    #[test]
    fn exit_command() {
        let mut shell = Shell::new();
        let mut cap = Cap { out: Vec::new() };
        let mut mem = Mem::default();
        assert_eq!(shell.handle_line("exit", &mut cap, &mut mem), ShellAction::Exit);
    }
}
