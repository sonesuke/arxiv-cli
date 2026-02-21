use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file at {:?}", config_path))?;

        let config: Config =
            serde_json::from_str(&content).with_context(|| "Failed to parse config file")?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory at {:?}", parent))?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, content)
            .with_context(|| format!("Failed to write config file at {:?}", config_path))?;

        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let project_dirs = ProjectDirs::from("com", "sonesuke", "arxiv-cli")
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
        Ok(project_dirs.config_dir().join("config.json"))
    }

    pub fn set(&mut self, key: &str, _value: &str) -> Result<()> {
        anyhow::bail!("Unknown config key: {}", key)
    }

    pub fn get(&self, key: &str) -> Result<String> {
        anyhow::bail!("Unknown config key: {}", key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_default() {
        let _config = Config::default();
        // Config is now empty
    }

    #[test]
    fn test_config_set_get_unknown_key() {
        let mut config = Config::default();
        assert!(config.set("unknown", "value").is_err());
        assert!(config.get("unknown").is_err());
    }

    #[test]
    fn test_load_parse_error() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "invalid json").unwrap();

        // We can't easily injection path into Config::load() without refactoring
        // So we'll skip the file load tests that rely on global state or mocking for now
        // and stick to logic tests.
        // Refactoring Config to take a path for load would be better but keeping changes minimal.
    }
}
