use super::{ArxivError, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default = "default_headless")]
    pub headless: bool,
    #[serde(default)]
    pub browser_path: Option<String>,
    #[serde(default)]
    pub chrome_args: Vec<String>,
}

fn default_headless() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self { headless: true, browser_path: None, chrome_args: Vec::new() }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        Self::load_from(&config_path)
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path).map_err(|e| {
            ArxivError::Config(format!("Failed to read config file at {:?}: {}", path, e))
        })?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| ArxivError::Config(format!("Failed to parse config file: {}", e)))?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        self.save_to(&config_path)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                ArxivError::Config(format!(
                    "Failed to create config directory at {:?}: {}",
                    parent, e
                ))
            })?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| ArxivError::Config(format!("Failed to serialize config: {}", e)))?;
        fs::write(path, content).map_err(|e| {
            ArxivError::Config(format!("Failed to write config file at {:?}: {}", path, e))
        })?;

        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let project_dirs = ProjectDirs::from("com", "sonesuke", "arxiv-cli").ok_or_else(|| {
            ArxivError::Config("Could not determine config directory".to_string())
        })?;
        Ok(project_dirs.config_dir().join(CONFIG_FILE))
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "headless" => {
                self.headless = value
                    .parse()
                    .map_err(|_| ArxivError::Config("Invalid boolean for headless".to_string()))?;
            }
            "browser_path" => {
                self.browser_path = if value.is_empty() { None } else { Some(value.to_string()) };
            }
            _ => return Err(ArxivError::Config(format!("Unknown config key: {}", key))),
        }
        Ok(())
    }

    pub fn get(&self, key: &str) -> Result<String> {
        match key {
            "headless" => Ok(self.headless.to_string()),
            "browser_path" => Ok(self.browser_path.clone().unwrap_or_default()),
            _ => Err(ArxivError::Config(format!("Unknown config key: {}", key))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.headless);
        assert!(config.browser_path.is_none());
        assert!(config.chrome_args.is_empty());
    }

    #[test]
    fn test_config_set_get() {
        let mut config = Config::default();

        config.set("headless", "false").unwrap();
        assert!(!config.headless);
        assert_eq!(config.get("headless").unwrap(), "false");

        config.set("browser_path", "/tmp/chrome").unwrap();
        assert_eq!(config.browser_path, Some("/tmp/chrome".to_string()));
        assert_eq!(config.get("browser_path").unwrap(), "/tmp/chrome");

        config.set("browser_path", "").unwrap();
        assert!(config.browser_path.is_none());
    }

    #[test]
    fn test_config_set_get_unknown_key() {
        let mut config = Config::default();
        assert!(config.set("unknown", "value").is_err());
        assert!(config.get("unknown").is_err());
    }

    #[test]
    fn test_config_path_returns_valid_path() {
        let path = Config::config_path().unwrap();
        assert!(path.to_str().unwrap().contains(CONFIG_FILE));
    }

    #[test]
    fn test_load_from_nonexistent_file() {
        let path = PathBuf::from("/tmp/arxiv_cli_test_nonexistent_config.toml");
        let config = Config::load_from(&path).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn test_load_from_valid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "headless = false\nbrowser_path = \"/usr/bin/chrome\"").unwrap();

        let config = Config::load_from(&path).unwrap();
        assert!(!config.headless);
        assert_eq!(config.browser_path, Some("/usr/bin/chrome".to_string()));
    }

    #[test]
    fn test_load_from_invalid_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(file, "invalid toml").unwrap();

        let result = Config::load_from(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_save_to_and_load_from_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let config = Config {
            headless: false,
            browser_path: Some("/custom/path".to_string()),
            chrome_args: vec!["--no-sandbox".to_string(), "--disable-gpu".to_string()],
        };
        config.save_to(&path).unwrap();

        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(config, loaded);
    }

    #[test]
    fn test_save_to_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("dir").join("config.toml");

        let config = Config::default();
        config.save_to(&path).unwrap();

        assert!(path.exists());
    }
}
