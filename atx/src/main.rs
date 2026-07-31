mod formatter;

use anyhow::Result;
use atlas_core::{
    ConfluenceConnector, Config, Connector, ConnectorConfig, ConnectorInstance, GithubConnector,
    JiraConnector, Storage, SyncEngine,
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

    /// Clear all synchronized context data and reset SQLite index
    Reset {
        /// Force clear without asking for confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Alias for reset command
    Clear {
        /// Force clear without asking for confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Run stdio Model Context Protocol (MCP) Server for AI tools
    Mcp,
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
                        },
                    );

                    cfg.save_to_path(&config_path)?;
                    println!("GitHub connector '{}' updated successfully!", id);
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
                let conn_instance = match connector_cfg.provider.as_str() {
                    "jira" => ConnectorInstance::Jira(JiraConnector::new(id.clone(), connector_cfg)?),
                    "confluence" => ConnectorInstance::Confluence(ConfluenceConnector::new(id.clone(), connector_cfg)?),
                    "github" => ConnectorInstance::Github(GithubConnector::new(id.clone(), connector_cfg)?),
                    other => {
                        println!("Skipping unknown provider '{}' for ID '{}'", other, id);
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
                println!("{}", serde_json::to_string_pretty(&results)?);
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

            match storage.get_artifact_by_id(&id)? {
                Some(artifact) => {
                    if json {
                        println!("{}", serde_json::to_string_pretty(&artifact)?);
                    } else {
                        println!(
                            "{}",
                            formatter::format_artifact_detail(&artifact, Some(&storage), verbose, raw)
                        );
                    }
                }
                None => {
                    println!("Artifact '{}' not found.", id);
                }
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

            let key_artifact = storage.get_artifact_by_id(&id).ok().flatten();
            let related = storage.get_related_artifacts(&id)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&related)?);
            } else {
                println!(
                    "{}",
                    formatter::format_related_results(
                        &id,
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
            json,
            verbose,
            raw,
        } => {
            let cfg = Config::load_from_path(&config_path)?;
            let storage = Storage::new(cfg.resolve_db_path())?;

            let (kind_param, id_param) = match target_id {
                Some(ref id) => (Some(target.as_str()), id.as_str()),
                None => (None, target.as_str()),
            };

            let builder = atlas_core::ContextBuilder::new(&storage);
            let options = atlas_core::ContextOptions::default();
            let pkg = builder.build(kind_param, id_param, &options)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&pkg)?);
            } else {
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
        }

        Commands::Reset { force } | Commands::Clear { force } => {
            let cfg = Config::load_from_path(&config_path)?;
            let db_path = cfg.resolve_db_path();
            let storage = Storage::new(&db_path)?;

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

        Commands::Mcp => {
            let cfg = Config::load_from_path(&config_path)?;
            let storage = Storage::new(cfg.resolve_db_path())?;

            atlas_core::mcp::run_stdio_mcp_server(storage).await?;
        }
    }

    Ok(())
}
