pub mod asana;
pub mod azure_devops;
pub mod bitbucket;
pub mod clickup;
pub mod confluence;
pub mod figma;
pub mod github;
pub mod gitlab;
pub mod jira;
pub mod linear;
pub mod local_git;
pub mod markdown;
pub mod notion;
pub mod openapi;
pub mod spreadsheet;

use crate::domain::KnowledgeArtifact;
use anyhow::Result;
use asana::AsanaConnector;
use azure_devops::AzureDevopsConnector;
use bitbucket::BitbucketConnector;
use chrono::{DateTime, Utc};
use clickup::ClickupConnector;
use confluence::ConfluenceConnector;
use figma::FigmaConnector;
use github::GithubConnector;
use gitlab::GitlabConnector;
use jira::JiraConnector;
use linear::LinearConnector;
use local_git::LocalGitConnector;
use markdown::MarkdownConnector;
use notion::NotionConnector;
use openapi::OpenapiConnector;
use spreadsheet::SpreadsheetConnector;

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
    Markdown(MarkdownConnector),
    LocalGit(LocalGitConnector),
    Clickup(ClickupConnector),
    Linear(LinearConnector),
    Asana(AsanaConnector),
    AzureDevops(AzureDevopsConnector),
    Gitlab(GitlabConnector),
    Bitbucket(BitbucketConnector),
    Openapi(OpenapiConnector),
    Figma(FigmaConnector),
    Notion(NotionConnector),
    Spreadsheet(SpreadsheetConnector),
}

#[async_trait::async_trait]
impl Connector for ConnectorInstance {
    fn id(&self) -> &str {
        match self {
            ConnectorInstance::Jira(c) => c.id(),
            ConnectorInstance::Confluence(c) => c.id(),
            ConnectorInstance::Github(c) => c.id(),
            ConnectorInstance::Markdown(c) => c.id(),
            ConnectorInstance::LocalGit(c) => c.id(),
            ConnectorInstance::Clickup(c) => c.id(),
            ConnectorInstance::Linear(c) => c.id(),
            ConnectorInstance::Asana(c) => c.id(),
            ConnectorInstance::AzureDevops(c) => c.id(),
            ConnectorInstance::Gitlab(c) => c.id(),
            ConnectorInstance::Bitbucket(c) => c.id(),
            ConnectorInstance::Openapi(c) => c.id(),
            ConnectorInstance::Figma(c) => c.id(),
            ConnectorInstance::Notion(c) => c.id(),
            ConnectorInstance::Spreadsheet(c) => c.id(),
        }
    }

    fn provider(&self) -> &str {
        match self {
            ConnectorInstance::Jira(c) => c.provider(),
            ConnectorInstance::Confluence(c) => c.provider(),
            ConnectorInstance::Github(c) => c.provider(),
            ConnectorInstance::Markdown(c) => c.provider(),
            ConnectorInstance::LocalGit(c) => c.provider(),
            ConnectorInstance::Clickup(c) => c.provider(),
            ConnectorInstance::Linear(c) => c.provider(),
            ConnectorInstance::Asana(c) => c.provider(),
            ConnectorInstance::AzureDevops(c) => c.provider(),
            ConnectorInstance::Gitlab(c) => c.provider(),
            ConnectorInstance::Bitbucket(c) => c.provider(),
            ConnectorInstance::Openapi(c) => c.provider(),
            ConnectorInstance::Figma(c) => c.provider(),
            ConnectorInstance::Notion(c) => c.provider(),
            ConnectorInstance::Spreadsheet(c) => c.provider(),
        }
    }

    async fn fetch_modified(&self, since: Option<DateTime<Utc>>) -> Result<Vec<KnowledgeArtifact>> {
        match self {
            ConnectorInstance::Jira(c) => c.fetch_modified(since).await,
            ConnectorInstance::Confluence(c) => c.fetch_modified(since).await,
            ConnectorInstance::Github(c) => c.fetch_modified(since).await,
            ConnectorInstance::Markdown(c) => c.fetch_modified(since).await,
            ConnectorInstance::LocalGit(c) => c.fetch_modified(since).await,
            ConnectorInstance::Clickup(c) => c.fetch_modified(since).await,
            ConnectorInstance::Linear(c) => c.fetch_modified(since).await,
            ConnectorInstance::Asana(c) => c.fetch_modified(since).await,
            ConnectorInstance::AzureDevops(c) => c.fetch_modified(since).await,
            ConnectorInstance::Gitlab(c) => c.fetch_modified(since).await,
            ConnectorInstance::Bitbucket(c) => c.fetch_modified(since).await,
            ConnectorInstance::Openapi(c) => c.fetch_modified(since).await,
            ConnectorInstance::Figma(c) => c.fetch_modified(since).await,
            ConnectorInstance::Notion(c) => c.fetch_modified(since).await,
            ConnectorInstance::Spreadsheet(c) => c.fetch_modified(since).await,
        }
    }
}
