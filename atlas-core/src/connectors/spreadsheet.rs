use crate::config::ConnectorConfig;
use crate::connectors::Connector;
use crate::domain::{ArtifactKind, ArtifactRelationship, KnowledgeArtifact};
use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde_json::Value;

pub struct SpreadsheetConnector {
    id: String,
    config: ConnectorConfig,
    client: reqwest::Client,
}

impl SpreadsheetConnector {
    pub fn new(id: String, config: ConnectorConfig) -> Result<Self> {
        let client = reqwest::Client::builder().build()?;
        Ok(Self { id, config, client })
    }

    fn parse_csv_content(&self, file_path: &str, content: &str, artifacts: &mut Vec<KnowledgeArtifact>) {
        let mut lines = content.lines().peekable();
        if lines.peek().is_none() {
            return;
        }

        let has_header = self.config.has_header_row.unwrap_or(true);
        let mut headers: Vec<String> = Vec::new();
        let max_rows = self.config.max_rows_per_sheet.unwrap_or(10000);

        let file_name = std::path::Path::new(file_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(file_path);

        let workbook_source_id = format!("workbook:{}", file_name);
        let workbook_canonical_id = KnowledgeArtifact::generate_id("spreadsheet", file_path, &workbook_source_id);
        let workbook_checksum = KnowledgeArtifact::compute_checksum(file_name, None, "", &[]);

        artifacts.push(KnowledgeArtifact {
            id: workbook_canonical_id,
            kind: ArtifactKind::Document,
            title: format!("Workbook: {}", file_name),
            summary: None,
            body: format!("Spreadsheet File: {}", file_path),
            provider: "spreadsheet".to_string(),
            source_id: workbook_source_id.clone(),
            source_url: file_path.to_string(),
            repository: None,
            tags: vec!["spreadsheet:workbook".to_string()],
            relationships: Vec::new(),
            created_at: None,
            updated_at: Utc::now(),
            synced_at: Utc::now(),
            checksum: workbook_checksum,
            metadata: Value::Null,
        });

        let mut row_idx = 0;
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let cols: Vec<&str> = trimmed.split(',').map(|s| s.trim_matches('"')).collect();

            if row_idx == 0 && has_header {
                headers = cols.iter().map(|s| s.to_string()).collect();
                row_idx += 1;
                continue;
            }

            if row_idx > max_rows {
                break;
            }

            let mut row_dict = serde_json::Map::new();
            let mut row_body = String::new();

            for (idx, val) in cols.iter().enumerate() {
                let col_name = headers
                    .get(idx)
                    .cloned()
                    .unwrap_or_else(|| format!("Column {}", idx + 1));
                row_dict.insert(col_name.clone(), Value::String(val.to_string()));
                row_body.push_str(&format!("{}: {}\n", col_name, val));
            }

            let row_source_id = format!("{}:row-{}", file_name, row_idx);
            let row_canonical_id = KnowledgeArtifact::generate_id("spreadsheet", file_path, &row_source_id);
            let row_checksum = KnowledgeArtifact::compute_checksum(&format!("Row {}", row_idx), None, &row_body, &[]);

            artifacts.push(KnowledgeArtifact {
                id: row_canonical_id,
                kind: ArtifactKind::Ticket,
                title: format!("{} [Row {}]", file_name, row_idx),
                summary: None,
                body: row_body,
                provider: "spreadsheet".to_string(),
                source_id: row_source_id.clone(),
                source_url: format!("{}#row-{}", file_path, row_idx),
                repository: None,
                tags: vec!["spreadsheet:row".to_string()],
                relationships: vec![ArtifactRelationship {
                    source_id: row_source_id,
                    target_id: workbook_source_id.clone(),
                    relationship_type: "belongs_to".to_string(),
                }],
                created_at: None,
                updated_at: Utc::now(),
                synced_at: Utc::now(),
                checksum: row_checksum,
                metadata: Value::Object(row_dict),
            });

            row_idx += 1;
        }
    }
}

#[async_trait::async_trait]
impl Connector for SpreadsheetConnector {
    fn id(&self) -> &str {
        &self.id
    }

    fn provider(&self) -> &str {
        "spreadsheet"
    }

    async fn verify(&self) -> Result<String> {
        let paths = self.config.get_paths();
        if paths.is_empty() {
            bail!(
                "No spreadsheet paths or URLs configured for connector '{}'",
                self.id
            );
        }

        let mut verified_count = 0;
        for path_str in &paths {
            if path_str.starts_with("http://") || path_str.starts_with("https://") {
                let target_url = transform_google_sheets_url(path_str);
                let mut req = self.client.get(&target_url);
                if let Some(token) = &self.config.api_token {
                    if !token.is_empty() {
                        req = req.bearer_auth(token);
                    }
                }
                let resp = req.send().await.with_context(|| {
                    format!("Failed to fetch spreadsheet from URL: {}", path_str)
                })?;
                if !resp.status().is_success() {
                    bail!(
                        "Spreadsheet fetch for '{}' returned status {}",
                        path_str,
                        resp.status()
                    );
                }
            } else if !std::path::Path::new(path_str).exists() {
                bail!(
                    "Spreadsheet file not found at local path: {}",
                    path_str
                );
            }
            verified_count += 1;
        }

        Ok(format!(
            "Successfully verified {} spreadsheet source(s).",
            verified_count
        ))
    }

    async fn fetch_modified(&self, _since: Option<chrono::DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        let mut artifacts = Vec::new();
        let paths = self.config.get_paths();

        for path_str in &paths {
            if path_str.starts_with("http://") || path_str.starts_with("https://") {
                let target_url = transform_google_sheets_url(path_str);
                let mut req = self.client.get(&target_url);

                if let Some(token) = &self.config.api_token {
                    if !token.is_empty() {
                        req = req.bearer_auth(token);
                    }
                }

                if let Ok(res) = req.send().await {
                    if res.status().is_success() {
                        if let Ok(text) = res.text().await {
                            self.parse_csv_content(path_str, &text, &mut artifacts);
                        }
                    }
                }
            } else if let Ok(content) = std::fs::read_to_string(path_str) {
                self.parse_csv_content(path_str, &content, &mut artifacts);
            }
        }

        Ok(artifacts)
    }
}

/// Automatically converts standard Google Sheets web URLs into direct CSV export URLs.
///
/// Example:
/// `https://docs.google.com/spreadsheets/d/1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms/edit#gid=0`
/// -> `https://docs.google.com/spreadsheets/d/1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms/export?format=csv&gid=0`
pub fn transform_google_sheets_url(url: &str) -> String {
    if !url.contains("docs.google.com/spreadsheets/d/") {
        return url.to_string();
    }

    if let Some(pos) = url.find("/d/") {
        let after_d = &url[pos + 3..];
        let sheet_id = match after_d.find('/') {
            Some(end) => &after_d[..end],
            None => after_d,
        };

        let gid = if let Some(gid_pos) = url.find("gid=") {
            let after_gid = &url[gid_pos + 4..];
            let end_pos = after_gid.find(|c: char| !c.is_ascii_digit()).unwrap_or(after_gid.len());
            Some(&after_gid[..end_pos])
        } else {
            None
        };

        if let Some(gid_val) = gid {
            format!("https://docs.google.com/spreadsheets/d/{}/export?format=csv&gid={}", sheet_id, gid_val)
        } else {
            format!("https://docs.google.com/spreadsheets/d/{}/export?format=csv", sheet_id)
        }
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_spreadsheet_connector_creation() {
        let mut cfg = ConnectorConfig::default();
        cfg.provider = "spreadsheet".to_string();

        let conn = SpreadsheetConnector::new("sheet-test".to_string(), cfg).unwrap();
        assert_eq!(conn.id(), "sheet-test");
        assert_eq!(conn.provider(), "spreadsheet");
    }

    #[tokio::test]
    async fn test_spreadsheet_parse_csv() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("data.csv");
        let csv_data = "Name,Role,Status\nAlice,Engineer,Active\nBob,Designer,Pending";
        let mut f = std::fs::File::create(&file_path).unwrap();
        f.write_all(csv_data.as_bytes()).unwrap();

        let mut cfg = ConnectorConfig::default();
        cfg.provider = "spreadsheet".to_string();
        cfg.path = Some(file_path.to_str().unwrap().to_string());
        cfg.has_header_row = Some(true);

        let conn = SpreadsheetConnector::new("sheet-local".to_string(), cfg).unwrap();
        let artifacts = conn.fetch_modified(None).await.unwrap();

        assert_eq!(artifacts.len(), 3); // 1 workbook + 2 rows
        assert!(artifacts.iter().any(|a| a.title.contains("data.csv [Row 1]")));
        assert!(artifacts.iter().any(|a| a.body.contains("Name: Alice")));
    }

    #[test]
    fn test_transform_google_sheets_url() {
        let input1 = "https://docs.google.com/spreadsheets/d/1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms/edit#gid=0";
        let expected1 = "https://docs.google.com/spreadsheets/d/1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms/export?format=csv&gid=0";
        assert_eq!(transform_google_sheets_url(input1), expected1);

        let input2 = "https://docs.google.com/spreadsheets/d/abc123xyz/edit";
        let expected2 = "https://docs.google.com/spreadsheets/d/abc123xyz/export?format=csv";
        assert_eq!(transform_google_sheets_url(input2), expected2);

        let input3 = "https://example.com/data.csv";
        assert_eq!(transform_google_sheets_url(input3), input3);
    }
}
