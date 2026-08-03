use crate::connectors::Connector;
use crate::progress::{ProgressEvent, ProgressEventBus};
use crate::storage::Storage;
use anyhow::{Context, Result};
use chrono::Utc;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SyncSummary {
    pub connector_id: String,
    pub fetched: usize,
    pub inserted: usize,
    pub updated: usize,
    pub skipped: usize,
}

pub struct SyncEngine;

impl SyncEngine {
    /// Run a single-connector sync, publishing per-item progress events to the
    /// given bus. The fetch and DB phases both emit so consumers (CLI renderer,
    /// desktop backend) can show live stage + item counters.
    pub async fn run_sync_with_progress(
        connector: &dyn Connector,
        storage: &Storage,
        force_full: bool,
        progress: &ProgressEventBus,
    ) -> Result<SyncSummary> {
        let connector_id = connector.id().to_string();
        let provider = connector.provider().to_string();

        progress.publish(ProgressEvent::SyncStarted {
            connector_id: connector_id.clone(),
            total_expected: None,
        });
        progress.publish(ProgressEvent::OperationChanged {
            connector_id: connector_id.clone(),
            operation: "fetching".to_string(),
            target: provider.clone(),
        });

        let last_sync = if force_full {
            None
        } else {
            storage.get_last_sync(&connector_id).unwrap_or(None)
        };

        let artifacts = connector
            .fetch_modified(last_sync)
            .await
            .with_context(|| format!("Error fetching items for connector '{}'", connector_id))?;

        let total = artifacts.len();

        progress.publish(ProgressEvent::ItemsDiscovered {
            connector_id: connector_id.clone(),
            total: total as u64,
        });
        progress.publish(ProgressEvent::OperationChanged {
            connector_id: connector_id.clone(),
            operation: "indexing".to_string(),
            target: provider.clone(),
        });

        let (inserted, updated, skipped) =
            storage.upsert_artifacts_batch(&artifacts, Some(progress))?;

        progress.publish(ProgressEvent::OperationChanged {
            connector_id: connector_id.clone(),
            operation: "saving".to_string(),
            target: provider.clone(),
        });

        storage.update_last_sync(
            &connector_id,
            &provider,
            Utc::now(),
            "success",
            None,
        )?;

        let summary = SyncSummary {
            connector_id: connector_id.clone(),
            fetched: total,
            inserted,
            updated,
            skipped,
        };

        progress.publish(ProgressEvent::SyncCompleted {
            connector_id: connector_id.clone(),
            total_synced: total as u64,
            elapsed_secs: 0.0,
        });

        Ok(summary)
    }

    pub async fn run_sync(
        connector: &dyn Connector,
        storage: &Storage,
        force_full: bool,
    ) -> Result<SyncSummary> {
        let progress = ProgressEventBus::new(16);
        Self::run_sync_with_progress(connector, storage, force_full, &progress).await
    }
}
