use basic_core::{format_statement, parse_program_line, sanitize_name, Program};
use std::collections::BTreeMap;
use std::fs as std_fs;
use std::path::{Path, PathBuf};

pub trait Storage {
    fn list_dir(&self) -> Vec<String>;
    fn save_program(&self, name: &str, program: &Program) -> bool;
    fn load_program(&self, name: &str) -> Result<Program, String>;
}

#[derive(Default)]
pub struct HostStorage {
    root: PathBuf,
}

impl HostStorage {
    pub fn new(root: PathBuf) -> Self {
        if !root.exists() {
            let _ = std_fs::create_dir_all(&root);
        }
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl Storage for HostStorage {
    fn list_dir(&self) -> Vec<String> {
        let mut entries = std_fs::read_dir(&self.root)
            .map(|it| it.filter_map(Result::ok).collect::<Vec<_>>())
            .unwrap_or_default();
        entries.sort_by_key(|e| e.file_name());
        entries
            .into_iter()
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect()
    }

    fn save_program(&self, name: &str, program: &Program) -> bool {
        let file = self.root.join(format!("{}.bas", sanitize_name(name)));
        let mut text = String::new();
        for (num, stmt) in program.iter() {
            text.push_str(&format!("{} {}\n", num, format_statement(stmt)));
        }
        std_fs::write(file, text).is_ok()
    }

    fn load_program(&self, name: &str) -> Result<Program, String> {
        let file = self.root.join(format!("{}.bas", sanitize_name(name)));
        let content = std_fs::read_to_string(file).map_err(|_| format!("file {} not found", name))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_round_trip() {
        let root = std::env::temp_dir().join("simpleos_kernel_fs_test");
        let _ = std_fs::remove_dir_all(&root);
        let fs = HostStorage::new(root.clone());
        let mut program = BTreeMap::new();
        program.insert(10, basic_core::Statement::Print { expr: "42".to_string() });

        assert!(fs.save_program("demo", &program));
        let loaded = fs.load_program("demo").unwrap();
        assert_eq!(loaded.len(), 1);
        let _ = std_fs::remove_dir_all(root);
    }
}
