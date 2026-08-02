use crate::config::ConnectorConfig;
use crate::connectors::Connector;
use crate::domain::{ArtifactKind, ArtifactRelationship, KnowledgeArtifact};
use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use serde_json::Value;

pub struct ClickUpConnector {
    id: String,
    config: ConnectorConfig,
    client: reqwest::Client,
}

impl ClickUpConnector {
    pub fn new(id: String, config: ConnectorConfig) -> Result<Self> {
        let token = config.get_api_token()?;

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&token)?);
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self { id, config, client })
    }

    fn base_url(&self) -> String {
        if self.config.instance_url.trim().is_empty() {
            "https://api.clickup.com/api/v2".to_string()
        } else {
            self.config.instance_url.trim_end_matches('/').to_string()
        }
    }

    fn parse_ms_timestamp(value: Option<&Value>) -> Option<DateTime<Utc>> {
        let raw = value?;
        let ms = raw
            .as_i64()
            .or_else(|| raw.as_str().and_then(|s| s.parse::<i64>().ok()))?;
        Utc.timestamp_millis_opt(ms).single()
    }

    fn status_name(task: &Value) -> String {
        task.get("status")
            .and_then(|v| v.get("status"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string()
    }

    fn task_tags(task: &Value, workspace_id: &str) -> Vec<String> {
        let mut tags = vec![
            "type:ticket".to_string(),
            "source:clickup".to_string(),
            format!("workspace:{}", workspace_id),
            format!("status:{}", Self::status_name(task)),
        ];

        if let Some(list_id) = task
            .get("list")
            .and_then(|v| v.get("id"))
            .and_then(|v| v.as_str())
        {
            tags.push(format!("list:{}", list_id));
        }

        if let Some(space_id) = task
            .get("space")
            .and_then(|v| v.get("id"))
            .and_then(|v| v.as_str())
        {
            tags.push(format!("space:{}", space_id));
        }

        if let Some(priority) = task
            .get("priority")
            .and_then(|v| v.get("priority"))
            .and_then(|v| v.as_str())
        {
            tags.push(format!("priority:{}", priority));
        }

        if let Some(task_tags) = task.get("tags").and_then(|v| v.as_array()) {
            for tag in task_tags {
                if let Some(name) = tag
                    .get("name")
                    .and_then(|v| v.as_str())
                    .or_else(|| tag.as_str())
                {
                    if !name.trim().is_empty() {
                        tags.push(name.trim().to_string());
                    }
                }
            }
        }

        tags
    }

    fn task_relationships(task: &Value, source_id: &str) -> Vec<ArtifactRelationship> {
        let mut relationships = Vec::new();

        if let Some(parent) = task.get("parent").and_then(|v| v.as_str()) {
            if !parent.is_empty() {
                relationships.push(ArtifactRelationship {
                    source_id: source_id.to_string(),
                    target_id: parent.to_string(),
                    relationship_type: "subtask_of".to_string(),
                });
            }
        }

        if let Some(dependencies) = task.get("dependencies").and_then(|v| v.as_array()) {
            for dep in dependencies {
                if let Some(target) = dep
                    .get("task_id")
                    .or_else(|| dep.get("depends_on"))
                    .and_then(|v| v.as_str())
                {
                    if !target.is_empty() {
                        relationships.push(ArtifactRelationship {
                            source_id: source_id.to_string(),
                            target_id: target.to_string(),
                            relationship_type: "depends_on".to_string(),
                        });
                    }
                }
            }
        }

        if let Some(linked_tasks) = task.get("linked_tasks").and_then(|v| v.as_array()) {
            for linked in linked_tasks {
                if let Some(target) = linked
                    .get("task_id")
                    .or_else(|| linked.get("link_id"))
                    .and_then(|v| v.as_str())
                {
                    if !target.is_empty() {
                        relationships.push(ArtifactRelationship {
                            source_id: source_id.to_string(),
                            target_id: target.to_string(),
                            relationship_type: "references".to_string(),
                        });
                    }
                }
            }
        }

        relationships
    }

    fn task_to_artifact(
        &self,
        task: Value,
        workspace_id: &str,
        now: DateTime<Utc>,
    ) -> Option<KnowledgeArtifact> {
        let task_id = task.get("id").and_then(|v| v.as_str())?.to_string();
        let title = task
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled ClickUp Task")
            .to_string();
        let status = Self::status_name(&task);
        let body = task
            .get("markdown_description")
            .or_else(|| task.get("description"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let source_url = task
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let updated_at = Self::parse_ms_timestamp(task.get("date_updated")).unwrap_or(now);
        let created_at = Self::parse_ms_timestamp(task.get("date_created"));
        let repository = task
            .get("list")
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| Some(format!("clickup-workspace-{}", workspace_id)));
        let tags = Self::task_tags(&task, workspace_id);
        let relationships = Self::task_relationships(&task, &task_id);
        let id = KnowledgeArtifact::generate_id("clickup", &self.base_url(), &task_id);
        let summary = Some(format!("Status: {}", status));
        let checksum =
            KnowledgeArtifact::compute_checksum(&title, summary.as_deref(), &body, &tags);

        Some(KnowledgeArtifact {
            id,
            kind: ArtifactKind::Ticket,
            title,
            summary,
            body,
            provider: "clickup".to_string(),
            source_id: task_id,
            source_url,
            repository,
            tags,
            relationships,
            created_at,
            updated_at,
            synced_at: now,
            checksum,
            metadata: task,
        })
    }
}

#[async_trait::async_trait]
impl Connector for ClickUpConnector {
    fn id(&self) -> &str {
        &self.id
    }

    fn provider(&self) -> &str {
        "clickup"
    }

    async fn fetch_modified(&self, since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        let base_url = self.base_url();
        let mut all_artifacts = Vec::new();
        let workspace_ids = &self.config.projects;

        if workspace_ids.is_empty() {
            anyhow::bail!(
                "ClickUp connector '{}' requires at least one Workspace/Team ID",
                self.id
            );
        }

        for workspace_id in workspace_ids {
            let workspace_id = workspace_id.trim();
            if workspace_id.is_empty() {
                continue;
            }

            let mut page = 0;
            loop {
                let url = format!("{}/team/{}/task", base_url, workspace_id);
                let mut req = self.client.get(&url).query(&[
                    ("page", page.to_string()),
                    ("order_by", "updated".to_string()),
                    ("reverse", "true".to_string()),
                    ("subtasks", "true".to_string()),
                    ("include_closed", "true".to_string()),
                    ("include_markdown_description", "true".to_string()),
                ]);

                if let Some(dt) = since {
                    req = req.query(&[("date_updated_gt", dt.timestamp_millis().to_string())]);
                }
                for space_id in &self.config.spaces {
                    if !space_id.trim().is_empty() {
                        req = req.query(&[("space_ids[]", space_id.trim())]);
                    }
                }
                for folder_id in &self.config.paths {
                    if !folder_id.trim().is_empty() {
                        req = req.query(&[("project_ids[]", folder_id.trim())]);
                    }
                }
                for list_id in &self.config.repos {
                    if !list_id.trim().is_empty() {
                        req = req.query(&[("list_ids[]", list_id.trim())]);
                    }
                }

                let res = req
                    .send()
                    .await
                    .with_context(|| format!("Failed to fetch tasks from ClickUp API ({})", url))?;

                if !res.status().is_success() {
                    let status = res.status();
                    let body = res.text().await.unwrap_or_default();
                    anyhow::bail!("ClickUp API error ({}): {}", status, body);
                }

                let json: Value = res.json().await?;
                let tasks = json
                    .get("tasks")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();

                if tasks.is_empty() {
                    break;
                }

                let count = tasks.len();
                let now = Utc::now();
                for task in tasks {
                    if let Some(artifact) = self.task_to_artifact(task, workspace_id, now) {
                        all_artifacts.push(artifact);
                    }
                }

                if count < 100 {
                    break;
                }
                page += 1;
            }
        }

        Ok(all_artifacts)
    }
}
