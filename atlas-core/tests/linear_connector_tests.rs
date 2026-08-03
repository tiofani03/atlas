use atlas_core::{
    ArtifactKind, ConnectorConfig, KnowledgeArtifact, Storage, LinearConnector, Connector,
};
use tempfile::tempdir;

#[test]
fn test_linear_config_deserialization() {
    let toml_content = r#"
        [[connectors]]
        type = "linear"
        token = "lin_api_test_123"
        workspace = "acme-engineering"
        enabled = true
        teams = ["ENG", "PROD"]
    "#;

    #[derive(serde::Deserialize)]
    struct Wrapper {
        connectors: Vec<ConnectorConfig>,
    }

    let parsed: Wrapper = toml::from_str(toml_content).unwrap();
    assert_eq!(parsed.connectors.len(), 1);
    let conn = &parsed.connectors[0];
    assert_eq!(conn.provider, "linear");
    assert_eq!(conn.api_token.as_deref(), Some("lin_api_test_123"));
    assert_eq!(conn.workspace.as_deref(), Some("acme-engineering"));
    assert_eq!(conn.enabled, Some(true));
    assert_eq!(conn.teams, vec!["ENG", "PROD"]);
}

#[test]
fn test_linear_artifact_storage_and_alias_resolution() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("test_linear.db");
    let storage = Storage::new(&db_path)?;

    let now = chrono::Utc::now();

    let linear_issue = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("linear", "https://api.linear.app/graphql", "ENG-501"),
        kind: ArtifactKind::Issue,
        title: "Optimize SQLite WAL mode performance".to_string(),
        summary: Some("Team: Engineering | State: In Progress".to_string()),
        body: "Investigate page size tuning and memory mapping.".to_string(),
        provider: "linear".to_string(),
        source_id: "ENG-501".to_string(),
        source_url: "https://linear.app/acme/issue/ENG-501".to_string(),
        repository: Some("ENG".to_string()),
        tags: vec!["team:Engineering".to_string(), "priority:high".to_string()],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "checksum_linear_501".to_string(),
        metadata: serde_json::json!({ "id": "issue-501", "identifier": "ENG-501" }),
    };

    let commit_artifact = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("github", "https://github.com", "sha998877"),
        kind: ArtifactKind::Commit,
        title: "perf(db): configure PRAGMA mmap_size ENG-501".to_string(),
        summary: None,
        body: "Ref ENG-501 for database performance".to_string(),
        provider: "github".to_string(),
        source_id: "acme/atlas@sha998877".to_string(),
        source_url: "https://github.com/acme/atlas/commit/sha998877".to_string(),
        repository: Some("acme/atlas".to_string()),
        tags: vec![],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "checksum_commit_998877".to_string(),
        metadata: serde_json::json!({}),
    };

    storage.upsert_artifacts_batch(&[linear_issue, commit_artifact], None)?;

    let matches = storage.resolve_artifact_by_alias("ENG-501")?;
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].source_id, "ENG-501");
    assert_eq!(matches[0].provider, "linear");

    let related = storage.get_related_artifacts("ENG-501")?;
    assert!(!related.is_empty(), "Should automatically link Git commit with Linear ticket ENG-501");

    let mut cfg = ConnectorConfig::default();
    cfg.provider = "linear".to_string();
    cfg.api_token = Some("lin_api_test".to_string());
    let conn = LinearConnector::new("linear-inst".to_string(), cfg)?;
    assert_eq!(conn.provider(), "linear");

    Ok(())
}
