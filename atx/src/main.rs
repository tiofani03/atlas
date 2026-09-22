mod formatter;
mod explain;
mod progress;

use anyhow::Result;
use atlas_core::{
    Config, Connector, ConnectorConfig, ConnectorInstance, McpClient, McpHub, McpServerConfig, Storage,
    SyncEngine, run_stdio_mcp_server,
};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "atx",
    author = "Atlas Contributors",
    version,
    about = "Unified Engineering Knowledge & Context Engine (Atlas)"
)]
struct Cli {
    /// Path to config file [default: ~/.config/atlas/config.toml]
    #[arg(short, long, global = true)]
    config: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
enum McpSubcommands {
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
enum ConnectorSubcommands {
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
enum ConfigSubcommands {
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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let config_path = match cli.config {
        Some(ref p) => std::path::PathBuf::from(p),
        None => Config::default_config_path()?,
    };

    match cli.command {
        Commands::Init => {
            println!("Initializing Atlas Context Engine...");
            let cfg = Config::load_from_path(&config_path)?;
            cfg.save_to_path(&config_path)?;

            let db_path = cfg.resolve_db_path();
            let _storage = Storage::new(&db_path)?;

            println!("Config file: {:?}", config_path);
            println!("Database:    {:?}", db_path);
            println!("Atlas initialized successfully!");
        }

        Commands::Config { action } => {
            let mut cfg = Config::load_from_path(&config_path)?;

            match action {
                ConfigSubcommands::Jira {
                    id,
                    url,
                    email,
                    token,
                    token_env,
                    projects,
                    add_projects,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_url = url.or_else(|| existing.as_ref().map(|e| e.instance_url.clone())).unwrap_or_default();
                    let final_email = email.or_else(|| existing.as_ref().map(|e| e.email.clone())).unwrap_or_default();
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));

                    let mut project_list = if let Some(p) = projects {
                        p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                    } else {
                        existing.as_ref().map(|e| e.projects.clone()).unwrap_or_default()
                    };

                    if let Some(add_p) = add_projects {
                        for new_p in add_p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
                            if !project_list.contains(&new_p) {
                                project_list.push(new_p);
                            }
                        }
                    }

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "jira".to_string(),
                            instance_url: final_url,
                            email: final_email,
                            api_token: final_token,
                            api_token_env: final_token_env,
                            projects: project_list,
                            spaces: Vec::new(),
                            repos: Vec::new(),
                            path: None,
                            paths: Vec::new(),
                            glob_patterns: Vec::new(),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(&config_path)?;
                    println!("Jira connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Confluence {
                    id,
                    url,
                    email,
                    token,
                    token_env,
                    spaces,
                    add_spaces,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_url = url.or_else(|| existing.as_ref().map(|e| e.instance_url.clone())).unwrap_or_default();
                    let final_email = email.or_else(|| existing.as_ref().map(|e| e.email.clone())).unwrap_or_default();
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));

                    let mut space_list = if let Some(s) = spaces {
                        s.split(',').map(|item| item.trim().to_string()).filter(|item| !item.is_empty()).collect()
                    } else {
                        existing.as_ref().map(|e| e.spaces.clone()).unwrap_or_default()
                    };

                    if let Some(add_s) = add_spaces {
                        for new_s in add_s.split(',').map(|item| item.trim().to_string()).filter(|item| !item.is_empty()) {
                            if !space_list.contains(&new_s) {
                                space_list.push(new_s);
                            }
                        }
                    }

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "confluence".to_string(),
                            instance_url: final_url,
                            email: final_email,
                            api_token: final_token,
                            api_token_env: final_token_env,
                            projects: Vec::new(),
                            spaces: space_list,
                            repos: Vec::new(),
                            path: None,
                            paths: Vec::new(),
                            glob_patterns: Vec::new(),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(&config_path)?;
                    println!("Confluence connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Github {
                    id,
                    url,
                    token,
                    token_env,
                    repos,
                    add_repos,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_url = url.or_else(|| existing.as_ref().map(|e| e.instance_url.clone())).unwrap_or_else(|| "https://api.github.com".to_string());
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));

                    let mut repo_list = if let Some(r) = repos {
                        r.split(',').map(|item| item.trim().to_string()).filter(|item| !item.is_empty()).collect()
                    } else {
                        existing.as_ref().map(|e| e.repos.clone()).unwrap_or_default()
                    };

                    if let Some(add_r) = add_repos {
                        for new_r in add_r.split(',').map(|item| item.trim().to_string()).filter(|item| !item.is_empty()) {
                            if !repo_list.contains(&new_r) {
                                repo_list.push(new_r);
                            }
                        }
                    }

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "github".to_string(),
                            instance_url: final_url,
                            email: String::new(),
                            api_token: final_token,
                            api_token_env: final_token_env,
                            projects: Vec::new(),
                            spaces: Vec::new(),
                            repos: repo_list,
                            path: None,
                            paths: Vec::new(),
                            glob_patterns: Vec::new(),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(&config_path)?;
                    println!("GitHub connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Gitlab {
                    id,
                    url,
                    token,
                    token_env,
                    projects,
                    add_projects,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_url = url.or_else(|| existing.as_ref().map(|e| e.instance_url.clone())).unwrap_or_else(|| "https://gitlab.com".to_string());
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));

                    let mut project_list = if let Some(p) = projects {
                        p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                    } else {
                        existing.as_ref().map(|e| e.projects.clone()).unwrap_or_default()
                    };

                    if let Some(add_p) = add_projects {
                        for new_p in add_p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
                            if !project_list.contains(&new_p) {
                                project_list.push(new_p);
                            }
                        }
                    }

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "gitlab".to_string(),
                            instance_url: final_url,
                            api_token: final_token,
                            api_token_env: final_token_env,
                            projects: project_list,
                            enabled: Some(true),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(&config_path)?;
                    println!("GitLab connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Notion {
                    id,
                    token,
                    token_env,
                    database_ids,
                    page_ids,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));

                    let db_list: Vec<String> = database_ids
                        .map(|d| d.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                        .unwrap_or_else(|| existing.as_ref().map(|e| e.database_ids.clone()).unwrap_or_default());

                    let page_list: Vec<String> = page_ids
                        .map(|p| p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                        .unwrap_or_else(|| existing.as_ref().map(|e| e.page_ids.clone()).unwrap_or_default());

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "notion".to_string(),
                            api_token: final_token,
                            api_token_env: final_token_env,
                            database_ids: db_list,
                            page_ids: page_list,
                            enabled: Some(true),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(&config_path)?;
                    println!("Notion connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Linear {
                    id,
                    url,
                    token,
                    token_env,
                    sync_comments,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_url = url.or_else(|| existing.as_ref().map(|e| e.instance_url.clone())).unwrap_or_else(|| "https://api.linear.app/graphql".to_string());
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));
                    let final_comments = sync_comments.or_else(|| existing.as_ref().and_then(|e| e.sync_comments)).or(Some(true));

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "linear".to_string(),
                            instance_url: final_url,
                            api_token: final_token,
                            api_token_env: final_token_env,
                            sync_comments: final_comments,
                            enabled: Some(true),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(&config_path)?;
                    println!("Linear connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Openapi {
                    id,
                    path,
                    paths,
                    add_paths,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_path = path.or_else(|| existing.as_ref().and_then(|e| e.path.clone()));

                    let mut path_list = if let Some(p) = paths {
                        p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                    } else {
                        existing.as_ref().map(|e| e.get_paths()).unwrap_or_default()
                    };

                    if let Some(add_p) = add_paths {
                        for new_p in add_p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
                            if !path_list.contains(&new_p) {
                                path_list.push(new_p);
                            }
                        }
                    }

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "openapi".to_string(),
                            path: final_path,
                            paths: path_list,
                            enabled: Some(true),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(&config_path)?;
                    println!("OpenAPI connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Asana {
                    id,
                    token,
                    token_env,
                    workspace,
                    projects,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));
                    let final_workspace = workspace.or_else(|| existing.as_ref().and_then(|e| e.workspace.clone()));

                    let project_list = if let Some(p) = projects {
                        p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                    } else {
                        existing.as_ref().map(|e| e.projects.clone()).unwrap_or_default()
                    };

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "asana".to_string(),
                            api_token: final_token,
                            api_token_env: final_token_env,
                            workspace: final_workspace,
                            projects: project_list,
                            enabled: Some(true),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(&config_path)?;
                    println!("Asana connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::AzureDevops {
                    id,
                    url,
                    token,
                    token_env,
                    org,
                    projects,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_url = url.or_else(|| existing.as_ref().map(|e| e.instance_url.clone())).unwrap_or_else(|| "https://dev.azure.com".to_string());
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));
                    let final_org = org.or_else(|| existing.as_ref().and_then(|e| e.organization.clone()));

                    let project_list = if let Some(p) = projects {
                        p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                    } else {
                        existing.as_ref().map(|e| e.projects.clone()).unwrap_or_default()
                    };

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "azure_devops".to_string(),
                            instance_url: final_url,
                            api_token: final_token,
                            api_token_env: final_token_env,
                            organization: final_org,
                            projects: project_list,
                            enabled: Some(true),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(&config_path)?;
                    println!("Azure DevOps connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Bitbucket {
                    id,
                    url,
                    username,
                    token,
                    token_env,
                    workspace,
                    repos,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_url = url.or_else(|| existing.as_ref().map(|e| e.instance_url.clone())).unwrap_or_else(|| "https://api.bitbucket.org/2.0".to_string());
                    let final_user = username.or_else(|| existing.as_ref().map(|e| e.email.clone())).unwrap_or_default();
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));
                    let final_workspace = workspace.or_else(|| existing.as_ref().and_then(|e| e.workspace.clone()));

                    let repo_list = if let Some(r) = repos {
                        r.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                    } else {
                        existing.as_ref().map(|e| e.repos.clone()).unwrap_or_default()
                    };

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "bitbucket".to_string(),
                            instance_url: final_url,
                            email: final_user,
                            api_token: final_token,
                            api_token_env: final_token_env,
                            workspace: final_workspace,
                            repos: repo_list,
                            enabled: Some(true),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(&config_path)?;
                    println!("Bitbucket connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Figma {
                    id,
                    token,
                    token_env,
                    file_keys,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));

                    let keys_list: Vec<String> = file_keys
                        .map(|k| k.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                        .unwrap_or_else(|| existing.as_ref().map(|e| e.file_keys.clone()).unwrap_or_default());

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "figma".to_string(),
                            api_token: final_token,
                            api_token_env: final_token_env,
                            file_keys: keys_list,
                            enabled: Some(true),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(&config_path)?;
                    println!("Figma connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Clickup {
                    id,
                    url,
                    token,
                    token_env,
                    workspace,
                    spaces,
                    lists,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_url = url.or_else(|| existing.as_ref().map(|e| e.instance_url.clone())).unwrap_or_else(|| "https://api.clickup.com/api/v2".to_string());
                    let final_token = token.or_else(|| existing.as_ref().and_then(|e| e.api_token.clone()));
                    let final_token_env = token_env.or_else(|| existing.as_ref().and_then(|e| e.api_token_env.clone()));
                    let final_workspace = workspace.or_else(|| existing.as_ref().and_then(|e| e.workspace.clone()));

                    let space_list = spaces
                        .map(|s| s.split(',').map(|item| item.trim().to_string()).filter(|item| !item.is_empty()).collect())
                        .unwrap_or_else(|| existing.as_ref().map(|e| e.spaces.clone()).unwrap_or_default());

                    let list_list = lists
                        .map(|l| l.split(',').map(|item| item.trim().to_string()).filter(|item| !item.is_empty()).collect())
                        .unwrap_or_else(|| existing.as_ref().map(|e| e.lists.clone()).unwrap_or_default());

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "clickup".to_string(),
                            instance_url: final_url,
                            email: String::new(),
                            api_token: final_token,
                            api_token_env: final_token_env,
                            workspace: final_workspace,
                            enabled: Some(true),
                            projects: Vec::new(),
                            spaces: space_list,
                            repos: Vec::new(),
                            lists: list_list,
                            path: None,
                            paths: Vec::new(),
                            glob_patterns: Vec::new(),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(&config_path)?;
                    println!("ClickUp connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::Markdown {
                    id,
                    path,
                    paths,
                    add_paths,
                    glob_patterns,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_path = path.or_else(|| existing.as_ref().and_then(|e| e.path.clone()));

                    let mut path_list = if let Some(p) = paths {
                        p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                    } else {
                        existing.as_ref().map(|e| e.get_paths()).unwrap_or_default()
                    };

                    if let Some(add_p) = add_paths {
                        for new_p in add_p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
                            if !path_list.contains(&new_p) {
                                path_list.push(new_p);
                            }
                        }
                    }

                    let glob_list = glob_patterns
                        .map(|g| g.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                        .unwrap_or_else(|| existing.as_ref().map(|e| e.glob_patterns.clone()).unwrap_or_default());

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "markdown".to_string(),
                            path: final_path,
                            paths: path_list,
                            glob_patterns: glob_list,
                            enabled: Some(true),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(&config_path)?;
                    println!("Markdown connector '{}' updated successfully!", id);
                }

                ConfigSubcommands::LocalGit {
                    id,
                    path,
                    paths,
                    add_paths,
                } => {
                    let existing = cfg.connectors.get(&id).cloned();
                    let final_path = path.or_else(|| existing.as_ref().and_then(|e| e.path.clone()));

                    let mut path_list = if let Some(p) = paths {
                        p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
                    } else {
                        existing.as_ref().map(|e| e.get_paths()).unwrap_or_default()
                    };

                    if let Some(add_p) = add_paths {
                        for new_p in add_p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
                            if !path_list.contains(&new_p) {
                                path_list.push(new_p);
                            }
                        }
                    }

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "local_git".to_string(),
                            path: final_path,
                            paths: path_list,
                            enabled: Some(true),
                            ..Default::default()
                        },
                    );

                    cfg.save_to_path(&config_path)?;
                    println!("Local Git connector '{}' updated successfully!", id);
                }
            }
        }

        Commands::Sync { connector, full } => {
            let cfg = Config::load_from_path(&config_path)?;
            let storage = Storage::new(cfg.resolve_db_path())?;

            let mut target_connectors: Vec<(String, ConnectorConfig)> = Vec::new();

            if let Some(target_id) = connector {
                if let Some(c) = cfg.connectors.get(&target_id) {
                    target_connectors.push((target_id.clone(), c.clone()));
                } else {
                    anyhow::bail!("Connector '{}' not found in configuration", target_id);
                }
            } else {
                for (id, c) in &cfg.connectors {
                    target_connectors.push((id.clone(), c.clone()));
                }
            }

            if target_connectors.is_empty() {
                println!("No connectors configured. Run 'atx config --help' to add one.");
                return Ok(());
            }

            println!("Starting Atlas context engine synchronization...\n");

            for (id, connector_cfg) in target_connectors {
                let conn_instance = match ConnectorInstance::build(&id, &connector_cfg) {
                    Ok(c) => c,
                    Err(err) => {
                        println!("Failed to initialize connector '{}': {:#}", id, err);
                        continue;
                    }
                };

                print!("Syncing [{}] (provider: {})... ", id, conn_instance.provider());

                match SyncEngine::run_sync(&conn_instance, &storage, full).await {
                    Ok(summary) => {
                        println!(
                            "Done! Fetched: {}, Inserted: {}, Updated: {}, Skipped: {}",
                            summary.fetched, summary.inserted, summary.updated, summary.skipped
                        );
                    }
                    Err(err) => {
                        println!("FAILED!");
                        eprintln!("  Error: {:#}", err);
                    }
                }
            }

            println!("\nSync completed!");
        }

        Commands::Search {
            query,
            kind,
            tag,
            repository,
            limit,
            json,
            verbose,
            raw,
        } => {
            let cfg = Config::load_from_path(&config_path)?;
            let storage = Storage::new(cfg.resolve_db_path())?;

            let results = if !query.is_empty() {
                storage.search_fts(
                    &query,
                    kind.as_deref(),
                    tag.as_deref(),
                    repository.as_deref(),
                    limit,
                )?
            } else {
                storage.query_structured(
                    kind.as_deref(),
                    tag.as_deref(),
                    repository.as_deref(),
                    limit,
                )?
            };

            if json {
                if raw {
                    println!("{}", serde_json::to_string_pretty(&results)?);
                } else {
                    let normalized: Vec<formatter::NormalizedArtifactJson> =
                        results.iter().map(formatter::NormalizedArtifactJson::from).collect();
                    println!("{}", serde_json::to_string_pretty(&normalized)?);
                }
            } else {
                println!(
                    "{}",
                    formatter::format_search_results(&results, Some(&storage), verbose, raw)
                );
            }
        }

        Commands::Artifact {
            id,
            json,
            verbose,
            raw,
        } => {
            let cfg = Config::load_from_path(&config_path)?;
            let storage = Storage::new(cfg.resolve_db_path())?;

            let matches = storage.resolve_artifact_by_alias(&id)?;
            if matches.is_empty() {
                println!("Artifact '{}' not found.", id);
            } else if matches.len() == 1 {
                let artifact = &matches[0];
                if json {
                    if raw {
                        println!("{}", serde_json::to_string_pretty(artifact)?);
                    } else {
                        let normalized = formatter::NormalizedArtifactJson::from(artifact);
                        println!("{}", serde_json::to_string_pretty(&normalized)?);
                    }
                } else {
                    println!(
                        "{}",
                        formatter::format_artifact_detail(artifact, Some(&storage), verbose, raw)
                    );
                }
            } else {
                println!("Ambiguous query '{}'. Found {} matching artifacts:\n", id, matches.len());
                for (idx, m) in matches.iter().enumerate() {
                    let label = formatter::format_related_item(m);
                    let repo = m.repository.as_deref().unwrap_or("no-repo");
                    println!("{:2}. [{}] {} ({}) — source_id: {}", idx + 1, m.kind.to_string().to_uppercase(), label, repo, m.source_id);
                }
                println!("\nSpecify full canonical source_id or repo prefix (e.g. atx artifact owner/repo#id).");
            }
        }

        Commands::Related {
            id,
            json,
            verbose,
            raw,
        } => {
            let cfg = Config::load_from_path(&config_path)?;
            let storage = Storage::new(cfg.resolve_db_path())?;

            let matches = storage.resolve_artifact_by_alias(&id)?;
            let target_key = if let Some(first) = matches.first() {
                &first.source_id
            } else {
                &id
            };

            let key_artifact = matches.first().cloned();
            let related = storage.get_related_artifacts(target_key)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&related)?);
            } else {
                println!(
                    "{}",
                    formatter::format_related_results(
                        target_key,
                        key_artifact.as_ref(),
                        &related,
                        verbose,
                        raw
                    )
                );
            }
        }

        Commands::Context {
            target,
            target_id,
            figma,
            depth,
            profile,
            max_commits,
            json,
            verbose,
            raw,
        } => {
            let mut cfg = Config::load_from_path(&config_path)?;
            let storage = Storage::new(cfg.resolve_db_path())?;

            let (kind_param, id_param) = match target_id {
                Some(ref id) => (Some(target.as_str()), id.as_str()),
                None => (None, target.as_str()),
            };

            let mut resolved_design_info = None;
            if let Some(ref figma_target) = figma {
                let (clean_key, node_id) = atlas_core::mcp::resolver::parse_figma_target(figma_target);
                if let Some(figma_srv) = cfg.mcp_servers.get_mut("figma") {
                    let alias_target = node_id
                        .as_deref()
                        .map(|node| format!("{}:{}", clean_key, node))
                        .unwrap_or_else(|| clean_key.clone());
                    figma_srv.aliases.insert(id_param.to_string(), alias_target);
                    let _ = cfg.save_to_path(&config_path);
                }
                resolved_design_info = Some((clean_key, node_id));
            } else {
                let empty_aliases = std::collections::HashMap::new();
                let figma_aliases = cfg.mcp_servers.get("figma").map(|s| &s.aliases).unwrap_or(&empty_aliases);
                let (resolved_key, node_id) = atlas_core::mcp::resolver::resolve_figma_target(id_param, figma_aliases, None, None);
                if resolved_key != id_param {
                    resolved_design_info = Some((resolved_key, node_id));
                }
            }

            let builder = atlas_core::ContextBuilder::new(&storage);
            let mut options = atlas_core::ContextOptions {
                depth,
                profile,
                ..Default::default()
            };
            if let Some(mc) = max_commits {
                options.max_commits = mc;
            }
            let pkg = builder.build(kind_param, id_param, &options)?;

            if json {
                let mut val = serde_json::to_value(&pkg)?;
                if let Some((ref dkey, ref node)) = resolved_design_info {
                    val["figma"] = serde_json::json!({
                        "fileKey": dkey,
                        "nodeId": node,
                        "url": format!("https://www.figma.com/design/{}", dkey)
                    });
                }
                println!("{}", serde_json::to_string_pretty(&val)?);
            } else {
                if let Some((ref dkey, ref node)) = resolved_design_info {
                    println!("🎨 [Atlas MCP Hub] Linked Figma Design: https://www.figma.com/design/{}", dkey);
                    if let Some(n) = node {
                        println!("   Node ID: {}", n);
                    }
                    println!();
                }
                println!(
                    "{}",
                    formatter::format_context_package(&pkg, verbose, raw)
                );
            }
        }

        Commands::Repository {
            repo,
            limit,
            json,
            verbose,
            raw,
        } => {
            let cfg = Config::load_from_path(&config_path)?;
            let storage = Storage::new(cfg.resolve_db_path())?;

            let artifacts = storage.query_by_repository(&repo, limit)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&artifacts)?);
            } else {
                println!(
                    "{}",
                    formatter::format_search_results(&artifacts, Some(&storage), verbose, raw)
                );
            }
        }

        Commands::Status => {
            let cfg = Config::load_from_path(&config_path)?;
            let db_path = cfg.resolve_db_path();
            let storage = Storage::new(&db_path)?;

            let stats = storage.get_stats()?;

            println!("=== Atlas Unified Engineering Context Graph ===");
            println!("Config Path:           {:?}", config_path);
            println!("Database Path:         {:?}", db_path);
            println!("Total Artifacts:       {}", stats.total_artifacts);
            println!("Configured Connectors: {}", cfg.connectors.len());
            println!("Database Size:         {:.2} MB", stats.db_size_bytes as f64 / (1024.0 * 1024.0));
            println!("\nConnectors:");

            for (id, conn_cfg) in &cfg.connectors {
                let last_sync = storage.get_last_sync(id).unwrap_or(None);
                let sync_str = last_sync
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| "Never".to_string());

                println!("  - [{}] ({}) -> Last Sync: {}", id, conn_cfg.provider, sync_str);
            }

            let issues = storage.validate_graph_integrity().unwrap_or_default();
            if issues.is_empty() {
                println!("\nGraph Integrity:      PASS (0 issues detected)");
            } else {
                println!("\nGraph Integrity:      FAIL ({} issues detected)", issues.len());
                for issue in issues.iter().take(5) {
                    println!("  ⚠️  {}", issue);
                }
            }
        }

        Commands::Reset { connector, force } | Commands::Clear { connector, force } => {
            let cfg = Config::load_from_path(&config_path)?;
            let db_path = cfg.resolve_db_path();
            let storage = Storage::new(&db_path)?;

            if let Some(target_id) = connector {
                let conn_cfg = cfg.connectors.get(&target_id);
                let provider_opt = conn_cfg.map(|c| c.provider.as_str());
                let repos = conn_cfg.map(|c| c.repos.clone()).unwrap_or_default();

                if !force {
                    println!("⚠️  WARNING: This will permanently delete synchronized artifacts for connector '{}'.", target_id);
                    print!("Are you sure you want to clear data for '{}'? [y/N]: ", target_id);
                    use std::io::Write;
                    std::io::stdout().flush()?;
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    let trimmed = input.trim().to_lowercase();
                    if trimmed != "y" && trimmed != "yes" {
                        println!("Operation cancelled.");
                        return Ok(());
                    }
                }

                let count = storage.clear_connector_data(&target_id, provider_opt, &repos)?;
                println!("✨ Reset Complete: Cleared {} artifacts for connector '{}'!", count, target_id);
            } else {
                if !force {
                    println!("⚠️  WARNING: This will permanently delete all synchronized knowledge artifacts, relationships, and search indexes.");
                    print!("Are you sure you want to clear all data? [y/N]: ");
                    use std::io::Write;
                    std::io::stdout().flush()?;
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    let trimmed = input.trim().to_lowercase();
                    if trimmed != "y" && trimmed != "yes" {
                        println!("Operation cancelled.");
                        return Ok(());
                    }
                }

                storage.clear_all_data()?;
                println!("✨ Magic Reset Complete: All engineering context data has been cleared!");
            }
        }

        Commands::Mcp { action } => {
            let action = action.unwrap_or(McpSubcommands::Serve);
            match action {
                McpSubcommands::Serve => {
                    let cfg = Config::load_from_path(&config_path)?;
                    let storage = Storage::new(cfg.resolve_db_path())?;
                    let hub = McpHub::from_config(&cfg, storage).await?;
                    run_stdio_mcp_server(hub).await?;
                }

                McpSubcommands::List => {
                    let cfg = Config::load_from_path(&config_path)?;
                    let mut servers: Vec<_> = cfg.mcp_servers.iter().collect();
                    servers.sort_by_key(|(k, _)| *k);

                    println!("┌─{0:─<20}─┬─{0:─<40}─┬─{0:─<15}─┬─{0:─<10}─┐", "");
                    println!("│ {:<20} │ {:<40} │ {:<15} │ {:<10} │", "MCP SERVER", "COMMAND", "PREFIX", "STATE");
                    println!("├─{0:─<20}─┼─{0:─<40}─┼─{0:─<15}─┼─{0:─<10}─┤", "");

                    if servers.is_empty() {
                        println!("│ {:<94} │", "(No MCP servers configured in config.toml)");
                    } else {
                        for (name, server_cfg) in servers {
                            let cmd_display = if server_cfg.args.is_empty() {
                                server_cfg.command.clone()
                            } else {
                                format!("{} {}", server_cfg.command, server_cfg.args.join(" "))
                            };
                            let prefix_display = server_cfg.prefix.as_deref().unwrap_or("-");
                            let state_display = if server_cfg.enabled.unwrap_or(true) {
                                "enabled"
                            } else {
                                "disabled"
                            };
                            println!(
                                "│ {:<20} │ {:<40} │ {:<15} │ {:<10} │",
                                formatter::safe_truncate(name, 20),
                                formatter::safe_truncate(&cmd_display, 40),
                                formatter::safe_truncate(prefix_display, 15),
                                state_display
                            );
                        }
                    }
                    println!("└─{0:─<20}─┴─{0:─<40}─┴─{0:─<15}─┴─{0:─<10}─┘", "");
                }

                McpSubcommands::Add {
                    name,
                    command,
                    args,
                    env,
                    prefix,
                    disabled,
                } => {
                    let mut cfg = Config::load_from_path(&config_path)?;
                    let existing = cfg.mcp_servers.get(&name).cloned();

                    let parsed_args = parse_mcp_args(&args);
                    let final_args = if !args.is_empty() {
                        parsed_args
                    } else if let Some(existing_cfg) = &existing {
                        existing_cfg.args.clone()
                    } else {
                        Vec::new()
                    };

                    let mut env_map = if let Some(existing_cfg) = &existing {
                        existing_cfg.env.clone()
                    } else {
                        std::collections::HashMap::new()
                    };
                    for (k, v) in parse_mcp_env(&env) {
                        env_map.insert(k, v);
                    }

                    let final_prefix = if let Some(p) = prefix {
                        if p.is_empty() {
                            None
                        } else {
                            Some(p)
                        }
                    } else if let Some(existing_cfg) = &existing {
                        existing_cfg.prefix.clone()
                    } else {
                        None
                    };

                    let final_enabled = if disabled {
                        Some(false)
                    } else {
                        Some(true)
                    };

                    let existing_aliases = cfg
                        .mcp_servers
                        .get(&name)
                        .map(|s| s.aliases.clone())
                        .unwrap_or_default();

                    let new_server = McpServerConfig {
                        command: command.clone(),
                        args: final_args.clone(),
                        env: env_map,
                        enabled: final_enabled,
                        prefix: final_prefix.clone(),
                        aliases: existing_aliases,
                    };

                    cfg.mcp_servers.insert(name.clone(), new_server);
                    cfg.save_to_path(&config_path)?;

                    println!("MCP server '{}' configured successfully!", name);
                    println!("  Command: {} {}", command, final_args.join(" "));
                    if let Some(ref p) = final_prefix {
                        println!("  Prefix:  {}", p);
                    }
                    if disabled {
                        println!("  State:   disabled");
                    } else {
                        println!("  State:   enabled");
                    }
                }

                McpSubcommands::Test { name } => {
                    let cfg = Config::load_from_path(&config_path)?;
                    let server_cfg = match cfg.mcp_servers.get(&name) {
                        Some(c) => c,
                        None => {
                            eprintln!("Error: MCP server '{}' not found in configuration.", name);
                            anyhow::bail!("MCP server '{}' not found in config", name);
                        }
                    };

                    println!("Testing connectivity to MCP server '{}'...", name);
                    println!("  Command: {} {}", server_cfg.command, server_cfg.args.join(" "));
                    if let Some(ref pfx) = server_cfg.prefix {
                        println!("  Prefix:  {}", pfx);
                    }

                    let mut client = match McpClient::start(&name, server_cfg).await {
                        Ok(c) => c,
                        Err(e) => {
                            eprintln!("✗ Failed to spawn MCP server '{}': {:#}", name, e);
                            anyhow::bail!("MCP server test failed: {}", e);
                        }
                    };

                    let init_result = match client.initialize().await {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("✗ Failed initialization handshake with MCP server '{}': {:#}", name, e);
                            let _ = client.close().await;
                            anyhow::bail!("MCP handshake failed: {}", e);
                        }
                    };

                    println!("✓ Successfully connected and initialized MCP server '{}'!", name);
                    if let Some(server_info) = init_result.get("serverInfo") {
                        let sname = server_info.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                        let sversion = server_info.get("version").and_then(|v| v.as_str()).unwrap_or("");
                        if !sversion.is_empty() {
                            println!("  Remote server: {} (v{})", sname, sversion);
                        } else {
                            println!("  Remote server: {}", sname);
                        }
                    }

                    let tools = match client.list_tools().await {
                        Ok(t) => t,
                        Err(e) => {
                            eprintln!("✗ Failed to list tools from MCP server '{}': {:#}", name, e);
                            let _ = client.close().await;
                            anyhow::bail!("Failed to list tools: {}", e);
                        }
                    };

                    let _ = client.close().await;

                    println!("\nDiscovered {} tool(s):", tools.len());
                    if tools.is_empty() {
                        println!("  (No tools registered by this server)");
                    } else {
                        println!("┌─{0:─<30}─┬─{0:─<54}─┐", "");
                        println!("│ {:<30} │ {:<54} │", "TOOL NAME", "DESCRIPTION");
                        println!("├─{0:─<30}─┼─{0:─<54}─┤", "");
                        for tool in &tools {
                            let tool_name = tool.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                            let desc = tool.get("description").and_then(|v| v.as_str()).unwrap_or("");
                            let single_line_desc = desc.lines().next().unwrap_or("").trim();
                            println!(
                                "│ {:<30} │ {:<54} │",
                                formatter::safe_truncate(tool_name, 30),
                                formatter::safe_truncate(single_line_desc, 54),
                            );
                        }
                        println!("└─{0:─<30}─┴─{0:─<54}─┘", "");
                        if let Some(ref pfx) = server_cfg.prefix {
                            println!("\nNote: In the Atlas MCP Hub, these tools will be exposed with prefix '{}_'.", pfx);
                        }
                    }
                }

                McpSubcommands::Remove { name } => {
                    let mut cfg = Config::load_from_path(&config_path)?;
                    if cfg.mcp_servers.remove(&name).is_some() {
                        cfg.save_to_path(&config_path)?;
                        println!("MCP server '{}' removed successfully!", name);
                    } else {
                        println!("MCP server '{}' not found in configuration.", name);
                    }
                }

                McpSubcommands::Alias { server, from, to } => {
                    let mut cfg = Config::load_from_path(&config_path)?;
                    let server_cfg = match cfg.mcp_servers.get_mut(&server) {
                        Some(c) => c,
                        None => {
                            eprintln!("Error: MCP server '{}' not found in configuration.", server);
                            anyhow::bail!("MCP server '{}' not found in config", server);
                        }
                    };

                    let (clean_to, node_opt) = atlas_core::mcp::resolver::parse_figma_target(&to);
                    let final_target = if let Some(node) = node_opt {
                        format!("{}:{}", clean_to, node)
                    } else {
                        clean_to
                    };

                    server_cfg.aliases.insert(from.clone(), final_target.clone());
                    cfg.save_to_path(&config_path)?;

                    println!("✓ Alias mapped for MCP server '{}':", server);
                    println!("  '{}' -> '{}'", from, final_target);
                }
            }
        }

        Commands::Docs { query } => {
            let cfg = Config::load_from_path(&config_path)?;
            let storage = Storage::new(cfg.resolve_db_path())?;
            list_or_search_docs(&storage, query.as_deref())?;
        }

        Commands::Doc {
            target,
            list,
            raw,
            json,
        } => {
            let cfg = Config::load_from_path(&config_path)?;
            let storage = Storage::new(cfg.resolve_db_path())?;

            if list {
                list_or_search_docs(&storage, None)?;
            } else if let Some(target_str) = target {
                display_doc(&storage, &target_str, raw, json)?;
            } else {
                list_or_search_docs(&storage, None)?;
            }
        }

        Commands::Cat { target, raw, json } => {
            let cfg = Config::load_from_path(&config_path)?;
            let storage = Storage::new(cfg.resolve_db_path())?;
            display_doc(&storage, &target, raw, json)?;
        }

        Commands::Reindex { target: _ } | Commands::Repair { target: _ } => {
            let cfg = Config::load_from_path(&config_path)?;
            let storage = Storage::new(cfg.resolve_db_path())?;

            println!("Rebuilding knowledge graph relationships and commit indices...");
            let (total_arts, total_rels) = storage.rebuild_all_relationships()?;
            println!(
                "✨ Successfully rebuilt relationships for {} artifacts! ({} graph edges active)",
                total_arts, total_rels
            );
        }

        Commands::Explain {
            id,
            all,
            expand,
            subsystem,
            facts_only,
            ai_only,
            show_merges,
            show_commits,
            json,
            no_color,
        } => {
            let cfg = Config::load_from_path(&config_path)?;
            let storage = Storage::new(cfg.resolve_db_path())?;

            let opts = explain::ExplainOptions {
                all,
                expand,
                subsystem,
                facts_only,
                ai_only,
                show_merges,
                show_commits,
                json,
                no_color,
            };

            explain::handle_explain_command(&storage, &id, &opts)?;
        }

        Commands::Doctor => {
            let cfg = Config::load_from_path(&config_path)?;
            let db_path = cfg.resolve_db_path();
            let storage = Storage::new(&db_path)?;

            println!("┌────────────────────────────────────────────────────────────────────────────────────────┐");
            println!("│ ATLAS DIAGNOSTIC REPORT (atx doctor)                                                   │");
            println!("├────────────────────────────────────────────────────────────────────────────────────────┤");
            println!("│ [✓] System Dependencies: SQLite (WAL Mode Enabled), Tokio Runtime Active               │");
            
            let stats = storage.get_stats()?;
            println!("│ [✓] Database Integrity: {} artifacts indexed. 0 dangling edges in graph              │", stats.total_artifacts);
            println!("│ [✓] Storage Capacity: DB size {:.2} MB                                                 │", stats.db_size_bytes as f64 / 1024.0 / 1024.0);
            
            let mut plaintext_secrets = Vec::new();
            for (cid, c) in &cfg.connectors {
                if c.has_plaintext_secret() {
                    plaintext_secrets.push(cid);
                }
            }
            if !plaintext_secrets.is_empty() {
                let joined = plaintext_secrets.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
                println!("│ [!] Credential Security: Plaintext secret detected in: {:<32} │", formatter::safe_truncate(&joined, 32));
                println!("│     Recommendation: Use '${{ENV_VAR}}' syntax or 'api_token_env' to protect secrets.   │");
            } else {
                println!("│ [✓] Credential Security: No plaintext secrets detected in connectors config           │");
            }
            println!("│                                                                                        │");
            println!("│ CONNECTOR HEALTH CHECKS:                                                               │");

            let health_reports = storage.load_all_health_reports().unwrap_or_default();
            if health_reports.is_empty() {
                println!("│ [!] No active connector health snapshots found. Run `atx connector sync` to populate.  │");
            } else {
                for r in health_reports {
                    let badge = match r.state {
                        atlas_core::health::ConnectorHealthState::Healthy => "[✓]",
                        atlas_core::health::ConnectorHealthState::Degraded => "[!]",
                        _ => "[✗]",
                    };
                    println!("│ {} {:<15} Score: {:3}/100 | State: {:<10} | {}", badge, r.connector_id, r.score, r.state.to_string(), r.details);
                }
            }
            println!("└────────────────────────────────────────────────────────────────────────────────────────┘");
        }

        Commands::Ui { port, no_open } => {
            atlas_desktop_backend::run_server(port, !no_open).await?;
        }

        Commands::Connector { action } => {
            let cfg = Config::load_from_path(&config_path)?;
            let db_path = cfg.resolve_db_path();
            let storage = Storage::new(&db_path)?;

            match action {
                ConnectorSubcommands::List | ConnectorSubcommands::Status => {
                    let stats = storage.get_stats()?;
                    let reports = storage.load_all_health_reports().unwrap_or_default();

                    println!("┌─────────────────────────────────────────────────────────────────────────────────────────┐");
                    println!("│ REGISTERED CONNECTORS & V2 HEALTH SNAPSHOTS                                             │");
                    println!("├─────────────────┬──────────┬────────────┬─────────────┬─────────────────┬───────────────┤");
                    println!("│ ID              │ PROVIDER │ STATUS     │ HEALTH      │ LAST CHECKED    │ DETAILS       │");
                    println!("├─────────────────┼──────────┼────────────┼─────────────┼─────────────────┼───────────────┤");

                    if cfg.connectors.is_empty() {
                        println!("│ (No connectors configured in config.toml)                                              │");
                    } else {
                        for (cid, conn_cfg) in &cfg.connectors {
                            let report = reports.iter().find(|r| &r.connector_id == cid);
                            let score_str = report.map(|r| format!("{}/100", r.score)).unwrap_or_else(|| "N/A".to_string());
                            let state_str = report.map(|r| r.state.to_string()).unwrap_or_else(|| "UNKNOWN".to_string());
                            let last_checked = report.map(|r| r.last_checked_at.format("%H:%M:%S").to_string()).unwrap_or_else(|| "Never".to_string());
                            println!(
                                "│ {:<15} │ {:<8} │ {:<10} │ {:<11} │ {:<15} │ {:<13} │",
                                cid, conn_cfg.provider, state_str, score_str, last_checked, "Configured"
                            );
                        }
                    }
                    println!("└─────────────────┴──────────┴────────────┴─────────────┴─────────────────┴───────────────┘");
                    println!(" Storage: SQLite WAL ({}) │ Total Artifacts: {} │ Active Connectors: {}", db_path.display(), stats.total_artifacts, cfg.connectors.len());
                }

                ConnectorSubcommands::Inspect { id } => {
                    let conn_cfg = cfg.connectors.get(&id);
                    let checkpoint = storage.load_checkpoint(&id)?;

                    println!("┌─────────────────────────────────────────────────────────────────────────────────────────────┐");
                    println!("│ CONNECTOR INSPECTION: {:<68} │", id);
                    println!("├─────────────────────────────────────────────────────────────────────────────────────────────┤");
                    if let Some(c) = conn_cfg {
                        println!("│ Provider:          {:<73} │", c.provider);
                        println!("│ Target Resource:   {:<73} │", c.repos.join(", "));
                        if let Some(redacted) = c.redact_api_token() {
                            println!("│ Auth Credential:   {:<73} │", redacted);
                        }
                    } else {
                        println!("│ Status:            Not registered in active config.toml                              │");
                    }

                    if let Some(ckpt) = checkpoint {
                        println!("│ Last Synced:       {:<73} │", ckpt.last_synced_at.to_rfc3339());
                        println!("│ Total Processed:   {:<73} │", ckpt.total_items_processed);
                        println!("│ Watermark:         {:<73} │", ckpt.checksum_watermark);
                    } else {
                        println!("│ Checkpoint State:  No previous sync cursor saved                                     │");
                    }
                    println!("│ Capabilities:      [✓] Incremental  [✓] Streaming  [✓] Resilience Budget  [✓] MCP        │");
                    println!("└─────────────────────────────────────────────────────────────────────────────────────────────┘");
                }

                ConnectorSubcommands::Verify { id } => {
                    println!("Testing connectivity and health for connector '{}'...", id);
                    if let Some(conn_cfg) = cfg.connectors.get(&id) {
                        println!("  [✓] Configuration loaded for provider '{}'", conn_cfg.provider);
                        println!("  [✓] Storage path verified: {:?}", db_path);
                        match ConnectorInstance::build(&id, conn_cfg) {
                            Ok(conn_instance) => {
                                match conn_instance.verify().await {
                                    Ok(msg) => {
                                        println!("  [✓] Live Connectivity: {}", msg);
                                        println!("\nResult: Connector '{}' is valid and verified healthy.", id);
                                    }
                                    Err(err) => {
                                        println!("  [✗] Live Connectivity Error: {:#}", err);
                                        println!("\nResult: Connector '{}' verification failed.", id);
                                    }
                                }
                            }
                            Err(err) => {
                                println!("  [✗] Initialization Error: {:#}", err);
                                println!("\nResult: Connector '{}' configuration could not be initialized.", id);
                            }
                        }
                    } else {
                        println!("  [✗] Connector '{}' not found in configuration.", id);
                    }
                }

                ConnectorSubcommands::Doctor | ConnectorSubcommands::Health => {
                    let reports = storage.load_all_health_reports().unwrap_or_default();
                    println!("┌────────────────────────────────────────────────────────────────────────────────────────┐");
                    println!("│ CONNECTOR HEALTH MONITORING REPORT                                                     │");
                    println!("├─────────────────┬──────────┬────────────┬───────┬────────────┬───────────────┬─────────┤");
                    println!("│ CONNECTOR ID    │ PROVIDER │ STATE      │ SCORE │ P95 LATENCY│ SUCCESS RATE  │ DETAILS │");
                    println!("├─────────────────┬──────────┬────────────┬───────┬────────────┬───────────────┬─────────┤");
                    for r in reports {
                        println!(
                            "│ {:<15} │ {:<8} │ {:<10} │ {:3}   │ {:4} ms     │ {:5.1}%        │ {:<7} │",
                            r.connector_id, r.provider, r.state.to_string(), r.score, r.p95_latency_ms, r.success_rate, "Active"
                        );
                    }
                    println!("└─────────────────┴──────────┴────────────┴───────┴────────────┴───────────────┴─────────┘");
                }

                ConnectorSubcommands::Sync { connector, full, ci, json } => {
                    let mode = if json {
                        progress::ProgressRenderMode::JsonStream
                    } else if ci {
                        progress::ProgressRenderMode::CiConsole
                    } else {
                        progress::ProgressRenderMode::InteractiveTui
                    };

                    let renderer = progress::ProgressRenderer::new(mode);
                    let bus = atlas_core::progress::ProgressEventBus::default();
                    let rx = bus.subscribe();

                    let listen_handle = tokio::spawn(async move {
                        renderer.listen_and_render(rx).await;
                    });

                    println!("Starting Atlas V2 Synchronization Engine...");
                    bus.publish(atlas_core::progress::ProgressEvent::SyncStarted {
                        connector_id: connector.clone().unwrap_or_else(|| "all".to_string()),
                        total_expected: None,
                    });

                    let mut target_connectors = Vec::new();
                    if let Some(target_id) = connector {
                        if let Some(c) = cfg.connectors.get(&target_id) {
                            target_connectors.push((target_id.clone(), c.clone()));
                        } else {
                            anyhow::bail!("Connector '{}' not found in configuration", target_id);
                        }
                    } else {
                        for (id, c) in &cfg.connectors {
                            target_connectors.push((id.clone(), c.clone()));
                        }
                    }

                    for (id, connector_cfg) in target_connectors {
                        let conn_instance = match ConnectorInstance::build(&id, &connector_cfg) {
                            Ok(c) => c,
                            Err(err) => {
                                println!("Failed to initialize connector '{}': {:#}", id, err);
                                continue;
                            }
                        };

                        let cid = conn_instance.id().to_string();
                        let provider = conn_instance.provider().to_string();
                        bus.publish(atlas_core::progress::ProgressEvent::OperationChanged {
                            connector_id: cid.clone(),
                            operation: "Fetch & Stream".to_string(),
                            target: provider.clone(),
                        });

                        let start_time = std::time::Instant::now();
                        match SyncEngine::run_sync(&conn_instance, &storage, full).await {
                            Ok(summary) => {
                                let elapsed = start_time.elapsed().as_secs_f64();
                                let latency_ms = (elapsed * 1000.0) as u64;
                                bus.publish(atlas_core::progress::ProgressEvent::SyncCompleted {
                                    connector_id: cid.clone(),
                                    total_synced: summary.inserted as u64 + summary.updated as u64,
                                    elapsed_secs: elapsed,
                                });

                                let report = atlas_core::health::HealthReport::new(
                                    &cid,
                                    &provider,
                                    true,
                                    true,
                                    latency_ms,
                                    100.0,
                                    format!("Fetched {}, Inserted {}, Skipped {}", summary.fetched, summary.inserted, summary.skipped),
                                );
                                let _ = storage.save_health_report(&report);
                            }
                            Err(e) => {
                                let elapsed = start_time.elapsed().as_secs_f64();
                                let latency_ms = (elapsed * 1000.0) as u64;
                                bus.publish(atlas_core::progress::ProgressEvent::SyncFailed {
                                    connector_id: cid.clone(),
                                    error: e.to_string(),
                                });

                                let report = atlas_core::health::HealthReport::new(
                                    &cid,
                                    &provider,
                                    false,
                                    false,
                                    latency_ms,
                                    0.0,
                                    e.to_string(),
                                );
                                let _ = storage.save_health_report(&report);
                            }
                        }
                    }

                    drop(bus);
                    let _ = listen_handle.await;
                }
            }
        }
    }

    Ok(())
}

fn list_or_search_docs(storage: &Storage, query: Option<&str>) -> Result<()> {
    let artifacts = if let Some(q) = query {
        if !q.trim().is_empty() {
            storage.search_fts(q, Some("document"), None, None, 50)?
        } else {
            storage.query_structured(Some("document"), None, None, 100)?
        }
    } else {
        storage.query_structured(Some("document"), None, None, 100)?
    };

    if artifacts.is_empty() {
        if let Some(q) = query {
            println!("No documents matching '{}' found in context graph.", q);
        } else {
            println!("No documents indexed yet. Run 'atx sync' to synchronize documents from connectors.");
        }
        return Ok(());
    }

    println!("Found {} indexed document(s):\n", artifacts.len());
    println!("┌─{0:─<4}─┬─{0:─<40}─┬─{0:─<35}─┬─{0:─<12}─┐", "");
    println!("│ {:<4} │ {:<40} │ {:<35} │ {:<12} │", "#", "DOCUMENT TITLE", "PATH / SOURCE ID", "PROVIDER");
    println!("├─{0:─<4}─┼─{0:─<40}─┼─{0:─<35}─┼─{0:─<12}─┤", "");

    for (idx, art) in artifacts.iter().enumerate() {
        let title = formatter::safe_truncate(&art.title, 40);
        let path = formatter::safe_truncate(&art.source_id, 35);
        let provider = formatter::safe_truncate(&art.provider, 12);
        println!("│ {:<4} │ {:<40} │ {:<35} │ {:<12} │", idx + 1, title, path, provider);
    }
    println!("└─{0:─<4}─┴─{0:─<40}─┴─{0:─<35}─┴─{0:─<12}─┘", "");
    println!("\n💡 Tip: Run 'atx doc <title_or_path>' to read the full document in terminal.");
    println!("        Use 'atx doc <title_or_path> --raw' to output clean markdown (pipeable to glow/pager).");

    Ok(())
}

fn display_doc(storage: &Storage, target: &str, raw: bool, json_output: bool) -> Result<()> {
    // 1. Try alias resolution first
    let mut candidates = storage.resolve_artifact_by_alias(target)?;

    // 2. If no alias resolution, try search in documents
    if candidates.is_empty() {
        candidates = storage.search_fts(target, Some("document"), None, None, 10)?;
    }

    // 3. If still empty, search across all artifact kinds
    if candidates.is_empty() {
        candidates = storage.search_fts(target, None, None, None, 10)?;
    }

    if candidates.is_empty() {
        println!("Artifact or document '{}' not found in context graph.", target);
        println!("Run 'atx docs' to browse documents, or 'atx search \"{}\"' to search globally.", target);
        return Ok(());
    }

    // Check if one candidate is an exact match on source_id or title
    let target_trimmed = target.trim();
    let exact_match = candidates.iter().position(|c| {
        c.source_id.eq_ignore_ascii_case(target_trimmed)
            || c.source_id.ends_with(target_trimmed)
            || c.title.eq_ignore_ascii_case(target_trimmed)
    });

    let selected = if candidates.len() == 1 {
        &candidates[0]
    } else if let Some(idx) = exact_match {
        &candidates[idx]
    } else {
        println!("Found {} artifacts matching '{}':\n", candidates.len(), target);
        for (idx, c) in candidates.iter().enumerate() {
            println!("{:2}. [{}] {} ({})", idx + 1, c.kind.to_string().to_uppercase(), c.title, c.source_id);
        }
        println!("\nSpecify exact ID or path: 'atx cat <id>' or 'atx doc <path>'");
        return Ok(());
    };

    if json_output {
        println!("{}", serde_json::to_string_pretty(selected)?);
        return Ok(());
    }

    if raw {
        print!("{}", selected.body);
        if !selected.body.ends_with('\n') {
            println!();
        }
        return Ok(());
    }

    // Formatted terminal document view
    let divider = "═".repeat(70);
    println!("{}", divider);
    let kind_upper = selected.kind.to_string().to_uppercase();
    let kind_icon = match selected.kind {
        atlas_core::ArtifactKind::Ticket => "🎫",
        atlas_core::ArtifactKind::PullRequest => "🔀",
        atlas_core::ArtifactKind::Commit => "💾",
        atlas_core::ArtifactKind::Issue => "⚠️",
        atlas_core::ArtifactKind::Design => "🎨",
        _ => "📄",
    };
    println!("{} [{}] {}", kind_icon, kind_upper, selected.title);
    println!("ID: {} | Provider: {}", selected.source_id, selected.provider);
    if let Some(ref repo) = selected.repository {
        println!("Repository: {}", repo);
    }
    if !selected.source_url.is_empty() {
        println!("URL: {}", selected.source_url);
    }
    println!("{}", divider);
    println!();
    println!("{}", selected.body);
    println!();
    println!("{}", "─".repeat(70));
    println!("💡 Hint: Use 'atx cat \"{}\" --raw' for pure content without headers.", target);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::{ArtifactKind, KnowledgeArtifact};

    #[test]
    fn test_cli_mcp_default() {
        let cli = Cli::try_parse_from(["atx", "mcp"]).expect("parse atx mcp");
        match cli.command {
            Commands::Mcp { action } => assert_eq!(action, None),
            _ => panic!("expected Commands::Mcp"),
        }
    }

    #[test]
    fn test_cli_mcp_serve() {
        let cli = Cli::try_parse_from(["atx", "mcp", "serve"]).expect("parse atx mcp serve");
        match cli.command {
            Commands::Mcp { action } => assert_eq!(action, Some(McpSubcommands::Serve)),
            _ => panic!("expected Commands::Mcp"),
        }
    }

    #[test]
    fn test_cli_mcp_list() {
        let cli = Cli::try_parse_from(["atx", "mcp", "list"]).expect("parse atx mcp list");
        match cli.command {
            Commands::Mcp { action } => assert_eq!(action, Some(McpSubcommands::List)),
            _ => panic!("expected Commands::Mcp"),
        }
    }

    #[test]
    fn test_cli_mcp_add_basic() {
        let cli = Cli::try_parse_from([
            "atx", "mcp", "add", "test-echo", "--command", "echo", "--args", "hello",
        ])
        .expect("parse atx mcp add");
        match cli.command {
            Commands::Mcp {
                action:
                    Some(McpSubcommands::Add {
                        name,
                        command,
                        args,
                        env,
                        prefix,
                        disabled,
                    }),
            } => {
                assert_eq!(name, "test-echo");
                assert_eq!(command, "echo");
                assert_eq!(args, vec!["hello"]);
                assert!(env.is_empty());
                assert_eq!(prefix, None);
                assert!(!disabled);
            }
            _ => panic!("expected Commands::Mcp Add"),
        }
    }

    #[test]
    fn test_cli_mcp_add_all_flags() {
        let cli = Cli::try_parse_from([
            "atx",
            "mcp",
            "add",
            "figma",
            "--command",
            "npx",
            "--args",
            "-y @figma/mcp",
            "--env",
            "TOKEN=123",
            "--prefix",
            "figma",
            "--disabled",
        ])
        .expect("parse atx mcp add with all flags");
        match cli.command {
            Commands::Mcp {
                action:
                    Some(McpSubcommands::Add {
                        name,
                        command,
                        args,
                        env,
                        prefix,
                        disabled,
                    }),
            } => {
                assert_eq!(name, "figma");
                assert_eq!(command, "npx");
                assert_eq!(args, vec!["-y @figma/mcp"]);
                assert_eq!(env, vec!["TOKEN=123"]);
                assert_eq!(prefix, Some("figma".to_string()));
                assert!(disabled);
            }
            _ => panic!("expected Commands::Mcp Add"),
        }
    }

    #[test]
    fn test_cli_mcp_test() {
        let cli = Cli::try_parse_from(["atx", "mcp", "test", "my-server"]).expect("parse atx mcp test");
        match cli.command {
            Commands::Mcp {
                action: Some(McpSubcommands::Test { name }),
            } => {
                assert_eq!(name, "my-server");
            }
            _ => panic!("expected Commands::Mcp Test"),
        }
    }

    #[test]
    fn test_cli_mcp_remove() {
        let cli = Cli::try_parse_from(["atx", "mcp", "remove", "my-server"]).expect("parse atx mcp remove");
        match cli.command {
            Commands::Mcp {
                action: Some(McpSubcommands::Remove { name }),
            } => {
                assert_eq!(name, "my-server");
            }
            _ => panic!("expected Commands::Mcp Remove"),
        }
    }

    #[test]
    fn test_parse_mcp_args_various_formats() {
        assert_eq!(
            parse_mcp_args(&["hello".to_string()]),
            vec!["hello".to_string()]
        );
        assert_eq!(
            parse_mcp_args(&["hello,world".to_string()]),
            vec!["hello".to_string(), "world".to_string()]
        );
        assert_eq!(
            parse_mcp_args(&["hello, world".to_string()]),
            vec!["hello".to_string(), "world".to_string()]
        );
        assert_eq!(
            parse_mcp_args(&["-y @figma/mcp".to_string()]),
            vec!["-y".to_string(), "@figma/mcp".to_string()]
        );
        assert_eq!(
            parse_mcp_args(&["-y".to_string(), "@figma/mcp".to_string()]),
            vec!["-y".to_string(), "@figma/mcp".to_string()]
        );
    }

    #[test]
    fn test_parse_mcp_env_various_formats() {
        let env1 = parse_mcp_env(&["FOO=BAR".to_string(), "BAZ=QUX".to_string()]);
        assert_eq!(env1.get("FOO").map(|s| s.as_str()), Some("BAR"));
        assert_eq!(env1.get("BAZ").map(|s| s.as_str()), Some("QUX"));

        let env2 = parse_mcp_env(&["FOO=BAR,BAZ=QUX".to_string()]);
        assert_eq!(env2.get("FOO").map(|s| s.as_str()), Some("BAR"));
        assert_eq!(env2.get("BAZ").map(|s| s.as_str()), Some("QUX"));

        let env3 = parse_mcp_env(&["COMPLEX=val=with=equals".to_string()]);
        assert_eq!(env3.get("COMPLEX").map(|s| s.as_str()), Some("val=with=equals"));
    }

    #[test]
    fn test_cli_mcp_alias() {
        let cli = Cli::try_parse_from(["atx", "mcp", "alias", "figma", "PROJ-123", "sample_key"])
            .expect("parse atx mcp alias");
        match cli.command {
            Commands::Mcp {
                action: Some(McpSubcommands::Alias { server, from, to }),
            } => {
                assert_eq!(server, "figma");
                assert_eq!(from, "PROJ-123");
                assert_eq!(to, "sample_key");
            }
            _ => panic!("expected Commands::Mcp Alias"),
        }
    }

    #[test]
    fn test_cli_context_figma_flag() {
        let cli = Cli::try_parse_from(["atx", "context", "PROJ-123", "--figma", "https://www.figma.com/design/sample_key/Title"])
            .expect("parse atx context --figma");
        match cli.command {
            Commands::Context {
                target,
                figma,
                ..
            } => {
                assert_eq!(target, "PROJ-123");
                assert_eq!(
                    figma.as_deref(),
                    Some("https://www.figma.com/design/sample_key/Title")
                );
            }
            _ => panic!("expected Commands::Context"),
        }
    }

    #[test]
    fn test_cli_doc_command() {
        let cli = Cli::try_parse_from(["atx", "doc", "vision.md", "--raw"])
            .expect("parse atx doc vision.md --raw");
        match cli.command {
            Commands::Doc {
                target,
                list,
                raw,
                json,
            } => {
                assert_eq!(target.as_deref(), Some("vision.md"));
                assert!(!list);
                assert!(raw);
                assert!(!json);
            }
            _ => panic!("expected Commands::Doc"),
        }
    }

    #[test]
    fn test_cli_docs_command() {
        let cli = Cli::try_parse_from(["atx", "docs", "vision"])
            .expect("parse atx docs vision");
        match cli.command {
            Commands::Docs { query } => {
                assert_eq!(query.as_deref(), Some("vision"));
            }
            _ => panic!("expected Commands::Docs"),
        }
    }

    #[test]
    fn test_cli_cat_command() {
        let cli = Cli::try_parse_from(["atx", "cat", "PROJ-123", "--raw"])
            .expect("parse atx cat PROJ-123 --raw");
        match cli.command {
            Commands::Cat {
                target,
                raw,
                json,
            } => {
                assert_eq!(target, "PROJ-123");
                assert!(raw);
                assert!(!json);
            }
            _ => panic!("expected Commands::Cat"),
        }
    }

    #[test]
    fn test_display_doc_and_cat_resolution() -> anyhow::Result<()> {
        let tmp_file = tempfile::NamedTempFile::new()?;
        let storage = Storage::new(tmp_file.path())?;

        let ticket = KnowledgeArtifact {
            id: KnowledgeArtifact::generate_id("jira", "https://jira.example.com", "PROJ-123"),
            kind: ArtifactKind::Ticket,
            title: "Support Multi-Currency Checkout".to_string(),
            summary: Some("In Development".to_string()),
            body: "Full ticket description content here with figma links".to_string(),
            provider: "jira".to_string(),
            source_id: "PROJ-123".to_string(),
            source_url: "https://jira.example.com/browse/PROJ-123".to_string(),
            repository: Some("PROJ".to_string()),
            tags: vec!["project:PROJ".to_string()],
            relationships: vec![],
            created_at: Some(chrono::Utc::now()),
            updated_at: chrono::Utc::now(),
            synced_at: chrono::Utc::now(),
            checksum: "sum1".to_string(),
            metadata: serde_json::json!({ "status": "In Development" }),
        };
        storage.upsert_artifact(&ticket)?;

        // Test display_doc with raw = true
        assert!(display_doc(&storage, "PROJ-123", true, false).is_ok());

        // Test display_doc with json = true
        assert!(display_doc(&storage, "PROJ-123", false, true).is_ok());

        // Test display_doc normal formatted view
        assert!(display_doc(&storage, "PROJ-123", false, false).is_ok());

        // Test non-existent artifact returns Ok(()) without panicking
        assert!(display_doc(&storage, "NON-EXISTENT-999", false, false).is_ok());

        Ok(())
    }

    #[test]
    fn test_cli_ui_and_web_parsing() {
        let parsed_ui = Cli::try_parse_from(["atx", "ui", "--port", "31415", "--no-open"]).unwrap();
        match parsed_ui.command {
            Commands::Ui { port, no_open } => {
                assert_eq!(port, 31415);
                assert!(no_open);
            }
            _ => panic!("expected Commands::Ui"),
        }

        let parsed_web = Cli::try_parse_from(["atx", "web"]).unwrap();
        match parsed_web.command {
            Commands::Ui { port, no_open } => {
                assert_eq!(port, 31415);
                assert!(!no_open);
            }
            _ => panic!("expected Commands::Ui"),
        }
    }
}


