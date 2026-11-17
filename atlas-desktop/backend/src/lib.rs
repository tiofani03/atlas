pub mod auth;
pub mod handlers;
pub mod routes;
pub mod state;

use atlas_core::{Config, Connector, ConnectorInstance, LocalPathWatcher, SyncEngine};
use state::AppState;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Open a URL in the user's default browser across Linux, macOS, and Windows
pub fn open_in_browser(url: &str) {
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd").args(["/C", "start", url]).spawn();
    }
}

/// Run the Atlas Desktop HTTP server on the specified port
pub async fn run_server(port: u16, open_browser: bool) -> anyhow::Result<()> {
    let config_path = Config::default_config_path()?;
    tracing::info!("Using configuration file at: {:?}", config_path);

    let state = AppState::new(config_path);
    spawn_background_sync(state.clone());
    let app = routes::create_router(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let url = format!("http://localhost:{}", port);

    println!("\n==========================================");
    println!("  🌟 Atlas Web UI & Context Visualizer");
    println!("==========================================");
    println!("  ▶ Access UI:   {}", url);
    println!("  ▶ API Base:    {}/api/status", url);
    println!("  ▶ Status:      Running (Press Ctrl+C to exit)\n");

    if open_browser {
        open_in_browser(&url);
    }

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Spawn a background task that watches local paths (`local_git`, `markdown`, `openapi`)
/// for filesystem changes and runs scheduled periodic syncs when `general.sync_interval_secs` is set.
pub fn spawn_background_sync(state: AppState) {
    tokio::spawn(async move {
        let mut watcher = LocalPathWatcher::new();
        let mut last_scheduled_sync = Instant::now();
        let mut interval = tokio::time::interval(Duration::from_secs(15));

        loop {
            interval.tick().await;

            // Skip if a manual sync is already in progress
            if state.sync_progress.read().await.is_running {
                continue;
            }

            let Ok(cfg) = state.load_config() else {
                continue;
            };
            let Ok(storage) = state.get_storage() else {
                continue;
            };

            let due_for_scheduled = cfg
                .general
                .sync_interval_secs
                .filter(|&secs| secs > 0)
                .map(|secs| last_scheduled_sync.elapsed() >= Duration::from_secs(secs))
                .unwrap_or(false);

            let mut ran_scheduled = false;
            for (id, conn_cfg) in &cfg.connectors {
                let is_local = matches!(
                    conn_cfg.provider.to_lowercase().as_str(),
                    "local_git" | "markdown" | "openapi" | "swagger"
                );
                let mut should_sync = due_for_scheduled;
                if is_local {
                    let paths: Vec<PathBuf> = conn_cfg.get_paths().into_iter().map(PathBuf::from).collect();
                    if !paths.is_empty() && watcher.has_changed(id, &paths).unwrap_or(false) {
                        tracing::info!("Detected local file change for connector '{}', triggering incremental sync", id);
                        should_sync = true;
                    }
                }

                if should_sync {
                    if let Ok(instance) = ConnectorInstance::build(id, conn_cfg) {
                        if let Err(err) =
                            SyncEngine::run_sync(&instance as &dyn Connector, &storage, false).await
                        {
                            tracing::warn!("Background sync failed for '{}': {}", id, err);
                        }
                    }
                    if due_for_scheduled {
                        ran_scheduled = true;
                    }
                }
            }

            if ran_scheduled {
                last_scheduled_sync = Instant::now();
            }
        }
    });
}
