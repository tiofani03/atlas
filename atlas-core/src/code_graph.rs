use crate::domain::{ArtifactKind, ArtifactRelationship, KnowledgeArtifact};
use chrono::Utc;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Structural symbol & import graph extractor for Rust, TypeScript/JS, Python, and Go source trees.
pub struct CodeGraphExtractor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedSymbol {
    pub name: String,
    pub symbol_kind: String,
    pub file_path: String,
    pub line: usize,
}

impl CodeGraphExtractor {
    /// Scan a local directory and emit `Component` artifacts for top-level symbols
    /// plus `references` edges for detected cross-file imports.
    pub fn extract_from_dir(root: &Path, repository: Option<&str>) -> Vec<KnowledgeArtifact> {
        let mut artifacts = Vec::new();
        let now = Utc::now();

        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !is_ignored(e.path()))
        {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
                continue;
            };
            if !matches!(ext, "rs" | "ts" | "tsx" | "js" | "py" | "go") {
                continue;
            }
            let Ok(content) = fs::read_to_string(path) else {
                continue;
            };
            let rel_path = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            let symbols = extract_symbols(&content, ext, &rel_path);
            let imports = extract_imports(&content, ext);

            for sym in symbols {
                let source_id = format!("{}#{}", sym.file_path, sym.name);
                let id = format!("code:{}", source_id);
                let mut hasher = Sha256::new();
                hasher.update(source_id.as_bytes());
                hasher.update(sym.line.to_le_bytes());
                let checksum = format!("{:x}", hasher.finalize());

                let relationships = imports
                    .iter()
                    .map(|imp| ArtifactRelationship {
                        source_id: source_id.clone(),
                        target_id: imp.clone(),
                        relationship_type: "references".to_string(),
                    })
                    .collect();

                artifacts.push(KnowledgeArtifact {
                    id,
                    kind: ArtifactKind::Component,
                    title: format!("{} ({})", sym.name, sym.symbol_kind),
                    summary: Some(format!("{} defined at {}:{}", sym.symbol_kind, sym.file_path, sym.line)),
                    body: format!(
                        "Symbol: `{}`\nKind: `{}`\nFile: `{}:{}`\nImports: {}",
                        sym.name,
                        sym.symbol_kind,
                        sym.file_path,
                        sym.line,
                        if imports.is_empty() { "none".to_string() } else { imports.join(", ") }
                    ),
                    provider: "code_graph".to_string(),
                    source_id,
                    source_url: String::new(),
                    repository: repository.map(|r| r.to_string()),
                    tags: vec!["code-graph".to_string(), ext.to_string(), sym.symbol_kind],
                    relationships,
                    created_at: Some(now),
                    updated_at: now,
                    synced_at: now,
                    checksum,
                    metadata: serde_json::json!({
                        "file": sym.file_path,
                        "line": sym.line,
                        "language": ext,
                    }),
                });
            }
        }

        artifacts
    }
}

fn is_ignored(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    matches!(name, "node_modules" | "target" | ".git" | "dist" | "__pycache__")
}

pub fn extract_symbols(content: &str, ext: &str, file_path: &str) -> Vec<ExtractedSymbol> {
    let pattern = match ext {
        "rs" => r"(?m)^(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(fn|struct|enum|trait|mod)\s+([A-Za-z_][A-Za-z0-9_]*)",
        "ts" | "tsx" | "js" => r"(?m)^(?:export\s+)?(?:async\s+)?(function|class|interface|type)\s+([A-Za-z_][A-Za-z0-9_]*)",
        "py" => r"(?m)^(?:async\s+)?(def|class)\s+([A-Za-z_][A-Za-z0-9_]*)",
        "go" => r"(?m)^(func|type)\s+([A-Za-z_][A-Za-z0-9_]*)",
        _ => return Vec::new(),
    };

    let Ok(re) = Regex::new(pattern) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        if let Some(caps) = re.captures(line) {
            if let (Some(kind), Some(name)) = (caps.get(1), caps.get(2)) {
                out.push(ExtractedSymbol {
                    name: name.as_str().to_string(),
                    symbol_kind: kind.as_str().to_string(),
                    file_path: file_path.to_string(),
                    line: idx + 1,
                });
            }
        }
    }
    out
}

pub fn extract_imports(content: &str, ext: &str) -> Vec<String> {
    let pattern = match ext {
        "rs" => r"(?m)^use\s+([A-Za-z0-9_:]+)",
        "ts" | "tsx" | "js" => r#"(?m)^import\s+.*?\s+from\s+['"]([^'"]+)['"]"#,
        "py" => r"(?m)^(?:from|import)\s+([A-Za-z0-9_.]+)",
        "go" => r#"(?m)^\s*"([^"]+)""#,
        _ => return Vec::new(),
    };

    let Ok(re) = Regex::new(pattern) else {
        return Vec::new();
    };
    re.captures_iter(content)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .take(32)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_extract_symbols_and_imports_rust() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("lib.rs");
        fs::write(
            &src,
            "use crate::storage::Storage;\n\npub struct Engine {}\npub async fn run_sync() {}\n",
        )
        .unwrap();

        let artifacts = CodeGraphExtractor::extract_from_dir(dir.path(), Some("acme/atlas"));
        assert_eq!(artifacts.len(), 2);
        assert!(artifacts.iter().any(|a| a.title == "Engine (struct)"));
        assert!(artifacts.iter().any(|a| a.title == "run_sync (fn)"));
        assert_eq!(artifacts[0].relationships[0].target_id, "crate::storage::Storage");
    }
}
