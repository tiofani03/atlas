use atlas_core::connectors::v2::{
    ConnectorV1Adapter, ConnectorV2, SyncContext,
};
use atlas_core::domain::{ArtifactKind, KnowledgeArtifact};
use atlas_core::health::HealthScoreCalculator;
use atlas_core::resilience::ResilienceManager;
use atlas_core::storage::Storage;
use atlas_core::Connector;
use async_trait::async_trait;
use chrono::Utc;
use std::time::Instant;
use tempfile::NamedTempFile;

struct HighVolumeDummyConnector {
    item_count: usize,
}

#[async_trait]
impl Connector for HighVolumeDummyConnector {
    fn id(&self) -> &str {
        "high-volume-bench"
    }

    fn provider(&self) -> &str {
        "benchmark_provider"
    }

    async fn fetch_modified(&self, _since: Option<chrono::DateTime<Utc>>) -> anyhow::Result<Vec<KnowledgeArtifact>> {
        let mut artifacts = Vec::with_capacity(self.item_count);
        for i in 0..self.item_count {
            let id = format!("item-{}", i);
            let art = KnowledgeArtifact {
                id: KnowledgeArtifact::generate_id("bench", "perf", &id),
                kind: ArtifactKind::Commit,
                title: format!("Benchmark Commit #{}", i),
                summary: Some("Performance testing summary text for Atlas V2".to_string()),
                body: "Full body contents of synthetic engineering artifact for benchmark analysis".to_string(),
                provider: "benchmark".to_string(),
                source_id: id,
                source_url: format!("https://benchmark.local/item/{}", i),
                repository: Some("org/bench-repo".to_string()),
                tags: vec!["benchmark".to_string(), "v2".to_string()],
                relationships: vec![],
                created_at: Some(Utc::now()),
                updated_at: Utc::now(),
                synced_at: Utc::now(),
                checksum: "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08".to_string(),
                metadata: serde_json::json!({"index": i}),
            };
            artifacts.push(art);
        }
        Ok(artifacts)
    }
}

#[tokio::test]
async fn benchmark_v2_streaming_ingestion() {
    let item_count = 50_000;
    println!("\n================================================================================");
    println!("  ATLAS V2 CONNECTOR STREAMING BENCHMARK (Item Count: {})", item_count);
    println!("================================================================================");

    let connector = HighVolumeDummyConnector { item_count };
    let adapter = ConnectorV1Adapter::new(connector);

    let start = Instant::now();
    let stream = adapter.stream_modified(SyncContext::new(1000), None).await.unwrap();

    use futures::StreamExt;
    let mut total_chunks = 0;
    let mut total_items = 0;

    let mut pin_stream = stream;
    while let Some(chunk) = pin_stream.next().await {
        let chunk = chunk.unwrap();
        total_chunks += 1;
        if let atlas_core::connectors::v2::IngestionChunk::Artifacts(arts) = chunk {
            total_items += arts.len();
        }
    }

    let elapsed = start.elapsed();
    let items_per_sec = total_items as f64 / elapsed.as_secs_f64();
    let mb_processed = (total_items * 350) as f64 / 1024.0 / 1024.0; // ~350 bytes per artifact
    let mb_per_sec = mb_processed / elapsed.as_secs_f64();

    println!("  Streaming Duration  : {:.3?} s", elapsed.as_secs_f64());
    println!("  Total Chunks        : {}", total_chunks);
    println!("  Total Items Streamed: {}", total_items);
    println!("  Throughput          : {:.0} items/sec", items_per_sec);
    println!("  Data Rate           : {:.2} MB/sec", mb_per_sec);
    println!("================================================================================\n");

    assert_eq!(total_items, item_count);
    assert!(items_per_sec > 10_000.0, "Throughput should exceed 10,000 items/sec");
}

#[tokio::test]
async fn benchmark_resilience_manager_overhead() {
    let res = ResilienceManager::new("bench-resilience", 100);
    let iterations = 100_000;

    let start = Instant::now();
    for _ in 0..iterations {
        let res_ref = &res;
        let _ = res_ref.execute(|| async { Ok::<i32, atlas_core::resilience::ConnectorError>(42) }).await;
    }
    let elapsed = start.elapsed();
    let nanos_per_op = elapsed.as_nanos() as f64 / iterations as f64;

    println!("================================================================================");
    println!("  RESILIENCE MANAGER OVERHEAD BENCHMARK (Iterations: {})", iterations);
    println!("  Total Duration : {:.3?} ms", elapsed.as_millis());
    println!("  Latency per Op : {:.2} ns/op", nanos_per_op);
    println!("================================================================================\n");

    assert!(nanos_per_op < 5000.0, "Resilience overhead should be under 5us per call");
}

#[test]
fn benchmark_health_score_calculator() {
    let iterations = 1_000_000;
    let start = Instant::now();

    for i in 0..iterations {
        let _ = HealthScoreCalculator::calculate(true, true, (i % 100) as f64, (i % 1000) as u64);
    }
    let elapsed = start.elapsed();
    let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();

    println!("================================================================================");
    println!("  HEALTH SCORE CALCULATOR BENCHMARK (Iterations: {})", iterations);
    println!("  Total Duration : {:.3?} ms", elapsed.as_millis());
    println!("  Throughput     : {:.0} ops/sec", ops_per_sec);
    println!("================================================================================\n");

    assert!(ops_per_sec > 10_000_000.0, "Health score calc should exceed 10M ops/sec");
}

#[tokio::test]
async fn benchmark_checkpoint_storage_persistence() {
    let tmp = NamedTempFile::new().unwrap();
    let storage = Storage::new(tmp.path()).unwrap();
    let iterations = 1_000;

    let start = Instant::now();
    for i in 0..iterations {
        let cursor = atlas_core::connectors::v2::SyncCursor {
            connector_id: "bench-conn".to_string(),
            last_synced_at: Utc::now(),
            pagination_page: i as u64,
            opaque_token: Some(format!("token-{}", i)),
            total_items_processed: i as u64 * 10,
            checksum_watermark: format!("wm-{}", i),
        };
        storage.save_checkpoint(&cursor).unwrap();
        let _ = storage.load_checkpoint("bench-conn").unwrap();
    }
    let elapsed = start.elapsed();
    let ms_per_checkpoint = elapsed.as_secs_f64() * 1000.0 / iterations as f64;

    println!("================================================================================");
    println!("  CHECKPOINT STORAGE PERSISTENCE BENCHMARK (Iterations: {})", iterations);
    println!("  Total Duration : {:.3?} s", elapsed.as_secs_f64());
    println!("  Latency / Save : {:.3} ms/op", ms_per_checkpoint);
    println!("================================================================================\n");
}
