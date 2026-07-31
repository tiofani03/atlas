pub mod confluence;
pub mod github;
pub mod jira;

use crate::domain::KnowledgeArtifact;
use anyhow::Result;
use chrono::{DateTime, Utc};
use confluence::ConfluenceConnector;
use github::GithubConnector;
use jira::JiraConnector;

#[async_trait::async_trait]
pub trait Connector: Send + Sync {
    /// Identifier for this connector instance (e.g. "jira-main")
    fn id(&self) -> &str;

    /// Provider type (e.g. "jira", "confluence", or "github")
    fn provider(&self) -> &str;

    /// Fetch artifacts modified since the optional timestamp
    async fn fetch_modified(&self, since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>>;
}

pub enum ConnectorInstance {
    Jira(JiraConnector),
    Confluence(ConfluenceConnector),
    Github(GithubConnector),
}

#[async_trait::async_trait]
impl Connector for ConnectorInstance {
    fn id(&self) -> &str {
        match self {
            ConnectorInstance::Jira(c) => c.id(),
            ConnectorInstance::Confluence(c) => c.id(),
            ConnectorInstance::Github(c) => c.id(),
        }
    }

    fn provider(&self) -> &str {
        match self {
            ConnectorInstance::Jira(c) => c.provider(),
            ConnectorInstance::Confluence(c) => c.provider(),
            ConnectorInstance::Github(c) => c.provider(),
        }
    }

    async fn fetch_modified(&self, since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        match self {
            ConnectorInstance::Jira(c) => c.fetch_modified(since).await,
            ConnectorInstance::Confluence(c) => c.fetch_modified(since).await,
            ConnectorInstance::Github(c) => c.fetch_modified(since).await,
        }
    }
}

