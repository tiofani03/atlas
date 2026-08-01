use crate::connectors::Connector;
use crate::domain::{ArtifactKind, KnowledgeArtifact};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};
use walkdir::WalkDir;

/// Metadata for a local Git repository
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalGitRepository {
    /// Deterministic ID computed from the canonical root path
    pub id: String,
    /// Repository display name (usually the folder name)
    pub name: String,
    /// Absolute canonical path on local disk
    pub root_path: PathBuf,
    /// Current checked-out branch name (None if detached HEAD)
    pub current_branch: Option<String>,
    /// Full 40-character commit SHA at HEAD
    pub head_commit: String,
    /// Resolved default branch (e.g., "main" or "master")
    pub default_branch: String,
    /// Git origin URL if configured in .git/config
    pub git_origin_url: Option<String>,
    /// Timestamp of last successful indexing run
    pub last_indexed_at: Option<DateTime<Utc>>,
}

impl LocalGitRepository {
    /// Inspect a local path and extract canonical repository metadata
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let raw_path = path.as_ref();
        let canonical_path = raw_path
            .canonicalize()
            .with_context(|| format!("Failed to canonicalize repository path: {:?}", raw_path))?;

        let git_dir = canonical_path.join(".git");
        if !git_dir.exists() {
            return Err(anyhow!("Not a valid Git repository (missing .git directory): {:?}", canonical_path));
        }

        let name = canonical_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed_repo")
            .to_string();

        let id = Self::generate_repo_id(&canonical_path);
        let (current_branch, head_commit) = Self::read_head(&git_dir)?;
        let default_branch = Self::read_default_branch(&git_dir, current_branch.as_deref());
        let git_origin_url = Self::read_origin_url(&git_dir);

        Ok(Self {
            id,
            name,
            root_path: canonical_path,
            current_branch,
            head_commit,
            default_branch,
            git_origin_url,
            last_indexed_at: None,
        })
    }

    /// Compute deterministic repo ID sha256(canonical_root_path)
    fn generate_repo_id(canonical_path: &Path) -> String {
        let mut hasher = Sha256::new();
        hasher.update(canonical_path.to_string_lossy().as_bytes());
        format!("repo_{:x}", hasher.finalize())[..20].to_string()
    }

    /// Resolve HEAD commit SHA and branch name from .git/HEAD
    fn read_head(git_dir: &Path) -> Result<(Option<String>, String)> {
        let head_file = git_dir.join("HEAD");
        if !head_file.is_file() {
            return Err(anyhow!("Missing HEAD file in .git directory: {:?}", git_dir));
        }

        let head_content = fs::read_to_string(&head_file)
            .with_context(|| format!("Failed to read HEAD file: {:?}", head_file))?
            .trim()
            .to_string();

        if let Some(ref_path) = head_content.strip_prefix("ref: ") {
            let branch_name = ref_path
                .strip_prefix("refs/heads/")
                .unwrap_or(ref_path)
                .to_string();

            let ref_file = git_dir.join(ref_path);
            let commit_sha = if ref_file.is_file() {
                fs::read_to_string(&ref_file)?.trim().to_string()
            } else {
                // Packed refs fallback
                Self::read_packed_ref(git_dir, ref_path)?
                    .ok_or_else(|| anyhow!("Could not resolve commit SHA for branch: {}", branch_name))?
            };

            Ok((Some(branch_name), commit_sha))
        } else {
            // Detached HEAD: content is the commit SHA directly
            Ok((None, head_content))
        }
    }

    /// Read packed-refs fallback for branch resolution
    fn read_packed_ref(git_dir: &Path, ref_path: &str) -> Result<Option<String>> {
        let packed_refs = git_dir.join("packed-refs");
        if !packed_refs.is_file() {
            return Ok(None);
        }

        let content = fs::read_to_string(&packed_refs)?;
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.starts_with('^') || line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1] == ref_path {
                return Ok(Some(parts[0].to_string()));
            }
        }
        Ok(None)
    }

    /// Determine default branch name (origin/HEAD or fallback to main/master)
    fn read_default_branch(git_dir: &Path, current_branch: Option<&str>) -> String {
        let origin_head = git_dir.join("refs/remotes/origin/HEAD");
        if origin_head.is_file() {
            if let Ok(content) = fs::read_to_string(&origin_head) {
                let content = content.trim();
                if let Some(branch) = content.strip_prefix("ref: refs/remotes/origin/") {
                    return branch.to_string();
                }
            }
        }

        if let Some(branch) = current_branch {
            if branch == "main" || branch == "master" {
                return branch.to_string();
            }
        }

        if git_dir.join("refs/heads/main").exists() {
            "main".to_string()
        } else if git_dir.join("refs/heads/master").exists() {
            "master".to_string()
        } else {
            current_branch.unwrap_or("main").to_string()
        }
    }

    /// Extract origin URL from .git/config if present
    fn read_origin_url(git_dir: &Path) -> Option<String> {
        let config_path = git_dir.join("config");
        if !config_path.is_file() {
            return None;
        }

        let content = fs::read_to_string(&config_path).ok()?;
        let mut in_origin_section = false;

        for line in content.lines() {
            let line = line.trim();
            if line.eq_ignore_ascii_case("[remote \"origin\"]") {
                in_origin_section = true;
                continue;
            }
            if line.starts_with('[') {
                in_origin_section = false;
            }
            if in_origin_section && line.starts_with("url =") {
                return line.split('=').nth(1).map(|u| u.trim().to_string());
            }
        }
        None
    }
}

/// Registry for managing multi-repository discovery and scanning
#[derive(Debug, Clone, Default)]
pub struct RepositoryRegistry {
    pub repositories: Vec<LocalGitRepository>,
}

impl RepositoryRegistry {
    pub fn new() -> Self {
        Self { repositories: Vec::new() }
    }

    /// Add a repository explicitly by path
    pub fn add_repository(&mut self, path: impl AsRef<Path>) -> Result<&LocalGitRepository> {
        let repo = LocalGitRepository::open(path)?;
        if !self.repositories.iter().any(|r| r.id == repo.id) {
            self.repositories.push(repo);
        }
        Ok(self.repositories.last().unwrap())
    }

    /// Scan a workspace directory (depth = 1) and auto-register valid Git repositories
    pub fn scan_workspace(&mut self, workspace_path: impl AsRef<Path>) -> Result<Vec<String>> {
        let raw_path = workspace_path.as_ref();
        let canonical_path = raw_path.canonicalize()?;

        let mut added_ids = Vec::new();

        // Check if workspace target itself is a repository
        if canonical_path.join(".git").exists() {
            if let Ok(repo) = LocalGitRepository::open(&canonical_path) {
                let id = repo.id.clone();
                if !self.repositories.iter().any(|r| r.id == id) {
                    self.repositories.push(repo);
                    added_ids.push(id);
                }
            }
            return Ok(added_ids);
        }

        // Scan depth=1 child directories
        let entries = fs::read_dir(&canonical_path)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join(".git").exists() {
                match LocalGitRepository::open(&path) {
                    Ok(repo) => {
                        let id = repo.id.clone();
                        if !self.repositories.iter().any(|r| r.id == id) {
                            self.repositories.push(repo);
                            added_ids.push(id);
                        }
                    }
                    Err(e) => {
                        warn!("Skipping invalid repository candidate at {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(added_ids)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LineBlameInfo {
    pub line: usize,
    pub commit_sha: String,
    pub author: String,
    pub date: String,
}

/// Local Git Connector implementation
pub struct LocalGitConnector {
    pub id: String,
    pub registry: RepositoryRegistry,
}

impl LocalGitConnector {
    pub fn new(id: impl Into<String>, registry: RepositoryRegistry) -> Self {
        Self {
            id: id.into(),
            registry,
        }
    }

    pub fn new_from_config(id: impl Into<String>, config: &crate::config::ConnectorConfig) -> Result<Self> {
        let mut registry = RepositoryRegistry::new();
        let paths = config.get_paths();
        if !paths.is_empty() {
            for p in paths {
                let _ = registry.scan_workspace(&p).or_else(|_| registry.add_repository(&p).map(|_| vec![]));
            }
        } else if let Some(ref p) = config.path {
            let _ = registry.scan_workspace(p).or_else(|_| registry.add_repository(p).map(|_| vec![]));
        } else {
            let _ = registry.scan_workspace(".").or_else(|_| registry.add_repository(".").map(|_| vec![]));
        }

        Ok(Self {
            id: id.into(),
            registry,
        })
    }

    /// Extract line-by-line Git blame attribution for a tracked file
    pub fn read_line_blame(repo_root: &Path, relative_path: &str) -> Vec<LineBlameInfo> {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(repo_root)
            .arg("blame")
            .arg("--porcelain")
            .arg(relative_path)
            .output();

        let Ok(output) = output else {
            return Vec::new();
        };

        if !output.status.success() {
            return Vec::new();
        }

        let content = String::from_utf8_lossy(&output.stdout);
        let mut line_blames = Vec::new();
        let mut current_sha = String::new();
        let mut current_author = String::new();
        let mut current_date = String::new();
        let mut current_line_no = 1;

        for line in content.lines() {
            if line.len() >= 40 && line.chars().take(40).all(|c| c.is_ascii_hexdigit()) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if !parts.is_empty() {
                    current_sha = parts[0].to_string();
                    if parts.len() >= 3 {
                        if let Ok(l) = parts[2].parse::<usize>() {
                            current_line_no = l;
                        }
                    }
                }
            } else if let Some(author) = line.strip_prefix("author ") {
                current_author = author.to_string();
            } else if let Some(time_str) = line.strip_prefix("author-time ") {
                if let Ok(ts) = time_str.trim().parse::<i64>() {
                    if let Some(dt) = DateTime::from_timestamp(ts, 0) {
                        current_date = dt.to_rfc3339();
                    }
                }
            } else if line.starts_with('\t') {
                line_blames.push(LineBlameInfo {
                    line: current_line_no,
                    commit_sha: current_sha.clone(),
                    author: current_author.clone(),
                    date: current_date.clone(),
                });
            }
        }

        line_blames
    }

    /// Enumerate commit history artifacts (kind: Commit) with parent graph relationships
    pub fn fetch_commit_artifacts(
        &self,
        repo: &LocalGitRepository,
        since: Option<DateTime<Utc>>,
    ) -> Vec<KnowledgeArtifact> {
        let mut cmd = std::process::Command::new("git");
        cmd.arg("-C")
            .arg(&repo.root_path)
            .arg("log")
            .arg("-n")
            .arg("50")
            .arg("--pretty=format:COMMIT_START%n%H%n%P%n%an%n%ae%n%at%n%s%n%b%nCOMMIT_END");

        if let Some(since_time) = since {
            cmd.arg(format!("--since={}", since_time.to_rfc3339()));
        }

        let output = match cmd.output() {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
            _ => return Vec::new(),
        };

        let mut commits = Vec::new();
        let blocks = output.split("COMMIT_START\n");

        for block in blocks {
            let block = block.trim();
            if block.is_empty() {
                continue;
            }

            let lines: Vec<&str> = block.lines().collect();
            if lines.len() < 6 {
                continue;
            }

            let sha = lines[0].to_string();
            let parent_shas: Vec<String> = lines[1]
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();
            let author_name = lines[2].to_string();
            let author_email = lines[3].to_string();
            let timestamp_sec = lines[4].parse::<i64>().unwrap_or(0);
            let subject = lines[5].to_string();

            let body_lines = if lines.len() > 6 {
                let end_idx = lines.iter().position(|l| *l == "COMMIT_END").unwrap_or(lines.len());
                lines[6..end_idx].join("\n")
            } else {
                String::new()
            };

            let committed_at = DateTime::from_timestamp(timestamp_sec, 0).unwrap_or_else(Utc::now);

            let source_id = format!("{}:commit:{}", repo.id, sha);
            let artifact_id = KnowledgeArtifact::generate_id(self.provider(), &repo.id, &source_id);

            let mut relationships = Vec::new();
            for parent_sha in &parent_shas {
                if parent_sha != &sha {
                    let parent_source_id = format!("{}:commit:{}", repo.id, parent_sha);
                    let parent_artifact_id = KnowledgeArtifact::generate_id(self.provider(), &repo.id, &parent_source_id);
                    relationships.push(crate::domain::ArtifactRelationship {
                        source_id: artifact_id.clone(),
                        target_id: parent_artifact_id,
                        relationship_type: "parent_commit".to_string(),
                    });
                }
            }

            let checksum = KnowledgeArtifact::compute_checksum(&subject, Some(&author_name), &body_lines, &[]);

            let commit_meta = serde_json::json!({
                "repo_id": repo.id,
                "repo_name": repo.name,
                "commit_sha": sha,
                "parent_shas": parent_shas,
                "author_name": author_name,
                "author_email": author_email,
                "committed_at": committed_at.to_rfc3339(),
            });

            let artifact = KnowledgeArtifact {
                id: artifact_id,
                kind: ArtifactKind::Commit,
                title: subject,
                summary: Some(format!("Commit by {} on {}", author_name, committed_at.format("%Y-%m-%d"))),
                body: body_lines,
                provider: self.provider().to_string(),
                source_id,
                source_url: format!("file://{}/.git/commits/{}", repo.root_path.to_string_lossy(), sha),
                repository: Some(repo.name.clone()),
                tags: vec!["local_git".to_string(), "commit".to_string()],
                relationships,
                created_at: Some(committed_at),
                updated_at: committed_at,
                synced_at: Utc::now(),
                checksum,
                metadata: commit_meta,
            };

            commits.push(artifact);
        }

        commits
    }

    /// Enumerate tracked Markdown files in a repository, filtering out common build/cache directories
    pub fn list_markdown_files(repo: &LocalGitRepository) -> Vec<PathBuf> {
        let mut markdown_files = Vec::new();

        let walker = WalkDir::new(&repo.root_path)
            .into_iter()
            .filter_entry(|e| {
                let file_name = e.file_name().to_string_lossy();
                // Filter out .git, target, node_modules, .idea, etc.
                !file_name.starts_with(".git")
                    && file_name != "target"
                    && file_name != "node_modules"
                    && file_name != ".idea"
                    && file_name != "vendor"
            });

        for entry in walker.flatten() {
            if entry.file_type().is_file() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown") {
                        markdown_files.push(path.to_path_buf());
                    }
                }
            }
        }

        markdown_files
    }
}

#[async_trait::async_trait]
impl Connector for LocalGitConnector {
    fn id(&self) -> &str {
        &self.id
    }

    fn provider(&self) -> &str {
        "local_git"
    }

    async fn fetch_modified(&self, since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        let mut artifacts = Vec::new();

        for repo in &self.registry.repositories {
            // 1. Fetch Commit History Graph artifacts
            let mut commit_artifacts = self.fetch_commit_artifacts(repo, since);
            artifacts.append(&mut commit_artifacts);

            // 2. Fetch Tracked Markdown Documents with Git Blame metadata
            let markdown_files = Self::list_markdown_files(repo);

            for file_path in markdown_files {
                let metadata = match fs::metadata(&file_path) {
                    Ok(m) => m,
                    Err(e) => {
                        warn!("Skipping unreadable file {:?}: {}", file_path, e);
                        continue;
                    }
                };

                let mtime: DateTime<Utc> = metadata
                    .modified()
                    .map(DateTime::from)
                    .unwrap_or_else(|_| Utc::now());

                if let Some(since_time) = since {
                    if mtime <= since_time {
                        continue;
                    }
                }

                let content = match fs::read_to_string(&file_path) {
                    Ok(c) => c,
                    Err(e) => {
                        debug!("Failed to read file content {:?}: {}", file_path, e);
                        continue;
                    }
                };

                let relative_path = file_path
                    .strip_prefix(&repo.root_path)
                    .unwrap_or(&file_path)
                    .to_string_lossy()
                    .to_string();

                let source_id = format!("{}:{}", repo.id, relative_path);
                let artifact_id = KnowledgeArtifact::generate_id(self.provider(), &repo.id, &source_id);

                let title = file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled")
                    .to_string();

                let checksum = KnowledgeArtifact::compute_checksum(&title, None, &content, &[]);

                let line_blames = Self::read_line_blame(&repo.root_path, &relative_path);

                let git_meta = serde_json::json!({
                    "repo_id": repo.id,
                    "repo_name": repo.name,
                    "relative_path": relative_path,
                    "commit_sha": repo.head_commit,
                    "branch": repo.current_branch.clone().unwrap_or_else(|| "HEAD".to_string()),
                    "default_branch": repo.default_branch,
                    "git_origin_url": repo.git_origin_url,
                    "line_blames": line_blames,
                });

                let artifact = KnowledgeArtifact {
                    id: artifact_id,
                    kind: ArtifactKind::Document,
                    title,
                    summary: None,
                    body: content,
                    provider: self.provider().to_string(),
                    source_id,
                    source_url: format!("file://{}", file_path.to_string_lossy()),
                    repository: Some(repo.name.clone()),
                    tags: vec!["local_git".to_string(), "markdown".to_string()],
                    relationships: vec![],
                    created_at: Some(mtime),
                    updated_at: mtime,
                    synced_at: Utc::now(),
                    checksum,
                    metadata: git_meta,
                };

                artifacts.push(artifact);
            }
        }

        Ok(artifacts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_mock_git_repo() -> Result<TempDir> {
        let dir = TempDir::new()?;
        let git_dir = dir.path().join(".git");
        fs::create_dir_all(&git_dir)?;

        // Mock HEAD
        fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n")?;

        // Mock branch ref
        let refs_dir = git_dir.join("refs/heads");
        fs::create_dir_all(&refs_dir)?;
        fs::write(refs_dir.join("main"), "a1b2c3d4e5f67890123456789012345678901234\n")?;

        // Mock config with origin
        let config_content = r#"[core]
	repositoryformatversion = 0
[remote "origin"]
	url = git@github.com:example/test-repo.git
"#;
        fs::write(git_dir.join("config"), config_content)?;

        // Mock sample Markdown files
        fs::write(dir.path().join("README.md"), "# Mock Repo\nHello world")?;
        let docs_dir = dir.path().join("docs");
        fs::create_dir_all(&docs_dir)?;
        fs::write(docs_dir.join("arch.md"), "# Architecture\nAtlas architecture")?;

        Ok(dir)
    }

    #[test]
    fn test_local_git_repo_open() -> Result<()> {
        let mock_dir = create_mock_git_repo()?;
        let repo = LocalGitRepository::open(mock_dir.path())?;

        assert_eq!(repo.current_branch.as_deref(), Some("main"));
        assert_eq!(repo.head_commit, "a1b2c3d4e5f67890123456789012345678901234");
        assert_eq!(repo.default_branch, "main");
        assert_eq!(
            repo.git_origin_url.as_deref(),
            Some("git@github.com:example/test-repo.git")
        );

        let markdown_files = LocalGitConnector::list_markdown_files(&repo);
        assert_eq!(markdown_files.len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_local_git_connector_fetch() -> Result<()> {
        let mock_dir = create_mock_git_repo()?;
        let mut registry = RepositoryRegistry::new();
        registry.add_repository(mock_dir.path())?;

        let connector = LocalGitConnector::new("local-git-test", registry);
        let artifacts = connector.fetch_modified(None).await?;

        assert!(artifacts.len() >= 2);
        assert!(artifacts.iter().any(|a| a.title == "README"));
        assert!(artifacts.iter().any(|a| a.title == "arch"));
        assert_eq!(artifacts[0].provider, "local_git");

        Ok(())
    }
}
