use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// High-level categories of normalized knowledge objects
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObjectType {
    Ticket,
    Document,
    Specification,
    Design,
    Component,
}

impl std::fmt::Display for ObjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectType::Ticket => write!(f, "ticket"),
            ObjectType::Document => write!(f, "document"),
            ObjectType::Specification => write!(f, "specification"),
            ObjectType::Design => write!(f, "design"),
            ObjectType::Component => write!(f, "component"),
        }
    }
}

impl std::str::FromStr for ObjectType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ticket" => Ok(ObjectType::Ticket),
            "document" => Ok(ObjectType::Document),
            "specification" => Ok(ObjectType::Specification),
            "design" => Ok(ObjectType::Design),
            "component" => Ok(ObjectType::Component),
            _ => Err(format!("Unknown object type: {}", s)),
        }
    }
}

/// Link or reference between knowledge objects
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Relationship {
    pub target_id: String,
    pub relationship_type: String,
}

/// Provenance information for a knowledge object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceInfo {
    pub provider: String,     // e.g. "jira", "confluence"
    pub instance_url: String, // e.g. "https://company.atlassian.net"
    pub original_id: String,  // e.g. "PAY-1042" or page ID "1928374"
    pub web_url: String,      // Direct link to the source document/ticket
}

/// The canonical, normalized knowledge record in Atlas
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeObject {
    /// Deterministic UUID or composite key: provider:instance:original_id
    pub id: String,
    pub object_type: ObjectType,
    pub title: String,
    pub summary: Option<String>,
    pub content: String,
    pub tags: Vec<String>,
    pub relationships: Vec<Relationship>,
    pub source: SourceInfo,
    pub source_metadata: serde_json::Value,
    pub updated_at: DateTime<Utc>,
    pub synced_at: DateTime<Utc>,
    pub checksum: String,
}

impl KnowledgeObject {
    /// Compute deterministic ID from source info
    pub fn generate_id(provider: &str, instance_url: &str, original_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(provider.as_bytes());
        hasher.update(b":");
        hasher.update(instance_url.as_bytes());
        hasher.update(b":");
        hasher.update(original_id.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Compute SHA256 content checksum for deduplication
    pub fn compute_checksum(
        title: &str,
        summary: Option<&str>,
        content: &str,
        tags: &[String],
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(title.as_bytes());
        hasher.update(summary.unwrap_or("").as_bytes());
        hasher.update(content.as_bytes());
        for tag in tags {
            hasher.update(tag.as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }
}
