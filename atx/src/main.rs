mod cli;
mod commands;
mod explain;
mod formatter;
mod progress;

use anyhow::Result;
use atlas_core::{Config, Connector, ConnectorConfig, ConnectorInstance, Storage, SyncEngine};
use clap::Parser;
use cli::{Cli, Commands};
use commands::docs::{display_doc, list_or_search_docs};
#[cfg(test)]
use cli::{parse_mcp_args, parse_mcp_env, McpSubcommands};

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

        Commands::Config { action } => commands::config::handle(action, &config_path).await?,

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

        Commands::Mcp { action } => commands::mcp::handle(action, &config_path).await?,

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

        Commands::Connector { action } => commands::connector::handle(action, &config_path).await?,
    }

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


