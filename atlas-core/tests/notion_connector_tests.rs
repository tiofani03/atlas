use atlas_core::{
    ArtifactKind, ConnectorConfig, KnowledgeArtifact, Storage, NotionConnector, Connector,
};
use tempfile::tempdir;

#[test]
fn test_notion_config_deserialization() {
    let toml_content = r#"
        [[connectors]]
        type = "notion"
        token = "secret_notion_test_123"
        enabled = true
        database_ids = ["4a98120c921a4f00", "8b123901a091bf23"]
    "#;

    #[derive(serde::Deserialize)]
    struct Wrapper {
        connectors: Vec<ConnectorConfig>,
    }

    let parsed: Wrapper = toml::from_str(toml_content).unwrap();
    assert_eq!(parsed.connectors.len(), 1);
    let conn = &parsed.connectors[0];
    assert_eq!(conn.provider, "notion");
    assert_eq!(conn.api_token.as_deref(), Some("secret_notion_test_123"));
    assert_eq!(conn.enabled, Some(true));
    assert_eq!(conn.database_ids, vec!["4a98120c921a4f00", "8b123901a091bf23"]);
}

#[test]
fn test_notion_page_storage_and_alias_resolution() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("test_notion.db");
    let storage = Storage::new(&db_path)?;

    let now = chrono::Utc::now();

    let notion_page = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("notion", "https://api.notion.com", "page:4a98120c921a4f00"),
        kind: ArtifactKind::Document,
        title: "Architecture Design: Atlas Distributed Storage".to_string(),
        summary: Some("Notion Database Page".to_string()),
        body: "Document contents detailing sharding, replication, and WAL sync.".to_string(),
        provider: "notion".to_string(),
        source_id: "page:4a98120c921a4f00".to_string(),
        source_url: "https://notion.so/4a98120c921a4f00".to_string(),
        repository: None,
        tags: vec!["notion:page".to_string()],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "checksum_notion_4a98".to_string(),
        metadata: serde_json::json!({ "id": "4a98120c921a4f00" }),
    };

    storage.upsert_artifacts_batch(&[notion_page])?;

    let matches = storage.resolve_artifact_by_alias("page:4a98120c921a4f00")?;
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].provider, "notion");

    let mut cfg = ConnectorConfig::default();
    cfg.provider = "notion".to_string();
    cfg.api_token = Some("secret_test".to_string());
    let conn = NotionConnector::new("notion-inst".to_string(), cfg)?;
    assert_eq!(conn.provider(), "notion");

    Ok(())
}
