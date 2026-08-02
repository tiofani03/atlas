use crate::config::ConnectorConfig;
use crate::connectors::Connector;
use crate::domain::{ArtifactKind, ArtifactRelationship, KnowledgeArtifact};
use anyhow::{bail, Context, Result};
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

    pub fn base_url(&self) -> String {
        if !self.config.instance_url.is_empty() {
            self.config.instance_url.trim_end_matches('/').to_string()
        } else {
            "https://gitlab.com".to_string()
        }
    }

    pub fn parse_merge_requests_json(&self, base: &str, project: &str, mrs: &[Value], artifacts: &mut Vec<KnowledgeArtifact>) {
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

            let mut relationships = vec![ArtifactRelationship {
                source_id: source_id.clone(),
                target_id: project.to_string(),
                relationship_type: "belongs_to".to_string(),
            }];

            if let Some(source_branch) = mr["source_branch"].as_str() {
                relationships.push(ArtifactRelationship {
                    source_id: source_id.clone(),
                    target_id: source_branch.to_string(),
                    relationship_type: "source_branch".to_string(),
                });
            }

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
                tags: vec!["gitlab:mr".to_string(), format!("project:{}", project)],
                relationships,
                created_at,
                updated_at,
                synced_at: Utc::now(),
                checksum,
                metadata: mr.clone(),
            });
        }
    }

    pub fn parse_issues_json(&self, base: &str, project: &str, issues: &[Value], artifacts: &mut Vec<KnowledgeArtifact>) {
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

            let relationships = vec![ArtifactRelationship {
                source_id: source_id.clone(),
                target_id: project.to_string(),
                relationship_type: "belongs_to".to_string(),
            }];

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
                tags: vec!["gitlab:issue".to_string(), format!("project:{}", project)],
                relationships,
                created_at,
                updated_at,
                synced_at: Utc::now(),
                checksum,
                metadata: issue.clone(),
            });
        }
    }

    pub fn parse_commits_json(&self, base: &str, project: &str, commits: &[Value], artifacts: &mut Vec<KnowledgeArtifact>) {
        for commit in commits {
            let sha = commit["id"].as_str().unwrap_or("");
            if sha.is_empty() {
                continue;
            }

            let title = commit["title"].as_str().unwrap_or("").to_string();
            let message = commit["message"].as_str().unwrap_or("");
            let author_name = commit["author_name"].as_str().unwrap_or("Unknown");
            let web_url = commit["web_url"].as_str().unwrap_or("").to_string();
            let source_id = format!("{}@{}", project, sha);

            let created_at = commit["created_at"]
                .as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)));
            let updated_at = commit["committed_date"]
                .as_str()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)))
                .unwrap_or_else(Utc::now);

            let canonical_id = KnowledgeArtifact::generate_id("gitlab", base, &source_id);
            let checksum = KnowledgeArtifact::compute_checksum(&title, None, message, &[]);

            let relationships = vec![ArtifactRelationship {
                source_id: source_id.clone(),
                target_id: project.to_string(),
                relationship_type: "belongs_to".to_string(),
            }];

            artifacts.push(KnowledgeArtifact {
                id: canonical_id,
                kind: ArtifactKind::Commit,
                title,
                summary: Some(format!("Commit by {}", author_name)),
                body: message.to_string(),
                provider: "gitlab".to_string(),
                source_id,
                source_url: web_url,
                repository: Some(project.to_string()),
                tags: vec!["gitlab:commit".to_string(), format!("author:{}", author_name)],
                relationships,
                created_at,
                updated_at,
                synced_at: Utc::now(),
                checksum,
                metadata: commit.clone(),
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

    async fn verify(&self) -> Result<String> {
        let base = self.base_url();
        let user_url = format!("{}/api/v4/user", base);
        let res = self.client.get(&user_url).send().await.context("Failed to connect to GitLab API")?;
        if res.status().is_success() {
            let user_info: Value = res.json().await.unwrap_or_default();
            let username = user_info["username"].as_str().unwrap_or("authenticated user");
            Ok(format!("Connected to GitLab at {} as @{}.", base, username))
        } else {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            bail!("GitLab verification failed with status {}: {}", status, err_text);
        }
    }

    async fn fetch_modified(&self, since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        let mut artifacts = Vec::new();
        let base = self.base_url();
        let updated_after = since.map(|s| s.to_rfc3339());

        for project in &self.config.projects {
            let encoded_project = project.replace('/', "%2F");

            // 1. Fetch Merge Requests with Pagination Loop
            let mut page = 1;
            let per_page = 100;
            loop {
                let mut mr_url = format!(
                    "{}/api/v4/projects/{}/merge_requests?per_page={}&page={}",
                    base, encoded_project, per_page, page
                );
                if let Some(ref ts) = updated_after {
                    mr_url.push_str(&format!("&updated_after={}", ts));
                }

                let resp = match self.client.get(&mr_url).send().await {
                    Ok(r) if r.status().is_success() => r,
                    _ => break,
                };

                let mrs: Vec<Value> = match resp.json().await {
                    Ok(items) => items,
                    Err(_) => break,
                };

                if mrs.is_empty() {
                    break;
                }

                let count = mrs.len();
                self.parse_merge_requests_json(&base, project, &mrs, &mut artifacts);

                if count < per_page {
                    break;
                }
                page += 1;
            }

            // 2. Fetch Issues with Pagination Loop
            page = 1;
            loop {
                let mut issues_url = format!(
                    "{}/api/v4/projects/{}/issues?per_page={}&page={}",
                    base, encoded_project, per_page, page
                );
                if let Some(ref ts) = updated_after {
                    issues_url.push_str(&format!("&updated_after={}", ts));
                }

                let resp = match self.client.get(&issues_url).send().await {
                    Ok(r) if r.status().is_success() => r,
                    _ => break,
                };

                let issues: Vec<Value> = match resp.json().await {
                    Ok(items) => items,
                    Err(_) => break,
                };

                if issues.is_empty() {
                    break;
                }

                let count = issues.len();
                self.parse_issues_json(&base, project, &issues, &mut artifacts);

                if count < per_page {
                    break;
                }
                page += 1;
            }

            // 3. Fetch Commits with Pagination Loop
            page = 1;
            loop {
                let mut commits_url = format!(
                    "{}/api/v4/projects/{}/repository/commits?per_page={}&page={}",
                    base, encoded_project, per_page, page
                );
                if let Some(ref ts) = updated_after {
                    commits_url.push_str(&format!("&since={}", ts));
                }

                let resp = match self.client.get(&commits_url).send().await {
                    Ok(r) if r.status().is_success() => r,
                    _ => break,
                };

                let commits: Vec<Value> = match resp.json().await {
                    Ok(items) => items,
                    Err(_) => break,
                };

                if commits.is_empty() {
                    break;
                }

                let count = commits.len();
                self.parse_commits_json(&base, project, &commits, &mut artifacts);

                if count < per_page {
                    break;
                }
                page += 1;
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
        assert_eq!(conn.base_url(), "https://gitlab.com");
    }

    #[test]
    fn test_gitlab_parse_mr_json() {
        let mut cfg = ConnectorConfig::default();
        cfg.provider = "gitlab".to_string();
        cfg.api_token = Some("glpat-test12345".to_string());
        let conn = GitlabConnector::new("gitlab-parse".to_string(), cfg).unwrap();

        let mr_json = serde_json::json!({
            "iid": 42,
            "title": "Add OAuth2 PKCE Flow",
            "description": "Implements PKCE security for mobile clients",
            "web_url": "https://gitlab.com/group/repo/-/merge_requests/42",
            "source_branch": "feature/pkce",
            "created_at": "2026-08-01T12:00:00Z",
            "updated_at": "2026-08-02T12:00:00Z"
        });

        let mut artifacts = Vec::new();
        conn.parse_merge_requests_json("https://gitlab.com", "group/repo", &[mr_json], &mut artifacts);

        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].kind, ArtifactKind::PullRequest);
        assert!(artifacts[0].title.contains("Add OAuth2 PKCE Flow"));
        assert!(artifacts[0].relationships.iter().any(|r| r.relationship_type == "source_branch"));
    }

    #[test]
    fn test_gitlab_parse_commits_json() {
        let mut cfg = ConnectorConfig::default();
        cfg.provider = "gitlab".to_string();
        cfg.api_token = Some("glpat-test12345".to_string());
        let conn = GitlabConnector::new("gitlab-parse-commits".to_string(), cfg).unwrap();

        let commit_json = serde_json::json!({
            "id": "279764426543b593685e13efec2698299a9cfcb8",
            "title": "fix: sanitize sql injection",
            "message": "fix: sanitize sql injection\n\nCloses #102",
            "author_name": "Alice Developer",
            "web_url": "https://gitlab.com/group/repo/-/commit/279764426543b593685e13efec2698299a9cfcb8",
            "created_at": "2026-08-01T14:00:00Z",
            "committed_date": "2026-08-01T14:05:00Z"
        });

        let mut artifacts = Vec::new();
        conn.parse_commits_json("https://gitlab.com", "group/repo", &[commit_json], &mut artifacts);

        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].kind, ArtifactKind::Commit);
        assert_eq!(artifacts[0].source_id, "group/repo@279764426543b593685e13efec2698299a9cfcb8");
        assert!(artifacts[0].tags.iter().any(|t| t == "author:Alice Developer"));
    }
}
