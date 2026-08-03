use crate::state::AppState;
use atlas_core::{
    AsanaConnector, AzureDevopsConnector, BitbucketConnector, ClickupConnector, ConfluenceConnector,
    ConnectorConfig, ConnectorInstance, FigmaConnector, GithubConnector, GitlabConnector, JiraConnector,
    LinearConnector, LocalGitConnector, MarkdownConnector, NotionConnector, OpenapiConnector,
    ProgressEvent, ProgressEventBus, SpreadsheetConnector, SyncAction, SyncEngine,
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
    {
        let progress = state.sync_progress.read().await;
        if progress.is_running {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({ "error": "Sync is already in progress" })),
            );
        }
    }

    // Reset progress snapshot.
    {
        let mut p = state.sync_progress.write().await;
        p.is_running = true;
        p.fetched = 0;
        p.inserted = 0;
        p.updated = 0;
        p.skipped = 0;
        p.current = 0;
        p.total = 0;
        p.percentage = 0.0;
        p.phase = Some("Queued...".to_string());
        p.error = None;
        p.current_connector = payload.connector_id.clone();
    }

    let event_bus = ProgressEventBus::new(1024);
    let cancel = tokio_util::sync::CancellationToken::new();
    let cancel_subscriber = cancel.clone();
    let cancel_run = cancel.clone();
    {
        let mut slot = state.cancel_token.lock().await;
        *slot = Some(cancel.clone());
    }

    let state_clone = state.clone();
    let connector_id = payload.connector_id;
    let force_full = payload.full.unwrap_or(false);

    // Subscriber task: translate bus events into the shared progress snapshot.
    let progress_state = state_clone.clone();
    let subscriber_bus = event_bus.clone();
    tokio::spawn(async move {
        let mut rx = subscriber_bus.subscribe();
        loop {
            tokio::select! {
                _ = cancel_subscriber.cancelled() => break,
                evt = rx.recv() => {
                    let Ok(event) = evt else { break };
                    let mut p = progress_state.sync_progress.write().await;
                    match event {
                        ProgressEvent::SyncStarted { connector_id, .. } => {
                            p.current_connector = Some(connector_id);
                            p.phase = Some("Connecting...".to_string());
                        }
                        ProgressEvent::ItemsDiscovered { total, .. } => {
                            p.total = total as usize;
                            if total == 0 {
                                p.percentage = 100.0;
                            } else {
                                p.percentage = 0.0;
                            }
                        }
                        ProgressEvent::OperationChanged { operation, .. } => {
                            p.phase = Some(match operation.as_str() {
                                "fetching" => "Fetching artifacts...".to_string(),
                                "indexing" => "Parsing & indexing...".to_string(),
                                "saving" => "Saving to database...".to_string(),
                                other => other.to_string(),
                            });
                        }
                        ProgressEvent::ItemProcessed { action, .. } => {
                            match action {
                                SyncAction::Created => p.inserted += 1,
                                SyncAction::Updated => p.updated += 1,
                                SyncAction::SkippedUnchanged => p.skipped += 1,
                                SyncAction::Deleted => {}
                            }
                            p.fetched += 1;
                            let done = p.inserted + p.updated + p.skipped;
                            if p.total > 0 {
                                p.percentage = (done as f32 / p.total as f32) * 100.0;
                            }
                        }
                        ProgressEvent::CheckpointSaved { .. } | ProgressEvent::RateLimitTriggered { .. } | ProgressEvent::WorkerStatUpdate { .. } => {}
                        ProgressEvent::SyncCompleted { .. } => {
                            p.is_running = false;
                            p.phase = Some("Completed".to_string());
                            p.percentage = 100.0;
                        }
                        ProgressEvent::SyncFailed { error, .. } => {
                            p.is_running = false;
                            p.error = Some(error);
                            break;
                        }
                    }
                }
            }
        }
    });

    let state_run = state.clone();
    tokio::spawn(async move {
        let run_result = async {
            let cfg = state_run.load_config()?;
            let storage = state_run.get_storage()?;

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
            let mut overall_fetched = 0usize;
            let mut overall_inserted = 0usize;
            let mut overall_updated = 0usize;
            let mut overall_skipped = 0usize;

            for (idx, (id, connector_cfg)) in target_connectors.into_iter().enumerate() {
                if cancel_run.is_cancelled() {
                    anyhow::bail!("Sync cancelled by user");
                }

                {
                    let mut p = state_run.sync_progress.write().await;
                    p.current_connector = Some(id.clone());
                    p.current = idx + 1;
                    // `p.total` is managed by the event subscriber as the item
                    // count; the connector count stays visible as "Connector X of Y".
                    p.phase = Some(format!(
                        "Connector {} of {}: {}",
                        idx + 1,
                        total_connectors,
                        id
                    ));
                    p.percentage = 0.0;
                }

                let conn_instance = match connector_cfg.provider.as_str() {
                    "jira" => ConnectorInstance::Jira(JiraConnector::new(id.clone(), connector_cfg)?),
                    "confluence" => ConnectorInstance::Confluence(ConfluenceConnector::new(id.clone(), connector_cfg)?),
                    "github" => ConnectorInstance::Github(GithubConnector::new(id.clone(), connector_cfg)?),
                    "clickup" => ConnectorInstance::Clickup(ClickupConnector::new(id.clone(), connector_cfg)?),
                    "linear" => ConnectorInstance::Linear(LinearConnector::new(id.clone(), connector_cfg)?),
                    "asana" => ConnectorInstance::Asana(AsanaConnector::new(id.clone(), connector_cfg)?),
                    "azure_devops" => ConnectorInstance::AzureDevops(AzureDevopsConnector::new(id.clone(), connector_cfg)?),
                    "gitlab" => ConnectorInstance::Gitlab(GitlabConnector::new(id.clone(), connector_cfg)?),
                    "bitbucket" => ConnectorInstance::Bitbucket(BitbucketConnector::new(id.clone(), connector_cfg)?),
                    "openapi" => ConnectorInstance::Openapi(OpenapiConnector::new(id.clone(), connector_cfg)?),
                    "figma" => ConnectorInstance::Figma(FigmaConnector::new(id.clone(), connector_cfg)?),
                    "notion" => ConnectorInstance::Notion(NotionConnector::new(id.clone(), connector_cfg)?),
                    "spreadsheet" => ConnectorInstance::Spreadsheet(SpreadsheetConnector::new(id.clone(), connector_cfg)?),
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

                let summary =
                    SyncEngine::run_sync_with_progress(&conn_instance, &storage, force_full, &event_bus).await?;

                overall_fetched += summary.fetched;
                overall_inserted += summary.inserted;
                overall_updated += summary.updated;
                overall_skipped += summary.skipped;

                let mut p = state_run.sync_progress.write().await;
                p.fetched = overall_fetched;
                p.inserted = overall_inserted;
                p.updated = overall_updated;
                p.skipped = overall_skipped;
            }

            Ok::<(), anyhow::Error>(())
        }
        .await;

        // Finalize after all connectors (or on first error/cancel).
        {
            let mut p = state_run.sync_progress.write().await;
            p.is_running = false;
            if let Err(e) = run_result {
                p.error = Some(format!("{:#}", e));
                p.phase = Some("Failed".to_string());
                // Publish a terminal event so the subscriber loop can stop cleanly.
                event_bus.publish(ProgressEvent::SyncFailed {
                    connector_id: "all".to_string(),
                    error: format!("{:#}", e),
                });
            } else {
                p.phase = Some("Completed".to_string());
                p.percentage = 100.0;
            }
            p.last_completed_at = Some(Utc::now().to_rfc3339());
        }

        {
            let mut slot = state_run.cancel_token.lock().await;
            *slot = None;
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

pub async fn cancel_sync(State(state): State<AppState>) -> impl IntoResponse {
    let cancel = { state.cancel_token.lock().await.clone() };
    match cancel {
        Some(token) => {
            token.cancel();
            {
                let mut p = state.sync_progress.write().await;
                p.phase = Some("Cancelling...".to_string());
            }
            (
                StatusCode::OK,
                Json(serde_json::json!({ "success": true, "message": "Cancellation requested." })),
            )
        }
        None => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": "No sync is currently running." })),
        ),
    }
}
