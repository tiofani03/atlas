use crate::connectors::Connector;
use crate::domain::KnowledgeArtifact;
use crate::health::HealthReport;
use crate::progress::{ProgressEvent, ProgressEventBus, SyncAction};
use crate::resilience::ConnectorError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::{self, BoxStream};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Dynamic Capability Discovery Matrix for Atlas V2 Connectors
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityMatrix {
    pub supports_incremental_sync: bool,
    pub supports_webhooks: bool,
    pub supports_streaming: bool,
    pub supports_schema_discovery: bool,
    pub supports_writeback: bool,
    pub supports_mcp: bool,
    pub max_parallel_workers: usize,
}

impl Default for CapabilityMatrix {
    fn default() -> Self {
        Self {
            supports_incremental_sync: true,
            supports_webhooks: false,
            supports_streaming: true,
            supports_schema_discovery: false,
            supports_writeback: false,
            supports_mcp: true,
            max_parallel_workers: 4,
        }
    }
}

/// Authentication Schema Metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfigSchema {
    pub auth_type: String,
    pub required_fields: Vec<String>,
}

/// Connector Plugin Manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorManifest {
    pub id: String,
    pub provider: String,
    pub version: String,
    pub name: String,
    pub description: String,
    pub capabilities: CapabilityMatrix,
    pub auth_schema: AuthConfigSchema,
}

/// Durable Checkpoint Cursor stored in SQLite/PostgreSQL
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncCursor {
    pub connector_id: String,
    pub last_synced_at: DateTime<Utc>,
    pub pagination_page: u64,
    pub opaque_token: Option<String>,
    pub total_items_processed: u64,
    pub checksum_watermark: String,
}

/// Stream-native Ingestion Chunk emitted by ConnectorV2
#[derive(Debug, Clone)]
pub enum IngestionChunk {
    /// Batch of normalized artifacts
    Artifacts(Vec<KnowledgeArtifact>),
    /// Persistent checkpoint marker
    Checkpoint(SyncCursor),
    /// Provider rate limit update
    RateLimitNotice { remaining: u64, reset_secs: u64 },
}

/// Runtime Sync Context
#[derive(Clone, Default)]
pub struct SyncContext {
    pub progress_bus: Option<ProgressEventBus>,
    pub batch_size: usize,
}

impl SyncContext {
    pub fn new(batch_size: usize) -> Self {
        Self {
            progress_bus: None,
            batch_size,
        }
    }

    pub fn with_progress_bus(mut self, bus: ProgressEventBus) -> Self {
        self.progress_bus = Some(bus);
        self
    }
}

/// Stream-Native Connector V2 Core Trait
#[async_trait]
pub trait ConnectorV2: Send + Sync {
    /// Returns static manifest and capability matrix
    fn manifest(&self) -> &ConnectorManifest;

    /// Health & Connectivity Check
    async fn health_check(&self) -> Result<HealthReport, ConnectorError>;

    /// Stream artifacts modified since cursor with backpressure
    async fn stream_modified<'a>(
        &'a self,
        ctx: SyncContext,
        cursor: Option<SyncCursor>,
    ) -> Result<BoxStream<'a, Result<IngestionChunk, ConnectorError>>, ConnectorError>;

    /// Execute a targeted action (e.g. MCP tool execution or write-back)
    async fn execute_action(
        &self,
        action: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, ConnectorError>;
}

/// Migration Adapter wrapping V1 Connector implementations into ConnectorV2
pub struct ConnectorV1Adapter<C: Connector + 'static> {
    inner: Arc<C>,
    manifest: ConnectorManifest,
}

impl<C: Connector + 'static> ConnectorV1Adapter<C> {
    pub fn new(connector: C) -> Self {
        let cid = connector.id().to_string();
        let provider = connector.provider().to_string();
        let manifest = ConnectorManifest {
            id: cid,
            provider: provider.clone(),
            version: "1.0.0-v2adapter".to_string(),
            name: format!("{} (V1 Adapter)", provider),
            description: format!("Atlas V1 Adapter for provider {}", provider),
            capabilities: CapabilityMatrix::default(),
            auth_schema: AuthConfigSchema {
                auth_type: "token".to_string(),
                required_fields: vec!["api_token".to_string()],
            },
        };

        Self {
            inner: Arc::new(connector),
            manifest,
        }
    }
}

#[async_trait]
impl<C: Connector + 'static> ConnectorV2 for ConnectorV1Adapter<C> {
    fn manifest(&self) -> &ConnectorManifest {
        &self.manifest
    }

    async fn health_check(&self) -> Result<HealthReport, ConnectorError> {
        let start = std::time::Instant::now();
        match self.inner.fetch_modified(None).await {
            Ok(_) => {
                let latency = start.elapsed().as_millis() as u64;
                Ok(HealthReport::new(
                    self.inner.id(),
                    self.inner.provider(),
                    true,
                    true,
                    latency,
                    100.0,
                    "V1 Adapter health check succeeded",
                ))
            }
            Err(e) => Ok(HealthReport::new(
                self.inner.id(),
                self.inner.provider(),
                false,
                false,
                start.elapsed().as_millis() as u64,
                0.0,
                format!("V1 Adapter health check failed: {}", e),
            )),
        }
    }

    async fn stream_modified<'a>(
        &'a self,
        ctx: SyncContext,
        cursor: Option<SyncCursor>,
    ) -> Result<BoxStream<'a, Result<IngestionChunk, ConnectorError>>, ConnectorError> {
        let since = cursor.as_ref().map(|c| c.last_synced_at);
        let cid = self.inner.id().to_string();

        if let Some(bus) = &ctx.progress_bus {
            bus.publish(ProgressEvent::SyncStarted {
                connector_id: cid.clone(),
                total_expected: None,
            });
        }

        let artifacts = match self.inner.fetch_modified(since).await {
            Ok(arts) => arts,
            Err(e) => {
                if let Some(bus) = &ctx.progress_bus {
                    bus.publish(ProgressEvent::SyncFailed {
                        connector_id: cid.clone(),
                        error: e.to_string(),
                    });
                }
                return Err(ConnectorError::Transient {
                    message: e.to_string(),
                    retry_after_secs: None,
                });
            }
        };

        let total_count = artifacts.len() as u64;

        if let Some(bus) = &ctx.progress_bus {
            for art in &artifacts {
                bus.publish(ProgressEvent::ItemProcessed {
                    connector_id: cid.clone(),
                    kind: art.kind.clone(),
                    action: SyncAction::Created,
                });
            }
        }

        let new_cursor = SyncCursor {
            connector_id: cid.clone(),
            last_synced_at: Utc::now(),
            pagination_page: cursor.as_ref().map(|c| c.pagination_page + 1).unwrap_or(1),
            opaque_token: None,
            total_items_processed: cursor.as_ref().map(|c| c.total_items_processed).unwrap_or(0) + total_count,
            checksum_watermark: format!("watermark-{}", Utc::now().timestamp()),
        };

        let chunks = vec![
            Ok(IngestionChunk::Artifacts(artifacts)),
            Ok(IngestionChunk::Checkpoint(new_cursor.clone())),
        ];

        if let Some(bus) = &ctx.progress_bus {
            bus.publish(ProgressEvent::CheckpointSaved {
                connector_id: cid.clone(),
                watermark: new_cursor.checksum_watermark.clone(),
                total_processed: new_cursor.total_items_processed,
            });
            bus.publish(ProgressEvent::SyncCompleted {
                connector_id: cid,
                total_synced: total_count,
                elapsed_secs: 0.1,
            });
        }

        Ok(stream::iter(chunks).boxed())
    }

    async fn execute_action(
        &self,
        action: &str,
        _params: serde_json::Value,
    ) -> Result<serde_json::Value, ConnectorError> {
        Err(ConnectorError::Permanent {
            message: format!("Action '{}' not supported on V1 Adapter", action),
        })
    }
}

/// Registry maintaining V2 connector instances
pub struct ConnectorRegistryV2 {
    connectors: std::sync::RwLock<std::collections::HashMap<String, Arc<dyn ConnectorV2>>>,
}

impl ConnectorRegistryV2 {
    pub fn new() -> Self {
        Self {
            connectors: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub fn register(&self, connector: Arc<dyn ConnectorV2>) {
        let manifest = connector.manifest();
        self.connectors
            .write()
            .unwrap()
            .insert(manifest.id.clone(), connector);
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn ConnectorV2>> {
        self.connectors.read().unwrap().get(id).cloned()
    }

    pub fn list(&self) -> Vec<ConnectorManifest> {
        self.connectors
            .read()
            .unwrap()
            .values()
            .map(|c| c.manifest().clone())
            .collect()
    }
}

impl Default for ConnectorRegistryV2 {
    fn default() -> Self {
        Self::new()
    }
}
