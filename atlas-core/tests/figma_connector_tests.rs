use atlas_core::{
    ArtifactKind, ConnectorConfig, KnowledgeArtifact, Storage, FigmaConnector, Connector,
};
use tempfile::tempdir;

#[test]
fn test_figma_config_deserialization() {
    let toml_content = r#"
        [[connectors]]
        type = "figma"
        token = "figd_test_token"
        enabled = true
        file_keys = ["aBC123xYz", "kD9812mN"]
        parse_depth = 4
    "#;

    #[derive(serde::Deserialize)]
    struct Wrapper {
        connectors: Vec<ConnectorConfig>,
    }

    let parsed: Wrapper = toml::from_str(toml_content).unwrap();
    assert_eq!(parsed.connectors.len(), 1);
    let conn = &parsed.connectors[0];
    assert_eq!(conn.provider, "figma");
    assert_eq!(conn.api_token.as_deref(), Some("figd_test_token"));
    assert_eq!(conn.enabled, Some(true));
    assert_eq!(conn.file_keys, vec!["aBC123xYz", "kD9812mN"]);
    assert_eq!(conn.parse_depth, Some(4));
}

#[test]
fn test_figma_node_storage_and_alias_resolution() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("test_figma.db");
    let storage = Storage::new(&db_path)?;

    let now = chrono::Utc::now();

    let figma_component = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("figma", "https://api.figma.com", "aBC123xYz:1-42"),
        kind: ArtifactKind::Component,
        title: "PrimaryButton (COMPONENT)".to_string(),
        summary: Some("Type: COMPONENT | Node ID: 1-42".to_string()),
        body: "Figma Component spec for PrimaryButton.".to_string(),
        provider: "figma".to_string(),
        source_id: "aBC123xYz:1-42".to_string(),
        source_url: "https://www.figma.com/file/aBC123xYz?node-id=1-42".to_string(),
        repository: None,
        tags: vec!["figma".to_string(), "component".to_string()],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "checksum_figma_1_42".to_string(),
        metadata: serde_json::json!({ "id": "1-42", "type": "COMPONENT" }),
    };

    storage.upsert_artifacts_batch(&[figma_component])?;

    let matches = storage.resolve_artifact_by_alias("aBC123xYz:1-42")?;
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].provider, "figma");

    let mut cfg = ConnectorConfig::default();
    cfg.provider = "figma".to_string();
    cfg.api_token = Some("figd_test".to_string());
    cfg.file_keys = vec!["aBC123xYz".to_string()];
    let conn = FigmaConnector::new("figma-inst".to_string(), cfg)?;
    assert_eq!(conn.provider(), "figma");

    Ok(())
}
