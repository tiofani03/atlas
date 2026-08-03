use atlas_desktop_backend::{AppState, create_test_router};
use atlas_core::Config;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let config_path = Config::default_config_path()?;
    tracing::info!("Using configuration file at: {:?}", config_path);

    let state = AppState::new(config_path);
    let app = create_test_router(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 31415));
    tracing::info!("Atlas Desktop Local Backend listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
