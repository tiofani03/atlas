use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

/// Optional enterprise bearer-token authentication middleware.
///
/// When `ATLAS_AUTH_TOKEN` is set in the environment, all `/api/*` requests must present
/// `Authorization: Bearer <token>`. When unset, local single-user mode remains active.
pub async fn require_bearer_if_configured(req: Request, next: Next) -> Response {
    let Ok(expected) = std::env::var("ATLAS_AUTH_TOKEN") else {
        return next.run(req).await;
    };
    let expected = expected.trim();
    if expected.is_empty() {
        return next.run(req).await;
    }

    let path = req.uri().path();
    if !path.starts_with("/api/") || path == "/api/status" {
        return next.run(req).await;
    }

    let provided = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
        .unwrap_or("");

    if provided != expected {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "Unauthorized: valid Bearer token required"
            })),
        )
            .into_response();
    }

    next.run(req).await
}
