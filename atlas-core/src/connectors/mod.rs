pub mod confluence;
pub mod jira;

use crate::domain::KnowledgeObject;
use anyhow::Result;
use chrono::{DateTime, Utc};
use confluence::ConfluenceConnector;
use jira::JiraConnector;

#[async_trait::async_trait]
pub trait Connector: Send + Sync {
    /// Identifier for this connector instance (e.g. "jira-main")
    fn id(&self) -> &str;

    /// Provider type (e.g. "jira" or "confluence")
    fn provider(&self) -> &str;

    /// Fetch objects modified since the optional timestamp
    async fn fetch_modified(&self, since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeObject>>;
}

pub enum ConnectorInstance {
    Jira(JiraConnector),
    Confluence(ConfluenceConnector),
}

#[async_trait::async_trait]
impl Connector for ConnectorInstance {
    fn id(&self) -> &str {
        match self {
            ConnectorInstance::Jira(c) => c.id(),
            ConnectorInstance::Confluence(c) => c.id(),
        }
    }

    fn provider(&self) -> &str {
        match self {
            ConnectorInstance::Jira(c) => c.provider(),
            ConnectorInstance::Confluence(c) => c.provider(),
        }
    }

    async fn fetch_modified(&self, since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeObject>> {
        match self {
            ConnectorInstance::Jira(c) => c.fetch_modified(since).await,
            ConnectorInstance::Confluence(c) => c.fetch_modified(since).await,
        }
    }
}
