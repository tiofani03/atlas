use crate::state::AppState;
use atlas_core::ConnectorConfig;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct ConnectorItemResponse {
    pub id: String,
    pub provider: String,
    pub instance_url: String,
    pub email: String,
    pub projects: Vec<String>,
    pub spaces: Vec<String>,
    pub repos: Vec<String>,
    pub last_synced_at: Option<String>,
}

#[derive(Deserialize)]
pub struct JiraConfigPayload {
    pub id: String,
    pub instance_url: String,
    pub email: String,
    pub api_token: Option<String>,
    pub api_token_env: Option<String>,
    pub projects: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct ConfluenceConfigPayload {
    pub id: String,
    pub instance_url: String,
    pub email: String,
    pub api_token: Option<String>,
    pub api_token_env: Option<String>,
    pub spaces: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct GithubConfigPayload {
    pub id: String,
    pub instance_url: Option<String>,
    pub api_token: Option<String>,
    pub api_token_env: Option<String>,
    pub repos: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct ValidatePayload {
    pub provider: String,
    pub instance_url: String,
    pub email: String,
    pub api_token: String,
}

pub async fn list_connectors(State(state): State<AppState>) -> impl IntoResponse {
    let cfg = match state.load_config() {
        Ok(c) => c,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": err.to_string() })),
            )
        }
    };

    let storage = match state.get_storage() {
        Ok(s) => s,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": err.to_string() })),
            )
        }
    };

    let mut result = Vec::new();
    for (id, conn_cfg) in &cfg.connectors {
        let last_sync = storage.get_last_sync(id).unwrap_or(None);
        result.push(ConnectorItemResponse {
            id: id.clone(),
            provider: conn_cfg.provider.clone(),
            instance_url: conn_cfg.instance_url.clone(),
            email: conn_cfg.email.clone(),
            projects: conn_cfg.projects.clone(),
            spaces: conn_cfg.spaces.clone(),
            repos: conn_cfg.repos.clone(),
            last_synced_at: last_sync.map(|d| d.to_rfc3339()),
        });
    }

    (StatusCode::OK, Json(serde_json::json!(result)))
}

pub async fn save_jira_connector(
    State(state): State<AppState>,
    Json(payload): Json<JiraConfigPayload>,
) -> impl IntoResponse {
    let mut cfg = match state.load_config() {
        Ok(c) => c,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": err.to_string() })),
            )
        }
    };

    cfg.connectors.insert(
        payload.id.clone(),
        ConnectorConfig {
            provider: "jira".to_string(),
            instance_url: payload.instance_url,
            email: payload.email,
            api_token: payload.api_token,
            api_token_env: payload.api_token_env,
            projects: payload.projects.unwrap_or_default(),
            spaces: Vec::new(),
            repos: Vec::new(),
        },
    );

    if let Err(err) = cfg.save_to_path(&state.config_path) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        );
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({ "success": true, "id": payload.id })),
    )
}

pub async fn save_confluence_connector(
    State(state): State<AppState>,
    Json(payload): Json<ConfluenceConfigPayload>,
) -> impl IntoResponse {
    let mut cfg = match state.load_config() {
        Ok(c) => c,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": err.to_string() })),
            )
        }
    };

    cfg.connectors.insert(
        payload.id.clone(),
        ConnectorConfig {
            provider: "confluence".to_string(),
            instance_url: payload.instance_url,
            email: payload.email,
            api_token: payload.api_token,
            api_token_env: payload.api_token_env,
            projects: Vec::new(),
            spaces: payload.spaces.unwrap_or_default(),
            repos: Vec::new(),
        },
    );

    if let Err(err) = cfg.save_to_path(&state.config_path) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        );
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({ "success": true, "id": payload.id })),
    )
}

pub async fn save_github_connector(
    State(state): State<AppState>,
    Json(payload): Json<GithubConfigPayload>,
) -> impl IntoResponse {
    let mut cfg = match state.load_config() {
        Ok(c) => c,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": err.to_string() })),
            )
        }
    };

    cfg.connectors.insert(
        payload.id.clone(),
        ConnectorConfig {
            provider: "github".to_string(),
            instance_url: payload.instance_url.unwrap_or_else(|| "https://api.github.com".to_string()),
            email: String::new(),
            api_token: payload.api_token,
            api_token_env: payload.api_token_env,
            projects: Vec::new(),
            spaces: Vec::new(),
            repos: payload.repos.unwrap_or_default(),
        },
    );

    if let Err(err) = cfg.save_to_path(&state.config_path) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        );
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({ "success": true, "id": payload.id })),
    )
}

pub async fn validate_credentials(
    Json(payload): Json<ValidatePayload>,
) -> impl IntoResponse {
    let test_cfg = ConnectorConfig {
        provider: payload.provider.clone(),
        instance_url: payload.instance_url,
        email: payload.email,
        api_token: Some(payload.api_token),
        api_token_env: None,
        projects: Vec::new(),
        spaces: Vec::new(),
        repos: Vec::new(),
    };

    let result = match payload.provider.as_str() {
        "jira" => atlas_core::JiraConnector::new("test".to_string(), test_cfg).map(|_| ()),
        "confluence" => atlas_core::ConfluenceConnector::new("test".to_string(), test_cfg).map(|_| ()),
        "github" => atlas_core::GithubConnector::new("test".to_string(), test_cfg).map(|_| ()),
        _ => Err(anyhow::anyhow!("Unsupported provider")),
    };

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({ "valid": true, "message": "Credentials structure is valid." })),
        ),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "valid": false, "message": err.to_string() })),
        ),
    }
}
