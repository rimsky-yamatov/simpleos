use std::path::PathBuf;

#[derive(Clone, Debug, Default)]
pub struct BootConfig {
    pub entry: String,
    pub fs_root: Option<PathBuf>,
    pub memory_mb: usize,
}

impl BootConfig {
    pub fn new(entry: impl Into<String>) -> Self {
        Self {
            entry: entry.into(),
            fs_root: None,
            memory_mb: 64,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct BootImage {
    pub config: BootConfig,
    pub kernel_name: String,
}

pub fn boot_kernel(config: &BootConfig) -> Result<(), String> {
    if config.entry.trim().is_empty() {
        return Err("boot entry cannot be empty".to_string());
    }
    if config.memory_mb == 0 {
        return Err("memory size must be > 0".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_config_accepts_valid_values() {
        let cfg = BootConfig::new("main");
        assert!(boot_kernel(&cfg).is_ok());
    }
}
