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


