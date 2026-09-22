pub mod handlers;
pub mod routes;
pub mod state;

use atlas_core::Config;
use state::AppState;
use std::net::SocketAddr;

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
