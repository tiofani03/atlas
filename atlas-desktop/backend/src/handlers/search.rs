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
    pub kind: Option<String>,
    pub object_type: Option<String>,
    pub provider: Option<String>,
    pub tag: Option<String>,
    pub repository: Option<String>,
    pub page: Option<usize>,
    pub limit: Option<usize>,
    pub mode: Option<String>,
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

    let limit = params.limit.unwrap_or(20).max(1);
    let page = params.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;

    let kind = params.kind.or(params.object_type);
    let provider = params.provider.filter(|p| !p.trim().is_empty());

    let query_str = params.query.unwrap_or_default();
    let mode = params.mode.as_deref().unwrap_or("bm25").to_lowercase();
    let results = if !query_str.trim().is_empty() {
        match mode.as_str() {
            "semantic" => storage
                .search_semantic(&query_str, kind.as_deref(), provider.as_deref(), limit)
                .map(|items| {
                    let total = items.len();
                    (items, total)
                }),
            "hybrid" => storage
                .search_hybrid(
                    &query_str,
                    kind.as_deref(),
                    provider.as_deref(),
                    params.tag.as_deref(),
                    params.repository.as_deref(),
                    limit,
                )
                .map(|items| {
                    let total = items.len();
                    (items, total)
                }),
            _ => storage.search_fts_paginated(
                &query_str,
                kind.as_deref(),
                provider.as_deref(),
                params.tag.as_deref(),
                params.repository.as_deref(),
                limit,
                offset,
            ),
        }
    } else {
        storage.query_structured_paginated(
            kind.as_deref(),
            provider.as_deref(),
            params.tag.as_deref(),
            params.repository.as_deref(),
            limit,
            offset,
        )
    };

    match results {
        Ok((items, total)) => {
            let total_pages = if total == 0 {
                1
            } else {
                total.div_ceil(limit)
            };

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "items": items,
                    "total": total,
                    "page": page,
                    "limit": limit,
                    "total_pages": total_pages
                })),
            )
        }
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

    match storage.get_artifact_by_id(&id) {
        Ok(Some(obj)) => (StatusCode::OK, Json(serde_json::json!(obj))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("KnowledgeArtifact with ID '{}' not found", id) })),
        ),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        ),
    }
}

#[derive(Deserialize)]
pub struct ContextQueryParams {
    pub kind: Option<String>,
    pub template: Option<String>,
}

pub async fn export_context_markdown(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<ContextQueryParams>,
) -> impl IntoResponse {
    let storage = match state.get_storage() {
        Ok(s) => s,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(axum::http::header::CONTENT_TYPE, "text/plain; charset=utf-8")],
                err.to_string(),
            );
        }
    };

    let builder = atlas_core::ContextBuilder::new(&storage);
    let options = atlas_core::ContextOptions::default();
    match builder.build(params.kind.as_deref(), &id, &options) {
        Ok(pkg) => {
            let md = atlas_core::render_context_markdown(&pkg, params.template.as_deref());
            (
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "text/markdown; charset=utf-8")],
                md,
            )
        }
        Err(err) => (
            StatusCode::NOT_FOUND,
            [(axum::http::header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            err.to_string(),
        ),
    }
}

pub async fn get_context(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<ContextQueryParams>,
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

    let builder = atlas_core::ContextBuilder::new(&storage);
    let options = atlas_core::ContextOptions::default();
    match builder.build(params.kind.as_deref(), &id, &options) {
        Ok(pkg) => (StatusCode::OK, Json(serde_json::json!(pkg))),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        ),
    }
}

