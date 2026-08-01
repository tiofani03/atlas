use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_db_path")]
    pub db_path: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            db_path: default_db_path(),
            log_level: default_log_level(),
        }
    }
}

fn default_db_path() -> String {
    let mut path = dirs_next::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("atlas");
    path.push("atlas.db");
    path.to_string_lossy().to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorConfig {
    pub provider: String, // "jira", "confluence", "github", or "markdown"
    #[serde(default)]
    pub instance_url: String,
    #[serde(default)]
    pub email: String,
    pub api_token: Option<String>,
    pub api_token_env: Option<String>,
    #[serde(default)]
    pub projects: Vec<String>,
    #[serde(default)]
    pub spaces: Vec<String>,
    #[serde(default)]
    pub repos: Vec<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub glob_patterns: Vec<String>,
}

impl ConnectorConfig {
    pub fn get_paths(&self) -> Vec<String> {
        if !self.paths.is_empty() {
            return self.paths.clone();
        }
        if let Some(ref p) = self.path {
            return p
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        Vec::new()
    }

    pub fn get_api_token(&self) -> Result<String> {
        if let Some(ref token) = self.api_token {
            if !token.is_empty() {
                return Ok(token.clone());
            }
        }
        if let Some(ref env_var) = self.api_token_env {
            if let Ok(token) = std::env::var(env_var) {
                if !token.is_empty() {
                    return Ok(token);
                }
            }
        }
        anyhow::bail!(
            "No API token provided for connector. Specify 'api_token' or 'api_token_env' in config"
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub connectors: HashMap<String, ConnectorConfig>,
}

impl Config {
    pub fn default_config_path() -> Result<PathBuf> {
        let mut path = dirs_next::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not resolve user config directory"))?;
        path.push("atlas");
        path.push("config.toml");
        Ok(path)
    }

    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Config::default());
        }
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file at {:?}", path))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML config at {:?}", path))?;
        Ok(config)
    }

    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {:?}", parent))?;
        }
        let content = toml::to_string_pretty(self).context("Failed to serialize config to TOML")?;
        fs::write(path, content)
            .with_context(|| format!("Failed to write config file at {:?}", path))?;
        Ok(())
    }

    pub fn resolve_db_path(&self) -> PathBuf {
        let expanded = shellexpand::tilde(&self.general.db_path).to_string();
        PathBuf::from(expanded)
    }
}

mod dirs_next {
    use std::path::PathBuf;

    pub fn config_dir() -> Option<PathBuf> {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config"))
    }

    pub fn data_dir() -> Option<PathBuf> {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("share"))
    }
}

mod shellexpand {
    pub fn tilde(path: &str) -> String {
        if path.starts_with("~/") {
            if let Some(home) = std::env::var_os("HOME") {
                return format!("{}/{}", home.to_string_lossy(), &path[2..]);
            }
        }
        path.to_string()
    }
}
