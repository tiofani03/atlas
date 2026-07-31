use crate::config::ConnectorConfig;
use crate::connectors::Connector;
use crate::domain::{ArtifactKind, KnowledgeArtifact};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use serde_json::Value;

pub struct ConfluenceConnector {
    id: String,
    config: ConnectorConfig,
    client: reqwest::Client,
}

impl ConfluenceConnector {
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

    fn strip_html_tags(html: &str) -> String {
        let mut result = String::new();
        let mut in_tag = false;
        for c in html.chars() {
            if c == '<' {
                in_tag = true;
            } else if c == '>' {
                in_tag = false;
                result.push(' ');
            } else if !in_tag {
                result.push(c);
            }
        }
        result
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[async_trait::async_trait]
impl Connector for ConfluenceConnector {
    fn id(&self) -> &str {
        &self.id
    }

    fn provider(&self) -> &str {
        "confluence"
    }

    async fn fetch_modified(&self, _since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        let base_url = self.config.instance_url.trim_end_matches('/');
        let url = format!("{}/wiki/api/v2/pages", base_url);

        let mut next_cursor: Option<String> = None;
        let mut all_pages = Vec::new();

        loop {
            let mut req = self.client.get(&url).query(&[("body-format", "storage"), ("limit", "50")]);
            if let Some(ref c) = next_cursor {
                req = req.query(&[("cursor", c.as_str())]);
            }

            let res = req
                .send()
                .await
                .with_context(|| format!("Failed to fetch pages from Confluence API ({})", url))?;

            if !res.status().is_success() {
                let status = res.status();
                let body = res.text().await.unwrap_or_default();
                anyhow::bail!("Confluence API error ({}): {}", status, body);
            }

            let json: Value = res.json().await?;
            let pages = json
                .get("results")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            if pages.is_empty() {
                break;
            }

            let count = pages.len();
            all_pages.extend(pages);

            let next_link = json.get("_links").and_then(|l| l.get("next")).and_then(|v| v.as_str());
            if let Some(link) = next_link {
                if let Some(cursor_idx) = link.find("cursor=") {
                    let cursor_val = &link[cursor_idx + 7..];
                    let end_idx = cursor_val.find('&').unwrap_or(cursor_val.len());
                    next_cursor = Some(cursor_val[..end_idx].to_string());
                } else {
                    break;
                }
            } else {
                break;
            }

            if count < 50 {
                break;
            }
        }

        let now = Utc::now();
        let mut objects = Vec::new();

        for page in all_pages {
            let page_id = page
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            if page_id.is_empty() {
                continue;
            }

            let title = page
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled Document")
                .to_string();

            let space_id = page
                .get("spaceId")
                .and_then(|v| v.as_str())
                .unwrap_or("general");

            let html_body = page
                .get("body")
                .and_then(|b| b.get("storage"))
                .and_then(|s| s.get("value"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let body = Self::strip_html_tags(html_body);

            let web_url = page
                .get("_links")
                .and_then(|l| l.get("webui"))
                .and_then(|w| w.as_str())
                .map(|link| format!("{}{}", base_url, link))
                .unwrap_or_else(|| format!("{}/wiki/pages/viewpage.action?pageId={}", base_url, page_id));

            let mut tags = vec![format!("space:{}", space_id)];
            tags.push("confluence".to_string());

            let updated_at = now;

            let id = KnowledgeArtifact::generate_id("confluence", &self.config.instance_url, &page_id);
            let checksum = KnowledgeArtifact::compute_checksum(
                &title,
                Some(&format!("Space: {}", space_id)),
                &body,
                &tags,
            );

            objects.push(KnowledgeArtifact {
                id,
                kind: ArtifactKind::Document,
                title,
                summary: Some(format!("Space: {}", space_id)),
                body,
                provider: "confluence".to_string(),
                source_id: page_id,
                source_url: web_url,
                repository: None,
                tags,
                relationships: Vec::new(),
                created_at: None,
                updated_at,
                synced_at: now,
                checksum,
                metadata: page.clone(),
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
