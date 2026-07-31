use crate::handlers::{connectors, search, status, sync};
use crate::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/status", get(status::get_status))
        .route("/api/connectors", get(connectors::list_connectors))
        .route("/api/connectors/jira", post(connectors::save_jira_connector))
        .route(
            "/api/connectors/confluence",
            post(connectors::save_confluence_connector),
        )
        .route(
            "/api/connectors/github",
            post(connectors::save_github_connector),
        )
        .route(
            "/api/connectors/validate",
            post(connectors::validate_credentials),
        )
        .route("/api/sync", post(sync::trigger_sync))
        .route("/api/sync/status", get(sync::get_sync_status))
        .route("/api/search", get(search::search_objects))
        .route("/api/objects/:id", get(search::get_object_by_id))
        .layer(cors)
        .with_state(state)
}
