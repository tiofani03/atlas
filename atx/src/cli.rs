use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "atx",
    author = "Atlas Contributors",
    version,
    about = "Unified Engineering Knowledge & Context Engine (Atlas)"
)]
pub(crate) struct Cli {
    /// Path to config file [default: ~/.config/atlas/config.toml]
    #[arg(short, long, global = true)]
    pub(crate) config: Option<String>,

    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Initialize local database directory and configuration file
    Init,

    /// Add or update connector configuration
    Config {
        #[command(subcommand)]
        action: ConfigSubcommands,
    },

    /// Synchronize knowledge from external connectors into local context graph
    Sync {
        /// Optional connector ID to sync specifically
        #[arg(long)]
        connector: Option<String>,

        /// Force full re-sync ignoring last sync watermarks
        #[arg(short, long)]
        full: bool,
    },

    /// Search engineering context graph using BM25 full-text search or metadata filters
    Search {
        /// Optional search query terms
        #[arg(default_value = "")]
        query: String,

        /// Filter by artifact kind (e.g. repository, issue, pull_request, commit, release, ticket, document)
        #[arg(short, long)]
        kind: Option<String>,

        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,

        /// Filter by repository (e.g. owner/repo)
        #[arg(short, long)]
        repository: Option<String>,

        /// Maximum results to return
        #[arg(short, long, default_value_t = 10)]
        limit: usize,

        /// Output results as JSON
        #[arg(long)]
        json: bool,

        /// Verbose output (show complete relationship details and disable truncation)
        #[arg(short, long)]
        verbose: bool,

        /// Output raw unformatted database records
        #[arg(long)]
        raw: bool,
    },

    /// Show detailed canonical engineering artifact by ID or source_id
    Artifact {
        /// Artifact ID or source_id (e.g., INIT-219, owner/repo#42, owner/repo@sha)
        id: String,

        /// Output result as JSON
        #[arg(long)]
        json: bool,

        /// Verbose output (disable description truncation)
        #[arg(short, long)]
        verbose: bool,

        /// Output raw unformatted database record
        #[arg(long)]
        raw: bool,
    },

    /// Show connected relationship graph for a given artifact
    Related {
        /// Artifact ID or source_id
        id: String,

        /// Output results as JSON
        #[arg(long)]
        json: bool,

        /// Verbose output (disable relationship item truncation)
        #[arg(short, long)]
        verbose: bool,

        /// Output raw unformatted database records
        #[arg(long)]
        raw: bool,
    },

    /// Build AI-ready engineering context for an issue, PR, repository, ADR, or artifact ID
    Context {
        /// Context target type (e.g., issue, pr, repository, adr) or artifact ID
        target: String,

        /// Artifact ID, source ID, or repository name (when target type is specified)
        target_id: Option<String>,

        /// Associate or override Figma design target (URL, key, or clone key) for this context
        #[arg(long)]
        figma: Option<String>,

        /// Relationship graph traversal depth limit [default: 2]
        #[arg(short, long, default_value_t = 2)]
        depth: usize,

        /// Display stage-level timing telemetry profiling breakdown
        #[arg(long)]
        profile: bool,

        /// Override maximum related commits limit
        #[arg(long)]
        max_commits: Option<usize>,

        /// Output result as JSON
        #[arg(long)]
        json: bool,

        /// Verbose output (disable item list truncation)
        #[arg(short, long)]
        verbose: bool,

        /// Output raw unformatted database records
        #[arg(long)]
        raw: bool,
    },

    /// Query engineering artifacts belonging to a specific repository
    Repository {
        /// Repository identifier (e.g. owner/repo)
        repo: String,

        /// Maximum results to return
        #[arg(short, long, default_value_t = 20)]
        limit: usize,

        /// Output results as JSON
        #[arg(long)]
        json: bool,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,

        /// Output raw unformatted database records
        #[arg(long)]
        raw: bool,
    },

    /// Show storage statistics, connector status, and graph size
    Status,

    /// Clear synchronized context data and reset SQLite index
    Reset {
        /// Optional connector ID to clear specifically
        #[arg(short = 'C', long)]
        connector: Option<String>,

        /// Force clear without asking for confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Alias for reset command
    Clear {
        /// Optional connector ID to clear specifically
        #[arg(short = 'C', long)]
        connector: Option<String>,

        /// Force clear without asking for confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Model Context Protocol (MCP) Hub & Gateway operations
    Mcp {
        #[command(subcommand)]
        action: Option<McpSubcommands>,
    },

    /// Display full document content in terminal (Markdown, Confluence, Notion, ADR)
    Doc {
        /// Document title, path, filename, or ID
        target: Option<String>,

        /// List all available documents
        #[arg(short, long)]
        list: bool,

        /// Output raw document body only without headers (ideal for piping to glow or pager)
        #[arg(long)]
        raw: bool,

        /// Output result as JSON
        #[arg(long)]
        json: bool,
    },

    /// List and browse indexed engineering documents
    Docs {
        /// Optional query filter
        query: Option<String>,
    },

    /// Print full content of any artifact (ticket, document, PR, issue) to terminal
    Cat {
        /// Artifact ID, key, title, path, or alias (e.g. PROJ-123, vision.md, PR#42)
        target: String,

        /// Output raw body only without headers (ideal for piping to glow or pager)
        #[arg(long)]
        raw: bool,

        /// Output result as JSON
        #[arg(long)]
        json: bool,
    },

    /// Rebuild relationship links and commit indices across existing database artifacts
    Reindex {
        /// Optional target (e.g. "links" or "relationships")
        #[arg(default_value = "links")]
        target: String,
    },

    /// Alias for reindex command
    Repair {
        /// Optional target (e.g. "relationships" or "links")
        #[arg(default_value = "relationships")]
        target: String,
    },

    /// Explain relationship graph context for an artifact
    Explain {
        /// Artifact ID or source_id
        id: String,
        /// Show all relationships without collapsing
        #[arg(short = 'a', long)]
        all: bool,
        /// Expand specific section (e.g. prs, tickets, docs, parents, releases)
        #[arg(short = 'e', long)]
        expand: Option<String>,
        /// Expand PRs for a specific subsystem (e.g. atlas-core)
        #[arg(long)]
        subsystem: Option<String>,
        /// Display only deterministic facts
        #[arg(long)]
        facts_only: bool,
        /// Display only AI-inferred findings
        #[arg(long)]
        ai_only: bool,
        /// Show all merge commits inline
        #[arg(long)]
        show_merges: bool,
        /// Show all commits without line collapsing
        #[arg(long)]
        show_commits: bool,
        /// Output presentation DTO as JSON
        #[arg(long)]
        json: bool,
        /// Disable ANSI color output
        #[arg(long)]
        no_color: bool,
    },

    /// Connector Framework V2 Operations (list, inspect, verify, doctor, health, sync)
    Connector {
        #[command(subcommand)]
        action: ConnectorSubcommands,
    },

    /// Run system and connector diagnostics (Doctor mode)
    Doctor,

    /// Launch the Atlas Web UI and Knowledge Explorer in your browser
    #[command(alias = "web")]
    Ui {
        /// Custom port to run the web server on
        #[arg(short, long, default_value_t = 31415)]
        port: u16,

        /// Do not automatically open the browser
        #[arg(long)]
        no_open: bool,
    },
}
 
#[derive(Subcommand, Debug, Clone, PartialEq)]
pub(crate) enum McpSubcommands {
    /// Start stdio MCP Hub server (default if no subcommand specified)
    Serve,

    /// List configured MCP servers
    List,

    /// Add or update an MCP server configuration
    Add {
        /// Server identifier name
        name: String,

        /// Command executable to run
        #[arg(long)]
        command: String,

        /// Arguments passed to the command (multiple flags, comma-separated, or space-separated)
        #[arg(short, long, allow_hyphen_values = true)]
        args: Vec<String>,

        /// Environment variables in KEY=VAL format (multiple flags or comma-separated)
        #[arg(short, long)]
        env: Vec<String>,

        /// Tool prefix for namespacing upstream tools
        #[arg(short, long)]
        prefix: Option<String>,

        /// Disable the server
        #[arg(long)]
        disabled: bool,
    },

    /// Test connectivity and discover tools for an MCP server
    Test {
        /// Server identifier name to test
        name: String,
    },

    /// Remove an MCP server configuration
    Remove {
        /// Server identifier name to remove
        name: String,
    },

    /// Manage aliases for an MCP server (e.g. atx mcp alias figma PROJ-123 sample_key)
    Alias {
        /// Server identifier name (e.g. figma)
        server: String,

        /// Source alias or ticket key (e.g. PROJ-123 or CANONICAL_KEY)
        from: String,

        /// Target cloned key or URL (e.g. sample_key)
        to: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum ConnectorSubcommands {
    /// List registered connectors with status and health scores
    List,
    /// Inspect connector capabilities, rate limit state, and checkpoint watermark
    Inspect { id: String },
    /// Run connectivity and credential verification for a connector
    Verify { id: String },
    /// Run diagnostic suite across all connectors and system dependencies
    Doctor,
    /// Display connector health states and latency histograms
    Health,
    /// Display connector status dashboard
    Status,
    /// Synchronize knowledge using V2 Progress Engine
    Sync {
        /// Optional connector ID to sync specifically
        #[arg(long)]
        connector: Option<String>,
        /// Force full re-sync ignoring checkpoints
        #[arg(short, long)]
        full: bool,
        /// CI/CD non-interactive console mode
        #[arg(long)]
        ci: bool,
        /// JSON stream output mode
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum ConfigSubcommands {
    /// Configure Jira connector
    Jira {
        /// Connector ID (e.g. "jira-main")
        #[arg(default_value = "jira-main")]
        id: String,
        /// Jira Instance URL (e.g. https://company.atlassian.net)
        #[arg(long)]
        url: Option<String>,
        /// User Email
        #[arg(long)]
        email: Option<String>,
        /// API Token
        #[arg(long)]
        token: Option<String>,
        /// Environment variable containing API Token
        #[arg(long)]
        token_env: Option<String>,
        /// Comma-separated project keys (replaces existing project list)
        #[arg(long)]
        projects: Option<String>,
        /// Add comma-separated project keys to existing list without overwriting credentials
        #[arg(long)]
        add_projects: Option<String>,
    },
    /// Configure Confluence connector
    Confluence {
        /// Connector ID (e.g. "confluence-docs")
        #[arg(default_value = "confluence-docs")]
        id: String,
        /// Confluence Instance URL
        #[arg(long)]
        url: Option<String>,
        /// User Email
        #[arg(long)]
        email: Option<String>,
        /// API Token
        #[arg(long)]
        token: Option<String>,
        /// Environment variable containing API Token
        #[arg(long)]
        token_env: Option<String>,
        /// Comma-separated space keys (replaces existing space list)
        #[arg(long)]
        spaces: Option<String>,
        /// Add comma-separated space keys to existing list without overwriting credentials
        #[arg(long)]
        add_spaces: Option<String>,
    },
    /// Configure GitHub connector
    Github {
        /// Connector ID (e.g. "github-main")
        #[arg(default_value = "github-main")]
        id: String,
        /// GitHub API Base URL [default: https://api.github.com]
        #[arg(long)]
        url: Option<String>,
        /// Personal Access Token
        #[arg(long)]
        token: Option<String>,
        /// Environment variable containing Personal Access Token
        #[arg(long)]
        token_env: Option<String>,
        /// Comma-separated repositories (replaces existing repo list)
        #[arg(long)]
        repos: Option<String>,
        /// Add comma-separated repositories to existing list without overwriting credentials
        #[arg(long)]
        add_repos: Option<String>,
    },
    /// Configure GitLab connector
    Gitlab {
        /// Connector ID (e.g. "gitlab-main")
        #[arg(default_value = "gitlab-main")]
        id: String,
        /// GitLab Base URL [default: https://gitlab.com]
        #[arg(long)]
        url: Option<String>,
        /// Personal Access Token
        #[arg(long)]
        token: Option<String>,
        /// Environment variable containing Personal Access Token
        #[arg(long)]
        token_env: Option<String>,
        /// Comma-separated projects/repos (e.g. "group/project")
        #[arg(long)]
        projects: Option<String>,
        /// Add comma-separated projects to existing list
        #[arg(long)]
        add_projects: Option<String>,
    },
    /// Configure ClickUp connector
    Clickup {
        /// Connector ID (e.g. "clickup-main")
        #[arg(default_value = "clickup-main")]
        id: String,
        /// ClickUp API Base URL [default: https://api.clickup.com/api/v2]
        #[arg(long)]
        url: Option<String>,
        /// Personal API Token
        #[arg(long)]
        token: Option<String>,
        /// Environment variable containing API Token
        #[arg(long)]
        token_env: Option<String>,
        /// ClickUp Workspace ID. Empty syncs all workspaces authorized for the token.
        #[arg(long)]
        workspace: Option<String>,
        /// Comma-separated space keys or IDs
        #[arg(long)]
        spaces: Option<String>,
        /// Comma-separated list keys or IDs
        #[arg(long)]
        lists: Option<String>,
    },
    /// Configure Notion connector
    Notion {
        /// Connector ID (e.g. "notion-docs")
        #[arg(default_value = "notion-docs")]
        id: String,
        /// Notion Internal Integration Secret / Token
        #[arg(long)]
        token: Option<String>,
        /// Environment variable containing Notion Integration Token
        #[arg(long)]
        token_env: Option<String>,
        /// Comma-separated database IDs
        #[arg(long)]
        database_ids: Option<String>,
        /// Comma-separated page IDs
        #[arg(long)]
        page_ids: Option<String>,
    },
    /// Configure Linear connector
    Linear {
        /// Connector ID (e.g. "linear-main")
        #[arg(default_value = "linear-main")]
        id: String,
        /// Linear API endpoint [default: https://api.linear.app/graphql]
        #[arg(long)]
        url: Option<String>,
        /// Linear API Key
        #[arg(long)]
        token: Option<String>,
        /// Environment variable containing Linear API Key
        #[arg(long)]
        token_env: Option<String>,
        /// Enable or disable comment thread syncing [default: true]
        #[arg(long)]
        sync_comments: Option<bool>,
    },
    /// Configure OpenAPI / Swagger connector
    Openapi {
        /// Connector ID (e.g. "openapi-specs")
        #[arg(default_value = "openapi-specs")]
        id: String,
        /// Primary file path or URL to OpenAPI spec (JSON or YAML)
        #[arg(long)]
        path: Option<String>,
        /// Comma-separated paths or URLs to OpenAPI specs
        #[arg(long)]
        paths: Option<String>,
        /// Add comma-separated paths or URLs
        #[arg(long)]
        add_paths: Option<String>,
    },
    /// Configure Asana connector
    Asana {
        /// Connector ID (e.g. "asana-main")
        #[arg(default_value = "asana-main")]
        id: String,
        /// Personal Access Token
        #[arg(long)]
        token: Option<String>,
        /// Environment variable containing Personal Access Token
        #[arg(long)]
        token_env: Option<String>,
        /// Asana Workspace ID
        #[arg(long)]
        workspace: Option<String>,
        /// Comma-separated project GIDs
        #[arg(long)]
        projects: Option<String>,
    },
    /// Configure Azure DevOps connector
    AzureDevops {
        /// Connector ID (e.g. "ado-main")
        #[arg(default_value = "ado-main")]
        id: String,
        /// Azure DevOps instance URL (e.g. https://dev.azure.com)
        #[arg(long)]
        url: Option<String>,
        /// Personal Access Token (PAT)
        #[arg(long)]
        token: Option<String>,
        /// Environment variable containing PAT
        #[arg(long)]
        token_env: Option<String>,
        /// Organization name
        #[arg(long)]
        org: Option<String>,
        /// Comma-separated project names
        #[arg(long)]
        projects: Option<String>,
    },
    /// Configure Bitbucket connector
    Bitbucket {
        /// Connector ID (e.g. "bitbucket-main")
        #[arg(default_value = "bitbucket-main")]
        id: String,
        /// Bitbucket Base URL [default: https://api.bitbucket.org/2.0]
        #[arg(long)]
        url: Option<String>,
        /// Bitbucket Username
        #[arg(long)]
        username: Option<String>,
        /// App Password / Token
        #[arg(long)]
        token: Option<String>,
        /// Environment variable containing App Password
        #[arg(long)]
        token_env: Option<String>,
        /// Workspace name / slug
        #[arg(long)]
        workspace: Option<String>,
        /// Comma-separated repository slugs
        #[arg(long)]
        repos: Option<String>,
    },
    /// Configure Figma connector
    Figma {
        /// Connector ID (e.g. "figma-designs")
        #[arg(default_value = "figma-designs")]
        id: String,
        /// Figma Personal Access Token
        #[arg(long)]
        token: Option<String>,
        /// Environment variable containing Figma PAT
        #[arg(long)]
        token_env: Option<String>,
        /// Comma-separated Figma file keys
        #[arg(long)]
        file_keys: Option<String>,
    },
    /// Configure Markdown connector
    Markdown {
        /// Connector ID (e.g. "markdown-docs")
        #[arg(default_value = "markdown-docs")]
        id: String,
        /// Primary directory path
        #[arg(long)]
        path: Option<String>,
        /// Comma-separated directory paths
        #[arg(long)]
        paths: Option<String>,
        /// Add comma-separated directory paths to existing list
        #[arg(long)]
        add_paths: Option<String>,
        /// Comma-separated glob patterns (e.g. "*.md,*.markdown")
        #[arg(long)]
        glob_patterns: Option<String>,
    },
    /// Configure Local Git connector
    LocalGit {
        /// Connector ID (e.g. "local-git-main")
        #[arg(default_value = "local-git-main")]
        id: String,
        /// Primary repository root path
        #[arg(long)]
        path: Option<String>,
        /// Comma-separated repository root paths
        #[arg(long)]
        paths: Option<String>,
        /// Add comma-separated repository paths to existing list
        #[arg(long)]
        add_paths: Option<String>,
    },
}

pub(crate) fn parse_mcp_args(raw_args: &[String]) -> Vec<String> {
    let mut parsed = Vec::new();
    for item in raw_args {
        if item.contains(',') {
            for part in item.split(',') {
                let trimmed = part.trim();
                if !trimmed.is_empty() {
                    parsed.push(trimmed.to_string());
                }
            }
        } else if item.contains(' ') {
            for part in item.split_whitespace() {
                let trimmed = part.trim();
                if !trimmed.is_empty() {
                    parsed.push(trimmed.to_string());
                }
            }
        } else if !item.trim().is_empty() {
            parsed.push(item.trim().to_string());
        }
    }
    parsed
}

pub(crate) fn parse_mcp_env(raw_env: &[String]) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for item in raw_env {
        for part in item.split(',') {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some((k, v)) = trimmed.split_once('=') {
                map.insert(k.trim().to_string(), v.trim().to_string());
            } else {
                eprintln!("Warning: Invalid env format '{}', expected KEY=VAL", trimmed);
            }
        }
    }
    map
}
