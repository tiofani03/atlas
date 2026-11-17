use crate::cli::ConnectorSubcommands;
use crate::progress;
use anyhow::Result;
use atlas_core::{Config, Connector, ConnectorInstance, Storage, SyncEngine};
use std::path::Path;

pub async fn handle(action: ConnectorSubcommands, config_path: &Path) -> Result<()> {
            let cfg = Config::load_from_path(config_path)?;
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

    Ok(())
}
