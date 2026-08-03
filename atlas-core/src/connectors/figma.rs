use crate::config::ConnectorConfig;
use crate::connectors::Connector;
use crate::domain::{ArtifactKind, ArtifactRelationship, KnowledgeArtifact};
use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::Value;

pub struct FigmaConnector {
    id: String,
    config: ConnectorConfig,
    client: reqwest::Client,
}

impl FigmaConnector {
    pub fn new(id: String, config: ConnectorConfig) -> Result<Self> {
        let token = config.get_api_token()?;
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Figma-Token",
            HeaderValue::from_str(&token)?,
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self { id, config, client })
    }

    fn extract_nodes(&self, file_key: &str, node: &Value, depth: usize, max_depth: usize, artifacts: &mut Vec<KnowledgeArtifact>, parent_source_id: Option<&str>) {
        if depth > max_depth {
            return;
        }

        let node_id = node["id"].as_str().unwrap_or("");
        let name = node["name"].as_str().unwrap_or("Untitled Node");
        let node_type = node["type"].as_str().unwrap_or("");

        let kind = match node_type {
            "DOCUMENT" | "CANVAS" => ArtifactKind::Design,
            "FRAME" | "SECTION" => ArtifactKind::Design,
            "COMPONENT" | "COMPONENT_SET" | "INSTANCE" => ArtifactKind::Component,
            _ => ArtifactKind::Design,
        };

        if !node_id.is_empty() && (node_type == "CANVAS" || node_type == "FRAME" || node_type == "COMPONENT" || node_type == "COMPONENT_SET") {
            let source_id = format!("{}:{}", file_key, node_id);
            let source_url = format!("https://www.figma.com/file/{}?node-id={}", file_key, node_id.replace(':', "-"));

            let canonical_id = KnowledgeArtifact::generate_id("figma", "https://api.figma.com", &source_id);
            let checksum = KnowledgeArtifact::compute_checksum(name, None, node_type, &[]);

            let mut relationships = Vec::new();
            if let Some(pid) = parent_source_id {
                relationships.push(ArtifactRelationship {
                    source_id: source_id.clone(),
                    target_id: pid.to_string(),
                    relationship_type: "child_of".to_string(),
                });
            }

            artifacts.push(KnowledgeArtifact {
                id: canonical_id,
                kind,
                title: format!("{} ({})", name, node_type),
                summary: None,
                body: format!("Figma Node Type: {}\nNode ID: {}", node_type, node_id),
                provider: "figma".to_string(),
                source_id: source_id.clone(),
                source_url,
                repository: None,
                tags: vec!["figma".to_string(), node_type.to_lowercase()],
                relationships,
                created_at: None,
                updated_at: Utc::now(),
                synced_at: Utc::now(),
                checksum,
                metadata: node.clone(),
            });

            if let Some(children) = node["children"].as_array() {
                for child in children {
                    self.extract_nodes(file_key, child, depth + 1, max_depth, artifacts, Some(&source_id));
                }
            }
        } else if let Some(children) = node["children"].as_array() {
            for child in children {
                self.extract_nodes(file_key, child, depth + 1, max_depth, artifacts, parent_source_id);
            }
        }
    }
}

#[async_trait::async_trait]
impl Connector for FigmaConnector {
    fn id(&self) -> &str {
        &self.id
    }

    fn provider(&self) -> &str {
        "figma"
    }

    async fn verify(&self) -> Result<String> {
        let url = "https://api.figma.com/v1/me";
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to Figma API: {}", e))?;

        let status = resp.status();
        let json: Value = resp.json().await.unwrap_or_default();

        if status.is_success() {
            let email = json["email"].as_str().unwrap_or("authenticated user");
            let handle = json["handle"].as_str().unwrap_or("");
            if handle.is_empty() {
                Ok(format!("Connected to Figma successfully as '{}'.", email))
            } else {
                Ok(format!("Connected to Figma successfully as '{}' ({}).", handle, email))
            }
        } else if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            let msg = json["err"]
                .as_str()
                .unwrap_or("Invalid or unauthorized credentials");
            bail!("Figma authentication failed: {}", msg)
        } else {
            let msg = json["err"].as_str().unwrap_or("unknown error");
            bail!("Figma verification failed with status {}: {}", status, msg)
        }
    }

    async fn fetch_modified(&self, _since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        let mut artifacts = Vec::new();
        let max_depth = self.config.parse_depth.unwrap_or(3);

        for file_key in &self.config.file_keys {
            let url = format!("https://api.figma.com/v1/files/{}", file_key);
            if let Ok(res) = self.client.get(&url).send().await {
                if res.status().is_success() {
                    let file_json: Value = res.json().await.unwrap_or_default();
                    let file_name = file_json["name"].as_str().unwrap_or("Untitled Figma File").to_string();
                    let last_modified = file_json["lastModified"]
                        .as_str()
                        .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)))
                        .unwrap_or_else(Utc::now);

                    let file_source_id = format!("file:{}", file_key);
                    let file_canonical_id = KnowledgeArtifact::generate_id("figma", "https://api.figma.com", &file_source_id);
                    let file_checksum = KnowledgeArtifact::compute_checksum(&file_name, None, &file_key, &[]);

                    artifacts.push(KnowledgeArtifact {
                        id: file_canonical_id,
                        kind: ArtifactKind::Design,
                        title: file_name,
                        summary: None,
                        body: format!("Figma Design File: {}", file_key),
                        provider: "figma".to_string(),
                        source_id: file_source_id.clone(),
                        source_url: format!("https://www.figma.com/file/{}", file_key),
                        repository: None,
                        tags: vec!["figma:file".to_string()],
                        relationships: Vec::new(),
                        created_at: None,
                        updated_at: last_modified,
                        synced_at: Utc::now(),
                        checksum: file_checksum,
                        metadata: file_json.clone(),
                    });

                    if let Some(doc) = file_json.get("document") {
                        self.extract_nodes(file_key, doc, 1, max_depth, &mut artifacts, Some(&file_source_id));
                    }
                }
            }
        }

        Ok(artifacts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_figma_connector_creation() {
        let mut cfg = ConnectorConfig::default();
        cfg.provider = "figma".to_string();
        cfg.api_token = Some("figd_token_12345".to_string());
        cfg.file_keys = vec!["aBC123xYz".to_string()];

        let conn = FigmaConnector::new("figma-test".to_string(), cfg).unwrap();
        assert_eq!(conn.id(), "figma-test");
        assert_eq!(conn.provider(), "figma");
    }

    #[test]
    fn test_figma_extract_nodes() {
        let mut cfg = ConnectorConfig::default();
        cfg.provider = "figma".to_string();
        cfg.api_token = Some("figd_token_12345".to_string());
        let conn = FigmaConnector::new("figma-nodes".to_string(), cfg).unwrap();

        let doc_json = serde_json::json!({
            "id": "0:0",
            "name": "Document",
            "type": "DOCUMENT",
            "children": [
                {
                    "id": "0:1",
                    "name": "Page 1",
                    "type": "CANVAS",
                    "children": [
                        {
                            "id": "1:2",
                            "name": "Button Component",
                            "type": "COMPONENT"
                        }
                    ]
                }
            ]
        });

        let mut artifacts = Vec::new();
        conn.extract_nodes("aBC123xYz", &doc_json, 1, 3, &mut artifacts, None);

        assert_eq!(artifacts.len(), 2); // CANVAS + COMPONENT
        assert!(artifacts.iter().any(|a| a.title.contains("Page 1")));
        assert!(artifacts.iter().any(|a| a.title.contains("Button Component")));
    }
}
