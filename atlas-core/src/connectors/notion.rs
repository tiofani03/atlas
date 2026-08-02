use crate::config::ConnectorConfig;
use crate::connectors::Connector;
use crate::domain::{ArtifactKind, ArtifactRelationship, KnowledgeArtifact};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::{json, Value};
use std::collections::HashSet;

pub struct NotionConnector {
    id: String,
    pub config: ConnectorConfig,
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

    /// Extract plain text from a Notion rich text array
    pub fn extract_rich_text(rich_text_arr: Option<&Vec<Value>>) -> String {
        let mut out = String::new();
        if let Some(arr) = rich_text_arr {
            for item in arr {
                if let Some(plain) = item["plain_text"].as_str() {
                    out.push_str(plain);
                }
            }
        }
        out
    }

    /// Extract page title from page object properties or icon/title
    pub fn extract_page_title(page: &Value) -> String {
        if let Some(props) = page["properties"].as_object() {
            for (_key, prop) in props {
                if prop["type"].as_str() == Some("title") {
                    if let Some(title_arr) = prop["title"].as_array() {
                        let title = Self::extract_rich_text(Some(title_arr));
                        if !title.is_empty() {
                            return title;
                        }
                    }
                }
            }
        }
        "Untitled Page".to_string()
    }

    /// Convert a Notion block AST node into Markdown representation
    pub fn format_block_to_markdown(block: &Value) -> String {
        let block_type = block["type"].as_str().unwrap_or("");
        let content_obj = &block[block_type];

        match block_type {
            "paragraph" => {
                let text = Self::extract_rich_text(content_obj["rich_text"].as_array());
                if text.is_empty() {
                    String::new()
                } else {
                    format!("{}\n\n", text)
                }
            }
            "heading_1" => {
                let text = Self::extract_rich_text(content_obj["rich_text"].as_array());
                format!("# {}\n\n", text)
            }
            "heading_2" => {
                let text = Self::extract_rich_text(content_obj["rich_text"].as_array());
                format!("## {}\n\n", text)
            }
            "heading_3" => {
                let text = Self::extract_rich_text(content_obj["rich_text"].as_array());
                format!("### {}\n\n", text)
            }
            "bulleted_list_item" => {
                let text = Self::extract_rich_text(content_obj["rich_text"].as_array());
                format!("* {}\n", text)
            }
            "numbered_list_item" => {
                let text = Self::extract_rich_text(content_obj["rich_text"].as_array());
                format!("1. {}\n", text)
            }
            "to_do" => {
                let text = Self::extract_rich_text(content_obj["rich_text"].as_array());
                let checked = content_obj["checked"].as_bool().unwrap_or(false);
                let mark = if checked { "[x]" } else { "[ ]" };
                format!("- {} {}\n", mark, text)
            }
            "toggle" => {
                let text = Self::extract_rich_text(content_obj["rich_text"].as_array());
                format!("<details><summary>{}</summary></details>\n\n", text)
            }
            "code" => {
                let text = Self::extract_rich_text(content_obj["rich_text"].as_array());
                let lang = content_obj["language"].as_str().unwrap_or("text");
                format!("```{}\n{}\n```\n\n", lang, text)
            }
            "quote" => {
                let text = Self::extract_rich_text(content_obj["rich_text"].as_array());
                format!("> {}\n\n", text)
            }
            "callout" => {
                let text = Self::extract_rich_text(content_obj["rich_text"].as_array());
                format!("> [!NOTE]\n> {}\n\n", text.replace('\n', "\n> "))
            }
            "divider" => "---\n\n".to_string(),
            "bookmark" => {
                let url = content_obj["url"].as_str().unwrap_or("");
                format!("[Bookmark: {}]({})\n\n", url, url)
            }
            "child_page" => {
                let title = content_obj["title"].as_str().unwrap_or("Subpage");
                format!("📄 Subpage: {}\n\n", title)
            }
            "child_database" => {
                let title = content_obj["title"].as_str().unwrap_or("Database");
                format!("📊 Database: {}\n\n", title)
            }
            _ => String::new(),
        }
    }

    /// Fetch all block children for a given page or block ID (up to max_blocks)
    pub async fn fetch_page_blocks(&self, block_id: &str, max_blocks: usize) -> Result<String> {
        let mut markdown_body = String::new();
        let mut start_cursor: Option<String> = None;
        let mut total_blocks = 0;

        loop {
            let mut url = format!("https://api.notion.com/v1/blocks/{}/children?page_size=100", block_id);
            if let Some(ref cursor) = start_cursor {
                url.push_str(&format!("&start_cursor={}", cursor));
            }

            let resp = self.client.get(&url).send().await?;
            if !resp.status().is_success() {
                break;
            }

            let data: Value = resp.json().await.unwrap_or_default();
            if let Some(results) = data["results"].as_array() {
                for block in results {
                    let formatted = Self::format_block_to_markdown(block);
                    markdown_body.push_str(&formatted);
                    total_blocks += 1;
                    if total_blocks >= max_blocks {
                        break;
                    }
                }
            }

            if total_blocks >= max_blocks || data["has_more"].as_bool() != Some(true) {
                break;
            }

            start_cursor = data["next_cursor"].as_str().map(|s| s.to_string());
            if start_cursor.is_none() {
                break;
            }
        }

        Ok(markdown_body.trim().to_string())
    }

    /// Extract key-value properties from page properties into tags and body summary
    pub fn extract_page_properties(page: &Value) -> (Vec<String>, String) {
        let mut tags = Vec::new();
        let mut props_summary = String::new();

        if let Some(props) = page["properties"].as_object() {
            for (key, prop) in props {
                let p_type = prop["type"].as_str().unwrap_or("");
                match p_type {
                    "select" => {
                        if let Some(name) = prop["select"]["name"].as_str() {
                            tags.push(format!("{}:{}", key.to_lowercase().replace(' ', "_"), name));
                            props_summary.push_str(&format!("- **{}**: {}\n", key, name));
                        }
                    }
                    "multi_select" => {
                        if let Some(options) = prop["multi_select"].as_array() {
                            let names: Vec<&str> = options.iter().filter_map(|o| o["name"].as_str()).collect();
                            for name in &names {
                                tags.push(format!("{}:{}", key.to_lowercase().replace(' ', "_"), name));
                            }
                            if !names.is_empty() {
                                props_summary.push_str(&format!("- **{}**: {}\n", key, names.join(", ")));
                            }
                        }
                    }
                    "status" => {
                        if let Some(name) = prop["status"]["name"].as_str() {
                            tags.push(format!("status:{}", name));
                            props_summary.push_str(&format!("- **Status**: {}\n", name));
                        }
                    }
                    "date" => {
                        if let Some(start) = prop["date"]["start"].as_str() {
                            props_summary.push_str(&format!("- **{}**: {}\n", key, start));
                        }
                    }
                    "people" => {
                        if let Some(people) = prop["people"].as_array() {
                            let names: Vec<&str> = people.iter().filter_map(|p| p["name"].as_str()).collect();
                            if !names.is_empty() {
                                props_summary.push_str(&format!("- **{}**: {}\n", key, names.join(", ")));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        (tags, props_summary)
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

    async fn verify(&self) -> Result<String> {
        let url = "https://api.notion.com/v1/users/me";
        let res = self.client.get(url).send().await.context("Failed to connect to Notion API")?;
        if res.status().is_success() {
            let user_info: Value = res.json().await.unwrap_or_default();
            let bot_name = user_info["name"].as_str().unwrap_or("Notion Integration");
            Ok(format!("Connected to Notion successfully as '{}'.", bot_name))
        } else {
            let status = res.status();
            let err_text = res.text().await.unwrap_or_default();
            bail!("Notion verification failed with status {}: {}", status, err_text);
        }
    }

    async fn fetch_modified(&self, since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        let mut artifacts = Vec::new();
        let mut seen_ids = HashSet::new();

        // 1. Search Notion Workspace with Cursor Pagination and Sort by last_edited_time
        let search_url = "https://api.notion.com/v1/search";
        let mut start_cursor: Option<String> = None;

        'search_loop: loop {
            let mut search_body = json!({
                "page_size": 100,
                "sort": {
                    "direction": "descending",
                    "timestamp": "last_edited_time"
                }
            });

            if let Some(ref cursor) = start_cursor {
                search_body["start_cursor"] = json!(cursor);
            }

            let resp = match self.client.post(search_url).json(&search_body).send().await {
                Ok(r) if r.status().is_success() => r,
                Ok(r) => {
                    tracing::warn!("Notion search returned non-success status: {}", r.status());
                    break;
                }
                Err(e) => {
                    tracing::warn!("Notion search request error: {}", e);
                    break;
                }
            };

            let json_res: Value = resp.json().await.unwrap_or_default();
            let results = match json_res["results"].as_array() {
                Some(r) if !r.is_empty() => r,
                _ => break,
            };

            for item in results {
                let object_type = item["object"].as_str().unwrap_or("");
                let item_id = item["id"].as_str().unwrap_or("");
                if item_id.is_empty() || !seen_ids.insert(item_id.to_string()) {
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

                // Incremental watermark check
                if let Some(since_time) = since {
                    if updated_at < since_time {
                        // Results are sorted descending by last_edited_time; we can break search pagination early
                        break 'search_loop;
                    }
                }

                if object_type == "page" {
                    let title = Self::extract_page_title(item);
                    let (mut tags, props_summary) = Self::extract_page_properties(item);
                    tags.push("notion:page".to_string());

                    // Retrieve actual body content from block children
                    let block_content = self.fetch_page_blocks(item_id, 300).await.unwrap_or_default();

                    let full_body = if props_summary.is_empty() {
                        block_content.clone()
                    } else if block_content.is_empty() {
                        format!("### Properties\n{}", props_summary)
                    } else {
                        format!("### Properties\n{}\n\n### Content\n{}", props_summary, block_content)
                    };

                    let canonical_id = KnowledgeArtifact::generate_id("notion", "https://api.notion.com", item_id);
                    let summary = if !block_content.is_empty() {
                        Some(block_content.lines().next().unwrap_or(&title).chars().take(200).collect())
                    } else {
                        None
                    };

                    let checksum = KnowledgeArtifact::compute_checksum(&title, summary.as_deref(), &full_body, &tags);

                    let mut relationships = Vec::new();
                    if let Some(parent) = item["parent"].as_object() {
                        if let Some(parent_page_id) = parent.get("page_id").and_then(|v| v.as_str()) {
                            relationships.push(ArtifactRelationship {
                                source_id: item_id.to_string(),
                                target_id: parent_page_id.to_string(),
                                relationship_type: "child_of".to_string(),
                            });
                        } else if let Some(parent_db_id) = parent.get("database_id").and_then(|v| v.as_str()) {
                            relationships.push(ArtifactRelationship {
                                source_id: item_id.to_string(),
                                target_id: parent_db_id.to_string(),
                                relationship_type: "database_entry".to_string(),
                            });
                        }
                    }

                    artifacts.push(KnowledgeArtifact {
                        id: canonical_id,
                        kind: ArtifactKind::Document,
                        title,
                        summary,
                        body: full_body,
                        provider: "notion".to_string(),
                        source_id: item_id.to_string(),
                        source_url: url,
                        repository: None,
                        tags,
                        relationships,
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

                    let desc = Self::extract_rich_text(item["description"].as_array());
                    let canonical_id = KnowledgeArtifact::generate_id("notion", "https://api.notion.com", item_id);
                    let checksum = KnowledgeArtifact::compute_checksum(&title, None, &desc, &[]);

                    artifacts.push(KnowledgeArtifact {
                        id: canonical_id,
                        kind: ArtifactKind::Specification,
                        title: format!("Database: {}", title),
                        summary: if desc.is_empty() { None } else { Some(desc.clone()) },
                        body: format!("## Notion Database: {}\n\n{}", title, desc),
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

            if json_res["has_more"].as_bool() != Some(true) {
                break;
            }

            start_cursor = json_res["next_cursor"].as_str().map(|s| s.to_string());
            if start_cursor.is_none() {
                break;
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

    #[test]
    fn test_notion_extract_page_title() {
        let page_json = serde_json::json!({
            "properties": {
                "Name": {
                    "type": "title",
                    "title": [
                        { "plain_text": "Engineering Architecture Plan" }
                    ]
                }
            }
        });
        assert_eq!(
            NotionConnector::extract_page_title(&page_json),
            "Engineering Architecture Plan"
        );
    }

    #[test]
    fn test_notion_format_block_to_markdown() {
        let block_heading = serde_json::json!({
            "type": "heading_1",
            "heading_1": {
                "rich_text": [
                    { "plain_text": "System Design" }
                ]
            }
        });
        assert_eq!(
            NotionConnector::format_block_to_markdown(&block_heading),
            "# System Design\n\n"
        );

        let block_code = serde_json::json!({
            "type": "code",
            "code": {
                "language": "rust",
                "rich_text": [
                    { "plain_text": "fn main() { println!(\"Atlas\"); }" }
                ]
            }
        });
        assert_eq!(
            NotionConnector::format_block_to_markdown(&block_code),
            "```rust\nfn main() { println!(\"Atlas\"); }\n```\n\n"
        );

        let block_todo = serde_json::json!({
            "type": "to_do",
            "to_do": {
                "checked": true,
                "rich_text": [
                    { "plain_text": "Complete audit" }
                ]
            }
        });
        assert_eq!(
            NotionConnector::format_block_to_markdown(&block_todo),
            "- [x] Complete audit\n"
        );
    }
}
