use crate::connectors::v2::{
    AuthConfigSchema, CapabilityMatrix, ConfigFieldSchema, ConfigFieldType, ConnectorManifest,
};

fn field(
    key: &str,
    label: &str,
    field_type: ConfigFieldType,
    required: bool,
    placeholder: &str,
    help_text: &str,
) -> ConfigFieldSchema {
    ConfigFieldSchema {
        key: key.to_string(),
        label: label.to_string(),
        field_type,
        required,
        placeholder: placeholder.to_string(),
        help_text: help_text.to_string(),
    }
}

fn manifest(
    provider: &str,
    name: &str,
    description: &str,
    auth_type: &str,
    required_fields: &[&str],
    webhooks: bool,
    config_fields: Vec<ConfigFieldSchema>,
) -> ConnectorManifest {
    ConnectorManifest {
        id: provider.to_string(),
        provider: provider.to_string(),
        version: "2.0.0".to_string(),
        name: name.to_string(),
        description: description.to_string(),
        capabilities: CapabilityMatrix {
            supports_webhooks: webhooks,
            ..Default::default()
        },
        auth_schema: AuthConfigSchema {
            auth_type: auth_type.to_string(),
            required_fields: required_fields.iter().map(|s| (*s).to_string()).collect(),
        },
        config_fields,
    }
}

/// Returns declarative manifests and config field schemas for all 15 built-in connectors.
pub fn all_connector_manifests() -> Vec<ConnectorManifest> {
    vec![
        manifest(
            "jira",
            "Jira",
            "Sync Jira issues, epics, sprints, and comments.",
            "basic_api_token",
            &["instance_url", "email", "api_token"],
            true,
            vec![
                field("instance_url", "Instance URL", ConfigFieldType::Url, true, "https://your-org.atlassian.net", "Atlassian Cloud or Server base URL"),
                field("email", "Account Email", ConfigFieldType::Text, true, "dev@company.com", "Atlassian account email"),
                field("api_token", "API Token", ConfigFieldType::Password, true, "${JIRA_API_TOKEN}", "API token or ${ENV_VAR} reference"),
                field("projects", "Project Keys", ConfigFieldType::List, false, "CORE, INFRA", "Comma-separated project keys (optional)"),
            ],
        ),
        manifest(
            "confluence",
            "Confluence",
            "Sync Confluence spaces, pages, and architecture docs.",
            "basic_api_token",
            &["instance_url", "email", "api_token"],
            false,
            vec![
                field("instance_url", "Instance URL", ConfigFieldType::Url, true, "https://your-org.atlassian.net/wiki", "Confluence base URL"),
                field("email", "Account Email", ConfigFieldType::Text, true, "dev@company.com", "Atlassian account email"),
                field("api_token", "API Token", ConfigFieldType::Password, true, "${CONFLUENCE_API_TOKEN}", "API token or ${ENV_VAR} reference"),
                field("spaces", "Space Keys", ConfigFieldType::List, false, "ENG, ARCH", "Comma-separated space keys (optional)"),
            ],
        ),
        manifest(
            "github",
            "GitHub",
            "Sync repositories, issues, pull requests, reviews, commits, and releases.",
            "bearer_token",
            &["api_token"],
            true,
            vec![
                field("api_token", "Personal Access Token", ConfigFieldType::Password, true, "${GITHUB_TOKEN}", "GitHub PAT with repo scope"),
                field("repos", "Repositories", ConfigFieldType::List, true, "owner/repo1, owner/repo2", "Comma-separated owner/repo slugs"),
                field("instance_url", "API Base URL", ConfigFieldType::Url, false, "https://api.github.com", "Leave blank for github.com or set GitHub Enterprise URL"),
            ],
        ),
        manifest(
            "gitlab",
            "GitLab",
            "Sync GitLab projects, issues, merge requests, and commits.",
            "bearer_token",
            &["api_token"],
            true,
            vec![
                field("instance_url", "GitLab URL", ConfigFieldType::Url, false, "https://gitlab.com", "GitLab instance URL"),
                field("api_token", "Personal Access Token", ConfigFieldType::Password, true, "${GITLAB_TOKEN}", "GitLab PAT with read_api scope"),
                field("projects", "Project Paths", ConfigFieldType::List, false, "group/project", "Comma-separated project paths or IDs"),
                field("ssl_cert_path", "Custom SSL Cert Path", ConfigFieldType::Path, false, "/path/to/ca.pem", "Optional CA certificate for self-hosted GitLab"),
            ],
        ),
        manifest(
            "bitbucket",
            "Bitbucket",
            "Sync Bitbucket repositories, pull requests, and commits.",
            "bearer_token",
            &["api_token"],
            false,
            vec![
                field("instance_url", "Bitbucket URL", ConfigFieldType::Url, false, "https://api.bitbucket.org/2.0", "Bitbucket API base URL"),
                field("workspace", "Workspace", ConfigFieldType::Text, true, "my-workspace", "Bitbucket workspace slug"),
                field("api_token", "App Password / Token", ConfigFieldType::Password, true, "${BITBUCKET_TOKEN}", "Access token or app password"),
                field("repos", "Repositories", ConfigFieldType::List, false, "repo-one, repo-two", "Comma-separated repository slugs"),
            ],
        ),
        manifest(
            "azure_devops",
            "Azure DevOps",
            "Sync Azure Boards work items, repos, and pull requests.",
            "pat",
            &["organization", "api_token"],
            false,
            vec![
                field("organization", "Organization", ConfigFieldType::Text, true, "my-org", "Azure DevOps organization name"),
                field("api_token", "Personal Access Token", ConfigFieldType::Password, true, "${AZURE_DEVOPS_PAT}", "Azure DevOps PAT"),
                field("projects", "Projects", ConfigFieldType::List, false, "Platform, Mobile", "Comma-separated project names"),
            ],
        ),
        manifest(
            "local_git",
            "Local Git",
            "Index local Git repositories, commit history, and file ownership.",
            "none",
            &["paths"],
            false,
            vec![
                field("paths", "Repository Paths", ConfigFieldType::Path, true, "/home/user/projects/repo", "Local filesystem path(s) to Git repositories"),
            ],
        ),
        manifest(
            "markdown",
            "Markdown Docs",
            "Index local Markdown files, ADRs, RFCs, and documentation folders.",
            "none",
            &["paths"],
            false,
            vec![
                field("paths", "Documentation Paths", ConfigFieldType::Path, true, "/home/user/docs", "Local folder(s) containing Markdown files"),
                field("glob_patterns", "Glob Patterns", ConfigFieldType::List, false, "**/*.md, **/*.mdx", "File glob patterns to include"),
            ],
        ),
        manifest(
            "linear",
            "Linear",
            "Sync Linear teams, issues, cycles, and projects.",
            "api_key",
            &["api_token"],
            false,
            vec![
                field("api_token", "API Key", ConfigFieldType::Password, true, "${LINEAR_API_KEY}", "Linear personal API key"),
                field("teams", "Team Keys", ConfigFieldType::List, false, "ENG, PROD", "Comma-separated team keys (optional)"),
            ],
        ),
        manifest(
            "clickup",
            "ClickUp",
            "Sync ClickUp spaces, folders, lists, and tasks.",
            "api_key",
            &["api_token"],
            false,
            vec![
                field("api_token", "API Token", ConfigFieldType::Password, true, "${CLICKUP_TOKEN}", "ClickUp personal or OAuth token"),
                field("workspace", "Workspace / Team ID", ConfigFieldType::Text, false, "1234567", "ClickUp Team/Workspace ID"),
                field("spaces", "Space IDs", ConfigFieldType::List, false, "901234", "Comma-separated space IDs"),
                field("lists", "List IDs", ConfigFieldType::List, false, "905678", "Comma-separated list IDs"),
            ],
        ),
        manifest(
            "asana",
            "Asana",
            "Sync Asana workspaces, projects, and tasks.",
            "bearer_token",
            &["api_token"],
            false,
            vec![
                field("api_token", "Personal Access Token", ConfigFieldType::Password, true, "${ASANA_TOKEN}", "Asana PAT"),
                field("workspace", "Workspace GID", ConfigFieldType::Text, false, "1200000000000", "Asana workspace GID"),
                field("projects", "Project GIDs", ConfigFieldType::List, false, "1200000000001", "Comma-separated project GIDs"),
            ],
        ),
        manifest(
            "notion",
            "Notion",
            "Sync Notion pages and databases as structured Markdown.",
            "bearer_token",
            &["api_token"],
            false,
            vec![
                field("api_token", "Integration Secret", ConfigFieldType::Password, true, "${NOTION_TOKEN}", "Notion internal integration token"),
                field("database_ids", "Database IDs", ConfigFieldType::List, false, "uuid1, uuid2", "Comma-separated Notion database IDs"),
                field("page_ids", "Page IDs", ConfigFieldType::List, false, "uuid3, uuid4", "Comma-separated root Notion page IDs"),
            ],
        ),
        manifest(
            "openapi",
            "OpenAPI / Swagger",
            "Parse OpenAPI 3.x & Swagger specs (JSON/YAML) into API artifacts.",
            "none",
            &["paths"],
            false,
            vec![
                field("paths", "Spec File Paths / URLs", ConfigFieldType::Path, true, "./openapi.yaml", "Local path(s) or URLs to OpenAPI JSON/YAML specs"),
            ],
        ),
        manifest(
            "figma",
            "Figma",
            "Sync Figma design files, frames, components, and comments.",
            "api_key",
            &["api_token", "file_keys"],
            false,
            vec![
                field("api_token", "Personal Access Token", ConfigFieldType::Password, true, "${FIGMA_TOKEN}", "Figma personal access token"),
                field("file_keys", "File Keys", ConfigFieldType::List, true, "abc123XYZ", "Comma-separated Figma file keys from URLs"),
            ],
        ),
        manifest(
            "spreadsheet",
            "Spreadsheet / CSV",
            "Index CSV, TSV, or Google Sheets tabular knowledge.",
            "none",
            &["paths"],
            false,
            vec![
                field("paths", "Spreadsheet Paths", ConfigFieldType::Path, false, "./data/roadmap.csv", "Local CSV/TSV file paths"),
                field("spreadsheet_ids", "Google Sheet IDs", ConfigFieldType::List, false, "1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs", "Comma-separated Google Spreadsheet IDs"),
                field("service_account_file", "Service Account JSON", ConfigFieldType::Path, false, "./service-account.json", "Optional Google service account credentials path"),
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_15_connector_manifests_present() {
        let manifests = all_connector_manifests();
        assert_eq!(manifests.len(), 15);
        for m in &manifests {
            assert!(!m.provider.is_empty());
            assert!(!m.name.is_empty());
            assert!(!m.config_fields.is_empty());
        }
    }
}
