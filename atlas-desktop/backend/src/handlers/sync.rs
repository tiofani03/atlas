use crate::state::AppState;
use atlas_core::{
    ConfluenceConnector, ConnectorConfig, ConnectorInstance, GithubConnector, JiraConnector,
    LocalGitConnector, MarkdownConnector, SyncEngine,
};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use chrono::Utc;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct TriggerSyncPayload {
    pub connector_id: Option<String>,
    pub full: Option<bool>,
}

pub async fn trigger_sync(
    State(state): State<AppState>,
    Json(payload): Json<TriggerSyncPayload>,
) -> impl IntoResponse {
    let mut progress = state.sync_progress.write().await;

    if progress.is_running {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": "Sync is already in progress" })),
        );
    }

    progress.is_running = true;
    progress.fetched = 0;
    progress.inserted = 0;
    progress.updated = 0;
    progress.skipped = 0;
    progress.current = 0;
    progress.total = 0;
    progress.percentage = 0.0;
    progress.phase = Some("Initializing pipeline...".to_string());
    progress.error = None;
    progress.current_connector = payload.connector_id.clone();
    drop(progress);

    let state_clone = state.clone();
    let connector_id = payload.connector_id;
    let force_full = payload.full.unwrap_or(false);

    tokio::spawn(async move {
        let run_result = async {
            let cfg = state_clone.load_config()?;
            let storage = state_clone.get_storage()?;

            let mut target_connectors: Vec<(String, ConnectorConfig)> = Vec::new();
            if let Some(ref target_id) = connector_id {
                if let Some(c) = cfg.connectors.get(target_id) {
                    target_connectors.push((target_id.clone(), c.clone()));
                } else {
                    anyhow::bail!("Connector '{}' not found in configuration", target_id);
                }
            } else {
                for (id, c) in &cfg.connectors {
                    target_connectors.push((id.clone(), c.clone()));
                }
            }

            let total_connectors = target_connectors.len();
            for (idx, (id, connector_cfg)) in target_connectors.into_iter().enumerate() {
                {
                    let mut p = state_clone.sync_progress.write().await;
                    p.current_connector = Some(id.clone());
                    p.phase = Some(format!("Ingesting context for {}...", id));
                    p.current = idx + 1;
                    p.total = total_connectors;
                    p.percentage = ((idx as f32) / (total_connectors as f32)) * 100.0;
                }

                let conn_instance = match connector_cfg.provider.as_str() {
                    "jira" => ConnectorInstance::Jira(JiraConnector::new(id.clone(), connector_cfg)?),
                    "confluence" => ConnectorInstance::Confluence(ConfluenceConnector::new(id.clone(), connector_cfg)?),
                    "github" => ConnectorInstance::Github(GithubConnector::new(id.clone(), connector_cfg)?),
                    "markdown" => {
                        let path_str = connector_cfg.path.as_deref().unwrap_or(".");
                        let mut conn = MarkdownConnector::new(id.clone(), path_str);
                        if !connector_cfg.glob_patterns.is_empty() {
                            conn = conn.with_glob_patterns(connector_cfg.glob_patterns.clone());
                        }
                        ConnectorInstance::Markdown(conn)
                    }
                    "local_git" => ConnectorInstance::LocalGit(LocalGitConnector::new_from_config(id.clone(), &connector_cfg)?),
                    _ => continue,
                };

                let summary = SyncEngine::run_sync(&conn_instance, &storage, force_full).await?;

                let mut p = state_clone.sync_progress.write().await;
                p.fetched += summary.fetched;
                p.inserted += summary.inserted;
                p.updated += summary.updated;
                p.skipped += summary.skipped;
                p.phase = Some(format!("Indexed {} artifacts", summary.fetched));
                p.percentage = (((idx + 1) as f32) / (total_connectors as f32)) * 100.0;
            }

            Ok::<(), anyhow::Error>(())
        }.await;

        let mut p = state_clone.sync_progress.write().await;
        p.is_running = false;
        p.percentage = 100.0;
        p.phase = None;
        p.last_completed_at = Some(Utc::now().to_rfc3339());
        if let Err(e) = run_result {
            p.error = Some(format!("{:#}", e));
        }
    });

    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({ "status": "started", "message": "Synchronization triggered." })),
    )
}

pub async fn get_sync_status(State(state): State<AppState>) -> impl IntoResponse {
    let progress = state.sync_progress.read().await;
    (StatusCode::OK, Json(serde_json::json!(*progress)))
}
