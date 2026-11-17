use crate::state::AppState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct TimelineParams {
    pub kind: Option<String>,
    pub provider: Option<String>,
    pub limit: Option<usize>,
}

pub async fn get_timeline(
    State(state): State<AppState>,
    Query(params): Query<TimelineParams>,
) -> impl IntoResponse {
    let storage = match state.get_storage() {
        Ok(s) => s,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": err.to_string() })),
            );
        }
    };

    let limit = params.limit.unwrap_or(100).clamp(1, 500);
    match storage.query_timeline(params.kind.as_deref(), params.provider.as_deref(), limit) {
        Ok(items) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "items": items,
                "total": items.len()
            })),
        ),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        ),
    }
}
