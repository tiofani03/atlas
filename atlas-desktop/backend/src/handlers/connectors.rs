use crate::state::AppState;
use atlas_core::{Connector, ConnectorConfig};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub teams: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lists: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub database_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    pub path: Option<String>,
    pub paths: Vec<String>,
    pub glob_patterns: Vec<String>,
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
pub struct MarkdownConfigPayload {
    pub id: String,
    pub path: Option<String>,
    pub paths: Option<Vec<String>>,
    pub glob_patterns: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct LocalGitConfigPayload {
    pub id: String,
    pub path: Option<String>,
    pub paths: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct ClickupConfigPayload {
    pub id: String,
    pub instance_url: Option<String>,
    pub api_token: Option<String>,
    pub api_token_env: Option<String>,
    pub workspace: Option<String>,
    pub spaces: Option<Vec<String>>,
    pub lists: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct LinearConfigPayload {
    pub id: String,
    pub instance_url: Option<String>,
    pub api_token: Option<String>,
    pub api_token_env: Option<String>,
    pub workspace: Option<String>,
    pub teams: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct GitlabConfigPayload {
    pub id: String,
    pub instance_url: Option<String>,
    pub api_token: Option<String>,
    pub api_token_env: Option<String>,
    pub projects: Option<Vec<String>>,
    pub ssl_cert_path: Option<String>,
}

#[derive(Deserialize)]
pub struct OpenapiConfigPayload {
    pub id: String,
    pub path: Option<String>,
    pub paths: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct AzureDevopsConfigPayload {
    pub id: String,
    pub instance_url: Option<String>,
    pub organization: Option<String>,
    pub api_token: Option<String>,
    pub api_token_env: Option<String>,
    pub projects: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct BitbucketConfigPayload {
    pub id: String,
    pub instance_url: Option<String>,
    pub workspace: Option<String>,
    pub email: Option<String>,
    pub api_token: Option<String>,
    pub api_token_env: Option<String>,
}

#[derive(Deserialize)]
pub struct FigmaConfigPayload {
    pub id: String,
    pub api_token: Option<String>,
    pub api_token_env: Option<String>,
    pub file_keys: Option<Vec<String>>,
    pub parse_depth: Option<usize>,
}

#[derive(Deserialize)]
pub struct NotionConfigPayload {
    pub id: String,
    pub api_token: Option<String>,
    pub api_token_env: Option<String>,
    pub database_ids: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct AsanaConfigPayload {
    pub id: String,
    pub workspace: Option<String>,
    pub api_token: Option<String>,
    pub api_token_env: Option<String>,
    pub projects: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct SpreadsheetConfigPayload {
    pub id: String,
    pub path: Option<String>,
    pub paths: Option<Vec<String>>,
    pub api_token: Option<String>,
    pub api_token_env: Option<String>,
    pub service_account_file: Option<String>,
    pub has_header_row: Option<bool>,
    pub max_rows_per_sheet: Option<usize>,
}

#[derive(Deserialize)]
pub struct ValidatePayload {
    pub provider: String,
    #[serde(default)]
    pub instance_url: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub api_token: String,
    #[serde(default)]
    pub organization: Option<String>,
    #[serde(default)]
    pub workspace: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
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
            workspace: conn_cfg.workspace.clone(),
            teams: conn_cfg.teams.clone(),
            lists: conn_cfg.lists.clone(),
            file_keys: conn_cfg.file_keys.clone(),
            database_ids: conn_cfg.database_ids.clone(),
            organization: conn_cfg.organization.clone(),
            path: conn_cfg.path.clone(),
            paths: conn_cfg.get_paths(),
            glob_patterns: conn_cfg.glob_patterns.clone(),
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

    let existing = cfg.connectors.get(&payload.id);

    let final_token = match payload.api_token {
        Some(ref t) if !t.trim().is_empty() => Some(t.trim().to_string()),
        _ => existing.and_then(|e| e.api_token.clone()),
    };

    let final_token_env = match payload.api_token_env {
        Some(ref e) if !e.trim().is_empty() => Some(e.trim().to_string()),
        _ => existing.and_then(|e| e.api_token_env.clone()),
    };

    let final_url = if !payload.instance_url.trim().is_empty() {
        payload.instance_url
    } else {
        existing.map(|e| e.instance_url.clone()).unwrap_or_default()
    };

    let final_email = if !payload.email.trim().is_empty() {
        payload.email
    } else {
        existing.map(|e| e.email.clone()).unwrap_or_default()
    };

    cfg.connectors.insert(
        payload.id.clone(),
        ConnectorConfig {
            provider: "jira".to_string(),
            instance_url: final_url,
            email: final_email,
            api_token: final_token,
            api_token_env: final_token_env,
            projects: payload.projects.unwrap_or_default(),
            spaces: Vec::new(),
            repos: Vec::new(),
            path: None,
            paths: Vec::new(),
            glob_patterns: Vec::new(),
            ..Default::default()
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

    let existing = cfg.connectors.get(&payload.id);

    let final_token = match payload.api_token {
        Some(ref t) if !t.trim().is_empty() => Some(t.trim().to_string()),
        _ => existing.and_then(|e| e.api_token.clone()),
    };

    let final_token_env = match payload.api_token_env {
        Some(ref e) if !e.trim().is_empty() => Some(e.trim().to_string()),
        _ => existing.and_then(|e| e.api_token_env.clone()),
    };

    let final_url = if !payload.instance_url.trim().is_empty() {
        payload.instance_url
    } else {
        existing.map(|e| e.instance_url.clone()).unwrap_or_default()
    };

    let final_email = if !payload.email.trim().is_empty() {
        payload.email
    } else {
        existing.map(|e| e.email.clone()).unwrap_or_default()
    };

    cfg.connectors.insert(
        payload.id.clone(),
        ConnectorConfig {
            provider: "confluence".to_string(),
            instance_url: final_url,
            email: final_email,
            api_token: final_token,
            api_token_env: final_token_env,
            projects: Vec::new(),
            spaces: payload.spaces.unwrap_or_default(),
            repos: Vec::new(),
            path: None,
            paths: Vec::new(),
            glob_patterns: Vec::new(),
            ..Default::default()
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

    let existing = cfg.connectors.get(&payload.id);

    let final_token = match payload.api_token {
        Some(ref t) if !t.trim().is_empty() => Some(t.trim().to_string()),
        _ => existing.and_then(|e| e.api_token.clone()),
    };

    let final_token_env = match payload.api_token_env {
        Some(ref e) if !e.trim().is_empty() => Some(e.trim().to_string()),
        _ => existing.and_then(|e| e.api_token_env.clone()),
    };

    let final_url = match payload.instance_url {
        Some(ref u) if !u.trim().is_empty() => u.clone(),
        _ => existing
            .map(|e| e.instance_url.clone())
            .unwrap_or_else(|| "https://api.github.com".to_string()),
    };

    cfg.connectors.insert(
        payload.id.clone(),
        ConnectorConfig {
            provider: "github".to_string(),
            instance_url: final_url,
            email: String::new(),
            api_token: final_token,
            api_token_env: final_token_env,
            projects: Vec::new(),
            spaces: Vec::new(),
            repos: payload.repos.unwrap_or_default(),
            path: None,
            paths: Vec::new(),
            glob_patterns: Vec::new(),
            ..Default::default()
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

pub async fn save_clickup_connector(
    State(state): State<AppState>,
    Json(payload): Json<ClickupConfigPayload>,
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

    let existing = cfg.connectors.get(&payload.id);

    let final_token = match payload.api_token {
        Some(ref t) if !t.trim().is_empty() => Some(t.trim().to_string()),
        _ => existing.and_then(|e| e.api_token.clone()),
    };

    let final_token_env = match payload.api_token_env {
        Some(ref e) if !e.trim().is_empty() => Some(e.trim().to_string()),
        _ => existing.and_then(|e| e.api_token_env.clone()),
    };

    let final_url = match payload.instance_url {
        Some(ref u) if !u.trim().is_empty() => u.clone(),
        _ => existing
            .map(|e| e.instance_url.clone())
            .unwrap_or_else(|| "https://api.clickup.com/api/v2".to_string()),
    };

    let final_workspace = match payload.workspace {
        Some(ref w) if !w.trim().is_empty() => Some(w.trim().to_string()),
        _ => existing.and_then(|e| e.workspace.clone()),
    };

    cfg.connectors.insert(
        payload.id.clone(),
        ConnectorConfig {
            provider: "clickup".to_string(),
            instance_url: final_url,
            email: String::new(),
            api_token: final_token,
            api_token_env: final_token_env,
            workspace: final_workspace,
            enabled: Some(true),
            projects: Vec::new(),
            spaces: payload.spaces.unwrap_or_default(),
            repos: Vec::new(),
            lists: payload.lists.unwrap_or_default(),
            path: None,
            paths: Vec::new(),
            glob_patterns: Vec::new(),
            ..Default::default()
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

pub async fn save_linear_connector(
    State(state): State<AppState>,
    Json(payload): Json<LinearConfigPayload>,
) -> impl IntoResponse {
    let mut cfg = match state.load_config() {
        Ok(c) => c,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": err.to_string() }))),
    };
    let existing = cfg.connectors.get(&payload.id);
    let final_token = payload.api_token.filter(|t| !t.trim().is_empty()).or_else(|| existing.and_then(|e| e.api_token.clone()));
    let final_token_env = payload.api_token_env.filter(|e| !e.trim().is_empty()).or_else(|| existing.and_then(|e| e.api_token_env.clone()));
    let final_url = payload.instance_url.filter(|u| !u.trim().is_empty()).unwrap_or_else(|| existing.map(|e| e.instance_url.clone()).unwrap_or_else(|| "https://api.linear.app/graphql".to_string()));

    cfg.connectors.insert(
        payload.id.clone(),
        ConnectorConfig {
            provider: "linear".to_string(),
            instance_url: final_url,
            api_token: final_token,
            api_token_env: final_token_env,
            workspace: payload.workspace,
            teams: payload.teams.unwrap_or_default(),
            enabled: Some(true),
            ..Default::default()
        },
    );

    if let Err(err) = cfg.save_to_path(&state.config_path) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": err.to_string() })));
    }
    (StatusCode::OK, Json(serde_json::json!({ "success": true, "id": payload.id })))
}

pub async fn save_gitlab_connector(
    State(state): State<AppState>,
    Json(payload): Json<GitlabConfigPayload>,
) -> impl IntoResponse {
    let mut cfg = match state.load_config() {
        Ok(c) => c,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": err.to_string() }))),
    };
    let existing = cfg.connectors.get(&payload.id);
    let final_token = payload.api_token.filter(|t| !t.trim().is_empty()).or_else(|| existing.and_then(|e| e.api_token.clone()));
    let final_token_env = payload.api_token_env.filter(|e| !e.trim().is_empty()).or_else(|| existing.and_then(|e| e.api_token_env.clone()));
    let final_url = payload.instance_url.filter(|u| !u.trim().is_empty()).unwrap_or_else(|| existing.map(|e| e.instance_url.clone()).unwrap_or_else(|| "https://gitlab.com".to_string()));

    cfg.connectors.insert(
        payload.id.clone(),
        ConnectorConfig {
            provider: "gitlab".to_string(),
            instance_url: final_url,
            api_token: final_token,
            api_token_env: final_token_env,
            projects: payload.projects.unwrap_or_default(),
            ssl_cert_path: payload.ssl_cert_path,
            enabled: Some(true),
            ..Default::default()
        },
    );

    if let Err(err) = cfg.save_to_path(&state.config_path) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": err.to_string() })));
    }
    (StatusCode::OK, Json(serde_json::json!({ "success": true, "id": payload.id })))
}

pub async fn save_openapi_connector(
    State(state): State<AppState>,
    Json(payload): Json<OpenapiConfigPayload>,
) -> impl IntoResponse {
    let mut cfg = match state.load_config() {
        Ok(c) => c,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": err.to_string() }))),
    };

    cfg.connectors.insert(
        payload.id.clone(),
        ConnectorConfig {
            provider: "openapi".to_string(),
            path: payload.path,
            paths: payload.paths.unwrap_or_default(),
            enabled: Some(true),
            ..Default::default()
        },
    );

    if let Err(err) = cfg.save_to_path(&state.config_path) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": err.to_string() })));
    }
    (StatusCode::OK, Json(serde_json::json!({ "success": true, "id": payload.id })))
}

pub async fn save_azure_devops_connector(
    State(state): State<AppState>,
    Json(payload): Json<AzureDevopsConfigPayload>,
) -> impl IntoResponse {
    let mut cfg = match state.load_config() {
        Ok(c) => c,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": err.to_string() }))),
    };
    let existing = cfg.connectors.get(&payload.id);
    let final_token = payload.api_token.filter(|t| !t.trim().is_empty()).or_else(|| existing.and_then(|e| e.api_token.clone()));
    let final_token_env = payload.api_token_env.filter(|e| !e.trim().is_empty()).or_else(|| existing.and_then(|e| e.api_token_env.clone()));
    let final_url = payload.instance_url.filter(|u| !u.trim().is_empty()).unwrap_or_else(|| existing.map(|e| e.instance_url.clone()).unwrap_or_else(|| "https://dev.azure.com".to_string()));

    cfg.connectors.insert(
        payload.id.clone(),
        ConnectorConfig {
            provider: "azure_devops".to_string(),
            instance_url: final_url,
            organization: payload.organization,
            api_token: final_token,
            api_token_env: final_token_env,
            projects: payload.projects.unwrap_or_default(),
            enabled: Some(true),
            ..Default::default()
        },
    );

    if let Err(err) = cfg.save_to_path(&state.config_path) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": err.to_string() })));
    }
    (StatusCode::OK, Json(serde_json::json!({ "success": true, "id": payload.id })))
}

pub async fn save_bitbucket_connector(
    State(state): State<AppState>,
    Json(payload): Json<BitbucketConfigPayload>,
) -> impl IntoResponse {
    let mut cfg = match state.load_config() {
        Ok(c) => c,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": err.to_string() }))),
    };
    let existing = cfg.connectors.get(&payload.id);
    let final_token = payload.api_token.filter(|t| !t.trim().is_empty()).or_else(|| existing.and_then(|e| e.api_token.clone()));
    let final_token_env = payload.api_token_env.filter(|e| !e.trim().is_empty()).or_else(|| existing.and_then(|e| e.api_token_env.clone()));
    let final_url = payload.instance_url.filter(|u| !u.trim().is_empty()).unwrap_or_else(|| existing.map(|e| e.instance_url.clone()).unwrap_or_else(|| "https://api.bitbucket.org/2.0".to_string()));

    cfg.connectors.insert(
        payload.id.clone(),
        ConnectorConfig {
            provider: "bitbucket".to_string(),
            instance_url: final_url,
            workspace: payload.workspace,
            email: payload.email.unwrap_or_default(),
            api_token: final_token,
            api_token_env: final_token_env,
            enabled: Some(true),
            ..Default::default()
        },
    );

    if let Err(err) = cfg.save_to_path(&state.config_path) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": err.to_string() })));
    }
    (StatusCode::OK, Json(serde_json::json!({ "success": true, "id": payload.id })))
}

pub async fn save_figma_connector(
    State(state): State<AppState>,
    Json(payload): Json<FigmaConfigPayload>,
) -> impl IntoResponse {
    let mut cfg = match state.load_config() {
        Ok(c) => c,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": err.to_string() }))),
    };
    let existing = cfg.connectors.get(&payload.id);
    let final_token = payload.api_token.filter(|t| !t.trim().is_empty()).or_else(|| existing.and_then(|e| e.api_token.clone()));
    let final_token_env = payload.api_token_env.filter(|e| !e.trim().is_empty()).or_else(|| existing.and_then(|e| e.api_token_env.clone()));

    cfg.connectors.insert(
        payload.id.clone(),
        ConnectorConfig {
            provider: "figma".to_string(),
            api_token: final_token,
            api_token_env: final_token_env,
            file_keys: payload.file_keys.unwrap_or_default(),
            parse_depth: payload.parse_depth,
            enabled: Some(true),
            ..Default::default()
        },
    );

    if let Err(err) = cfg.save_to_path(&state.config_path) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": err.to_string() })));
    }
    (StatusCode::OK, Json(serde_json::json!({ "success": true, "id": payload.id })))
}

pub async fn save_notion_connector(
    State(state): State<AppState>,
    Json(payload): Json<NotionConfigPayload>,
) -> impl IntoResponse {
    let mut cfg = match state.load_config() {
        Ok(c) => c,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": err.to_string() }))),
    };
    let existing = cfg.connectors.get(&payload.id);
    let final_token = payload.api_token.filter(|t| !t.trim().is_empty()).or_else(|| existing.and_then(|e| e.api_token.clone()));
    let final_token_env = payload.api_token_env.filter(|e| !e.trim().is_empty()).or_else(|| existing.and_then(|e| e.api_token_env.clone()));

    cfg.connectors.insert(
        payload.id.clone(),
        ConnectorConfig {
            provider: "notion".to_string(),
            api_token: final_token,
            api_token_env: final_token_env,
            database_ids: payload.database_ids.unwrap_or_default(),
            enabled: Some(true),
            ..Default::default()
        },
    );

    if let Err(err) = cfg.save_to_path(&state.config_path) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": err.to_string() })));
    }
    (StatusCode::OK, Json(serde_json::json!({ "success": true, "id": payload.id })))
}

pub async fn save_asana_connector(
    State(state): State<AppState>,
    Json(payload): Json<AsanaConfigPayload>,
) -> impl IntoResponse {
    let mut cfg = match state.load_config() {
        Ok(c) => c,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": err.to_string() }))),
    };
    let existing = cfg.connectors.get(&payload.id);
    let final_token = payload.api_token.filter(|t| !t.trim().is_empty()).or_else(|| existing.and_then(|e| e.api_token.clone()));
    let final_token_env = payload.api_token_env.filter(|e| !e.trim().is_empty()).or_else(|| existing.and_then(|e| e.api_token_env.clone()));

    cfg.connectors.insert(
        payload.id.clone(),
        ConnectorConfig {
            provider: "asana".to_string(),
            workspace: payload.workspace,
            api_token: final_token,
            api_token_env: final_token_env,
            projects: payload.projects.unwrap_or_default(),
            enabled: Some(true),
            ..Default::default()
        },
    );

    if let Err(err) = cfg.save_to_path(&state.config_path) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": err.to_string() })));
    }
    (StatusCode::OK, Json(serde_json::json!({ "success": true, "id": payload.id })))
}

pub async fn save_spreadsheet_connector(
    State(state): State<AppState>,
    Json(payload): Json<SpreadsheetConfigPayload>,
) -> impl IntoResponse {
    let mut cfg = match state.load_config() {
        Ok(c) => c,
        Err(err) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": err.to_string() }))),
    };

    let existing = cfg.connectors.get(&payload.id);
    let final_token = payload.api_token.filter(|t| !t.trim().is_empty()).or_else(|| existing.and_then(|e| e.api_token.clone()));
    let final_token_env = payload.api_token_env.filter(|e| !e.trim().is_empty()).or_else(|| existing.and_then(|e| e.api_token_env.clone()));

    cfg.connectors.insert(
        payload.id.clone(),
        ConnectorConfig {
            provider: "spreadsheet".to_string(),
            path: payload.path,
            paths: payload.paths.unwrap_or_default(),
            api_token: final_token,
            api_token_env: final_token_env,
            service_account_file: payload.service_account_file,
            has_header_row: payload.has_header_row,
            max_rows_per_sheet: payload.max_rows_per_sheet,
            enabled: Some(true),
            ..Default::default()
        },
    );

    if let Err(err) = cfg.save_to_path(&state.config_path) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": err.to_string() })));
    }
    (StatusCode::OK, Json(serde_json::json!({ "success": true, "id": payload.id })))
}

pub async fn validate_credentials(
    Json(payload): Json<ValidatePayload>,
) -> impl IntoResponse {
    // Providers whose verification is path/filesystem-based, not credential-based.
    let path_based = matches!(
        payload.provider.as_str(),
        "markdown" | "local_git" | "spreadsheet"
    );

    let test_cfg = ConnectorConfig {
        provider: payload.provider.clone(),
        instance_url: payload.instance_url,
        email: payload.email,
        api_token: (!payload.api_token.is_empty()).then(|| payload.api_token),
        api_token_env: None,
        organization: payload.organization,
        workspace: payload.workspace,
        path: payload.path.clone(),
        paths: payload
            .path
            .as_ref()
            .map(|p| p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
            .unwrap_or_default(),
        ..Default::default()
    };

    let timeout_secs = if path_based { 15 } else { 30 };

    let conn = match atlas_core::ConnectorInstance::build("test", &test_cfg) {
        Ok(c) => c,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "valid": false, "message": err.to_string() })),
            )
        }
    };

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        conn.verify(),
    )
    .await;

    match result {
        Ok(Ok(message)) => (
            StatusCode::OK,
            Json(serde_json::json!({ "valid": true, "message": message })),
        ),
        Ok(Err(err)) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "valid": false, "message": err.to_string() })),
        ),
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "valid": false,
                "message": format!("Verification timed out after {} seconds.", timeout_secs)
            })),
        ),
    }
}

pub async fn save_markdown_connector(
    State(state): State<AppState>,
    Json(payload): Json<MarkdownConfigPayload>,
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

    let paths_vec = payload.paths.unwrap_or_default();

    cfg.connectors.insert(
        payload.id.clone(),
        ConnectorConfig {
            provider: "markdown".to_string(),
            instance_url: String::new(),
            email: String::new(),
            api_token: None,
            api_token_env: None,
            projects: Vec::new(),
            spaces: Vec::new(),
            repos: Vec::new(),
            path: payload.path,
            paths: paths_vec,
            glob_patterns: payload.glob_patterns.unwrap_or_default(),
            ..Default::default()
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

pub async fn save_local_git_connector(
    State(state): State<AppState>,
    Json(payload): Json<LocalGitConfigPayload>,
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

    let paths_vec = payload.paths.unwrap_or_default();

    cfg.connectors.insert(
        payload.id.clone(),
        ConnectorConfig {
            provider: "local_git".to_string(),
            instance_url: String::new(),
            email: String::new(),
            api_token: None,
            api_token_env: None,
            projects: Vec::new(),
            spaces: Vec::new(),
            repos: Vec::new(),
            path: payload.path,
            paths: paths_vec,
            glob_patterns: Vec::new(),
            ..Default::default()
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


pub async fn select_folder() -> impl IntoResponse {
    let path: Option<String> = tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "macos")]
        {
            let output = std::process::Command::new("osascript")
                .arg("-e")
                .arg("POSIX path of (choose folder with prompt \"Select Markdown Directory\")")
                .output();

            if let Ok(out) = output {
                if out.status.success() {
                    let res = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !res.is_empty() {
                        return Some(res);
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            let output = std::process::Command::new("powershell")
                .arg("-Command")
                .arg("Add-Type -AssemblyName System.windows.forms; $f = New-Object System.Windows.Forms.FolderBrowserDialog; if ($f.ShowDialog() -eq 'OK') { $f.SelectedPath }")
                .output();

            if let Ok(out) = output {
                if out.status.success() {
                    let res = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !res.is_empty() {
                        return Some(res);
                    }
                }
            }
        }

        None
    })
    .await
    .unwrap_or(None);

    if let Some(p) = path {
        (
            StatusCode::OK,
            Json(serde_json::json!({ "success": true, "path": p })),
        )
    } else {
        (
            StatusCode::OK,
            Json(serde_json::json!({ "success": false, "path": null })),
        )
    }
}

#[derive(Deserialize)]
pub struct DeleteConnectorPayload {
    pub id: String,
    pub clear_data: Option<bool>,
}

pub async fn delete_connector(
    State(state): State<AppState>,
    Json(payload): Json<DeleteConnectorPayload>,
) -> impl IntoResponse {
    let mut cfg = match state.load_config() {
        Ok(c) => c,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": err.to_string() })),
            );
        }
    };

    let conn_cfg = cfg.connectors.remove(&payload.id);

    if let Err(err) = cfg.save_to_path(&state.config_path) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err.to_string() })),
        );
    }

    let mut cleared_count = 0;
    if payload.clear_data.unwrap_or(true) {
        if let Ok(storage) = state.get_storage() {
            let provider_opt = conn_cfg.as_ref().map(|c| c.provider.as_str());
            let repos = conn_cfg.as_ref().map(|c| c.repos.clone()).unwrap_or_default();
            cleared_count = storage
                .clear_connector_data(&payload.id, provider_opt, &repos)
                .unwrap_or(0);
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "id": payload.id,
            "cleared_artifacts": cleared_count
        })),
    )
}
