use atlas_desktop_backend::{create_test_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::path::PathBuf;
use tower::ServiceExt;

#[tokio::test]
async fn cancel_without_running_sync_returns_409() {
    let state = AppState::new(PathBuf::from("/nonexistent/config.toml"));
    let app = create_test_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/sync/cancel")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn status_returns_progress_snapshot() {
    let state = AppState::new(PathBuf::from("/nonexistent/config.toml"));
    let app = create_test_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/sync/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["is_running"], false);
}

#[tokio::test]
async fn trigger_sync_returns_accepted_or_conflict() {
    // No connectors configured in the nonexistent config => the spawned run
    // fails fast, but the endpoint itself must respond 202 Accepted.
    let state = AppState::new(PathBuf::from("/nonexistent/config.toml"));
    let app = create_test_router(state);

    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/sync")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"full": false}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}
