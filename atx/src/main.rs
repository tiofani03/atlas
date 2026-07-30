use anyhow::Result;
use atlas_core::{
    ConfluenceConnector, Config, Connector, ConnectorConfig, ConnectorInstance, JiraConnector,
    KnowledgeObject, Storage, SyncEngine,
};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "atx",
    author = "Atlas Contributors",
    version,
    about = "Unified Engineering Knowledge Layer (Atlas)"
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

    /// Synchronize knowledge from external connectors into local database
    Sync {
        /// Optional connector ID to sync specifically
        #[arg(short, long)]
        connector: Option<String>,

        /// Force full re-sync ignoring last sync watermarks
        #[arg(short, long)]
        full: bool,
    },

    /// Search engineering knowledge using BM25 full-text search or metadata filters
    Search {
        /// Optional search query terms
        #[arg(default_value = "")]
        query: String,

        /// Filter by object type (e.g. ticket, document, specification)
        #[arg(short, long)]
        object_type: Option<String>,

        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,

        /// Maximum results to return
        #[arg(short, long, default_value_t = 10)]
        limit: usize,

        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show storage statistics and connector status
    Status,

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
        url: String,
        /// User Email
        #[arg(long)]
        email: String,
        /// API Token
        #[arg(long)]
        token: Option<String>,
        /// Environment variable containing API Token
        #[arg(long)]
        token_env: Option<String>,
        /// Comma-separated project keys (e.g. "PAY,DEV")
        #[arg(long)]
        projects: Option<String>,
    },
    /// Configure Confluence connector
    Confluence {
        /// Connector ID (e.g. "confluence-docs")
        #[arg(default_value = "confluence-docs")]
        id: String,
        /// Confluence Instance URL
        #[arg(long)]
        url: String,
        /// User Email
        #[arg(long)]
        email: String,
        /// API Token
        #[arg(long)]
        token: Option<String>,
        /// Environment variable containing API Token
        #[arg(long)]
        token_env: Option<String>,
        /// Comma-separated space keys (e.g. "ENG,ARCH")
        #[arg(long)]
        spaces: Option<String>,
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
            println!("Initializing Atlas...");
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
                } => {
                    let project_list = projects
                        .map(|p| p.split(',').map(|s| s.trim().to_string()).collect())
                        .unwrap_or_default();

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "jira".to_string(),
                            instance_url: url,
                            email,
                            api_token: token,
                            api_token_env: token_env,
                            projects: project_list,
                            spaces: Vec::new(),
                        },
                    );

                    cfg.save_to_path(&config_path)?;
                    println!("Jira connector '{}' configured successfully!", id);
                }

                ConfigSubcommands::Confluence {
                    id,
                    url,
                    email,
                    token,
                    token_env,
                    spaces,
                } => {
                    let space_list = spaces
                        .map(|s| s.split(',').map(|item| item.trim().to_string()).collect())
                        .unwrap_or_default();

                    cfg.connectors.insert(
                        id.clone(),
                        ConnectorConfig {
                            provider: "confluence".to_string(),
                            instance_url: url,
                            email,
                            api_token: token,
                            api_token_env: token_env,
                            projects: Vec::new(),
                            spaces: space_list,
                        },
                    );

                    cfg.save_to_path(&config_path)?;
                    println!("Confluence connector '{}' configured successfully!", id);
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

            println!("Starting Atlas synchronization...\n");

            for (id, connector_cfg) in target_connectors {
                let conn_instance = match connector_cfg.provider.as_str() {
                    "jira" => ConnectorInstance::Jira(JiraConnector::new(id.clone(), connector_cfg)?),
                    "confluence" => ConnectorInstance::Confluence(ConfluenceConnector::new(id.clone(), connector_cfg)?),
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
            object_type,
            tag,
            limit,
            json,
        } => {
            let cfg = Config::load_from_path(&config_path)?;
            let storage = Storage::new(cfg.resolve_db_path())?;

            let results = if !query.is_empty() {
                storage.search_fts(&query, limit)?
            } else {
                storage.query_structured(object_type.as_deref(), tag.as_deref(), limit)?
            };

            if json {
                println!("{}", serde_json::to_string_pretty(&results)?);
            } else {
                print_cli_results(&results);
            }
        }

        Commands::Status => {
            let cfg = Config::load_from_path(&config_path)?;
            let db_path = cfg.resolve_db_path();
            let storage = Storage::new(&db_path)?;

            let stats = storage.get_stats()?;

            println!("=== Atlas Engineering Knowledge Status ===");
            println!("Config Path:        {:?}", config_path);
            println!("Database Path:      {:?}", db_path);
            println!("Total Objects:      {}", stats.total_objects);
            println!("Configured Connectors: {}", cfg.connectors.len());
            println!("Database Size:      {:.2} MB", stats.db_size_bytes as f64 / (1024.0 * 1024.0));
            println!("\nConnectors:");

            for (id, conn_cfg) in &cfg.connectors {
                let last_sync = storage.get_last_sync(id).unwrap_or(None);
                let sync_str = last_sync
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| "Never".to_string());

                println!("  - [{}] ({}) -> Last Sync: {}", id, conn_cfg.provider, sync_str);
            }
        }

        Commands::Mcp => {
            let cfg = Config::load_from_path(&config_path)?;
            let storage = Storage::new(cfg.resolve_db_path())?;

            atlas_core::mcp::run_stdio_mcp_server(storage).await?;
        }
    }

    Ok(())
}

fn print_cli_results(objects: &[KnowledgeObject]) {
    if objects.is_empty() {
        println!("No matching knowledge objects found.");
        return;
    }

    println!("Found {} results:\n", objects.len());

    for (i, obj) in objects.iter().enumerate() {
        println!(
            "{}. [{}] {}",
            i + 1,
            obj.object_type.to_string().to_uppercase(),
            obj.title
        );
        println!("   ID:     {}", obj.id);
        println!("   Source: {} ({})", obj.source.original_id, obj.source.web_url);
        if let Some(ref s) = obj.summary {
            println!("   Summary: {}", s);
        }
        if !obj.tags.is_empty() {
            println!("   Tags:   {}", obj.tags.join(", "));
        }
        println!();
    }
}
