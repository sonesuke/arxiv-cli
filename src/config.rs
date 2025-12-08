use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub headless: bool,
    pub browser_path: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self { headless: true, browser_path: None }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file at {:?}", config_path))?;

        let config: Config = serde_json::from_str(&content)
            .with_context(|| "Failed to parse config file")?;

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

    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "headless" => {
                self.headless = value.parse().with_context(|| "Invalid boolean for headless")?;
            }
            "browser_path" => {
                self.browser_path = if value.is_empty() {
                     None
                } else {
                    Some(value.to_string())
                };
            }
            _ => anyhow::bail!("Unknown config key: {}", key),
        }
        Ok(())
    }
    
    pub fn get(&self, key: &str) -> Result<String> {
         match key {
            "headless" => Ok(self.headless.to_string()),
            "browser_path" => Ok(self.browser_path.clone().unwrap_or_default()),
            _ => anyhow::bail!("Unknown config key: {}", key),
        }
    }
}
