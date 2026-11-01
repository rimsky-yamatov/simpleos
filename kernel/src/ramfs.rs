//! メモリ上の簡易ファイルシステム(再起動で消える)。起動時にサンプルを登録する。

use alloc::{collections::BTreeMap, format, string::{String, ToString}, vec::Vec};
use basic_core::Storage;

pub struct RamFs {
    files: BTreeMap<String, String>,
}

impl RamFs {
    pub fn with_samples() -> Self {
        let mut fs = Self { files: BTreeMap::new() };
        fs.add("demo", include_str!("../programs/demo.bas"));
        fs.add("fib", include_str!("../programs/fib.bas"));
        fs.add("primes", include_str!("../programs/primes.bas"));
        fs.add("table", include_str!("../programs/table.bas"));
        fs.add("square", include_str!("../programs/square.bas"));
        fs
    }

    fn add(&mut self, name: &str, text: &str) {
        self.files.insert(name.to_string(), text.to_string());
    }
}

impl Storage for RamFs {
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
