use crate::connectors::Connector;
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
    pub async fn run_sync(
        connector: &dyn Connector,
        storage: &Storage,
        force_full: bool,
    ) -> Result<SyncSummary> {
        let connector_id = connector.id().to_string();
        let last_sync = if force_full {
            None
        } else {
            storage
                .get_last_sync(&connector_id)
                .unwrap_or(None)
        };

        let artifacts = connector
            .fetch_modified(last_sync)
            .await
            .with_context(|| format!("Error fetching items for connector '{}'", connector_id))?;

        let mut summary = SyncSummary {
            connector_id: connector_id.clone(),
            fetched: artifacts.len(),
            ..Default::default()
        };

        let now = Utc::now();

        let (inserted, updated, skipped) = storage.upsert_artifacts_batch(&artifacts)?;
        summary.inserted = inserted;
        summary.updated = updated;
        summary.skipped = skipped;

        storage.update_last_sync(
            &connector_id,
            connector.provider(),
            now,
            "success",
            None,
        )?;

        Ok(summary)
    }
}

