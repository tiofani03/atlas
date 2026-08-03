use crate::config::ConnectorConfig;
use crate::connectors::Connector;
use crate::domain::{ArtifactKind, KnowledgeArtifact};
use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use serde_json::{json, Value};

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

pub struct AzureDevopsConnector {
    id: String,
    config: ConnectorConfig,
    client: reqwest::Client,
}

impl AzureDevopsConnector {
    pub fn new(id: String, config: ConnectorConfig) -> Result<Self> {
        let token = config.get_api_token()?;
        let auth_raw = format!(":{}", token);
        let auth_b64 = base64_encode(auth_raw.as_bytes());

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Basic {}", auth_b64))?,
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self { id, config, client })
    }

    fn base_url(&self) -> String {
        let org = self.config.organization.as_deref().unwrap_or("");
        if !self.config.instance_url.is_empty() {
            self.config.instance_url.trim_end_matches('/').to_string()
        } else if !org.is_empty() {
            format!("https://dev.azure.com/{}", org)
        } else {
            "https://dev.azure.com".to_string()
        }
    }
}

#[async_trait::async_trait]
impl Connector for AzureDevopsConnector {
    fn id(&self) -> &str {
        &self.id
    }

    fn provider(&self) -> &str {
        "azure_devops"
    }

    async fn verify(&self) -> Result<String> {
        let base = self.base_url();

        // Azure DevOps has no org-level whoami endpoint. Probe the account's
        // profile endpoint; fall back to the first configured project's
        // repository list when only a Bearer-style token (PAT) is available.
        let profile_url = format!("{}/_apis/profile/profiles/me?api-version=7.0", base);
        let resp = self
            .client
            .get(&profile_url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to Azure DevOps API: {}", e))?;

        let status = resp.status();
        if status.is_success() {
            let json: Value = resp.json().await.unwrap_or_default();
            let display_name = json["displayName"]
                .as_str()
                .unwrap_or("authenticated user");
            return Ok(format!(
                "Connected to Azure DevOps successfully as '{}'.",
                display_name
            ));
        }

        if let Some(project) = self.config.projects.first() {
            let repos_url = format!(
                "{}/{}/_apis/git/repositories?api-version=7.0",
                base, project
            );
            let repos_resp = self
                .client
                .get(&repos_url)
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to connect to Azure DevOps API: {}", e))?;
            if repos_resp.status().is_success() {
                let json: Value = repos_resp.json().await.unwrap_or_default();
                let count = json["count"].as_u64().unwrap_or(0);
                return Ok(format!(
                    "Connected to Azure DevOps successfully (project '{}', {} repositories).",
                    project, count
                ));
            }
        }

        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            bail!("Azure DevOps authentication failed: invalid or unauthorized PAT")
        }
        let err = resp.text().await.unwrap_or_default();
        bail!("Azure DevOps verification failed with status {}: {}", status, err)
    }

    async fn fetch_modified(&self, since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        let mut artifacts = Vec::new();
        let base = self.base_url();

        for project in &self.config.projects {
            // 1. Fetch Work Items via WIQL query
            let wiql_url = format!("{}/{}/_apis/wit/wiql?api-version=7.0", base, project);
            let mut date_clause = String::new();
            if let Some(ts) = since {
                date_clause = format!("AND [System.ChangedDate] > '{}'", ts.format("%Y-%m-%dT%H:%M:%SZ"));
            }

            let wiql_query = json!({
                "query": format!("SELECT [System.Id] FROM WorkItems WHERE [System.TeamProject] = '{}' {} ORDER BY [System.ChangedDate] DESC", project, date_clause)
            });

            if let Ok(res) = self.client.post(&wiql_url).json(&wiql_query).send().await {
                if res.status().is_success() {
                    let wiql_res: Value = res.json().await.unwrap_or_default();
                    if let Some(work_items) = wiql_res["workItems"].as_array() {
                        let ids: Vec<String> = work_items
                            .iter()
                            .filter_map(|wi| wi["id"].as_u64().map(|id| id.to_string()))
                            .take(200)
                            .collect();

                        if !ids.is_empty() {
                            let ids_str = ids.join(",");
                            let details_url = format!(
                                "{}/{}/_apis/wit/workitems?ids={}&$expand=all&api-version=7.0",
                                base, project, ids_str
                            );

                            if let Ok(details_res) = self.client.get(&details_url).send().await {
                                if details_res.status().is_success() {
                                    let details_json: Value = details_res.json().await.unwrap_or_default();
                                    if let Some(items) = details_json["value"].as_array() {
                                        for item in items {
                                            let wi_id = item["id"].as_u64().unwrap_or(0).to_string();
                                            let fields = &item["fields"];
                                            let title = fields["System.Title"].as_str().unwrap_or("Untitled Work Item").to_string();
                                            let description = fields["System.Description"].as_str().unwrap_or("");
                                            let wi_type = fields["System.WorkItemType"].as_str().unwrap_or("Task");
                                            let web_url = item["_links"]["html"]["href"]
                                                .as_str()
                                                .unwrap_or("")
                                                .to_string();

                                            let source_id = format!("{}:{}", project, wi_id);
                                            let created_at = fields["System.CreatedDate"]
                                                .as_str()
                                                .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)));
                                            let updated_at = fields["System.ChangedDate"]
                                                .as_str()
                                                .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)))
                                                .unwrap_or_else(Utc::now);

                                            let canonical_id = KnowledgeArtifact::generate_id("azure_devops", &base, &source_id);
                                            let checksum = KnowledgeArtifact::compute_checksum(&title, None, description, &[]);

                                            artifacts.push(KnowledgeArtifact {
                                                id: canonical_id,
                                                kind: ArtifactKind::Ticket,
                                                title,
                                                summary: None,
                                                body: description.to_string(),
                                                provider: "azure_devops".to_string(),
                                                source_id,
                                                source_url: web_url,
                                                repository: Some(project.clone()),
                                                tags: vec!["azure_devops:work_item".to_string(), wi_type.to_lowercase()],
                                                relationships: Vec::new(),
                                                created_at,
                                                updated_at,
                                                synced_at: Utc::now(),
                                                checksum,
                                                metadata: item.clone(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 2. Fetch Repositories and Pull Requests
            let repos_url = format!("{}/{}/_apis/git/repositories?api-version=7.0", base, project);
            if let Ok(res) = self.client.get(&repos_url).send().await {
                if res.status().is_success() {
                    let repos_json: Value = res.json().await.unwrap_or_default();
                    if let Some(repos) = repos_json["value"].as_array() {
                        for repo in repos {
                            let repo_id = repo["id"].as_str().unwrap_or("");
                            let repo_name = repo["name"].as_str().unwrap_or("Untitled Repo");

                            let prs_url = format!(
                                "{}/{}/_apis/git/repositories/{}/pullrequests?searchCriteria.status=all&api-version=7.0",
                                base, project, repo_id
                            );

                            if let Ok(pr_res) = self.client.get(&prs_url).send().await {
                                if pr_res.status().is_success() {
                                    let pr_json: Value = pr_res.json().await.unwrap_or_default();
                                    if let Some(prs) = pr_json["value"].as_array() {
                                        for pr in prs {
                                            let pr_id = pr["pullRequestId"].as_u64().unwrap_or(0).to_string();
                                            let title = pr["title"].as_str().unwrap_or("Untitled PR").to_string();
                                            let description = pr["description"].as_str().unwrap_or("");
                                            let pr_web_url = pr["url"].as_str().unwrap_or("").to_string();
                                            let source_id = format!("{}/{}/pr/{}", project, repo_name, pr_id);

                                            let created_at = pr["creationDate"]
                                                .as_str()
                                                .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)));

                                            let canonical_id = KnowledgeArtifact::generate_id("azure_devops", &base, &source_id);
                                            let checksum = KnowledgeArtifact::compute_checksum(&title, None, description, &[]);

                                            artifacts.push(KnowledgeArtifact {
                                                id: canonical_id,
                                                kind: ArtifactKind::PullRequest,
                                                title,
                                                summary: None,
                                                body: description.to_string(),
                                                provider: "azure_devops".to_string(),
                                                source_id,
                                                source_url: pr_web_url,
                                                repository: Some(repo_name.to_string()),
                                                tags: vec!["azure_devops:pull_request".to_string()],
                                                relationships: Vec::new(),
                                                created_at,
                                                updated_at: created_at.unwrap_or_else(Utc::now),
                                                synced_at: Utc::now(),
                                                checksum,
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
        }

        Ok(artifacts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_azure_devops_connector_creation() {
        let mut cfg = ConnectorConfig::default();
        cfg.provider = "azure_devops".to_string();
        cfg.api_token = Some("ado_pat_token_12345".to_string());
        cfg.organization = Some("acme".to_string());

        let conn = AzureDevopsConnector::new("ado-test".to_string(), cfg).unwrap();
        assert_eq!(conn.id(), "ado-test");
        assert_eq!(conn.provider(), "azure_devops");
    }
}
