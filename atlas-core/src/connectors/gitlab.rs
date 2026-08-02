use crate::config::ConnectorConfig;
use crate::connectors::Connector;
use crate::domain::{ArtifactKind, KnowledgeArtifact};
use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde_json::Value;

pub struct GitlabConnector {
    id: String,
    config: ConnectorConfig,
    client: reqwest::Client,
}

impl GitlabConnector {
    pub fn new(id: String, config: ConnectorConfig) -> Result<Self> {
        let token = config.get_api_token()?;
        let mut headers = HeaderMap::new();
        let private_token_header = HeaderName::from_static("private-token");
        headers.insert(
            private_token_header,
            HeaderValue::from_str(&token)?,
        );

        let mut builder = reqwest::Client::builder().default_headers(headers);
        if let Some(ref cert_path) = config.ssl_cert_path {
            if let Ok(cert_bytes) = std::fs::read(cert_path) {
                if let Ok(cert) = reqwest::Certificate::from_pem(&cert_bytes) {
                    builder = builder.add_root_certificate(cert);
                }
            }
        }

        let client = builder.build()?;
        Ok(Self { id, config, client })
    }

    fn base_url(&self) -> String {
        if !self.config.instance_url.is_empty() {
            self.config.instance_url.trim_end_matches('/').to_string()
        } else {
            "https://gitlab.com".to_string()
        }
    }

    fn parse_merge_requests_json(&self, base: &str, project: &str, mrs: &[Value], artifacts: &mut Vec<KnowledgeArtifact>) {
        for mr in mrs {
            let iid = mr["iid"].as_u64().unwrap_or(0);
            let title = mr["title"].as_str().unwrap_or("Untitled MR").to_string();
            let description = mr["description"].as_str().unwrap_or("");
            let web_url = mr["web_url"].as_str().unwrap_or("").to_string();
            let source_id = format!("{}/mr/{}", project, iid);

            let created_at = mr["created_at"]
                .as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)));
            let updated_at = mr["updated_at"]
                .as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)))
                .unwrap_or_else(Utc::now);

            let canonical_id = KnowledgeArtifact::generate_id("gitlab", base, &source_id);
            let checksum = KnowledgeArtifact::compute_checksum(&title, None, description, &[]);

            artifacts.push(KnowledgeArtifact {
                id: canonical_id,
                kind: ArtifactKind::PullRequest,
                title,
                summary: None,
                body: description.to_string(),
                provider: "gitlab".to_string(),
                source_id,
                source_url: web_url,
                repository: Some(project.to_string()),
                tags: vec!["gitlab:mr".to_string()],
                relationships: Vec::new(),
                created_at,
                updated_at,
                synced_at: Utc::now(),
                checksum,
                metadata: mr.clone(),
            });
        }
    }

    fn parse_issues_json(&self, base: &str, project: &str, issues: &[Value], artifacts: &mut Vec<KnowledgeArtifact>) {
        for issue in issues {
            let iid = issue["iid"].as_u64().unwrap_or(0);
            let title = issue["title"].as_str().unwrap_or("Untitled Issue").to_string();
            let description = issue["description"].as_str().unwrap_or("");
            let web_url = issue["web_url"].as_str().unwrap_or("").to_string();
            let source_id = format!("{}/issue/{}", project, iid);

            let created_at = issue["created_at"]
                .as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)));
            let updated_at = issue["updated_at"]
                .as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)))
                .unwrap_or_else(Utc::now);

            let canonical_id = KnowledgeArtifact::generate_id("gitlab", base, &source_id);
            let checksum = KnowledgeArtifact::compute_checksum(&title, None, description, &[]);

            artifacts.push(KnowledgeArtifact {
                id: canonical_id,
                kind: ArtifactKind::Issue,
                title,
                summary: None,
                body: description.to_string(),
                provider: "gitlab".to_string(),
                source_id,
                source_url: web_url,
                repository: Some(project.to_string()),
                tags: vec!["gitlab:issue".to_string()],
                relationships: Vec::new(),
                created_at,
                updated_at,
                synced_at: Utc::now(),
                checksum,
                metadata: issue.clone(),
            });
        }
    }
}

#[async_trait::async_trait]
impl Connector for GitlabConnector {
    fn id(&self) -> &str {
        &self.id
    }

    fn provider(&self) -> &str {
        "gitlab"
    }

    async fn fetch_modified(&self, since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        let mut artifacts = Vec::new();
        let base = self.base_url();
        let updated_after = since.map(|s| s.to_rfc3339());

        for project in &self.config.projects {
            let encoded_project = project.replace('/', "%2F");

            // 1. Fetch Merge Requests
            let mut mr_url = format!("{}/api/v4/projects/{}/merge_requests?per_page=100", base, encoded_project);
            if let Some(ref ts) = updated_after {
                mr_url.push_str(&format!("&updated_after={}", ts));
            }

            if let Ok(res) = self.client.get(&mr_url).send().await {
                if res.status().is_success() {
                    if let Ok(mrs) = res.json::<Vec<Value>>().await {
                        self.parse_merge_requests_json(&base, project, &mrs, &mut artifacts);
                    }
                }
            }

            // 2. Fetch Issues
            let mut issues_url = format!("{}/api/v4/projects/{}/issues?per_page=100", base, encoded_project);
            if let Some(ref ts) = updated_after {
                issues_url.push_str(&format!("&updated_after={}", ts));
            }

            if let Ok(res) = self.client.get(&issues_url).send().await {
                if res.status().is_success() {
                    if let Ok(issues) = res.json::<Vec<Value>>().await {
                        self.parse_issues_json(&base, project, &issues, &mut artifacts);
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
    fn test_gitlab_connector_creation() {
        let mut cfg = ConnectorConfig::default();
        cfg.provider = "gitlab".to_string();
        cfg.api_token = Some("glpat-test12345".to_string());
        cfg.projects = vec!["group/repo".to_string()];

        let conn = GitlabConnector::new("gitlab-test".to_string(), cfg).unwrap();
        assert_eq!(conn.id(), "gitlab-test");
        assert_eq!(conn.provider(), "gitlab");
    }

    #[test]
    fn test_gitlab_parse_json() {
        let mut cfg = ConnectorConfig::default();
        cfg.provider = "gitlab".to_string();
        cfg.api_token = Some("glpat-test12345".to_string());
        let conn = GitlabConnector::new("gitlab-parse".to_string(), cfg).unwrap();

        let mr_json = serde_json::json!({
            "iid": 42,
            "title": "Add OAuth2 PKCE Flow",
            "description": "Implements PKCE security for mobile clients",
            "web_url": "https://gitlab.com/group/repo/-/merge_requests/42",
            "created_at": "2026-08-01T12:00:00Z",
            "updated_at": "2026-08-02T12:00:00Z"
        });

        let mut artifacts = Vec::new();
        conn.parse_merge_requests_json("https://gitlab.com", "group/repo", &[mr_json], &mut artifacts);

        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].kind, ArtifactKind::PullRequest);
        assert!(artifacts[0].title.contains("Add OAuth2 PKCE Flow"));
    }
}
