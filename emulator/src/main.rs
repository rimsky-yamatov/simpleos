//! SimpleOS エミュレータ(ホスト上で動く BASIC シェル)。
//! カーネルと同じ `basic_core::Shell` を使うので、動作確認用に使える。
//!
//! 使い方:
//!   cargo run --release -p simpleos-emulator              # 対話シェル
//!   cargo run --release -p simpleos-emulator -- foo.bas   # プログラムを実行して終了

use basic_core::{Console, Shell, ShellAction, Storage};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

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

    fn write_prompt(&mut self, s: &str) {
        print!("{}", s);
        let _ = io::stdout().flush();
    }

    fn clear_screen(&mut self) {
        // ANSI: 画面消去 + カーソルをホームへ
        print!("\x1b[2J\x1b[H");
        let _ = io::stdout().flush();
    }
}

struct HostStorage {
    root: PathBuf,
}

impl HostStorage {
    fn new(root: PathBuf) -> Self {
        let _ = fs::create_dir_all(&root);
        Self { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(format!("{}.bas", name))
    }
}

impl Storage for HostStorage {
    fn list(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(&self.root)
            .map(|it| {
                it.filter_map(Result::ok)
                    .map(|e| e.path())
                    .filter(|p| p.extension().map_or(false, |x| x == "bas"))
                    .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()))
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }

    fn save(&mut self, name: &str, text: &str) -> Result<(), String> {
        fs::write(self.path(name), text).map_err(|e| format!("cannot save {}: {}", name, e))
    }

    fn load(&self, name: &str) -> Result<String, String> {
        fs::read_to_string(self.path(name)).map_err(|_| format!("file {} not found", name))
    }

    fn delete(&mut self, name: &str) -> Result<(), String> {
        fs::remove_file(self.path(name)).map_err(|_| format!("file {} not found", name))
    }
}

fn main() {
    let fs_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("fs");
    let mut storage = HostStorage::new(fs_dir.clone());
    let mut console = StdConsole;
    let mut shell = Shell::new();

    // 引数でファイルが渡されたら、それを実行して終了する
    if let Some(path) = std::env::args().nth(1) {
        match fs::read_to_string(&path) {
            Ok(text) => {
                shell.load_text(&text);
                shell.handle_line("RUN", &mut console, &mut storage);
            }
            Err(e) => {
                eprintln!("cannot read {}: {}", path, e);
                std::process::exit(1);
            }
        }
        return;
    }

    shell.banner(&mut console);
    println!("Filesystem directory: {}", fs_dir.display());

    loop {
        print!("> ");
        let _ = io::stdout().flush();

        let Some(line) = console.read_line() else {
            break;
        };
        if shell.handle_line(&line, &mut console, &mut storage) == ShellAction::Exit {
            println!("Goodbye.");
            break;
        }
    }
}
