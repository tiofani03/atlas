use crate::config::ConnectorConfig;
use crate::connectors::Connector;
use crate::domain::{ArtifactKind, ArtifactRelationship, KnowledgeArtifact};
use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::time::Duration;
use tracing::{info, warn};

pub struct ClickupConnector {
    id: String,
    config: ConnectorConfig,
    client: reqwest::Client,
    base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClickupTeam {
    id: String,
    name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClickupTeamsResponse {
    teams: Vec<ClickupTeam>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClickupSpace {
    id: String,
    name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClickupSpacesResponse {
    spaces: Vec<ClickupSpace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClickupList {
    id: String,
    name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClickupFolder {
    id: String,
    name: Option<String>,
    #[serde(default)]
    lists: Vec<ClickupList>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClickupFoldersResponse {
    folders: Vec<ClickupFolder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClickupListsResponse {
    lists: Vec<ClickupList>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClickupTasksResponse {
    tasks: Vec<Value>,
    #[serde(default)]
    last_page: Option<bool>,
}

impl ClickupConnector {
    pub fn new(id: String, config: ConnectorConfig) -> Result<Self> {
        let token = config.get_api_token()?;

        let base_url = if !config.instance_url.is_empty() {
            config.instance_url.trim_end_matches('/').to_string()
        } else {
            "https://api.clickup.com/api/v2".to_string()
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&token)
                .with_context(|| format!("Invalid API token header value for ClickUp connector '{}'", id))?,
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            id,
            config,
            client,
            base_url,
        })
    }

    async fn send_with_retry(&self, req_builder: reqwest::RequestBuilder) -> Result<reqwest::Response> {
        let mut retries = 0;
        let max_retries = 4;
        let mut delay = Duration::from_millis(500);

        loop {
            let req = req_builder
                .try_clone()
                .ok_or_else(|| anyhow::anyhow!("Failed to clone request for retry"))?;

            let res = req.send().await;

            match res {
                Ok(response) => {
                    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS && retries < max_retries {
                        let retry_after = response
                            .headers()
                            .get("retry-after")
                            .and_then(|h| h.to_str().ok())
                            .and_then(|s| s.parse::<u64>().ok())
                            .map(Duration::from_secs)
                            .unwrap_or(delay);

                        warn!(
                            "ClickUp API rate limited (429). Retrying in {:?} (attempt {}/{})",
                            retry_after,
                            retries + 1,
                            max_retries
                        );
                        tokio::time::sleep(retry_after).await;
                        retries += 1;
                        delay *= 2;
                        continue;
                    }
                    return Ok(response);
                }
                Err(err) if retries < max_retries => {
                    warn!(
                        "ClickUp request error: {}. Retrying in {:?} (attempt {}/{})",
                        err,
                        delay,
                        retries + 1,
                        max_retries
                    );
                    tokio::time::sleep(delay).await;
                    retries += 1;
                    delay *= 2;
                }
                Err(err) => return Err(err.into()),
            }
        }
    }

    async fn fetch_teams(&self) -> Result<Vec<ClickupTeam>> {
        if let Some(ref ws) = self.config.workspace {
            if !ws.is_empty() {
                return Ok(vec![ClickupTeam {
                    id: ws.clone(),
                    name: Some("Configured Workspace".to_string()),
                }]);
            }
        }

        let url = format!("{}/team", self.base_url);
        let res = self.send_with_retry(self.client.get(&url)).await?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            anyhow::bail!("ClickUp GET /team error ({}): {}", status, body);
        }

        let resp: ClickupTeamsResponse = res.json().await?;
        Ok(resp.teams)
    }

    async fn fetch_spaces(&self, team_id: &str) -> Result<Vec<ClickupSpace>> {
        let url = format!("{}/team/{}/space?archived=false", self.base_url, team_id);
        let res = self.send_with_retry(self.client.get(&url)).await?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            anyhow::bail!("ClickUp GET /team/{}/space error ({}): {}", team_id, status, body);
        }

        let resp: ClickupSpacesResponse = res.json().await?;
        
        if !self.config.spaces.is_empty() {
            let space_filter: HashSet<_> = self.config.spaces.iter().map(|s| s.to_lowercase()).collect();
            Ok(resp
                .spaces
                .into_iter()
                .filter(|sp| {
                    sp.name
                        .as_ref()
                        .map(|n| space_filter.contains(&n.to_lowercase()) || space_filter.contains(&sp.id.to_lowercase()))
                        .unwrap_or(false)
                })
                .collect())
        } else {
            Ok(resp.spaces)
        }
    }

    async fn fetch_lists_for_space(&self, space_id: &str) -> Result<Vec<(ClickupList, Option<String>, String)>> {
        let mut result = Vec::new();

        // 1. Folders in space
        let folder_url = format!("{}/space/{}/folder?archived=false", self.base_url, space_id);
        if let Ok(res) = self.send_with_retry(self.client.get(&folder_url)).await {
            if res.status().is_success() {
                if let Ok(resp) = res.json::<ClickupFoldersResponse>().await {
                    for folder in resp.folders {
                        let folder_name = folder.name.clone().unwrap_or_default();
                        for list in folder.lists {
                            result.push((list, Some(folder_name.clone()), space_id.to_string()));
                        }
                    }
                }
            }
        }

        // 2. Folderless lists in space
        let list_url = format!("{}/space/{}/list?archived=false", self.base_url, space_id);
        if let Ok(res) = self.send_with_retry(self.client.get(&list_url)).await {
            if res.status().is_success() {
                if let Ok(resp) = res.json::<ClickupListsResponse>().await {
                    for list in resp.lists {
                        result.push((list, None, space_id.to_string()));
                    }
                }
            }
        }

        if !self.config.lists.is_empty() {
            let list_filter: HashSet<_> = self.config.lists.iter().map(|l| l.to_lowercase()).collect();
            Ok(result
                .into_iter()
                .filter(|(l, _, _)| {
                    l.name
                        .as_ref()
                        .map(|n| list_filter.contains(&n.to_lowercase()) || list_filter.contains(&l.id.to_lowercase()))
                        .unwrap_or(false)
                })
                .collect())
        } else {
            Ok(result)
        }
    }

    fn parse_timestamp(val: &Value) -> Option<DateTime<Utc>> {
        let millis = if let Some(n) = val.as_i64() {
            Some(n)
        } else if let Some(s) = val.as_str() {
            s.parse::<i64>().ok()
        } else {
            None
        };

        millis.and_then(|m| {
            let secs = m / 1000;
            let nsecs = ((m % 1000) * 1_000_000) as u32;
            Utc.timestamp_opt(secs, nsecs).single()
        })
    }
}

#[async_trait::async_trait]
impl Connector for ClickupConnector {
    fn id(&self) -> &str {
        &self.id
    }

    fn provider(&self) -> &str {
        "clickup"
    }

    async fn fetch_modified(&self, since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        if self.config.enabled == Some(false) {
            info!("ClickUp connector [{}] is disabled in config.", self.id);
            return Ok(Vec::new());
        }

        let teams = self.fetch_teams().await?;
        if teams.is_empty() {
            warn!("ClickUp connector [{}] found no teams/workspaces.", self.id);
            return Ok(Vec::new());
        }

        let mut target_lists = Vec::new();

        for team in teams {
            let spaces = self.fetch_spaces(&team.id).await?;
            for space in spaces {
                let lists_with_context = self.fetch_lists_for_space(&space.id).await?;
                for (list, folder_opt, space_id) in lists_with_context {
                    target_lists.push((list, folder_opt, space_id));
                }
            }
        }

        info!(
            "ClickUp connector [{}] discovered {} lists for task indexing.",
            self.id,
            target_lists.len()
        );

        let now = Utc::now();
        let mut objects = Vec::new();
        let mut seen_task_ids = HashSet::new();

        let since_millis = since.map(|dt| dt.timestamp_millis());

        for (list, folder_opt, space_id) in target_lists {
            let list_name = list.name.unwrap_or_default();
            let mut page = 0;

            loop {
                let mut url = format!(
                    "{}/list/{}/task?archived=false&subtasks=true&include_closed=true&page={}",
                    self.base_url, list.id, page
                );

                if let Some(gt) = since_millis {
                    url.push_str(&format!("&date_updated_gt={}", gt));
                }

                let res = self.send_with_retry(self.client.get(&url)).await?;

                if !res.status().is_success() {
                    let status = res.status();
                    let body = res.text().await.unwrap_or_default();
                    warn!(
                        "ClickUp GET /list/{}/task page {} failed ({}): {}. Skipping list.",
                        list.id, page, status, body
                    );
                    break;
                }

                let resp: ClickupTasksResponse = res.json().await?;
                let tasks = resp.tasks;

                if tasks.is_empty() {
                    break;
                }

                let mut new_in_page = 0;

                for task in &tasks {
                    let task_id = task.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if task_id.is_empty() || !seen_task_ids.insert(task_id.clone()) {
                        continue;
                    }
                    new_in_page += 1;

                    let custom_id = task.get("custom_id").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let canonical_source_id = custom_id.clone().unwrap_or_else(|| task_id.clone());

                    let title = task
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Untitled Task")
                        .to_string();

                    let body = task
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let status_obj = task.get("status");
                    let status_name = status_obj
                        .and_then(|s| s.get("status").or_else(|| s.get("name")))
                        .and_then(|v| v.as_str())
                        .unwrap_or("open");

                    let priority_obj = task.get("priority");
                    let priority_name = priority_obj
                        .and_then(|p| p.get("priority").or_else(|| p.get("name")))
                        .and_then(|v| v.as_str())
                        .unwrap_or("none");

                    let summary = format!("Status: {} | Priority: {}", status_name, priority_name);

                    // Tags & Labels
                    let mut tags = Vec::new();
                    if let Some(task_tags) = task.get("tags").and_then(|v| v.as_array()) {
                        for t in task_tags {
                            if let Some(name) = t.get("name").and_then(|v| v.as_str()) {
                                tags.push(name.to_string());
                            }
                        }
                    }
                    tags.push(format!("status:{}", status_name));
                    tags.push(format!("space:{}", space_id));
                    if let Some(ref f) = folder_opt {
                        tags.push(format!("folder:{}", f));
                    }
                    if !list_name.is_empty() {
                        tags.push(format!("list:{}", list_name));
                    }

                    // ArtifactKind determination
                    let kind = {
                        let combined = format!("{} {} {}", list_name, title, tags.join(" ")).to_lowercase();
                        if combined.contains("bug") || combined.contains("defect") {
                            ArtifactKind::Issue
                        } else if combined.contains("epic") {
                            ArtifactKind::Other("epic".to_string())
                        } else if combined.contains("milestone") {
                            ArtifactKind::Other("milestone".to_string())
                        } else if combined.contains("feature") {
                            ArtifactKind::Ticket
                        } else {
                            ArtifactKind::Ticket
                        }
                    };

                    // Relationships
                    let mut relationships = Vec::new();

                    // Parent relationship
                    if let Some(parent_val) = task.get("parent") {
                        let parent_id = if parent_val.is_string() {
                            parent_val.as_str().map(|s| s.to_string())
                        } else if parent_val.is_object() {
                            parent_val.get("id").and_then(|v| v.as_str()).map(|s| s.to_string())
                        } else {
                            None
                        };

                        if let Some(pid) = parent_id {
                            if !pid.is_empty() && pid != task_id {
                                relationships.push(ArtifactRelationship {
                                    source_id: canonical_source_id.clone(),
                                    target_id: pid.clone(),
                                    relationship_type: "child_of".to_string(),
                                });
                                relationships.push(ArtifactRelationship {
                                    source_id: pid,
                                    target_id: canonical_source_id.clone(),
                                    relationship_type: "parent_of".to_string(),
                                });
                            }
                        }
                    }

                    // Dependencies relationship
                    if let Some(deps) = task.get("dependencies").and_then(|v| v.as_array()) {
                        for dep in deps {
                            if let Some(dep_task_id) = dep.get("task_id").and_then(|v| v.as_str()) {
                                if !dep_task_id.is_empty() {
                                    relationships.push(ArtifactRelationship {
                                        source_id: canonical_source_id.clone(),
                                        target_id: dep_task_id.to_string(),
                                        relationship_type: "depends_on".to_string(),
                                    });
                                }
                            }
                        }
                    }

                    // Linked tasks relationship
                    if let Some(linked) = task.get("linked_tasks").and_then(|v| v.as_array()) {
                        for l in linked {
                            if let Some(link_task_id) = l.get("task_id").and_then(|v| v.as_str()) {
                                if !link_task_id.is_empty() {
                                    relationships.push(ArtifactRelationship {
                                        source_id: canonical_source_id.clone(),
                                        target_id: link_task_id.to_string(),
                                        relationship_type: "relates_to".to_string(),
                                    });
                                }
                            }
                        }
                    }

                    let web_url = task
                        .get("url")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("https://app.clickup.com/t/{}", task_id));

                    let created_at = task.get("date_created").and_then(Self::parse_timestamp);
                    let updated_at = task.get("date_updated").and_then(Self::parse_timestamp).unwrap_or(now);

                    let artifact_id = KnowledgeArtifact::generate_id("clickup", &self.base_url, &task_id);
                    let checksum = KnowledgeArtifact::compute_checksum(
                        &title,
                        Some(&summary),
                        &body,
                        &tags,
                    );

                    objects.push(KnowledgeArtifact {
                        id: artifact_id,
                        kind,
                        title,
                        summary: Some(summary),
                        body,
                        provider: "clickup".to_string(),
                        source_id: canonical_source_id,
                        source_url: web_url,
                        repository: None,
                        tags,
                        relationships,
                        created_at,
                        updated_at,
                        synced_at: now,
                        checksum,
                        metadata: task.clone(),
                    });
                }

                if new_in_page == 0 || resp.last_page == Some(true) {
                    break;
                }

                page += 1;
            }
        }

        Ok(objects)
    }
}
