use crate::state::AppState;
use atlas_core::{Connector, ConnectorInstance, SyncEngine};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};

/// Receive inbound webhook events (e.g. GitHub, GitLab, Jira) and trigger incremental sync
/// for matching configured connectors.
pub async fn handle_webhook(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let provider_lower = provider.to_lowercase();
    if !matches!(provider_lower.as_str(), "github" | "gitlab" | "jira") {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("Webhook provider '{}' is not supported", provider)
            })),
        );
    }

    // Optional shared secret check when ATLAS_WEBHOOK_SECRET is configured
    if let Ok(expected_secret) = std::env::var("ATLAS_WEBHOOK_SECRET") {
        if !expected_secret.trim().is_empty() {
            let provided = headers
                .get("x-gitlab-token")
                .or_else(|| headers.get("x-atlas-webhook-secret"))
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            if provided != expected_secret.trim() {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({ "error": "Invalid webhook secret" })),
                );
            }
        }
    }

    let cfg = match state.load_config() {
        Ok(c) => c,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": err.to_string() })),
            );
        }
    };

    let matching: Vec<(String, atlas_core::ConnectorConfig)> = cfg
        .connectors
        .into_iter()
        .filter(|(_, c)| c.provider.eq_ignore_ascii_case(&provider_lower))
        .collect();

    if matching.is_empty() {
        return (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({
                "accepted": true,
                "triggered": 0,
                "message": format!("No configured connectors for provider '{}'", provider_lower),
                "event_keys": payload.as_object().map(|m| m.keys().cloned().collect::<Vec<_>>()).unwrap_or_default()
            })),
        );
    }

    let state_clone = state.clone();
    let count = matching.len();
    tokio::spawn(async move {
        let Ok(storage) = state_clone.get_storage() else {
            return;
        };
        for (id, conn_cfg) in matching {
            if let Ok(instance) = ConnectorInstance::build(&id, &conn_cfg) {
                let _ = SyncEngine::run_sync(&instance as &dyn Connector, &storage, false).await;
            }
        }
    });

    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "accepted": true,
            "provider": provider_lower,
            "triggered": count
        })),
    )
}
