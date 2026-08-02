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
pub struct ClickUpConfigPayload {
    pub id: String,
    pub instance_url: Option<String>,
    pub api_token: Option<String>,
    pub api_token_env: Option<String>,
    pub workspaces: Option<Vec<String>>,
    pub spaces: Option<Vec<String>>,
    pub folders: Option<Vec<String>>,
    pub lists: Option<Vec<String>>,
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
    Json(payload): Json<ClickUpConfigPayload>,
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
        Some(ref u) if !u.trim().is_empty() => u.trim().to_string(),
        _ => existing
            .map(|e| e.instance_url.clone())
            .unwrap_or_else(|| "https://api.clickup.com/api/v2".to_string()),
    };

    cfg.connectors.insert(
        payload.id.clone(),
        ConnectorConfig {
            provider: "clickup".to_string(),
            instance_url: final_url,
            email: String::new(),
            api_token: final_token,
            api_token_env: final_token_env,
            projects: payload.workspaces.unwrap_or_default(),
            spaces: payload.spaces.unwrap_or_default(),
            repos: payload.lists.unwrap_or_default(),
            path: None,
            paths: payload.folders.unwrap_or_default(),
            glob_patterns: Vec::new(),
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

pub async fn validate_credentials(Json(payload): Json<ValidatePayload>) -> impl IntoResponse {
    let test_cfg = ConnectorConfig {
        provider: payload.provider.clone(),
        instance_url: payload.instance_url,
        email: payload.email,
        api_token: Some(payload.api_token),
        api_token_env: None,
        projects: Vec::new(),
        spaces: Vec::new(),
        repos: Vec::new(),
        path: None,
        paths: Vec::new(),
        glob_patterns: Vec::new(),
    };

    let result = match payload.provider.as_str() {
        "jira" => atlas_core::JiraConnector::new("test".to_string(), test_cfg).map(|_| ()),
        "confluence" => {
            atlas_core::ConfluenceConnector::new("test".to_string(), test_cfg).map(|_| ())
        }
        "github" => atlas_core::GithubConnector::new("test".to_string(), test_cfg).map(|_| ()),
        "clickup" => atlas_core::ClickUpConnector::new("test".to_string(), test_cfg).map(|_| ()),
        _ => Err(anyhow::anyhow!("Unsupported provider")),
    };

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(
                serde_json::json!({ "valid": true, "message": "Credentials structure is valid." }),
            ),
        ),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "valid": false, "message": err.to_string() })),
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
            let repos = conn_cfg
                .as_ref()
                .map(|c| c.repos.clone())
                .unwrap_or_default();
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
