use crate::config::ConnectorConfig;
use crate::connectors::Connector;
use crate::domain::{ArtifactKind, ArtifactRelationship, KnowledgeArtifact};
use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};

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

    fn endpoint_url(&self) -> String {
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

                if let Some(team_name) = node["team"]["name"].as_str() {
                    tags.push(format!("team:{}", team_name));
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
                    repository: node["team"]["key"].as_str().map(|s| s.to_string()),
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
}

#[async_trait::async_trait]
impl Connector for LinearConnector {
    fn id(&self) -> &str {
        &self.id
    }

    fn provider(&self) -> &str {
        "linear"
    }

    async fn fetch_modified(&self, since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        let mut artifacts = Vec::new();
        let base_url = if self.config.instance_url.is_empty() {
            "https://linear.app"
        } else {
            self.config.instance_url.trim_end_matches('/')
        };

        // 1. GraphQL query for Issues
        let query = r#"
        query FetchIssues($filter: IssueFilter) {
            issues(filter: $filter, first: 100) {
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

        let body = json!({
            "query": query,
            "variables": { "filter": filter }
        });

        let res = self
            .client
            .post(self.endpoint_url())
            .json(&body)
            .send()
            .await;

        if let Ok(response) = res {
            if response.status().is_success() {
                let json_val: Value = response.json().await.unwrap_or_default();
                self.parse_issues_json(base_url, &json_val, &mut artifacts);
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
                    "nodes": [
                        {
                            "id": "issue-123",
                            "identifier": "ENG-101",
                            "title": "Fix GraphQL memory leak",
                            "description": "Investigate query pooling",
                            "url": "https://linear.app/acme/issue/ENG-101",
                            "createdAt": "2026-08-01T10:00:00Z",
                            "updatedAt": "2026-08-02T12:00:00Z",
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
        conn.parse_issues_json("https://api.linear.app/graphql", &json_val, &mut artifacts);

        assert_eq!(artifacts.len(), 2); // 1 Issue + 1 Comment
        assert!(artifacts.iter().any(|a| a.title == "Fix GraphQL memory leak"));
        assert!(artifacts.iter().any(|a| a.body.contains("Found root cause")));
    }
}
