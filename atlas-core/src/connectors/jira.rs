use crate::config::ConnectorConfig;
use crate::connectors::Connector;
use crate::domain::{KnowledgeObject, ObjectType, Relationship, SourceInfo};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use serde_json::Value;

pub struct JiraConnector {
    id: String,
    config: ConnectorConfig,
    client: reqwest::Client,
}

impl JiraConnector {
    pub fn new(id: String, config: ConnectorConfig) -> Result<Self> {
        let token = config.get_api_token()?;
        let auth_raw = format!("{}:{}", config.email, token);
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

    fn extract_adf_text(val: &Value) -> String {
        let mut text = String::new();
        if let Some(t) = val.get("text").and_then(|v| v.as_str()) {
            text.push_str(t);
        }
        if let Some(content) = val.get("content").and_then(|v| v.as_array()) {
            for child in content {
                text.push_str(&Self::extract_adf_text(child));
                text.push(' ');
            }
        }
        text
    }
}

#[async_trait::async_trait]
impl Connector for JiraConnector {
    fn id(&self) -> &str {
        &self.id
    }

    fn provider(&self) -> &str {
        "jira"
    }

    async fn fetch_modified(&self, since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeObject>> {
        let mut jql = String::new();

        if !self.config.projects.is_empty() {
            let projects_str = self
                .config
                .projects
                .iter()
                .map(|p| format!("\"{}\"", p))
                .collect::<Vec<_>>()
                .join(", ");
            jql.push_str(&format!("project IN ({})", projects_str));
        }

        if let Some(dt) = since {
            if !jql.is_empty() {
                jql.push_str(" AND ");
            }
            jql.push_str(&format!("updated >= \"{}\"", dt.format("%Y-%m-%d %H:%M")));
        }

        if jql.is_empty() {
            jql = "ORDER BY updated DESC".to_string();
        } else {
            jql.push_str(" ORDER BY updated DESC");
        }

        let url = format!(
            "{}/rest/api/3/search/jql",
            self.config.instance_url.trim_end_matches('/')
        );

        let mut seen_keys = std::collections::HashSet::new();
        let mut next_page_token: Option<String> = None;
        let mut start_at = 0;
        let max_results = 100;
        let mut all_issues = Vec::new();

        loop {
            let mut payload = serde_json::json!({
                "jql": jql,
                "maxResults": max_results,
                "fields": ["*all"]
            });

            if let Some(ref token) = next_page_token {
                payload["nextPageToken"] = serde_json::json!(token);
            } else if start_at > 0 {
                payload["startAt"] = serde_json::json!(start_at);
            }

            let mut req = self.client.post(&url).json(&payload);
            if let Some(ref token) = next_page_token {
                req = req.query(&[("nextPageToken", token.as_str())]);
            }

            let res = req
                .send()
                .await
                .with_context(|| format!("Failed to fetch issues from Jira API ({})", url))?;

            if !res.status().is_success() {
                let status = res.status();
                let body = res.text().await.unwrap_or_default();
                anyhow::bail!("Jira API error ({}): {}", status, body);
            }

            let json: Value = res.json().await?;
            let issues = json
                .get("issues")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            if issues.is_empty() {
                break;
            }

            let mut new_in_batch = 0;
            for issue in &issues {
                if let Some(key) = issue.get("key").and_then(|v| v.as_str()) {
                    if seen_keys.insert(key.to_string()) {
                        new_in_batch += 1;
                    }
                }
            }

            if new_in_batch == 0 {
                tracing::info!("Jira connector [{}] reached repeated issues, stopping pagination.", self.id);
                break;
            }

            let count = issues.len();
            all_issues.extend(issues);

            tracing::info!(
                "Jira connector [{}] fetched batch of {} issues (accumulated unique: {})",
                self.id,
                count,
                all_issues.len()
            );

            let is_last = json.get("isLast").and_then(|v| v.as_bool()).unwrap_or(false);
            let token_opt = json.get("nextPageToken").and_then(|v| v.as_str()).map(|s| s.to_string());
            let total = json.get("total").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

            if is_last || count < max_results {
                break;
            }

            if let Some(new_token) = token_opt {
                if Some(&new_token) == next_page_token.as_ref() {
                    break;
                }
                next_page_token = Some(new_token);
            } else if total > 0 {
                start_at += count;
                if start_at >= total {
                    break;
                }
            } else {
                start_at += count;
            }
        }

        let now = Utc::now();
        let mut objects = Vec::new();

        for issue in all_issues {
            let key = issue
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if key.is_empty() {
                continue;
            }

            let fields = issue.get("fields").unwrap_or(&Value::Null);

            let summary = fields
                .get("summary")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled Ticket")
                .to_string();

            let status_name = fields
                .get("status")
                .and_then(|v| v.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");

            let desc_val = fields.get("description");
            let content = match desc_val {
                Some(v) if v.is_object() => Self::extract_adf_text(v),
                Some(v) if v.is_string() => v.as_str().unwrap_or("").to_string(),
                _ => String::new(),
            };

            let updated_str = fields
                .get("updated")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let updated_at = DateTime::parse_from_rfc3339(updated_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or(now);

            let mut tags = Vec::new();
            if let Some(labels) = fields.get("labels").and_then(|v| v.as_array()) {
                for label in labels {
                    if let Some(l) = label.as_str() {
                        tags.push(l.to_string());
                    }
                }
            }
            if let Some(project_key) = fields
                .get("project")
                .and_then(|v| v.get("key"))
                .and_then(|v| v.as_str())
            {
                tags.push(format!("project:{}", project_key));
            }

            let mut relationships = Vec::new();
            if let Some(issuelinks) = fields.get("issuelinks").and_then(|v| v.as_array()) {
                for link in issuelinks {
                    let rel_type = link
                        .get("type")
                        .and_then(|v| v.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("relates_to")
                        .to_string();

                    if let Some(out_issue) = link.get("outwardIssue").and_then(|v| v.get("key")) {
                        if let Some(target_key) = out_issue.as_str() {
                            relationships.push(Relationship {
                                target_id: KnowledgeObject::generate_id(
                                    "jira",
                                    &self.config.instance_url,
                                    target_key,
                                ),
                                relationship_type: rel_type.clone(),
                            });
                        }
                    }
                }
            }

            let web_url = format!(
                "{}/browse/{}",
                self.config.instance_url.trim_end_matches('/'),
                key
            );

            let id = KnowledgeObject::generate_id("jira", &self.config.instance_url, &key);
            let checksum = KnowledgeObject::compute_checksum(
                &summary,
                Some(status_name),
                &content,
                &tags,
            );

            objects.push(KnowledgeObject {
                id,
                object_type: ObjectType::Ticket,
                title: summary,
                summary: Some(format!("Status: {}", status_name)),
                content,
                tags,
                relationships,
                source: SourceInfo {
                    provider: "jira".to_string(),
                    instance_url: self.config.instance_url.clone(),
                    original_id: key,
                    web_url,
                },
                source_metadata: fields.clone(),
                updated_at,
                synced_at: now,
                checksum,
            });
        }

        Ok(objects)
    }
}

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
