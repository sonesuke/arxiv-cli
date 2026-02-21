use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Config {}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        Self::load_from(&config_path)
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file at {:?}", path))?;

        let config: Config =
            serde_json::from_str(&content).with_context(|| "Failed to parse config file")?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        self.save_to(&config_path)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory at {:?}", parent))?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)
            .with_context(|| format!("Failed to write config file at {:?}", path))?;

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

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config, Config {});
    }

    #[test]
    fn test_config_set_get_unknown_key() {
        let mut config = Config::default();
        assert!(config.set("unknown", "value").is_err());
        assert!(config.get("unknown").is_err());
    }

    #[test]
    fn test_load_from_nonexistent_file() {
        let path = PathBuf::from("/tmp/arxiv_cli_test_nonexistent_config.json");
        let config = Config::load_from(&path).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn test_load_from_valid_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "{{}}").unwrap();

        let config = Config::load_from(&path).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn test_load_from_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "invalid json").unwrap();

        let result = Config::load_from(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_save_to_and_load_from_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let config = Config::default();
        config.save_to(&path).unwrap();

        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(config, loaded);
    }

    #[test]
    fn test_save_to_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("dir").join("config.json");

        let config = Config::default();
        config.save_to(&path).unwrap();

        assert!(path.exists());
    }

    #[test]
    fn test_config_path_returns_valid_path() {
        let path = Config::config_path().unwrap();
        assert!(path.to_str().unwrap().contains("config.json"));
    }
}
