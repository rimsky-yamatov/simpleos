use basic_core::{
    parse_let, parse_program_line, sanitize_name, split_leading_number, format_statement,
    BasicInterpreter, Console, Program, Statement,
};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Default)]
struct StdConsole;

impl Console for StdConsole {
    fn read_line(&mut self) -> Option<String> {
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => None,
            Ok(_) => Some(input.trim_end_matches(['\r', '\n']).to_string()),
            Err(_) => None,
        }
    }

    fn write_line(&mut self, s: &str) {
        println!("{}", s);
    }
}

#[derive(Default)]
struct HostStorage {
    root: PathBuf,
}

impl HostStorage {
    fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn list_dir(&self) -> Vec<String> {
        let mut entries = fs::read_dir(&self.root)
            .map(|it| it.filter_map(Result::ok).collect::<Vec<_>>())
            .unwrap_or_default();
        entries.sort_by_key(|e| e.file_name());
        entries
            .into_iter()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect()
    }

    fn save_program(&self, name: &str, program: &Program) -> bool {
        let file = self.root.join(format!("{}.bas", sanitize_name(name)));
        let mut text = String::new();
        for (num, stmt) in program.iter() {
            text.push_str(&format!("{} {}\n", num, format_statement(stmt)));
        }
        fs::write(file, text).is_ok()
    }

    fn load_program(&self, name: &str) -> Result<Program, String> {
        let file = self.root.join(format!("{}.bas", sanitize_name(name)));
        let content = fs::read_to_string(file).map_err(|_| format!("file {} not found", name))?;
        let mut program = BTreeMap::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some((num, stmt)) = parse_program_line(trimmed) {
                program.insert(num, stmt);
            }
        }
        Ok(program)
    }
}

fn ensure_fs_dir() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("fs");
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn main() {
    let fs_dir = ensure_fs_dir();
    let mut storage = HostStorage::new(fs_dir.clone());
    let mut console = StdConsole;
    let mut interpreter = BasicInterpreter::new();
    let mut program: Program = BTreeMap::new();

    println!("SimpleOS emulator v0.2");
    println!("BASIC shell ready. Type HELP, LIST, RUN, SAVE <name>, LOAD <name>, DIR, NEW or EXIT.");
    println!("Filesystem directory: {}", fs_dir.display());

    loop {
        print!("BASIC> ");
        io::stdout().flush().unwrap();

        let Some(input) = console.read_line() else { break; };
        let line = input.trim();
        if line.is_empty() {
            continue;
        }

        let upper = line.to_uppercase();
        if upper == "EXIT" || upper == "QUIT" {
            println!("Goodbye.");
            break;
        }

        if upper == "NEW" {
            program.clear();
            interpreter.clear();
            println!("Program and variables cleared.");
            continue;
        }

        if upper == "HELP" {
            print_help();
            continue;
        }

        if upper == "DIR" || upper == "TREE" {
            let entries = storage.list_dir();
            if entries.is_empty() {
                println!("<empty>");
            } else {
                for entry in entries {
                    println!("{}", entry);
                }
            }
            continue;
        }

        if upper == "LIST" {
            if program.is_empty() {
                println!("<empty program>");
            } else {
                for (num, stmt) in &program {
                    println!("{:>5}  {}", num, format_statement(stmt));
                }
            }
            continue;
        }

        if upper == "RUN" {
            match interpreter.execute_program(&program, &mut console) {
                Ok(_) => {}
                Err(e) => println!("ERROR: {}", e),
            }
            continue;
        }

        if let Some(name) = line.strip_prefix("SAVE ") {
            if storage.save_program(name.trim(), &program) {
                println!("Saved {}", name.trim());
            } else {
                println!("FAILED: save {}", name.trim());
            }
            continue;
        }

        if let Some(name) = line.strip_prefix("LOAD ") {
            match storage.load_program(name.trim()) {
                Ok(p) => {
                    program = p;
                    println!("Loaded {}", name.trim());
                }
                Err(e) => println!("ERROR: {}", e),
            }
            continue;
        }

        if split_leading_number(line).is_some() {
            if let Some((number, stmt)) = parse_program_line(line) {
                program.insert(number, stmt);
                continue;
            }
        }

        if let Some((name, expr)) = parse_let(&line[4..]) {
            match interpreter.eval_expr(expr) {
                Ok(v) => {
                    interpreter.set_var(&name, v);
                    println!("{} = {}", name, v);
                }
                Err(e) => println!("ERROR: {}", e),
            }
            continue;
        }

        if let Some(expr) = line.strip_prefix("PRINT ") {
            match interpreter.eval_expr(expr) {
                Ok(v) => println!("{}", v),
                Err(e) => println!("ERROR: {}", e),
            }
            continue;
        }

        if let Some(var) = line.strip_prefix("INPUT ") {
            let trimmed = var.trim();
            print!("{}? ", trimmed);
            io::stdout().flush().unwrap();
            let Some(response) = console.read_line() else {
                println!("\nERROR: INPUT read failed");
                continue;
            };
            let value = match response.trim().parse::<i64>() {
                Ok(v) => v,
                Err(_) => {
                    println!("ERROR: INPUT expects integer");
                    continue;
                }
            };
            interpreter.set_var(trimmed, value);
            continue;
        }

        match interpreter.eval_expr(line) {
            Ok(v) => println!("{}", v),
            Err(e) => println!("ERROR: {}", e),
        }
    }
}

fn print_help() {
    println!("Commands:");
    println!("  10 PRINT 1 + 2");
    println!("  RUN");
    println!("  LIST");
    println!("  SAVE <basename>");
    println!("  LOAD <basename>");
    println!("  DIR / TREE");
    println!("  NEW");
    println!("  FOR I = 1 TO 3");
    println!("  NEXT I");
    println!("  LET A = 10");
    println!("  PRINT A + 5");
    println!("  IF A > 5 THEN PRINT 42");
    println!("  GOTO 20");
    println!("  EXIT");
}
