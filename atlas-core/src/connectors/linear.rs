use crate::config::ConnectorConfig;
use crate::connectors::Connector;
use crate::domain::{ArtifactKind, ArtifactRelationship, KnowledgeArtifact};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};
use std::collections::HashSet;

pub struct LinearConnector {
    id: String,
    config: ConnectorConfig,
    client: reqwest::Client,
}

impl LinearConnector {
    pub fn new(id: String, config: ConnectorConfig) -> Result<Self> {
        let token = config.get_api_token()?;
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&token)?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self { id, config, client })
    }

    pub fn endpoint_url(&self) -> String {
        if !self.config.instance_url.is_empty() {
            self.config.instance_url.clone()
        } else {
            "https://api.linear.app/graphql".to_string()
        }
    }

    pub fn parse_issues_json(&self, base_url: &str, json_val: &Value, artifacts: &mut Vec<KnowledgeArtifact>) {
        if let Some(nodes) = json_val["data"]["issues"]["nodes"].as_array() {
            for node in nodes {
                let source_id = node["identifier"]
                    .as_str()
                    .or_else(|| node["id"].as_str())
                    .unwrap_or("")
                    .to_string();

                if source_id.is_empty() {
                    continue;
                }

                let title = node["title"].as_str().unwrap_or("Untitled Issue").to_string();
                let description = node["description"].as_str().unwrap_or("");
                let issue_url = node["url"]
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("{}/issue/{}", base_url, source_id));

                let created_at = node["createdAt"]
                    .as_str()
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)));
                let updated_at = node["updatedAt"]
                    .as_str()
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)))
                    .unwrap_or_else(Utc::now);

                let mut relationships = Vec::new();
                let mut tags = Vec::new();

                if let Some(state_name) = node["state"]["name"].as_str() {
                    tags.push(format!("status:{}", state_name.to_lowercase()));
                }
                if let Some(priority_label) = node["priorityLabel"].as_str() {
                    tags.push(format!("priority:{}", priority_label.to_lowercase()));
                }
                if let Some(team_name) = node["team"]["name"].as_str() {
                    tags.push(format!("team:{}", team_name));
                }
                if let Some(team_key) = node["team"]["key"].as_str() {
                    relationships.push(ArtifactRelationship {
                        source_id: source_id.clone(),
                        target_id: team_key.to_string(),
                        relationship_type: "belongs_to".to_string(),
                    });
                }
                if let Some(proj_id) = node["project"]["id"].as_str() {
                    relationships.push(ArtifactRelationship {
                        source_id: source_id.clone(),
                        target_id: proj_id.to_string(),
                        relationship_type: "part_of".to_string(),
                    });
                }
                if let Some(cycle_id) = node["cycle"]["id"].as_str() {
                    relationships.push(ArtifactRelationship {
                        source_id: source_id.clone(),
                        target_id: cycle_id.to_string(),
                        relationship_type: "part_of_cycle".to_string(),
                    });
                }

                let canonical_id = KnowledgeArtifact::generate_id("linear", base_url, &source_id);
                let checksum = KnowledgeArtifact::compute_checksum(&title, None, description, &tags);

                let artifact_source_id = source_id.clone();
                let artifact_source_url = issue_url.clone();
                let artifact_repo = node["team"]["key"].as_str().map(|s| s.to_string());

                let artifact = KnowledgeArtifact {
                    id: canonical_id,
                    kind: ArtifactKind::Issue,
                    title,
                    summary: None,
                    body: description.to_string(),
                    provider: "linear".to_string(),
                    source_id,
                    source_url: issue_url,
                    repository: artifact_repo.clone(),
                    tags,
                    relationships,
                    created_at,
                    updated_at,
                    synced_at: Utc::now(),
                    checksum,
                    metadata: node.clone(),
                };
                artifacts.push(artifact);

                // Extract nested comment discussions if sync_comments is enabled
                if self.config.sync_comments.unwrap_or(true) {
                    if let Some(comments) = node["comments"]["nodes"].as_array() {
                        for comment in comments {
                            let cid = comment["id"].as_str().unwrap_or("");
                            let cbody = comment["body"].as_str().unwrap_or("");
                            if cid.is_empty() || cbody.is_empty() {
                                continue;
                            }
                            let c_canonical_id = KnowledgeArtifact::generate_id("linear", base_url, cid);
                            let c_checksum = KnowledgeArtifact::compute_checksum(&format!("Comment on {}", artifact_source_id), None, cbody, &[]);
                            artifacts.push(KnowledgeArtifact {
                                id: c_canonical_id,
                                kind: ArtifactKind::Discussion,
                                title: format!("Comment on {}", artifact_source_id),
                                summary: None,
                                body: cbody.to_string(),
                                provider: "linear".to_string(),
                                source_id: cid.to_string(),
                                source_url: artifact_source_url.clone(),
                                repository: artifact_repo.clone(),
                                tags: vec!["linear:comment".to_string()],
                                relationships: vec![ArtifactRelationship {
                                    source_id: cid.to_string(),
                                    target_id: artifact_source_id.clone(),
                                    relationship_type: "child_of".to_string(),
                                }],
                                created_at: comment["createdAt"].as_str().and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc))),
                                updated_at,
                                synced_at: Utc::now(),
                                checksum: c_checksum,
                                metadata: comment.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    pub fn parse_projects_json(&self, base_url: &str, json_val: &Value, artifacts: &mut Vec<KnowledgeArtifact>) {
        if let Some(nodes) = json_val["data"]["projects"]["nodes"].as_array() {
            for node in nodes {
                let id = node["id"].as_str().unwrap_or("");
                let name = node["name"].as_str().unwrap_or("Untitled Project");
                let description = node["description"].as_str().unwrap_or("");
                let url = node["url"].as_str().unwrap_or("");
                let state = node["state"].as_str().unwrap_or("planned");

                if id.is_empty() {
                    continue;
                }

                let canonical_id = KnowledgeArtifact::generate_id("linear", base_url, id);
                let checksum = KnowledgeArtifact::compute_checksum(name, None, description, &[]);

                artifacts.push(KnowledgeArtifact {
                    id: canonical_id,
                    kind: ArtifactKind::Document,
                    title: format!("Project: {}", name),
                    summary: if description.is_empty() { None } else { Some(description.to_string()) },
                    body: format!("## Project: {}\nStatus: {}\n\n{}", name, state, description),
                    provider: "linear".to_string(),
                    source_id: id.to_string(),
                    source_url: url.to_string(),
                    repository: None,
                    tags: vec!["linear:project".to_string(), format!("state:{}", state)],
                    relationships: Vec::new(),
                    created_at: node["createdAt"].as_str().and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc))),
                    updated_at: node["updatedAt"].as_str().and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc))).unwrap_or_else(Utc::now),
                    synced_at: Utc::now(),
                    checksum,
                    metadata: node.clone(),
                });
            }
        }
    }
}

#[async_trait::async_trait]
impl Connector for LinearConnector {
    fn id(&self) -> &str {
        &self.id
    }

    fn provider(&self) -> &str {
        "linear"
    }

    async fn verify(&self) -> Result<String> {
        let query = r#"
        query VerifyAuth {
            viewer {
                id
                name
                email
            }
        }
        "#;
        let body = json!({ "query": query });
        let res = self
            .client
            .post(self.endpoint_url())
            .json(&body)
            .send()
            .await
            .context("Failed to connect to Linear API")?;

        if res.status().is_success() {
            let json_val: Value = res.json().await.unwrap_or_default();
            if let Some(viewer) = json_val.get("data").and_then(|d| d.get("viewer")) {
                let name = viewer["name"].as_str().unwrap_or("authenticated user");
                let email = viewer["email"].as_str().unwrap_or("");
                Ok(format!("Connected to Linear as {} ({})", name, email))
            } else {
                Ok("Connected to Linear successfully.".to_string())
            }
        } else {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            bail!("Linear verification failed with status {}: {}", status, err_text);
        }
    }

    async fn fetch_modified(&self, since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        let mut artifacts = Vec::new();
        let base_url = if self.config.instance_url.is_empty() {
            "https://linear.app"
        } else {
            self.config.instance_url.trim_end_matches('/')
        };

        // 1. GraphQL query for Issues with cursor pagination loop
        let query_issues = r#"
        query FetchIssues($filter: IssueFilter, $after: String) {
            issues(filter: $filter, first: 100, after: $after) {
                pageInfo {
                    hasNextPage
                    endCursor
                }
                nodes {
                    id
                    identifier
                    title
                    description
                    priority
                    priorityLabel
                    createdAt
                    updatedAt
                    url
                    state { name }
                    team { id name key }
                    project { id name }
                    cycle { id name number }
                    assignee { id name email }
                    comments {
                        nodes {
                            id
                            body
                            createdAt
                            user { name }
                        }
                    }
                }
            }
        }
        "#;

        let filter = if let Some(ts) = since {
            json!({ "updatedAt": { "gt": ts.to_rfc3339() } })
        } else {
            json!({})
        };

        let mut after_cursor: Option<String> = None;
        let mut seen_issue_ids = HashSet::new();

        loop {
            let mut variables = json!({
                "filter": filter,
            });
            if let Some(ref c) = after_cursor {
                variables["after"] = json!(c);
            }

            let body = json!({
                "query": query_issues,
                "variables": variables,
            });

            let res = self
                .client
                .post(self.endpoint_url())
                .json(&body)
                .send()
                .await;

            let response = match res {
                Ok(r) if r.status().is_success() => r,
                _ => break,
            };

            let json_val: Value = response.json().await.unwrap_or_default();
            self.parse_issues_json(base_url, &json_val, &mut artifacts);

            let page_info = &json_val["data"]["issues"]["pageInfo"];
            let has_next = page_info["hasNextPage"].as_bool().unwrap_or(false);
            let end_cursor = page_info["endCursor"].as_str().map(|s| s.to_string());

            if !has_next || end_cursor.is_none() {
                break;
            }

            if let Some(ref c) = end_cursor {
                if !seen_issue_ids.insert(c.clone()) {
                    break;
                }
            }
            after_cursor = end_cursor;
        }

        // 2. Fetch Projects if enabled
        let query_projects = r#"
        query FetchProjects {
            projects(first: 50) {
                nodes {
                    id
                    name
                    description
                    state
                    url
                    createdAt
                    updatedAt
                }
            }
        }
        "#;
        let body_proj = json!({ "query": query_projects });
        if let Ok(res_proj) = self.client.post(self.endpoint_url()).json(&body_proj).send().await {
            if res_proj.status().is_success() {
                let json_proj: Value = res_proj.json().await.unwrap_or_default();
                self.parse_projects_json(base_url, &json_proj, &mut artifacts);
            }
        }

        Ok(artifacts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_connector_creation() {
        let mut cfg = ConnectorConfig::default();
        cfg.provider = "linear".to_string();
        cfg.api_token = Some("lin_api_test_key".to_string());

        let conn = LinearConnector::new("linear-test".to_string(), cfg).unwrap();
        assert_eq!(conn.id(), "linear-test");
        assert_eq!(conn.provider(), "linear");
    }

    #[test]
    fn test_linear_parse_issue_json() {
        let mut cfg = ConnectorConfig::default();
        cfg.provider = "linear".to_string();
        cfg.api_token = Some("lin_api_test_key".to_string());
        let conn = LinearConnector::new("linear-parse".to_string(), cfg).unwrap();

        let json_val = serde_json::json!({
            "data": {
                "issues": {
                    "pageInfo": { "hasNextPage": false, "endCursor": null },
                    "nodes": [
                        {
                            "id": "issue-123",
                            "identifier": "ENG-101",
                            "title": "Fix GraphQL memory leak",
                            "description": "Investigate query pooling",
                            "url": "https://linear.app/acme/issue/ENG-101",
                            "createdAt": "2026-08-01T10:00:00Z",
                            "updatedAt": "2026-08-02T12:00:00Z",
                            "state": { "name": "In Progress" },
                            "priorityLabel": "High",
                            "team": { "key": "ENG", "name": "Engineering" },
                            "project": { "id": "proj-1" },
                            "cycle": { "id": "cycle-42" },
                            "comments": {
                                "nodes": [
                                    {
                                        "id": "comment-1",
                                        "body": "Found root cause in pool allocator",
                                        "createdAt": "2026-08-02T11:00:00Z"
                                    }
                                ]
                            }
                        }
                    ]
                }
            }
        });

        let mut artifacts = Vec::new();
        conn.parse_issues_json("https://linear.app", &json_val, &mut artifacts);

        assert_eq!(artifacts.len(), 2); // 1 Issue + 1 Comment
        assert!(artifacts.iter().any(|a| a.title == "Fix GraphQL memory leak"));
        assert!(artifacts.iter().any(|a| a.body.contains("Found root cause")));
        assert!(artifacts[0].tags.iter().any(|t| t == "priority:high"));
    }

    #[test]
    fn test_linear_parse_projects_json() {
        let mut cfg = ConnectorConfig::default();
        cfg.provider = "linear".to_string();
        cfg.api_token = Some("lin_api_test_key".to_string());
        let conn = LinearConnector::new("linear-parse-proj".to_string(), cfg).unwrap();

        let json_proj = serde_json::json!({
            "data": {
                "projects": {
                    "nodes": [
                        {
                            "id": "proj-99",
                            "name": "Q3 Infrastructure Overhaul",
                            "description": "Migrate to multi-region Kubernetes",
                            "state": "in_progress",
                            "url": "https://linear.app/acme/project/proj-99",
                            "createdAt": "2026-07-01T00:00:00Z",
                            "updatedAt": "2026-08-01T00:00:00Z"
                        }
                    ]
                }
            }
        });

        let mut artifacts = Vec::new();
        conn.parse_projects_json("https://linear.app", &json_proj, &mut artifacts);

        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].title, "Project: Q3 Infrastructure Overhaul");
        assert_eq!(artifacts[0].kind, ArtifactKind::Document);
    }
}
