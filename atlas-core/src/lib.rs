pub mod config;
pub mod connectors;
pub mod domain;
pub mod mcp;
pub mod storage;
pub mod sync;

pub use config::{Config, ConnectorConfig};
pub use connectors::{
    confluence::ConfluenceConnector, github::GithubConnector, jira::JiraConnector, Connector,
    ConnectorInstance,
};
pub use domain::{ArtifactKind, ArtifactRelationship, KnowledgeArtifact};
pub use storage::{Storage, StorageStats};
pub use sync::{SyncEngine, SyncSummary};

