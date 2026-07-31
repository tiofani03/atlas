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
}

