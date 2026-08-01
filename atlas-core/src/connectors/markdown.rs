use crate::domain::{ArtifactKind, ArtifactRelationship, KnowledgeArtifact};
use anyhow::Result;
use chrono::{DateTime, TimeZone, Utc};
use pulldown_cmark::{Event, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Classification category for Markdown engineering documents
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DocumentClassification {
    Readme,
    Adr,
    Rfc,
    Changelog,
    Contributing,
    ApiDocumentation,
    Guide,
    GeneralDoc,
}

impl std::fmt::Display for DocumentClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentClassification::Readme => write!(f, "readme"),
            DocumentClassification::Adr => write!(f, "adr"),
            DocumentClassification::Rfc => write!(f, "rfc"),
            DocumentClassification::Changelog => write!(f, "changelog"),
            DocumentClassification::Contributing => write!(f, "contributing"),
            DocumentClassification::ApiDocumentation => write!(f, "api_documentation"),
            DocumentClassification::Guide => write!(f, "guide"),
            DocumentClassification::GeneralDoc => write!(f, "general_doc"),
        }
    }
}

/// Information extracted from a single Markdown file during single-pass parsing
#[derive(Debug, Default)]
pub struct ParsedMarkdown {
    pub frontmatter: HashMap<String, String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub clean_body: String,
    pub headings: Vec<HeadingInfo>,
    pub code_blocks: Vec<CodeBlockInfo>,
    pub outgoing_links: Vec<String>,
    pub classification: Option<DocumentClassification>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadingInfo {
    pub level: u32,
    pub text: String,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlockInfo {
    pub language: Option<String>,
    pub code: String,
}

/// Local Markdown connector for Atlas Knowledge Platform
#[derive(Debug, Clone)]
pub struct MarkdownConnector {
    id: String,
    root_paths: Vec<PathBuf>,
    glob_patterns: Vec<String>,
}

impl MarkdownConnector {
    pub fn new(id: impl Into<String>, root_path: impl AsRef<Path>) -> Self {
        let path_str = root_path.as_ref().to_string_lossy();
        let paths: Vec<PathBuf> = path_str
            .split(',')
            .map(|s| PathBuf::from(s.trim()))
            .filter(|p| !p.as_os_str().is_empty())
            .collect();

        Self {
            id: id.into(),
            root_paths: if paths.is_empty() {
                vec![root_path.as_ref().to_path_buf()]
            } else {
                paths
            },
            glob_patterns: vec![
                "*.md".to_string(),
                "*.markdown".to_string(),
                "*.mdx".to_string(),
            ],
        }
    }

    pub fn with_paths(mut self, paths: Vec<PathBuf>) -> Self {
        if !paths.is_empty() {
            self.root_paths = paths;
        }
        self
    }

    pub fn with_glob_patterns(mut self, patterns: Vec<String>) -> Self {
        if !patterns.is_empty() {
            self.glob_patterns = patterns;
        }
        self
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn provider(&self) -> &str {
        "markdown"
    }

    /// Classify a document based on relative path and content structural signals
    pub fn classify_document(rel_path: &Path, content: &str, frontmatter: &HashMap<String, String>) -> DocumentClassification {
        let file_name = rel_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_uppercase();

        let path_str = rel_path.to_string_lossy().to_lowercase();

        // 1. Filename & Path Heuristics
        if file_name.starts_with("README") {
            return DocumentClassification::Readme;
        }
        if file_name.starts_with("CHANGELOG") || file_name.starts_with("HISTORY") || file_name.starts_with("RELEASES") {
            return DocumentClassification::Changelog;
        }
        if file_name.starts_with("CONTRIBUTING") {
            return DocumentClassification::Contributing;
        }
        if file_name.starts_with("ADR") || path_str.contains("/adr/") || path_str.contains("/adrs/") {
            return DocumentClassification::Adr;
        }
        if file_name.starts_with("RFC") || path_str.contains("/rfc/") || path_str.contains("/rfcs/") {
            return DocumentClassification::Rfc;
        }
        if path_str.contains("/api/") || path_str.contains("api-ref") || path_str.contains("openapi") {
            return DocumentClassification::ApiDocumentation;
        }

        // 2. Frontmatter Explicit Type
        if let Some(doc_type) = frontmatter.get("type").or_else(|| frontmatter.get("kind")) {
            match doc_type.to_lowercase().as_str() {
                "adr" => return DocumentClassification::Adr,
                "rfc" => return DocumentClassification::Rfc,
                "guide" | "tutorial" => return DocumentClassification::Guide,
                "api" => return DocumentClassification::ApiDocumentation,
                "changelog" => return DocumentClassification::Changelog,
                _ => {}
            }
        }

        // 3. Structural Content Heuristics
        if content.contains("## Context") && content.contains("## Decision") && (content.contains("## Consequences") || content.contains("## Status")) {
            return DocumentClassification::Adr;
        }

        if path_str.contains("/guides/") || path_str.contains("/tutorials/") || path_str.contains("/docs/") {
            return DocumentClassification::Guide;
        }

        DocumentClassification::GeneralDoc
    }

    /// Single-pass extraction of Markdown frontmatter, AST headers, code blocks, links, and text
    pub fn parse_markdown(raw_text: &str, rel_path: &Path) -> ParsedMarkdown {
        let mut parsed = ParsedMarkdown::default();

        // Step 1: Extract Frontmatter (YAML block enclosed by ---)
        let (content_body, frontmatter) = Self::extract_frontmatter(raw_text);
        parsed.frontmatter = frontmatter;

        if let Some(fm_title) = parsed.frontmatter.get("title") {
            parsed.title = Some(fm_title.trim().to_string());
        }
        if let Some(fm_summary) = parsed.frontmatter.get("summary").or_else(|| parsed.frontmatter.get("description")) {
            parsed.summary = Some(fm_summary.trim().to_string());
        }

        // Step 2: Single-pass AST parsing using pulldown-cmark
        let parser = pulldown_cmark::Parser::new(content_body);
        let mut in_heading: Option<u32> = None;
        let mut current_heading_text = String::new();

        let mut in_code_block = false;
        let mut current_code_lang: Option<String> = None;
        let mut current_code_text = String::new();

        let mut first_paragraph: Option<String> = None;
        let mut in_first_paragraph = false;

        for event in parser {
            match event {
                // Heading Start / End
                Event::Start(Tag::Heading { level, .. }) => {
                    in_heading = Some(level as u32);
                    current_heading_text.clear();
                }
                Event::End(TagEnd::Heading(_)) => {
                    if let Some(level) = in_heading.take() {
                        let text = current_heading_text.trim().to_string();
                        if !text.is_empty() {
                            let slug = Self::slugify(&text);
                            // If title was not set by frontmatter, set first H1 as title
                            if parsed.title.is_none() && level == 1 {
                                parsed.title = Some(text.clone());
                            }
                            parsed.headings.push(HeadingInfo { level, text, slug });
                        }
                    }
                }

                // Code Block Start / End
                Event::Start(Tag::CodeBlock(kind)) => {
                    in_code_block = true;
                    current_code_text.clear();
                    current_code_lang = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                            let lang_str = lang.trim().to_string();
                            if lang_str.is_empty() {
                                None
                            } else {
                                Some(lang_str)
                            }
                        }
                        pulldown_cmark::CodeBlockKind::Indented => None,
                    };
                }
                Event::End(TagEnd::CodeBlock) => {
                    if in_code_block {
                        in_code_block = false;
                        parsed.code_blocks.push(CodeBlockInfo {
                            language: current_code_lang.take(),
                            code: current_code_text.clone(),
                        });
                        current_code_text.clear();
                    }
                }

                // Links
                Event::Start(Tag::Link { dest_url, .. }) => {
                    let dest = dest_url.trim().to_string();
                    if !dest.is_empty() && !parsed.outgoing_links.contains(&dest) {
                        parsed.outgoing_links.push(dest);
                    }
                }

                // Paragraph for summary fallback
                Event::Start(Tag::Paragraph) => {
                    if first_paragraph.is_none() && parsed.summary.is_none() {
                        in_first_paragraph = true;
                    }
                }
                Event::End(TagEnd::Paragraph) => {
                    if in_first_paragraph {
                        in_first_paragraph = false;
                    }
                }

                // Text Events
                Event::Text(text) | Event::Code(text) => {
                    if let Some(_) = in_heading {
                        current_heading_text.push_str(&text);
                    } else if in_code_block {
                        current_code_text.push_str(&text);
                    } else if in_first_paragraph {
                        let para = first_paragraph.get_or_insert_with(String::new);
                        para.push_str(&text);
                    }
                }

                _ => {}
            }
        }

        // Set summary fallback if not present
        if parsed.summary.is_none() {
            if let Some(para) = first_paragraph {
                let trimmed = para.trim();
                if !trimmed.is_empty() {
                    let summary_text = if trimmed.chars().count() > 200 {
                        format!("{}...", trimmed.chars().take(197).collect::<String>())
                    } else {
                        trimmed.to_string()
                    };
                    parsed.summary = Some(summary_text);
                }
            }
        }

        // Title Fallback to File Stem
        if parsed.title.is_none() {
            let file_stem = rel_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled Document");
            parsed.title = Some(file_stem.to_string());
        }

        parsed.clean_body = content_body.trim().to_string();
        parsed.classification = Some(Self::classify_document(rel_path, content_body, &parsed.frontmatter));

        parsed
    }

    /// Extract key-value frontmatter from Markdown text
    fn extract_frontmatter(text: &str) -> (&str, HashMap<String, String>) {
        let mut map = HashMap::new();
        let trimmed = text.trim_start();
        if !trimmed.starts_with("---") {
            return (text, map);
        }

        let rest = &trimmed[3..];
        if let Some(end_idx) = rest.find("\n---") {
            let fm_block = &rest[..end_idx];
            let body = &rest[end_idx + 4..];

            for line in fm_block.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some(colon_idx) = line.find(':') {
                    let key = line[..colon_idx].trim().to_string();
                    let val = line[colon_idx + 1..].trim().trim_matches('"').trim_matches('\'').to_string();
                    if !key.is_empty() {
                        map.insert(key, val);
                    }
                }
            }
            (body, map)
        } else {
            (text, map)
        }
    }

    /// Simple slugify helper for anchor link IDs
    fn slugify(text: &str) -> String {
        text.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }

    /// Scan directory and yield canonical KnowledgeArtifact records
    pub fn scan_directory(&self, since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        let mut artifacts = Vec::new();
        let mut valid_path_count = 0;

        for root_path in &self.root_paths {
            if !root_path.exists() {
                tracing::warn!("Markdown root path does not exist: {:?}", root_path);
                continue;
            }
            valid_path_count += 1;

            let root_canonical = fs::canonicalize(root_path)
                .unwrap_or_else(|_| root_path.clone());

            for entry in WalkDir::new(root_path)
                .into_iter()
                .filter_entry(|e| {
                    let name = e.file_name().to_string_lossy();
                    // Skip hidden dirs, target, node_modules, build outputs
                    !(e.file_type().is_dir() && (name.starts_with('.') || name == "target" || name == "node_modules" || name == "dist" || name == "build"))
                })
            {
                let entry = match entry {
                    Ok(e) => e,
                    Err(err) => {
                        tracing::warn!("Error traversing directory entry: {:?}", err);
                        continue;
                    }
                };

                if !entry.file_type().is_file() {
                    continue;
                }

                let path = entry.path();
                let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                if extension != "md" && extension != "markdown" && extension != "mdx" {
                    continue;
                }

                // Check modification timestamp
                let metadata = match fs::metadata(path) {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let mtime: DateTime<Utc> = metadata
                    .modified()
                    .ok()
                    .and_then(|t| {
                        let duration = t.duration_since(std::time::UNIX_EPOCH).ok()?;
                        Utc.timestamp_opt(duration.as_secs() as i64, duration.subsec_nanos()).single()
                    })
                    .unwrap_or_else(Utc::now);

                if let Some(since_time) = since {
                    if mtime <= since_time {
                        continue;
                    }
                }

                // Calculate relative path
                let rel_path = path
                    .strip_prefix(root_path)
                    .unwrap_or(path);

                let raw_text = match fs::read_to_string(path) {
                    Ok(t) => t,
                    Err(err) => {
                        tracing::warn!("Failed to read markdown file {:?}: {:?}", path, err);
                        continue;
                    }
                };

                let parsed = Self::parse_markdown(&raw_text, rel_path);

                // Build relationships from outgoing links
                let mut relationships = Vec::new();
                for link in &parsed.outgoing_links {
                    // Ignore external HTTP links in artifact relationships
                    if !link.starts_with("http://") && !link.starts_with("https://") && !link.starts_with("mailto:") {
                        let target_id = KnowledgeArtifact::generate_id("markdown", &root_canonical.to_string_lossy(), link);
                        relationships.push(ArtifactRelationship {
                            source_id: String::new(), // Populated by Artifact ID
                            target_id,
                            relationship_type: "references".to_string(),
                        });
                    }
                }

                // Tags
                let mut tags = Vec::new();
                if let Some(ref class) = parsed.classification {
                    tags.push(format!("doc_kind:{}", class));
                }
                if let Some(fm_tags) = parsed.frontmatter.get("tags") {
                    for t in fm_tags.split(',').map(|s| s.trim()) {
                        if !t.is_empty() {
                            tags.push(t.to_string());
                        }
                    }
                }

                let source_id_str = rel_path.to_string_lossy().to_string();

                // Compute Artifact ID
                let artifact_id = KnowledgeArtifact::generate_id(
                    "markdown",
                    &root_canonical.to_string_lossy(),
                    &source_id_str,
                );

                // Fix relationships source_id
                for rel in &mut relationships {
                    rel.source_id = artifact_id.clone();
                }

                let checksum = KnowledgeArtifact::compute_checksum(
                    parsed.title.as_deref().unwrap_or(""),
                    parsed.summary.as_deref(),
                    &parsed.clean_body,
                    &tags,
                );

                let kind = match parsed.classification {
                    Some(DocumentClassification::Adr) | Some(DocumentClassification::Rfc) => ArtifactKind::Design,
                    Some(DocumentClassification::ApiDocumentation) => ArtifactKind::Specification,
                    _ => ArtifactKind::Document,
                };

                let metadata_json = serde_json::json!({
                    "classification": parsed.classification.map(|c| c.to_string()),
                    "frontmatter": parsed.frontmatter,
                    "headings": parsed.headings,
                    "code_blocks": parsed.code_blocks,
                    "outgoing_links": parsed.outgoing_links,
                });

                let artifact = KnowledgeArtifact {
                    id: artifact_id,
                    kind,
                    title: parsed.title.unwrap_or_else(|| source_id_str.clone()),
                    summary: parsed.summary,
                    body: parsed.clean_body,
                    provider: "markdown".to_string(),
                    source_id: source_id_str,
                    source_url: format!("file://{}", path.to_string_lossy()),
                    repository: None,
                    tags,
                    relationships,
                    created_at: Some(mtime),
                    updated_at: mtime,
                    synced_at: Utc::now(),
                    checksum,
                    metadata: metadata_json,
                };

                artifacts.push(artifact);
            }
        }

        if valid_path_count == 0 && !self.root_paths.is_empty() {
            anyhow::bail!("None of the specified Markdown root paths exist: {:?}", self.root_paths);
        }

        Ok(artifacts)
    }
}

#[async_trait::async_trait]
impl crate::connectors::Connector for MarkdownConnector {
    fn id(&self) -> &str {
        &self.id
    }

    fn provider(&self) -> &str {
        "markdown"
    }

    async fn fetch_modified(&self, since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        self.scan_directory(since)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_parse_markdown_frontmatter_and_headers() {
        let raw = r#"---
title: System Architecture ADR
type: adr
tags: architecture, security
---

# Architecture Overview

This document describes the Atlas system architecture.

## Context

Modern engineering context is fragmented.

```rust
fn main() {
    println!("Hello Atlas!");
}
```

[Getting Started](./getting-started.md)
"#;

        let parsed = MarkdownConnector::parse_markdown(raw, Path::new("docs/adr/001-arch.md"));

        assert_eq!(parsed.title, Some("System Architecture ADR".to_string()));
        assert_eq!(parsed.classification, Some(DocumentClassification::Adr));
        assert_eq!(parsed.frontmatter.get("type"), Some(&"adr".to_string()));
        assert_eq!(parsed.headings.len(), 2);
        assert_eq!(parsed.headings[0].text, "Architecture Overview");
        assert_eq!(parsed.headings[0].slug, "architecture-overview");
        assert_eq!(parsed.headings[1].text, "Context");
        assert_eq!(parsed.code_blocks.len(), 1);
        assert_eq!(parsed.code_blocks[0].language, Some("rust".to_string()));
        assert_eq!(parsed.outgoing_links, vec!["./getting-started.md".to_string()]);
    }

    #[test]
    fn test_scan_directory_and_classify() -> Result<()> {
        let dir = tempdir()?;
        let docs_dir = dir.path().join("docs");
        fs::create_dir_all(&docs_dir)?;

        let readme_path = docs_dir.join("README.md");
        fs::write(
            &readme_path,
            "# Atlas Project\n\nWelcome to the Atlas Engineering Platform.",
        )?;

        let adr_path = docs_dir.join("ADR-001.md");
        fs::write(
            &adr_path,
            "# ADR 001\n\n## Context\nContext here.\n\n## Decision\nDecision here.\n\n## Consequences\nConsequences here.",
        )?;

        let connector = MarkdownConnector::new("local-docs", &docs_dir);
        let artifacts = connector.scan_directory(None)?;

        assert_eq!(artifacts.len(), 2);

        let readme_art = artifacts.iter().find(|a| a.source_id == "README.md").unwrap();
        assert_eq!(readme_art.title, "Atlas Project");
        assert!(readme_art.tags.contains(&"doc_kind:readme".to_string()));

        let adr_art = artifacts.iter().find(|a| a.source_id == "ADR-001.md").unwrap();
        assert_eq!(adr_art.kind, ArtifactKind::Design);
        assert!(adr_art.tags.contains(&"doc_kind:adr".to_string()));

        Ok(())
    }

    #[test]
    fn test_scan_multiple_directories() -> Result<()> {
        let dir1 = tempdir()?;
        let dir2 = tempdir()?;

        let docs1 = dir1.path().join("docs1");
        let docs2 = dir2.path().join("docs2");

        fs::create_dir_all(&docs1)?;
        fs::create_dir_all(&docs2)?;

        fs::write(docs1.join("doc1.md"), "# Document 1\n\nContent 1")?;
        fs::write(docs2.join("doc2.md"), "# Document 2\n\nContent 2")?;

        let multi_path_str = format!("{}, {}", docs1.to_string_lossy(), docs2.to_string_lossy());
        let connector = MarkdownConnector::new("multi-docs", multi_path_str);
        let artifacts = connector.scan_directory(None)?;

        assert_eq!(artifacts.len(), 2);
        let titles: Vec<&str> = artifacts.iter().map(|a| a.title.as_str()).collect();
        assert!(titles.contains(&"Document 1"));
        assert!(titles.contains(&"Document 2"));

        Ok(())
    }
}
