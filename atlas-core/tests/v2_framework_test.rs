use atlas_core::connectors::v2::{
    ConnectorV1Adapter, ConnectorV2, SyncContext,
};
use atlas_core::domain::{ArtifactKind, KnowledgeArtifact};
use atlas_core::health::{ConnectorHealthState, HealthReport, HealthScoreCalculator};
use atlas_core::progress::ProgressEventBus;
use atlas_core::resilience::{
    CircuitBreaker, CircuitState, RetryBudget,
};
use atlas_core::storage::Storage;
use atlas_core::Connector;
use async_trait::async_trait;
use chrono::Utc;
use tempfile::NamedTempFile;

/// Dummy V1 Connector for Testing
struct DummyV1Connector {
    id: String,
    provider: String,
}

#[async_trait]
impl Connector for DummyV1Connector {
    fn id(&self) -> &str {
        &self.id
    }

    fn provider(&self) -> &str {
        &self.provider
    }

    async fn fetch_modified(&self, _since: Option<chrono::DateTime<Utc>>) -> anyhow::Result<Vec<KnowledgeArtifact>> {
        let art = KnowledgeArtifact {
            id: KnowledgeArtifact::generate_id("test", "dummy", "item-1"),
            kind: ArtifactKind::Issue,
            title: "Test Issue".to_string(),
            summary: Some("Test summary".to_string()),
            body: "Test body content".to_string(),
            provider: "test".to_string(),
            source_id: "item-1".to_string(),
            source_url: "https://example.com/item-1".to_string(),
            repository: Some("test/repo".to_string()),
            tags: vec!["bug".to_string()],
            relationships: vec![],
            created_at: Some(Utc::now()),
            updated_at: Utc::now(),
            synced_at: Utc::now(),
            checksum: KnowledgeArtifact::compute_checksum("Test Issue", Some("Test summary"), "Test body content", &["bug".to_string()]),
            metadata: serde_json::json!({}),
        };
        Ok(vec![art])
    }
}

#[tokio::test]
async fn test_v2_connector_v1_adapter() {
    let dummy = DummyV1Connector {
        id: "dummy-1".to_string(),
        provider: "test_provider".to_string(),
    };

    let adapter = ConnectorV1Adapter::new(dummy);
    assert_eq!(adapter.manifest().id, "dummy-1");
    assert_eq!(adapter.manifest().provider, "test_provider");

    let bus = ProgressEventBus::default();
    let ctx = SyncContext::new(100).with_progress_bus(bus.clone());

    let health = adapter.health_check().await.unwrap();
    assert_eq!(health.state, ConnectorHealthState::Healthy);
    assert_eq!(health.score, 100);

    let mut stream = adapter.stream_modified(ctx, None).await.unwrap();
    use futures::StreamExt;
    let first_chunk = stream.next().await.unwrap().unwrap();

    if let atlas_core::connectors::v2::IngestionChunk::Artifacts(arts) = first_chunk {
        assert_eq!(arts.len(), 1);
        assert_eq!(arts[0].title, "Test Issue");
    } else {
        panic!("Expected Artifacts chunk");
    }

    let second_chunk = stream.next().await.unwrap().unwrap();
    if let atlas_core::connectors::v2::IngestionChunk::Checkpoint(cursor) = second_chunk {
        assert_eq!(cursor.total_items_processed, 1);
        assert_eq!(cursor.connector_id, "dummy-1");
    } else {
        panic!("Expected Checkpoint chunk");
    }
}

#[tokio::test]
async fn test_resilience_circuit_breaker() {
    let cb = CircuitBreaker::new("test-cb", 3, 10);
    assert_eq!(cb.state(), CircuitState::Closed);

    cb.record_failure();
    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Closed);

    cb.record_failure();
    assert_eq!(cb.state(), CircuitState::Open);
}

#[tokio::test]
async fn test_retry_budget() {
    let budget = RetryBudget::new(50);
    assert!(budget.can_retry());

    // Consume retries until exhausted
    for _ in 0..5 {
        let ok = budget.consume_retry();
        assert!(ok);
    }
    assert!(!budget.can_retry());

    budget.record_success();
    // Replenishing tokens
    assert!(budget.available_tokens() > 0);
}

#[test]
fn test_health_score_calculation() {
    let score_healthy = HealthScoreCalculator::calculate(true, true, 100.0, 100);
    assert!(score_healthy >= 90);

    let score_degraded = HealthScoreCalculator::calculate(true, true, 60.0, 2000);
    assert!(score_degraded >= 50 && score_degraded < 90);

    let score_unavail = HealthScoreCalculator::calculate(false, false, 0.0, 5000);
    assert!(score_unavail < 50);
}

#[tokio::test]
async fn test_storage_checkpoint_and_health_persistence() {
    let tmp = NamedTempFile::new().unwrap();
    let storage = Storage::new(tmp.path()).unwrap();

    let cursor = atlas_core::connectors::v2::SyncCursor {
        connector_id: "conn-99".to_string(),
        last_synced_at: Utc::now(),
        pagination_page: 5,
        opaque_token: Some("token_abc".to_string()),
        total_items_processed: 4200,
        checksum_watermark: "wm-12345".to_string(),
    };

    storage.save_checkpoint(&cursor).unwrap();
    let loaded = storage.load_checkpoint("conn-99").unwrap().unwrap();
    assert_eq!(loaded.connector_id, "conn-99");
    assert_eq!(loaded.pagination_page, 5);
    assert_eq!(loaded.total_items_processed, 4200);

    let report = HealthReport::new("conn-99", "github", true, true, 120, 99.5, "Operational");
    storage.save_health_report(&report).unwrap();

    let reports = storage.load_all_health_reports().unwrap();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].connector_id, "conn-99");
    assert_eq!(reports[0].score, report.score);
}
