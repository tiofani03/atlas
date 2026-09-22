use crate::handlers::{connectors, graph, mcp, search, status, sync};
use crate::state::AppState;
use axum::{
    routing::{delete, get, post},
    Router,
};
use std::path::{Path, PathBuf};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

pub fn find_frontend_dist() -> Option<PathBuf> {
    if let Ok(dir_str) = std::env::var("ATLAS_FRONTEND_DIR") {
        let p = PathBuf::from(dir_str);
        if p.join("index.html").exists() {
            return Some(p);
        }
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let candidate1 = exe_dir.join("dist");
            if candidate1.join("index.html").exists() {
                return Some(candidate1);
            }
            let candidate2 = exe_dir.join("frontend/dist");
            if candidate2.join("index.html").exists() {
                return Some(candidate2);
            }
        }
    }

    let candidates = [
        Path::new("atlas-desktop/frontend/dist"),
        Path::new("frontend/dist"),
        Path::new("dist"),
        Path::new("../frontend/dist"),
    ];

    for c in candidates {
        if c.join("index.html").exists() {
            return Some(c.to_path_buf());
        }
    }

    None
}

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let mut router = Router::new()
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
            "/api/connectors/linear",
            post(connectors::save_linear_connector),
        )
        .route(
            "/api/connectors/gitlab",
            post(connectors::save_gitlab_connector),
        )
        .route(
            "/api/connectors/openapi",
            post(connectors::save_openapi_connector),
        )
        .route(
            "/api/connectors/azure_devops",
            post(connectors::save_azure_devops_connector),
        )
        .route(
            "/api/connectors/bitbucket",
            post(connectors::save_bitbucket_connector),
        )
        .route(
            "/api/connectors/figma",
            post(connectors::save_figma_connector),
        )
        .route(
            "/api/connectors/notion",
            post(connectors::save_notion_connector),
        )
        .route(
            "/api/connectors/asana",
            post(connectors::save_asana_connector),
        )
        .route(
            "/api/connectors/spreadsheet",
            post(connectors::save_spreadsheet_connector),
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
        .route("/api/graph/recent", get(graph::get_recent_nodes))
        .route("/api/graph/:id", get(graph::get_graph_by_id))
        .route(
            "/api/mcp/servers",
            get(mcp::list_mcp_servers).post(mcp::save_mcp_server),
        )
        .route("/api/mcp/servers/:name", delete(mcp::delete_mcp_server))
        .route("/api/mcp/servers/:name/test", post(mcp::test_mcp_server))
        .route("/api/mcp/snippet", get(mcp::get_mcp_snippets))
        .route("/api/mcp/snippets", get(mcp::get_mcp_snippets))
        .layer(cors)
        .with_state(state);

    if let Some(dist_dir) = find_frontend_dist() {
        let index_file = dist_dir.join("index.html");
        tracing::info!("Serving Atlas Desktop static frontend from disk: {:?}", dist_dir);
        let serve_dir = ServeDir::new(&dist_dir).fallback(ServeFile::new(index_file));
        router = router.fallback_service(serve_dir);
    } else {
        tracing::info!("Serving Atlas Desktop static frontend from embedded binary assets");
        router = router.fallback(static_handler);
    }

    router
}

#[derive(rust_embed::RustEmbed)]
#[folder = "../frontend/dist"]
pub struct EmbeddedAssets;

pub async fn static_handler(uri: axum::http::Uri) -> axum::response::Response {
    use axum::response::IntoResponse;

    let mut path = uri.path().trim_start_matches('/').to_string();

    if path.is_empty() {
        path = "index.html".to_string();
    }

    match EmbeddedAssets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            (
                [(axum::http::header::CONTENT_TYPE, mime.as_ref())],
                content.data,
            )
                .into_response()
        }
        None => {
            // SPA fallback: return index.html
            if let Some(index) = EmbeddedAssets::get("index.html") {
                (
                    [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
                    index.data,
                )
                    .into_response()
            } else {
                (
                    axum::http::StatusCode::NOT_FOUND,
                    "404 Not Found",
                )
                    .into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_create_router_no_panics() {
        let state = AppState::new(PathBuf::from("/tmp/nonexistent_config.toml"));
        let _router = create_router(state);
    }
}
