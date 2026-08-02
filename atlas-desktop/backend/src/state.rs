use atlas_core::{Config, Storage};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct ActiveSyncProgress {
    pub is_running: bool,
    pub current_connector: Option<String>,
    pub phase: Option<String>,
    pub current: usize,
    pub total: usize,
    pub percentage: f32,
    pub fetched: usize,
    pub inserted: usize,
    pub updated: usize,
    pub skipped: usize,
    pub error: Option<String>,
    pub last_completed_at: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub config_path: PathBuf,
    pub sync_progress: Arc<RwLock<ActiveSyncProgress>>,
}

impl AppState {
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            sync_progress: Arc::new(RwLock::new(ActiveSyncProgress::default())),
        }
    }

    pub fn load_config(&self) -> anyhow::Result<Config> {
        Config::load_from_path(&self.config_path)
    }

    pub fn get_storage(&self) -> anyhow::Result<Storage> {
        let cfg = self.load_config()?;
        let db_path = cfg.resolve_db_path();
        Storage::new(db_path)
    }
}
