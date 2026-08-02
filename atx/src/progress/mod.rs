use atlas_core::progress::{ProgressEvent, SyncAction};

/// CLI Progress Render Mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressRenderMode {
    InteractiveTui,
    CiConsole,
    JsonStream,
    Quiet,
}

/// Renderer consuming ProgressEvents from the event bus
pub struct ProgressRenderer {
    mode: ProgressRenderMode,
}

impl ProgressRenderer {
    pub fn new(mode: ProgressRenderMode) -> Self {
        Self { mode }
    }

    pub async fn listen_and_render(&self, mut bus_rx: tokio::sync::broadcast::Receiver<ProgressEvent>) {
        let start = std::time::Instant::now();
        let mut total_processed: u64 = 0;
        let mut skipped_count: u64 = 0;
        let mut created_count: u64 = 0;

        while let Ok(event) = bus_rx.recv().await {
            match self.mode {
                ProgressRenderMode::InteractiveTui | ProgressRenderMode::CiConsole => match event {
                    ProgressEvent::SyncStarted { connector_id, total_expected } => {
                        let exp_str = total_expected.map(|n| n.to_string()).unwrap_or_else(|| "unknown".to_string());
                        println!("┌─ [SYNC STARTED] Connector: '{}' | Expected Items: {}", connector_id, exp_str);
                    }
                    ProgressEvent::OperationChanged { connector_id, operation, target } => {
                        println!("│  ▶ [{}] Operation: {} -> {}", connector_id, operation, target);
                    }
                    ProgressEvent::ItemProcessed { connector_id, kind: _, action } => {
                        total_processed += 1;
                        match action {
                            SyncAction::Created | SyncAction::Updated => created_count += 1,
                            SyncAction::SkippedUnchanged => skipped_count += 1,
                            SyncAction::Deleted => {}
                        }

                        if total_processed % 500 == 0 || total_processed == 1 {
                            let elapsed = start.elapsed().as_secs_f64().max(0.001);
                            let rate = total_processed as f64 / elapsed;
                            println!(
                                "│  ⟳ [{}] Synced {} items ({:.0} items/sec) | Created: {} | Skipped: {}",
                                connector_id, total_processed, rate, created_count, skipped_count
                            );
                        }
                    }
                    ProgressEvent::CheckpointSaved { connector_id, watermark, total_processed } => {
                        println!("│  [✓] [{}] Checkpoint Committed | Watermark: {} | Total: {}", connector_id, watermark, total_processed);
                    }
                    ProgressEvent::RateLimitTriggered { connector_id, retry_after_secs } => {
                        println!("│  [!] [{}] Rate limit hit! Backing off {}s...", connector_id, retry_after_secs);
                    }
                    ProgressEvent::SyncCompleted { connector_id, total_synced, elapsed_secs } => {
                        println!(
                            "└─ [SYNC COMPLETED] Connector: '{}' | Synced: {} items in {:.2}s",
                            connector_id, total_synced, elapsed_secs
                        );
                        break;
                    }
                    ProgressEvent::SyncFailed { connector_id, error } => {
                        println!("└─ [SYNC FAILED] Connector: '{}' | Error: {}", connector_id, error);
                        break;
                    }
                    _ => {}
                },
                ProgressRenderMode::JsonStream => {
                    if let Ok(json) = serde_json::to_string(&event) {
                        println!("{}", json);
                    }
                }
                ProgressRenderMode::Quiet => {}
            }
        }
    }
}
