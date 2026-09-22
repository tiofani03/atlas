use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectFigmaConfig {
    pub aliases: HashMap<String, String>,
    pub file_key: Option<String>,
    pub node_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct LocalFigmaTarget {
    #[serde(default)]
    file_key: Option<String>,
    #[serde(default)]
    node_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct LocalFigmaConfigFile {
    #[serde(default)]
    aliases: HashMap<String, String>,
    #[serde(default)]
    figma: Option<LocalFigmaTarget>,
}

/// A Figma file candidate from an indexed catalog or a local cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FigmaFileCandidate {
    pub file_key: String,
    pub name: String,
    pub body: String,
    pub node_id: Option<String>,
}

/// Extracts clean Figma `file_key` and optional `node_id` from a raw input string.
/// The input can be a full Figma URL (`/design/`, `/file/`, `/proto/`),
/// a key with a node suffix (e.g. `key:1:2` or `key#1:2`), or a bare key.
pub fn parse_figma_target(input: &str) -> (String, Option<String>) {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return (String::new(), None);
    }

    // Check if input is a URL or contains figma.com
    if trimmed.contains("figma.com/") {
        let after_host = match trimmed.split_once("figma.com/") {
            Some((_, rest)) => rest,
            None => trimmed,
        };

        let (path_part, query_part) = match after_host.split_once('?') {
            Some((p, q)) => (p, Some(q)),
            None => (after_host, None),
        };

        let segments: Vec<&str> = path_part
            .split('/')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        let mut file_key = String::new();
        for (i, seg) in segments.iter().enumerate() {
            if (*seg == "design" || *seg == "file" || *seg == "proto") && i + 1 < segments.len() {
                file_key = segments[i + 1].to_string();
                break;
            }
        }

        if file_key.is_empty() && !segments.is_empty() {
            file_key = segments[0].to_string();
        }

        let mut node_id = None;
        if let Some(query) = query_part {
            for param in query.split('&') {
                if let Some((k, v)) = param.split_once('=') {
                    if k == "node-id" || k == "node_id" {
                        node_id = Some(normalize_node_id(v));
                        break;
                    }
                }
            }
        }

        return (file_key, node_id);
    }

    // Check if input contains node separator e.g. key:1:2 or key#1-2
    if let Some((k, n)) = trimmed.split_once('#') {
        return (k.trim().to_string(), Some(normalize_node_id(n.trim())));
    }

    // If it contains a single colon between key and node (or multiple colons for node id)
    if let Some(first_colon) = trimmed.find(':') {
        let key = trimmed[..first_colon].trim().to_string();
        let node = trimmed[first_colon + 1..].trim().to_string();
        if !key.is_empty() && !node.is_empty() {
            return (key, Some(normalize_node_id(&node)));
        }
    }

    (trimmed.to_string(), None)
}

/// Normalize Figma node IDs (e.g. "6236-33268" -> "6236:33268")
fn normalize_node_id(node: &str) -> String {
    let unescaped = node.replace("%3A", ":").replace("%3a", ":");
    if unescaped.contains(':') {
        unescaped
    } else if let Some((prefix, suffix)) = unescaped.split_once('-') {
        if prefix.chars().all(|c| c.is_ascii_digit()) && suffix.chars().all(|c| c.is_ascii_digit()) {
            format!("{}:{}", prefix, suffix)
        } else {
            unescaped
        }
    } else {
        unescaped
    }
}

/// Load project-level Figma configuration from `.atlas/figma.toml` or `figma.toml`.
///
/// The preferred format is:
///
/// ```toml
/// [figma]
/// file_key = "clone-key"
/// node_id = "6236-33268"
///
/// [aliases]
/// "PROJ-123" = "clone-key"
/// ```
///
/// The aliases table remains supported for backwards compatibility.
pub fn load_local_project_config<P: AsRef<Path>>(dir: P) -> ProjectFigmaConfig {
    let mut current = dir.as_ref().to_path_buf();
    for _ in 0..5 {
        let candidate1 = current.join(".atlas").join("figma.toml");
        if candidate1.exists() {
            if let Ok(content) = fs::read_to_string(&candidate1) {
                if let Ok(parsed) = toml::from_str::<LocalFigmaConfigFile>(&content) {
                    return project_config_from_file(parsed);
                }
            }
        }
        let candidate2 = current.join("figma.toml");
        if candidate2.exists() {
            if let Ok(content) = fs::read_to_string(&candidate2) {
                if let Ok(parsed) = toml::from_str::<LocalFigmaConfigFile>(&content) {
                    return project_config_from_file(parsed);
                }
            }
        }
        if !current.pop() {
            break;
        }
    }
    ProjectFigmaConfig::default()
}

fn project_config_from_file(parsed: LocalFigmaConfigFile) -> ProjectFigmaConfig {
    let (file_key, node_id) = parsed
        .figma
        .map(|figma| {
            (
                figma.file_key.filter(|key| !key.trim().is_empty()),
                figma.node_id.map(|node| normalize_node_id(node.trim())),
            )
        })
        .unwrap_or((None, None));

    ProjectFigmaConfig {
        aliases: parsed.aliases,
        file_key,
        node_id,
    }
}

/// Load only project aliases. Kept as a compatibility helper for existing callers.
pub fn load_local_project_aliases<P: AsRef<Path>>(dir: P) -> HashMap<String, String> {
    load_local_project_config(dir).aliases
}

/// Default cache directory used by `mcp-figma` (`~/.mcp-figma/cache`)
pub fn default_figma_cache_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|h| h.join(".mcp-figma").join("cache"))
}

/// Scan `cache_dir` for a cached Figma file corresponding to `ticket_code` (e.g. `PROJ-123` or `PROJ 123`).
pub fn find_cached_figma_file_for_ticket_in_dir(ticket_code: &str, cache_dir: &Path) -> Option<String> {
    if !cache_dir.exists() || !cache_dir.is_dir() {
        return None;
    }

    let cleaned_ticket = ticket_code.trim_matches(|c| c == '[' || c == ']' || c == ' ' || c == '"');

    if cleaned_ticket.is_empty() {
        return None;
    }

    // Split words, e.g. "PROJ-123" -> ["PROJ", "123"]
    let parts = ticket_parts(cleaned_ticket);

    if parts.is_empty() {
        return None;
    }

    let entries = fs::read_dir(cache_dir).ok()?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };

        // We only want file_<key>_<timestamp>.json, skipping file_nodes_
        if !file_name.starts_with("file_") || file_name.starts_with("file_nodes_") || !file_name.ends_with(".json") {
            continue;
        }

        // Extract key: file_<key>_<timestamp>.json
        let remainder = &file_name[5..file_name.len() - 5];
        let file_key = match remainder.rfind('_') {
            Some(idx) => &remainder[..idx],
            None => remainder,
        };

        // Prefer the actual Figma file name. Falling back to raw content keeps
        // compatibility with older cache entries that were not valid JSON.
        if let Ok(content) = fs::read_to_string(&path) {
            let searchable_name = serde_json::from_str::<Value>(&content)
                .ok()
                .and_then(|value| {
                    value
                        .get("name")
                        .and_then(Value::as_str)
                        .or_else(|| value.pointer("/document/name").and_then(Value::as_str))
                        .map(str::to_string)
                })
                .unwrap_or(content);
            let matches_all = ticket_parts_match(&searchable_name, &parts);
            if matches_all {
                return Some(file_key.to_string());
            }
        }
    }

    None
}

/// Find a Figma file in indexed metadata using its ticket/name convention.
pub fn find_figma_file_for_ticket_in_candidates(
    ticket_code: &str,
    candidates: &[FigmaFileCandidate],
) -> Option<(String, Option<String>)> {
    let cleaned_ticket = ticket_code
        .trim_matches(|c| c == '[' || c == ']' || c == ' ' || c == '"');
    let parts = ticket_parts(cleaned_ticket);
    if parts.is_empty() {
        return None;
    }

    let mut matches: Vec<&FigmaFileCandidate> = candidates
        .iter()
        .filter(|candidate| {
            ticket_parts_match(&candidate.name, &parts)
                || ticket_parts_match(&candidate.body, &parts)
        })
        .collect();
    matches.sort_by(|a, b| a.file_key.cmp(&b.file_key));

    matches.first().map(|candidate| {
        (
            candidate.file_key.clone(),
            candidate.node_id.clone(),
        )
    })
}

fn ticket_parts(input: &str) -> Vec<String> {
    input
        .to_uppercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

fn ticket_parts_match(text: &str, parts: &[String]) -> bool {
    let upper = text.to_uppercase();
    let normalized_parts = ticket_parts(&upper);
    parts.iter().all(|part| {
        normalized_parts.iter().any(|candidate| candidate == part)
            || upper.contains(part)
    })
}

fn looks_like_ticket_reference(input: &str) -> bool {
    let parts = ticket_parts(input);
    parts.len() >= 2 && parts.last().is_some_and(|part| part.chars().all(|c| c.is_ascii_digit()))
}

/// Resolve a raw Figma target (URL, key, or ticket code) to its target `file_key` and `node_id`.
pub fn resolve_figma_target(
    raw_input: &str,
    server_aliases: &HashMap<String, String>,
    project_dir: Option<&Path>,
    override_cache_dir: Option<&Path>,
) -> (String, Option<String>) {
    resolve_figma_target_with_candidates(
        raw_input,
        server_aliases,
        project_dir,
        override_cache_dir,
        &[],
    )
}

/// Resolve a Figma target using explicit aliases, project config, indexed
/// catalog candidates, and the local MCP Figma cache in that order.
pub fn resolve_figma_target_with_candidates(
    raw_input: &str,
    server_aliases: &HashMap<String, String>,
    project_dir: Option<&Path>,
    override_cache_dir: Option<&Path>,
    candidates: &[FigmaFileCandidate],
) -> (String, Option<String>) {
    let (key, node_id) = parse_figma_target(raw_input);

    // 1. Check direct match in server aliases
    if let Some(target) = server_aliases.get(&key) {
        let (resolved_key, resolved_node) = parse_figma_target(target);
        return (resolved_key, node_id.or(resolved_node));
    }

    // 2. Check case-insensitive match in server aliases
    for (k, v) in server_aliases {
        if k.eq_ignore_ascii_case(&key) {
            let (resolved_key, resolved_node) = parse_figma_target(v);
            return (resolved_key, node_id.or(resolved_node));
        }
    }

    // 3. Check project-level aliases (.atlas/figma.toml)
    let project_config = match project_dir {
        Some(p) => load_local_project_config(p),
        None => {
            if let Ok(cwd) = std::env::current_dir() {
                load_local_project_config(cwd)
            } else {
                ProjectFigmaConfig::default()
            }
        }
    };

    if let Some(target) = project_config.aliases.get(&key) {
        let (resolved_key, resolved_node) = parse_figma_target(target);
        return (resolved_key, node_id.or(resolved_node));
    }
    for (k, v) in &project_config.aliases {
        if k.eq_ignore_ascii_case(&key) {
            let (resolved_key, resolved_node) = parse_figma_target(v);
            return (resolved_key, node_id.or(resolved_node));
        }
    }

    // A project-level default clone is intentionally only applied to ticket
    // references; it must not replace an explicit Figma key or URL.
    if looks_like_ticket_reference(&key) {
        if let Some(file_key) = project_config.file_key.as_deref() {
            return (
                file_key.to_string(),
                node_id.or_else(|| project_config.node_id.clone()),
            );
        }
    }

    if let Some((candidate_key, candidate_node)) =
        find_figma_file_for_ticket_in_candidates(&key, candidates)
    {
        return (candidate_key, node_id.or(candidate_node));
    }

    // Cache auto-detection (e.g. user passes ticket ID "PROJ-123" or "[PROJ 123]")
    let cache_dir = override_cache_dir
        .map(PathBuf::from)
        .or_else(default_figma_cache_dir);

    if let Some(ref cdir) = cache_dir {
        if let Some(cached_key) = find_cached_figma_file_for_ticket_in_dir(&key, cdir) {
            return (cached_key, node_id);
        }
    }

    (key, node_id)
}
