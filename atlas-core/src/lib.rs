pub mod config;
pub mod connectors;
pub mod context;
pub mod domain;
pub mod mcp;
pub mod storage;
pub mod sync;

pub use config::{Config, ConnectorConfig};
pub use connectors::{
    asana::AsanaConnector, azure_devops::AzureDevopsConnector, bitbucket::BitbucketConnector,
    clickup::ClickupConnector, confluence::ConfluenceConnector, figma::FigmaConnector,
    github::GithubConnector, gitlab::GitlabConnector, jira::JiraConnector, linear::LinearConnector,
    local_git::{LocalGitConnector, LocalGitRepository, RepositoryRegistry},
    markdown::MarkdownConnector, notion::NotionConnector, openapi::OpenapiConnector,
    spreadsheet::SpreadsheetConnector, Connector, ConnectorInstance,
};
pub use context::{
    AiBriefing, CategoryAvailability, CategoryScore, CompletenessReport, ContextBuilder, ContextOptions, ContextPackage,
    ContextTelemetry, DependencyEdge, EngineeringReadiness, LabeledArtifact, NextAction, RecommendedItem, SourceInfo,
};
pub use domain::{ArtifactKind, ArtifactRelationship, KnowledgeArtifact};
pub use storage::{ArtifactHeader, Storage, StorageStats};
pub use sync::{SyncEngine, SyncSummary};


