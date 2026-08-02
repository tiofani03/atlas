use crate::config::ConnectorConfig;
use crate::connectors::Connector;
use crate::domain::{ArtifactKind, ArtifactRelationship, KnowledgeArtifact};
use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::Value;

pub struct AsanaConnector {
    id: String,
    config: ConnectorConfig,
    client: reqwest::Client,
}

impl AsanaConnector {
    pub fn new(id: String, config: ConnectorConfig) -> Result<Self> {
        let token = config.get_api_token()?;
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", token))?,
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self { id, config, client })
    }

    fn base_url(&self) -> String {
        "https://app.asana.com/api/1.0".to_string()
    }
}

#[async_trait::async_trait]
impl Connector for AsanaConnector {
    fn id(&self) -> &str {
        &self.id
    }

    fn provider(&self) -> &str {
        "asana"
    }

    async fn fetch_modified(&self, _since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        let mut artifacts = Vec::new();
        let base = self.base_url();

        for project_gid in &self.config.projects {
            // 1. Fetch Project Metadata
            let proj_url = format!("{}/projects/{}", base, project_gid);
            if let Ok(res) = self.client.get(&proj_url).send().await {
                if res.status().is_success() {
                    let proj_json: Value = res.json().await.unwrap_or_default();
                    let data = &proj_json["data"];
                    let name = data["name"].as_str().unwrap_or("Untitled Project");
                    let notes = data["notes"].as_str().unwrap_or("");
                    let source_id = format!("project:{}", project_gid);

                    let canonical_id = KnowledgeArtifact::generate_id("asana", &base, &source_id);
                    let checksum = KnowledgeArtifact::compute_checksum(name, None, notes, &[]);

                    artifacts.push(KnowledgeArtifact {
                        id: canonical_id,
                        kind: ArtifactKind::Specification,
                        title: format!("Project: {}", name),
                        summary: None,
                        body: notes.to_string(),
                        provider: "asana".to_string(),
                        source_id,
                        source_url: format!("https://app.asana.com/0/{}", project_gid),
                        repository: None,
                        tags: vec!["asana:project".to_string()],
                        relationships: Vec::new(),
                        created_at: None,
                        updated_at: Utc::now(),
                        synced_at: Utc::now(),
                        checksum,
                        metadata: data.clone(),
                    });
                }
            }

            // 2. Fetch Tasks in Project
            let tasks_url = format!("{}/projects/{}/tasks?opt_fields=gid,name,notes,completed,created_at,modified_at", base, project_gid);
            if let Ok(res) = self.client.get(&tasks_url).send().await {
                if res.status().is_success() {
                    let tasks_json: Value = res.json().await.unwrap_or_default();
                    if let Some(tasks) = tasks_json["data"].as_array() {
                        for task in tasks {
                            let task_gid = task["gid"].as_str().unwrap_or("");
                            let name = task["name"].as_str().unwrap_or("Untitled Task");
                            let notes = task["notes"].as_str().unwrap_or("");
                            let source_id = format!("task:{}", task_gid);

                            let created_at = task["created_at"]
                                .as_str()
                                .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)));
                            let updated_at = task["modified_at"]
                                .as_str()
                                .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)))
                                .unwrap_or_else(Utc::now);

                            let canonical_id = KnowledgeArtifact::generate_id("asana", &base, &source_id);
                            let checksum = KnowledgeArtifact::compute_checksum(name, None, notes, &[]);

                            artifacts.push(KnowledgeArtifact {
                                id: canonical_id,
                                kind: ArtifactKind::Ticket,
                                title: name.to_string(),
                                summary: None,
                                body: notes.to_string(),
                                provider: "asana".to_string(),
                                source_id: source_id.clone(),
                                source_url: format!("https://app.asana.com/0/{}/{}", project_gid, task_gid),
                                repository: None,
                                tags: vec!["asana:task".to_string()],
                                relationships: vec![ArtifactRelationship {
                                    source_id,
                                    target_id: format!("project:{}", project_gid),
                                    relationship_type: "belongs_to".to_string(),
                                }],
                                created_at,
                                updated_at,
                                synced_at: Utc::now(),
                                checksum,
                                metadata: task.clone(),
                            });
                        }
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
    fn test_asana_connector_creation() {
        let mut cfg = ConnectorConfig::default();
        cfg.provider = "asana".to_string();
        cfg.api_token = Some("1/1209381029:asana_pat_12345".to_string());
        cfg.projects = vec!["12093810293".to_string()];

        let conn = AsanaConnector::new("asana-test".to_string(), cfg).unwrap();
        assert_eq!(conn.id(), "asana-test");
        assert_eq!(conn.provider(), "asana");
    }
}
