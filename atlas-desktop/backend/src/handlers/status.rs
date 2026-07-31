use crate::state::AppState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

pub async fn get_status(State(state): State<AppState>) -> impl IntoResponse {
    match state.load_config() {
        Ok(cfg) => {
            let db_path = cfg.resolve_db_path();
            match atlas_core::Storage::new(&db_path) {
                Ok(storage) => match storage.get_stats() {
                    Ok(stats) => (
                        StatusCode::OK,
                        Json(json!({
                            "version": env!("CARGO_PKG_VERSION"),
                            "config_path": state.config_path.to_string_lossy(),
                            "db_path": db_path.to_string_lossy(),
                            "total_artifacts": stats.total_artifacts,
                            "total_objects": stats.total_artifacts,
                            "connectors_count": cfg.connectors.len(),
                            "db_size_bytes": stats.db_size_bytes,
                        })),
                    ),
                    Err(err) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": err.to_string() })),
                    ),
                },
                Err(err) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": err.to_string() })),
                ),
            }
        }
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": err.to_string() })),
        ),
    }
}


