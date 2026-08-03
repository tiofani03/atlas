use crate::domain::ArtifactKind;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Ingestion action executed on a normalized artifact
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncAction {
    Created,
    Updated,
    SkippedUnchanged,
    Deleted,
}

/// Strongly-typed Event Architecture for Progress & Metrics Engine
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "payload", rename_all = "snake_case")]
pub enum ProgressEvent {
    SyncStarted {
        connector_id: String,
        total_expected: Option<u64>,
    },
    /// Published once a fetch completes and the number of artifacts to index is
    /// known, so consumers can compute a real percentage during the DB phase.
    ItemsDiscovered {
        connector_id: String,
        total: u64,
    },
    OperationChanged {
        connector_id: String,
        operation: String,
        target: String,
    },
    ItemProcessed {
        connector_id: String,
        kind: ArtifactKind,
        action: SyncAction,
    },
    CheckpointSaved {
        connector_id: String,
        watermark: String,
        total_processed: u64,
    },
    RateLimitTriggered {
        connector_id: String,
        retry_after_secs: u64,
    },
    WorkerStatUpdate {
        worker_id: usize,
        items_per_sec: f64,
        bytes_per_sec: f64,
    },
    SyncCompleted {
        connector_id: String,
        total_synced: u64,
        elapsed_secs: f64,
    },
    SyncFailed {
        connector_id: String,
        error: String,
    },
}

/// Pub-Sub Broadcast Progress Event Bus
#[derive(Clone)]
pub struct ProgressEventBus {
    sender: broadcast::Sender<ProgressEvent>,
}

impl ProgressEventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn publish(&self, event: ProgressEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ProgressEvent> {
        self.sender.subscribe()
    }
}

impl Default for ProgressEventBus {
    fn default() -> Self {
        Self::new(1024)
    }
}
