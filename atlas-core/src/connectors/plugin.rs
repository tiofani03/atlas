use crate::config::ConnectorConfig;
use crate::connectors::Connector;
use crate::domain::KnowledgeArtifact;
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Out-of-process JSON-RPC/stdio Plugin SDK connector.
///
/// Spawns `plugin_command` with `plugin_args`, writes a single JSON request line on stdin:
/// `{"method": "verify" | "fetch_modified", "id": "<id>", "since": "<rfc3339|null>"}`
/// and reads a JSON response from stdout.
pub struct ExternalPluginConnector {
    id: String,
    command: String,
    args: Vec<String>,
}

impl ExternalPluginConnector {
    pub fn new(id: String, cfg: ConnectorConfig) -> Result<Self> {
        let command = cfg
            .plugin_command
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("Plugin connector '{}' requires 'plugin_command'", id))?;
        Ok(Self {
            id,
            command,
            args: cfg.plugin_args,
        })
    }

    async fn invoke(&self, method: &str, since: Option<DateTime<Utc>>) -> Result<serde_json::Value> {
        let mut child = Command::new(&self.command)
            .args(&self.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn plugin command '{}'", self.command))?;

        let req = serde_json::json!({
            "method": method,
            "id": self.id,
            "since": since.map(|d| d.to_rfc3339()),
        });

        if let Some(mut stdin) = child.stdin.take() {
            let line = format!("{}\n", req);
            stdin.write_all(line.as_bytes()).await?;
        }

        let output = child.wait_with_output().await?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Plugin '{}' exited with {}: {}", self.command, output.status, stderr);
        }

        let stdout = String::from_utf8(output.stdout).context("Plugin stdout is not UTF-8")?;
        serde_json::from_str(stdout.trim()).context("Plugin stdout is not valid JSON")
    }
}

#[async_trait]
impl Connector for ExternalPluginConnector {
    fn id(&self) -> &str {
        &self.id
    }

    fn provider(&self) -> &str {
        "plugin"
    }

    async fn verify(&self) -> Result<String> {
        let res = self.invoke("verify", None).await?;
        Ok(res["message"]
            .as_str()
            .unwrap_or("Plugin verified")
            .to_string())
    }

    async fn fetch_modified(&self, since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        let res = self.invoke("fetch_modified", since).await?;
        let artifacts: Vec<KnowledgeArtifact> = serde_json::from_value(
            res.get("artifacts").cloned().unwrap_or(res),
        )
        .context("Failed to deserialize plugin artifacts")?;
        Ok(artifacts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_external_plugin_connector_echo_script() {
        let cfg = ConnectorConfig {
            provider: "plugin".to_string(),
            plugin_command: Some("sh".to_string()),
            plugin_args: vec![
                "-c".to_string(),
                r#"read _line; echo '{"message":"Plugin OK","artifacts":[]}'"#.to_string(),
            ],
            ..Default::default()
        };
        let plugin = ExternalPluginConnector::new("custom-1".to_string(), cfg).unwrap();
        let msg = plugin.verify().await.unwrap();
        assert_eq!(msg, "Plugin OK");
        let items = plugin.fetch_modified(None).await.unwrap();
        assert!(items.is_empty());
    }
}
