use crate::config::ConnectorConfig;
use crate::connectors::Connector;
use crate::domain::{ArtifactKind, ArtifactRelationship, KnowledgeArtifact};
use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use serde_json::Value;

fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i < input.len() {
        let b0 = input[i];
        let b1 = if i + 1 < input.len() { input[i + 1] } else { 0 };
        let b2 = if i + 2 < input.len() { input[i + 2] } else { 0 };

        out.push(CHARS[(b0 >> 2) as usize] as char);
        out.push(CHARS[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);

        if i + 1 < input.len() {
            out.push(CHARS[(((b1 & 0x0F) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }

        if i + 2 < input.len() {
            out.push(CHARS[(b2 & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }

        i += 3;
    }
    out
}

pub struct BitbucketConnector {
    id: String,
    config: ConnectorConfig,
    client: reqwest::Client,
}

impl BitbucketConnector {
    pub fn new(id: String, config: ConnectorConfig) -> Result<Self> {
        let token = config.get_api_token()?;
        let mut headers = HeaderMap::new();

        if !config.email.is_empty() {
            let auth_raw = format!("{}:{}", config.email, token);
            let auth_b64 = base64_encode(auth_raw.as_bytes());
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Basic {}", auth_b64))?,
            );
        } else {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", token))?,
            );
        }

        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self { id, config, client })
    }

    fn base_url(&self) -> String {
        if !self.config.instance_url.is_empty() {
            self.config.instance_url.trim_end_matches('/').to_string()
        } else {
            "https://api.bitbucket.org/2.0".to_string()
        }
    }
}

#[async_trait::async_trait]
impl Connector for BitbucketConnector {
    fn id(&self) -> &str {
        &self.id
    }

    fn provider(&self) -> &str {
        "bitbucket"
    }

    async fn fetch_modified(&self, _since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        let mut artifacts = Vec::new();
        let base = self.base_url();
        let workspace = self.config.workspace.as_deref().unwrap_or("");

        let repos_url = if !workspace.is_empty() {
            format!("{}/repositories/{}", base, workspace)
        } else {
            format!("{}/repositories", base)
        };

        if let Ok(res) = self.client.get(&repos_url).send().await {
            if res.status().is_success() {
                let json_val: Value = res.json().await.unwrap_or_default();
                if let Some(values) = json_val["values"].as_array() {
                    for repo in values {
                        let repo_slug = repo["slug"].as_str().unwrap_or("");
                        let repo_name = repo["name"].as_str().unwrap_or("Untitled Repository");
                        let full_name = repo["full_name"].as_str().unwrap_or(repo_slug);
                        let html_url = repo["links"]["html"]["href"].as_str().unwrap_or("").to_string();

                        let canonical_id = KnowledgeArtifact::generate_id("bitbucket", &base, full_name);
                        let checksum = KnowledgeArtifact::compute_checksum(repo_name, None, "", &[]);

                        artifacts.push(KnowledgeArtifact {
                            id: canonical_id,
                            kind: ArtifactKind::Repository,
                            title: repo_name.to_string(),
                            summary: None,
                            body: repo["description"].as_str().unwrap_or("").to_string(),
                            provider: "bitbucket".to_string(),
                            source_id: full_name.to_string(),
                            source_url: html_url,
                            repository: Some(full_name.to_string()),
                            tags: vec!["bitbucket:repository".to_string()],
                            relationships: Vec::new(),
                            created_at: repo["created_on"].as_str().and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc))),
                            updated_at: repo["updated_on"].as_str().and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc))).unwrap_or_else(Utc::now),
                            synced_at: Utc::now(),
                            checksum,
                            metadata: repo.clone(),
                        });

                        // Fetch Pull Requests for each repository
                        let prs_url = format!("{}/repositories/{}/pullrequests", base, full_name);
                        if let Ok(pr_res) = self.client.get(&prs_url).send().await {
                            if pr_res.status().is_success() {
                                let pr_json: Value = pr_res.json().await.unwrap_or_default();
                                if let Some(prs) = pr_json["values"].as_array() {
                                    for pr in prs {
                                        let pr_id = pr["id"].as_u64().unwrap_or(0).to_string();
                                        let title = pr["title"].as_str().unwrap_or("Untitled PR").to_string();
                                        let summary_raw = pr["summary"]["raw"].as_str().unwrap_or("");
                                        let pr_web_url = pr["links"]["html"]["href"].as_str().unwrap_or("").to_string();
                                        let pr_source_id = format!("{}/pr/{}", full_name, pr_id);

                                        let created_at = pr["created_on"]
                                            .as_str()
                                            .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)));
                                        let updated_at = pr["updated_on"]
                                            .as_str()
                                            .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)))
                                            .unwrap_or_else(Utc::now);

                                        let pr_canonical_id = KnowledgeArtifact::generate_id("bitbucket", &base, &pr_source_id);
                                        let pr_checksum = KnowledgeArtifact::compute_checksum(&title, None, summary_raw, &[]);

                                        artifacts.push(KnowledgeArtifact {
                                            id: pr_canonical_id,
                                            kind: ArtifactKind::PullRequest,
                                            title,
                                            summary: None,
                                            body: summary_raw.to_string(),
                                            provider: "bitbucket".to_string(),
                                            source_id: pr_source_id,
                                            source_url: pr_web_url,
                                            repository: Some(full_name.to_string()),
                                            tags: vec!["bitbucket:pull_request".to_string()],
                                            relationships: vec![ArtifactRelationship {
                                                source_id: format!("{}/pr/{}", full_name, pr_id),
                                                target_id: full_name.to_string(),
                                                relationship_type: "belongs_to".to_string(),
                                            }],
                                            created_at,
                                            updated_at,
                                            synced_at: Utc::now(),
                                            checksum: pr_checksum,
                                            metadata: pr.clone(),
                                        });
                                    }
                                }
                            }
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
    fn test_bitbucket_connector_creation() {
        let mut cfg = ConnectorConfig::default();
        cfg.provider = "bitbucket".to_string();
        cfg.api_token = Some("bb_app_pwd_12345".to_string());
        cfg.email = "dev@acme.com".to_string();

        let conn = BitbucketConnector::new("bitbucket-test".to_string(), cfg).unwrap();
        assert_eq!(conn.id(), "bitbucket-test");
        assert_eq!(conn.provider(), "bitbucket");
    }
}
