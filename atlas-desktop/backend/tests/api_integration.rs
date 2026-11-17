use atlas_core::{ArtifactKind, Config, KnowledgeArtifact, Storage};
use atlas_desktop_backend::{routes::create_router, state::AppState};
use chrono::Utc;
use reqwest::StatusCode;
use tempfile::tempdir;

async fn spawn_test_server() -> (String, tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("atlas.db");
    let config_path = dir.path().join("config.toml");

    let mut cfg = Config::default();
    cfg.general.db_path = db_path.to_string_lossy().to_string();
    cfg.save_to_path(&config_path).unwrap();

    // Seed one artifact so search & hybrid search have data
    let storage = Storage::new(&db_path).unwrap();
    let art = KnowledgeArtifact {
        id: "doc:auth-guide".to_string(),
        kind: ArtifactKind::Document,
        title: "JWT Authentication Guide".to_string(),
        summary: Some("How token validation middleware works".to_string()),
        body: "Use HS256 or RS256 JWT tokens in the Authorization header.".to_string(),
        provider: "markdown".to_string(),
        source_id: "docs/auth.md".to_string(),
        source_url: "".to_string(),
        repository: None,
        tags: vec!["auth".to_string(), "security".to_string()],
        relationships: vec![],
        created_at: Some(Utc::now()),
        updated_at: Utc::now(),
        synced_at: Utc::now(),
        checksum: "chk-auth-1".to_string(),
        metadata: serde_json::json!({}),
    };
    storage.upsert_artifacts_batch(&[art]).unwrap();

    let state = AppState::new(config_path.clone());
    let app = create_router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (format!("http://{}", addr), dir, config_path)
}

#[tokio::test]
async fn test_status_and_schemas_and_cors() {
    let (base_url, _dir, _cfg) = spawn_test_server().await;
    let client = reqwest::Client::new();

    // 1. Status endpoint
    let status_res = client
        .get(format!("{}/api/status", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(status_res.status(), StatusCode::OK);
    let status_json: serde_json::Value = status_res.json().await.unwrap();
    assert_eq!(status_json["total_artifacts"], 1);

    // 2. Connector schemas endpoint returns all 15 manifests
    let schemas_res = client
        .get(format!("{}/api/connectors/schemas", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(schemas_res.status(), StatusCode::OK);
    let schemas: Vec<serde_json::Value> = schemas_res.json().await.unwrap();
    assert_eq!(schemas.len(), 15);

    // 3. CORS allows localhost origin and blocks untrusted origin
    let allowed_cors = client
        .get(format!("{}/api/status", base_url))
        .header("Origin", "http://localhost:31420")
        .send()
        .await
        .unwrap();
    assert_eq!(
        allowed_cors
            .headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("http://localhost:31420")
    );

    let blocked_cors = client
        .get(format!("{}/api/status", base_url))
        .header("Origin", "https://evil.example.com")
        .send()
        .await
        .unwrap();
    assert!(blocked_cors
        .headers()
        .get("access-control-allow-origin")
        .is_none());
}

#[tokio::test]
async fn test_generic_connector_save_hybrid_search_and_webhook() {
    let (base_url, dir, _cfg) = spawn_test_server().await;
    let client = reqwest::Client::new();

    // 1. Save markdown connector via generic schema-driven endpoint
    let save_res = client
        .post(format!("{}/api/connectors/markdown", base_url))
        .json(&serde_json::json!({
            "id": "md-docs",
            "paths": [dir.path().to_string_lossy()]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(save_res.status(), StatusCode::OK);
    let save_json: serde_json::Value = save_res.json().await.unwrap();
    assert_eq!(save_json["success"], true);

    // 2. Hybrid & semantic search
    let hybrid_res = client
        .get(format!("{}/api/search?query=JWT&mode=hybrid", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(hybrid_res.status(), StatusCode::OK);
    let hybrid_json: serde_json::Value = hybrid_res.json().await.unwrap();
    assert_eq!(hybrid_json["total"], 1);
    assert_eq!(hybrid_json["items"][0]["id"], "doc:auth-guide");

    let semantic_res = client
        .get(format!(
            "{}/api/search?query=authentication+middleware&mode=semantic",
            base_url
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(semantic_res.status(), StatusCode::OK);
    let semantic_json: serde_json::Value = semantic_res.json().await.unwrap();
    assert_eq!(semantic_json["total"], 1);

    // 3. Inbound webhook endpoint
    let webhook_res = client
        .post(format!("{}/api/webhooks/github", base_url))
        .json(&serde_json::json!({ "action": "opened" }))
        .send()
        .await
        .unwrap();
    assert_eq!(webhook_res.status(), StatusCode::ACCEPTED);
}
