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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConnectorConfig {
    #[serde(alias = "type")]
    pub provider: String, // "jira", "confluence", "github", "markdown", "local_git", or "clickup"
    #[serde(default)]
    pub instance_url: String,
    #[serde(default)]
    pub email: String,
    #[serde(alias = "token")]
    pub api_token: Option<String>,
    pub api_token_env: Option<String>,
    #[serde(default, alias = "team_id")]
    pub workspace: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub projects: Vec<String>,
    #[serde(default)]
    pub spaces: Vec<String>,
    #[serde(default)]
    pub repos: Vec<String>,
    #[serde(default)]
    pub lists: Vec<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub glob_patterns: Vec<String>,
    #[serde(default)]
    pub teams: Vec<String>,
    #[serde(default)]
    pub groups: Vec<String>,
    #[serde(default)]
    pub organization: Option<String>,
    #[serde(default)]
    pub file_keys: Vec<String>,
    #[serde(default)]
    pub database_ids: Vec<String>,
    #[serde(default)]
    pub page_ids: Vec<String>,
    #[serde(default)]
    pub spreadsheet_ids: Vec<String>,
    #[serde(default)]
    pub service_account_file: Option<String>,
    #[serde(default)]
    pub ssl_cert_path: Option<String>,
    #[serde(default)]
    pub sync_wiki: Option<bool>,
    #[serde(default)]
    pub sync_comments: Option<bool>,
    #[serde(default)]
    pub sync_cycles: Option<bool>,
    #[serde(default)]
    pub sync_work_items: Option<bool>,
    #[serde(default)]
    pub sync_pull_requests: Option<bool>,
    #[serde(default)]
    pub sync_pipelines: Option<bool>,
    #[serde(default)]
    pub parse_depth: Option<usize>,
    #[serde(default)]
    pub recursive_page_depth: Option<usize>,
    #[serde(default)]
    pub auto_resolve_refs: Option<bool>,
    #[serde(default)]
    pub has_header_row: Option<bool>,
    #[serde(default)]
    pub max_rows_per_sheet: Option<usize>,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct McpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub aliases: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub connectors: HashMap<String, ConnectorConfig>,
    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerConfig>,
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
        if let Some(stripped) = path.strip_prefix("~/") {
            if let Some(home) = std::env::var_os("HOME") {
                return format!("{}/{}", home.to_string_lossy(), stripped);
            }
        }
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_servers_config_serde() {
        let toml_str = r#"
            [mcp_servers.figma]
            command = "npx"
            args = ["-y", "@modelcontextprotocol/server-figma"]
            prefix = "figma"
            enabled = true

            [mcp_servers.figma.env]
            FIGMA_PERSONAL_ACCESS_TOKEN = "figd_test123"
        "#;
        let config: Config = toml::from_str(toml_str).expect("parse config");
        assert!(config.mcp_servers.contains_key("figma"));
        let figma = &config.mcp_servers["figma"];
        assert_eq!(figma.command, "npx");
        assert_eq!(figma.prefix.as_deref(), Some("figma"));
        assert_eq!(figma.env.get("FIGMA_PERSONAL_ACCESS_TOKEN").map(|s| s.as_str()), Some("figd_test123"));

        let serialized = toml::to_string(&config).expect("serialize config");
        let config_roundtrip: Config = toml::from_str(&serialized).expect("deserialize roundtrip");
        assert_eq!(config_roundtrip.mcp_servers["figma"], *figma);
    }

    #[test]
    fn test_mcp_server_aliases_serde() {
        let toml_str = r#"
            [mcp_servers.figma]
            command = "npx"
            args = ["-y", "mcp-figma"]

            [mcp_servers.figma.aliases]
            "PROJ-123" = "wOeG8ZbAQwzyrtZbWpAmIB"
            "ORIGINAL_KEY_123" = "wOeG8ZbAQwzyrtZbWpAmIB"
        "#;
        let config: Config = toml::from_str(toml_str).expect("parse config");
        let figma = &config.mcp_servers["figma"];
        assert_eq!(
            figma.aliases.get("PROJ-123").map(|s| s.as_str()),
            Some("wOeG8ZbAQwzyrtZbWpAmIB")
        );
        assert_eq!(
            figma.aliases.get("ORIGINAL_KEY_123").map(|s| s.as_str()),
            Some("wOeG8ZbAQwzyrtZbWpAmIB")
        );

        let serialized = toml::to_string(&config).expect("serialize config");
        let roundtrip: Config = toml::from_str(&serialized).expect("deserialize roundtrip");
        assert_eq!(roundtrip.mcp_servers["figma"].aliases, figma.aliases);
    }
}

