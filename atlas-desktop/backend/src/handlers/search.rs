use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SearchQueryParams {
    pub query: Option<String>,
    pub object_type: Option<String>,
    pub tag: Option<String>,
    pub limit: Option<usize>,
}

pub async fn search_objects(
    State(state): State<AppState>,
    Query(params): Query<SearchQueryParams>,
) -> impl IntoResponse {
    let storage = match state.get_storage() {
        Ok(s) => s,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": err.to_string() })),
            )
        }
    };

    let limit = params.limit.unwrap_or(20);

    let query_str = params.query.unwrap_or_default();
    let results = if !query_str.trim().is_empty() {
        storage.search_fts(&query_str, limit)
    } else {
        storage.query_structured(params.object_type.as_deref(), params.tag.as_deref(), limit)
    };

    match results {
        Ok(objs) => (StatusCode::OK, Json(serde_json::json!(objs))),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        ),
    }
}

pub async fn get_object_by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let storage = match state.get_storage() {
        Ok(s) => s,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": err.to_string() })),
            )
        }
    };

    match storage.get_object_by_id(&id) {
        Ok(Some(obj)) => (StatusCode::OK, Json(serde_json::json!(obj))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("KnowledgeObject with ID '{}' not found", id) })),
        ),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        ),
    }
}
