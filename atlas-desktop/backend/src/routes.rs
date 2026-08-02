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
        .route("/api/storage/clear", post(status::clear_data))
        .route("/api/clear", post(status::clear_data))
        .route("/api/data/clear", post(status::clear_data))
        .route("/api/connectors", get(connectors::list_connectors))
        .route(
            "/api/connectors/jira",
            post(connectors::save_jira_connector),
        )
        .route(
            "/api/connectors/confluence",
            post(connectors::save_confluence_connector),
        )
        .route(
            "/api/connectors/github",
            post(connectors::save_github_connector),
        )
        .route(
            "/api/connectors/clickup",
            post(connectors::save_clickup_connector),
        )
        .route(
            "/api/connectors/markdown",
            post(connectors::save_markdown_connector),
        )
        .route(
            "/api/connectors/local_git",
            post(connectors::save_local_git_connector),
        )
        .route(
            "/api/dialog/select-folder",
            get(connectors::select_folder).post(connectors::select_folder),
        )
        .route(
            "/api/connectors/validate",
            post(connectors::validate_credentials),
        )
        .route("/api/connectors/delete", post(connectors::delete_connector))
        .route("/api/sync", post(sync::trigger_sync))
        .route("/api/sync/status", get(sync::get_sync_status))
        .route("/api/search", get(search::search_objects))
        .route("/api/objects/:id", get(search::get_object_by_id))
        .route("/api/context/:id", get(search::get_context))
        .layer(cors)
        .with_state(state)
}
