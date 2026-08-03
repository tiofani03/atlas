pub mod config;
pub mod connectors;
pub mod context;
pub mod domain;
pub mod health;
pub mod mcp;
pub mod progress;
pub mod resilience;
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
    AiBriefing, AiGuidance, CategoryAvailability, CategoryScore, ClassifiedKnowledgeGap, CompletenessReport,
    ContextBuilder, ContextOptions, ContextPackage, ContextTelemetry, CurrentUnderstanding, DependencyEdge,
    EngineeringReadiness, EvidenceItem, ImplementationHypothesis, ImplementationRisk, InvestigationStep,
    KnownFact, LabeledArtifact, Mission, ModuleRating, NextAction, PossibleImplementationAreas,
    PrioritizedKnowledgeGaps, QueueStep, RecommendedItem, ScopeItem, SourceInfo, StatusCheck,
};
pub use domain::{ArtifactKind, ArtifactRelationship, DomainAspect, KnowledgeArtifact};
pub use progress::{ProgressEvent, ProgressEventBus, SyncAction};
pub use storage::{ArtifactHeader, Storage, StorageStats};
pub use sync::{SyncEngine, SyncSummary};


