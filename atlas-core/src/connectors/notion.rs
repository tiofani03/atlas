use crate::config::ConnectorConfig;
use crate::connectors::Connector;
use crate::domain::{ArtifactKind, KnowledgeArtifact};
use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::{json, Value};

pub struct NotionConnector {
    id: String,
    config: ConnectorConfig,
    client: reqwest::Client,
}

impl NotionConnector {
    pub fn new(id: String, config: ConnectorConfig) -> Result<Self> {
        let token = config.get_api_token()?;
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", token))?,
        );
        headers.insert("Notion-Version", HeaderValue::from_static("2022-06-28"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self { id, config, client })
    }

    fn extract_page_title(page: &Value) -> String {
        if let Some(props) = page["properties"].as_object() {
            for (_key, prop) in props {
                if prop["type"].as_str() == Some("title") {
                    if let Some(title_arr) = prop["title"].as_array() {
                        let mut text = String::new();
                        for t in title_arr {
                            if let Some(plain) = t["plain_text"].as_str() {
                                text.push_str(plain);
                            }
                        }
                        if !text.is_empty() {
                            return text;
                        }
                    }
                }
            }
        }
        "Untitled Page".to_string()
    }
}

#[async_trait::async_trait]
impl Connector for NotionConnector {
    fn id(&self) -> &str {
        &self.id
    }

    fn provider(&self) -> &str {
        "notion"
    }

    async fn fetch_modified(&self, _since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        let mut artifacts = Vec::new();

        // 1. Search Notion Workspace
        let search_url = "https://api.notion.com/v1/search";
        let search_body = json!({
            "page_size": 100
        });

        if let Ok(res) = self.client.post(search_url).json(&search_body).send().await {
            if res.status().is_success() {
                let json_res: Value = res.json().await.unwrap_or_default();
                if let Some(results) = json_res["results"].as_array() {
                    for item in results {
                        let object_type = item["object"].as_str().unwrap_or("");
                        let item_id = item["id"].as_str().unwrap_or("");
                        if item_id.is_empty() {
                            continue;
                        }

                        let url = item["url"].as_str().unwrap_or("").to_string();
                        let created_at = item["created_time"]
                            .as_str()
                            .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)));
                        let updated_at = item["last_edited_time"]
                            .as_str()
                            .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|d| d.with_timezone(&Utc)))
                            .unwrap_or_else(Utc::now);

                        if object_type == "page" {
                            let title = Self::extract_page_title(item);
                            let canonical_id = KnowledgeArtifact::generate_id("notion", "https://api.notion.com", item_id);
                            let checksum = KnowledgeArtifact::compute_checksum(&title, None, "", &[]);

                            artifacts.push(KnowledgeArtifact {
                                id: canonical_id,
                                kind: ArtifactKind::Document,
                                title,
                                summary: None,
                                body: format!("Notion Page ID: {}", item_id),
                                provider: "notion".to_string(),
                                source_id: item_id.to_string(),
                                source_url: url,
                                repository: None,
                                tags: vec!["notion:page".to_string()],
                                relationships: Vec::new(),
                                created_at,
                                updated_at,
                                synced_at: Utc::now(),
                                checksum,
                                metadata: item.clone(),
                            });
                        } else if object_type == "database" {
                            let title = item["title"]
                                .as_array()
                                .and_then(|arr| arr.first())
                                .and_then(|v| v["plain_text"].as_str())
                                .unwrap_or("Untitled Database")
                                .to_string();

                            let canonical_id = KnowledgeArtifact::generate_id("notion", "https://api.notion.com", item_id);
                            let checksum = KnowledgeArtifact::compute_checksum(&title, None, "", &[]);

                            artifacts.push(KnowledgeArtifact {
                                id: canonical_id,
                                kind: ArtifactKind::Specification,
                                title: format!("Database: {}", title),
                                summary: None,
                                body: format!("Notion Database ID: {}", item_id),
                                provider: "notion".to_string(),
                                source_id: item_id.to_string(),
                                source_url: url,
                                repository: None,
                                tags: vec!["notion:database".to_string()],
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

        Ok(artifacts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notion_connector_creation() {
        let mut cfg = ConnectorConfig::default();
        cfg.provider = "notion".to_string();
        cfg.api_token = Some("secret_notion_token_12345".to_string());

        let conn = NotionConnector::new("notion-test".to_string(), cfg).unwrap();
        assert_eq!(conn.id(), "notion-test");
        assert_eq!(conn.provider(), "notion");
    }
}
