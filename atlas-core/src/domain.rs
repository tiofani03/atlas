use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::str::FromStr;

/// Extensible kinds of normalized engineering artifacts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Repository,
    Issue,
    PullRequest,
    PullRequestReview,
    ReviewComment,
    Commit,
    Release,
    Discussion,
    WorkflowRun,
    Deployment,
    Ticket,
    Document,
    Specification,
    Design,
    Component,
    Other(String),
}

impl std::fmt::Display for ArtifactKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactKind::Repository => write!(f, "repository"),
            ArtifactKind::Issue => write!(f, "issue"),
            ArtifactKind::PullRequest => write!(f, "pull_request"),
            ArtifactKind::PullRequestReview => write!(f, "pull_request_review"),
            ArtifactKind::ReviewComment => write!(f, "review_comment"),
            ArtifactKind::Commit => write!(f, "commit"),
            ArtifactKind::Release => write!(f, "release"),
            ArtifactKind::Discussion => write!(f, "discussion"),
            ArtifactKind::WorkflowRun => write!(f, "workflow_run"),
            ArtifactKind::Deployment => write!(f, "deployment"),
            ArtifactKind::Ticket => write!(f, "ticket"),
            ArtifactKind::Document => write!(f, "document"),
            ArtifactKind::Specification => write!(f, "specification"),
            ArtifactKind::Design => write!(f, "design"),
            ArtifactKind::Component => write!(f, "component"),
            ArtifactKind::Other(s) => write!(f, "{}", s),
        }
    }
}

impl FromStr for ArtifactKind {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let kind = match s.to_lowercase().as_str() {
            "repository" | "repo" => ArtifactKind::Repository,
            "issue" => ArtifactKind::Issue,
            "pull_request" | "pullrequest" | "pr" => ArtifactKind::PullRequest,
            "pull_request_review" | "pullrequestreview" | "review" => ArtifactKind::PullRequestReview,
            "review_comment" | "reviewcomment" => ArtifactKind::ReviewComment,
            "commit" => ArtifactKind::Commit,
            "release" => ArtifactKind::Release,
            "discussion" => ArtifactKind::Discussion,
            "workflow_run" | "workflowrun" => ArtifactKind::WorkflowRun,
            "deployment" => ArtifactKind::Deployment,
            "ticket" => ArtifactKind::Ticket,
            "document" => ArtifactKind::Document,
            "specification" => ArtifactKind::Specification,
            "design" => ArtifactKind::Design,
            "component" => ArtifactKind::Component,
            other => ArtifactKind::Other(other.to_string()),
        };
        Ok(kind)
    }
}

/// Generic directed relationship between engineering artifacts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ArtifactRelationship {
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,
}

/// The canonical, normalized engineering artifact record in Atlas
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeArtifact {
    /// Deterministic UUID or composite key: provider:instance:source_id
    pub id: String,
    pub kind: ArtifactKind,
    pub title: String,
    pub summary: Option<String>,
    pub body: String,
    pub provider: String,
    pub source_id: String,
    pub source_url: String,
    pub repository: Option<String>,
    pub tags: Vec<String>,
    pub relationships: Vec<ArtifactRelationship>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub synced_at: DateTime<Utc>,
    pub checksum: String,
    pub metadata: serde_json::Value,
}

impl KnowledgeArtifact {
    /// Compute deterministic ID from provider, instance_url, and source_id
    pub fn generate_id(provider: &str, instance_url: &str, source_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(provider.as_bytes());
        hasher.update(b":");
        hasher.update(instance_url.as_bytes());
        hasher.update(b":");
        hasher.update(source_id.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Compute SHA256 content checksum for deduplication
    pub fn compute_checksum(
        title: &str,
        summary: Option<&str>,
        body: &str,
        tags: &[String],
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(title.as_bytes());
        hasher.update(summary.unwrap_or("").as_bytes());
        hasher.update(body.as_bytes());
        for tag in tags {
            hasher.update(tag.as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }

    /// Extract structured PullRequestMetadata if this artifact is a Pull Request
    pub fn pull_request_metadata(&self) -> Option<PullRequestMetadata> {
        if self.kind != ArtifactKind::PullRequest {
            return None;
        }

        let repo = self.repository.clone().or_else(|| {
            self.metadata
                .get("repository")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })?;

        let number = self
            .metadata
            .get("number")
            .or_else(|| self.metadata.get("pr_number"))
            .and_then(|v| v.as_u64())
            .or_else(|| {
                // Fallback to parsing integer from source_id ending (e.g. repo#23)
                if let Some(pos) = self.source_id.rfind('#') {
                    self.source_id[pos + 1..].parse::<u64>().ok()
                } else {
                    None
                }
            })?;

        let title = self
            .metadata
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.title)
            .to_string();

        let state = self
            .metadata
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("open")
            .to_string();

        let branch = self
            .metadata
            .get("branch")
            .or_else(|| self.metadata.get("head_branch"))
            .or_else(|| {
                self.metadata
                    .get("head")
                    .and_then(|h| h.get("ref").or_else(|| h.get("label")))
            })
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let merged_at = self
            .metadata
            .get("merged_at")
            .and_then(|v| v.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc)));

        let author = self
            .metadata
            .get("author")
            .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| {
                v.get("login").and_then(|l| l.as_str()).map(|s| s.to_string())
            }))
            .or_else(|| {
                self.metadata
                    .get("user")
                    .and_then(|u| u.get("login"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            });

        Some(PullRequestMetadata {
            repository: repo,
            number,
            title,
            state,
            branch,
            merged_at,
            author,
            provider: self.provider.clone(),
        })
    }

    /// Classify artifact into its active set of domain aspects
    pub fn classify_aspects(&self) -> std::collections::HashSet<DomainAspect> {
        let mut aspects = std::collections::HashSet::new();

        match self.kind {
            ArtifactKind::Repository
            | ArtifactKind::PullRequest
            | ArtifactKind::PullRequestReview
            | ArtifactKind::ReviewComment
            | ArtifactKind::Commit
            | ArtifactKind::WorkflowRun
            | ArtifactKind::Deployment => {
                aspects.insert(DomainAspect::CodeImplementation);
            }
            ArtifactKind::Issue | ArtifactKind::Ticket => {
                aspects.insert(DomainAspect::TaskTracking);
            }
            ArtifactKind::Discussion => {
                aspects.insert(DomainAspect::Collaboration);
            }
            ArtifactKind::Document
            | ArtifactKind::Specification => {
                aspects.insert(DomainAspect::Documentation);
            }
            ArtifactKind::Design => {
                aspects.insert(DomainAspect::Design);
            }
            ArtifactKind::Release => {
                aspects.insert(DomainAspect::CodeImplementation);
            }
            _ => {}
        }

        let lower_id = self.source_id.to_lowercase();
        let lower_title = self.title.to_lowercase();
        let lower_provider = self.provider.to_lowercase();

        // Check for Architecture indicators
        if lower_id.starts_with("adr-")
            || lower_id.contains("adr")
            || lower_title.contains("adr")
            || lower_title.contains("rfc")
            || lower_title.contains("architecture")
            || lower_title.contains("design decision")
            || self.tags.iter().any(|t| {
                let lt = t.to_lowercase();
                lt.contains("adr") || lt.contains("architecture") || lt.contains("rfc")
            })
        {
            aspects.insert(DomainAspect::Architecture);
        }

        // Check for Collaboration indicators
        if lower_title.contains("retro")
            || lower_title.contains("sprint planning")
            || lower_title.contains("meeting notes")
            || lower_title.contains("standup")
            || lower_provider == "slack"
            || lower_provider == "discord"
        {
            aspects.insert(DomainAspect::Collaboration);
        }

        // Check for Design indicators
        if lower_provider == "figma"
            || lower_title.contains("figma")
            || lower_title.contains("wireframe")
            || lower_title.contains("mockup")
            || lower_title.contains("ui spec")
        {
            aspects.insert(DomainAspect::Design);
        }

        // Check for Data / Spreadsheet indicators
        if lower_provider == "spreadsheet"
            || lower_title.contains("dashboard")
            || lower_title.contains("analytics")
            || lower_title.contains("metrics")
        {
            aspects.insert(DomainAspect::MetricsData);
        }

        // If Collaboration, Architecture, Design, or Documentation aspects are present,
        // prune generic TaskTracking / CodeImplementation defaults unless explicitly code-tagged
        if (aspects.contains(&DomainAspect::Collaboration)
            || aspects.contains(&DomainAspect::Architecture)
            || aspects.contains(&DomainAspect::Design)
            || aspects.contains(&DomainAspect::Documentation)
            || aspects.contains(&DomainAspect::MetricsData))
            && !self.tags.iter().any(|t| {
                let lt = t.to_lowercase();
                lt.contains("code") || lt.contains("pr") || lt.contains("commit") || lt.contains("bug")
            })
        {
            aspects.remove(&DomainAspect::TaskTracking);
            aspects.remove(&DomainAspect::CodeImplementation);
        }

        // Fallback default
        if aspects.is_empty() {
            aspects.insert(DomainAspect::CodeImplementation);
        }

        aspects
    }
}

/// Canonical domain aspects representing different functional dimensions of knowledge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainAspect {
    CodeImplementation,
    Architecture,
    TaskTracking,
    Design,
    Documentation,
    Collaboration,
    MetricsData,
}

impl std::fmt::Display for DomainAspect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainAspect::CodeImplementation => write!(f, "Code & Implementation"),
            DomainAspect::Architecture => write!(f, "Architecture & Design Decisions"),
            DomainAspect::TaskTracking => write!(f, "Tasks & Work Items"),
            DomainAspect::Design => write!(f, "UI / UX Design"),
            DomainAspect::Documentation => write!(f, "Documentation & Knowledge Base"),
            DomainAspect::Collaboration => write!(f, "Team Collaboration & Planning"),
            DomainAspect::MetricsData => write!(f, "Metrics & Data"),
        }
    }
}

/// Structured metadata for a Pull Request artifact
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PullRequestMetadata {
    pub repository: String,
    pub number: u64,
    pub title: String,
    pub state: String,
    pub branch: Option<String>,
    pub merged_at: Option<DateTime<Utc>>,
    pub author: Option<String>,
    pub provider: String,
}



