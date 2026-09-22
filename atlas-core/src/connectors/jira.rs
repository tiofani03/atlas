use crate::config::ConnectorConfig;
use crate::connectors::Connector;
use crate::domain::{ArtifactKind, ArtifactRelationship, KnowledgeArtifact};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use serde_json::Value;
use std::collections::HashSet;

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
        let mut out = String::new();
        Self::parse_adf_node(val, &mut out, 0);
        let mut clean = String::new();
        for line in out.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                clean.push_str(trimmed);
                clean.push('\n');
            }
        }
        clean.trim().to_string()
    }

    fn parse_adf_node(val: &Value, out: &mut String, depth: usize) {
        if depth > 32 || !val.is_object() {
            return;
        }

        let node_type = val.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match node_type {
            "text" => {
                if let Some(t) = val.get("text").and_then(|v| v.as_str()) {
                    let mut formatted = t.to_string();
                    if let Some(marks) = val.get("marks").and_then(|v| v.as_array()) {
                        for mark in marks {
                            let mark_type = mark.get("type").and_then(|v| v.as_str()).unwrap_or("");
                            if mark_type == "link" {
                                if let Some(href) = mark.get("attrs").and_then(|a| a.get("href")).and_then(|v| v.as_str()) {
                                    if !formatted.contains(href) {
                                        formatted = format!("{} ({})", formatted, href);
                                    }
                                }
                            }
                        }
                    }
                    out.push_str(&formatted);
                }
            }
            "inlineCard" | "blockCard" => {
                if let Some(url) = val.get("attrs").and_then(|a| a.get("url")).and_then(|v| v.as_str()) {
                    out.push_str(&format!(" {}", url));
                }
            }
            "hardBreak" => {
                out.push('\n');
            }
            "heading" => {
                out.push_str("\n\n");
                let level = val.get("attrs").and_then(|a| a.get("level")).and_then(|v| v.as_u64()).unwrap_or(3);
                for _ in 0..level {
                    out.push('#');
                }
                out.push(' ');
                if let Some(content) = val.get("content").and_then(|v| v.as_array()) {
                    for child in content {
                        Self::parse_adf_node(child, out, depth + 1);
                    }
                }
                out.push_str("\n\n");
            }
            "paragraph" => {
                out.push('\n');
                if let Some(content) = val.get("content").and_then(|v| v.as_array()) {
                    for child in content {
                        Self::parse_adf_node(child, out, depth + 1);
                    }
                }
                out.push('\n');
            }
            "listItem" => {
                out.push_str("\n- ");
                if let Some(content) = val.get("content").and_then(|v| v.as_array()) {
                    for child in content {
                        Self::parse_adf_node(child, out, depth + 1);
                    }
                }
            }
            "codeBlock" => {
                out.push_str("\n```\n");
                if let Some(content) = val.get("content").and_then(|v| v.as_array()) {
                    for child in content {
                        Self::parse_adf_node(child, out, depth + 1);
                    }
                }
                out.push_str("\n```\n");
            }
            _ => {
                if let Some(content) = val.get("content").and_then(|v| v.as_array()) {
                    for child in content {
                        Self::parse_adf_node(child, out, depth + 1);
                    }
                }
            }
        }
    }

    pub fn parse_issue_json(issue: &Value, instance_url: &str) -> Option<KnowledgeArtifact> {
        let key = issue
            .get("key")
            .and_then(|v| v.as_str())?
            .trim()
            .to_string();
        if key.is_empty() {
            return None;
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
        let body = match desc_val {
            Some(v) if v.is_object() => Self::extract_adf_text(v),
            Some(v) if v.is_string() => v.as_str().unwrap_or("").to_string(),
            _ => String::new(),
        };

        let now = Utc::now();
        let created_str = fields
            .get("created")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let created_at = DateTime::parse_from_rfc3339(created_str)
            .map(|d| d.with_timezone(&Utc))
            .ok();

        let updated_str = fields
            .get("updated")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let updated_at = DateTime::parse_from_rfc3339(updated_str)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or(now);

        let mut project_key_opt = None;
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
            project_key_opt = Some(project_key.to_string());
        }

        let mut relationships = Vec::new();

        // 1. Parent relationship (Epic -> Story or Story -> Subtask)
        if let Some(parent) = fields.get("parent") {
            if let Some(parent_key) = parent.get("key").and_then(|v| v.as_str()) {
                if !parent_key.trim().is_empty() {
                    relationships.push(ArtifactRelationship {
                        source_id: key.clone(),
                        target_id: parent_key.trim().to_string(),
                        relationship_type: "parent".to_string(),
                    });
                }
            }
        }

        // 2. Subtasks
        if let Some(subtasks) = fields.get("subtasks").and_then(|v| v.as_array()) {
            for sub in subtasks {
                if let Some(sub_key) = sub.get("key").and_then(|v| v.as_str()) {
                    if !sub_key.trim().is_empty() {
                        relationships.push(ArtifactRelationship {
                            source_id: key.clone(),
                            target_id: sub_key.trim().to_string(),
                            relationship_type: "subtask".to_string(),
                        });
                    }
                }
            }
        }

        // 3. Issue links (both outward and inward)
        if let Some(issuelinks) = fields.get("issuelinks").and_then(|v| v.as_array()) {
            for link in issuelinks {
                let outward_type = link
                    .get("type")
                    .and_then(|v| v.get("outward").or_else(|| v.get("name")))
                    .and_then(|v| v.as_str())
                    .unwrap_or("relates_to")
                    .to_string();

                let inward_type = link
                    .get("type")
                    .and_then(|v| v.get("inward").or_else(|| v.get("name")))
                    .and_then(|v| v.as_str())
                    .unwrap_or("is_related_to")
                    .to_string();

                if let Some(out_issue) = link.get("outwardIssue").and_then(|v| v.get("key")) {
                    if let Some(target_key) = out_issue.as_str() {
                        relationships.push(ArtifactRelationship {
                            source_id: key.clone(),
                            target_id: target_key.to_string(),
                            relationship_type: outward_type,
                        });
                    }
                }

                if let Some(in_issue) = link.get("inwardIssue").and_then(|v| v.get("key")) {
                    if let Some(target_key) = in_issue.as_str() {
                        relationships.push(ArtifactRelationship {
                            source_id: key.clone(),
                            target_id: target_key.to_string(),
                            relationship_type: inward_type,
                        });
                    }
                }
            }
        }

        // 4. Epic Link custom field if present (e.g. customfield_XXXXX or "epic")
        if let Some(epic_field) = fields.get("epic") {
            if let Some(epic_key) = epic_field.get("key").and_then(|v| v.as_str()) {
                if !epic_key.trim().is_empty() && !relationships.iter().any(|r| r.target_id == epic_key) {
                    relationships.push(ArtifactRelationship {
                        source_id: key.clone(),
                        target_id: epic_key.trim().to_string(),
                        relationship_type: "parent".to_string(),
                    });
                }
            }
        }

        let web_url = format!(
            "{}/browse/{}",
            instance_url.trim_end_matches('/'),
            key
        );

        let id = KnowledgeArtifact::generate_id("jira", instance_url, &key);
        let checksum = KnowledgeArtifact::compute_checksum(
            &summary,
            Some(status_name),
            &body,
            &tags,
        );

        Some(KnowledgeArtifact {
            id,
            kind: ArtifactKind::Ticket,
            title: summary,
            summary: Some(format!("Status: {}", status_name)),
            body,
            provider: "jira".to_string(),
            source_id: key,
            source_url: web_url,
            repository: project_key_opt,
            tags,
            relationships,
            created_at,
            updated_at,
            synced_at: now,
            checksum,
            metadata: fields.clone(),
        })
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

    async fn verify(&self) -> Result<String> {
        let base_url = self.config.instance_url.trim_end_matches('/');
        let url = format!("{}/rest/api/3/myself", base_url);
        let resp = self.client.get(&url).send().await.context("Failed to connect to Jira API")?;
        if resp.status().is_success() {
            let json: Value = resp.json().await.unwrap_or_default();
            let display_name = json["displayName"].as_str().unwrap_or("authenticated user");
            Ok(format!("Connected to Jira successfully as '{}'.", display_name))
        } else {
            let fallback_url = format!("{}/rest/api/2/myself", base_url);
            if let Ok(f_resp) = self.client.get(&fallback_url).send().await {
                if f_resp.status().is_success() {
                    let json: Value = f_resp.json().await.unwrap_or_default();
                    let display_name = json["displayName"].as_str().unwrap_or("authenticated user");
                    return Ok(format!("Connected to Jira successfully as '{}'.", display_name));
                }
            }
            let status = resp.status();
            let err = resp.text().await.unwrap_or_default();
            anyhow::bail!("Jira verification failed with status {}: {}", status, err);
        }
    }

    async fn fetch_modified(&self, since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
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

        if let Some(since_dt) = since {
            let formatted_date = since_dt.format("%Y-%m-%d %H:%M").to_string();
            if !jql.is_empty() {
                jql.push_str(" AND ");
            }
            jql.push_str(&format!("updated >= \"{}\"", formatted_date));
        }

        if !jql.is_empty() {
            jql.push_str(" ORDER BY updated ASC");
        }

        let mut start_at = 0;
        let max_results = 50;
        let mut all_issues = Vec::new();
        let mut seen_keys = HashSet::new();
        let mut next_page_token: Option<String> = None;

        loop {
            let base_url = self.config.instance_url.trim_end_matches('/');
            let url = format!("{}/rest/api/3/search/jql", base_url);
            let mut req = self
                .client
                .get(&url)
                .query(&[("maxResults", max_results.to_string())])
                // Jira's /search/jql endpoint returns only the issue id when
                // fields are omitted. The parser needs key plus these fields
                // to build a complete local artifact.
                .query(&[(
                    "fields",
                    "summary,status,description,created,updated,labels,project,parent,subtasks,issuelinks",
                )]);

            if !jql.is_empty() {
                req = req.query(&[("jql", &jql)]);
            }

            req = req.query(&[("startAt", start_at.to_string())]);

            if let Some(token) = &next_page_token {
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

        let mut objects = Vec::new();
        for issue in all_issues {
            if let Some(artifact) = Self::parse_issue_json(&issue, &self.config.instance_url) {
                objects.push(artifact);
            }
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
