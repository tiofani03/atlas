use atlas_core::{
    ArtifactKind, ConnectorConfig, KnowledgeArtifact, Storage,
};
use tempfile::tempdir;

#[test]
fn test_clickup_config_deserialization() {
    let toml_content = r#"
        [[connectors]]
        type = "clickup"
        token = "pk_test_12345"
        workspace = "123456"
        enabled = true
        spaces = ["Engineering"]
        lists = ["Sprint 1"]
    "#;

    #[derive(serde::Deserialize)]
    struct Wrapper {
        connectors: Vec<ConnectorConfig>,
    }

    let parsed: Wrapper = toml::from_str(toml_content).unwrap();
    assert_eq!(parsed.connectors.len(), 1);
    let conn = &parsed.connectors[0];
    assert_eq!(conn.provider, "clickup");
    assert_eq!(conn.api_token.as_deref(), Some("pk_test_12345"));
    assert_eq!(conn.workspace.as_deref(), Some("123456"));
    assert_eq!(conn.enabled, Some(true));
    assert_eq!(conn.spaces, vec!["Engineering"]);
    assert_eq!(conn.lists, vec!["Sprint 1"]);
}

#[test]
fn test_clickup_relationship_extraction_and_alias_resolution() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("test.db");
    let storage = Storage::new(&db_path)?;

    let now = chrono::Utc::now();

    let clickup_task = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("clickup", "https://api.clickup.com/api/v2", "8685abc123"),
        kind: ArtifactKind::Ticket,
        title: "Implement authentication service".to_string(),
        summary: Some("Status: in progress | Priority: high".to_string()),
        body: "Detailed description of authentication task.".to_string(),
        provider: "clickup".to_string(),
        source_id: "CU-123".to_string(),
        source_url: "https://app.clickup.com/t/8685abc123".to_string(),
        repository: None,
        tags: vec!["status:in progress".to_string(), "space:Dev".to_string()],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "abc123checksum".to_string(),
        metadata: serde_json::json!({
            "id": "8685abc123",
            "custom_id": "CU-123"
        }),
    };

    let commit_artifact = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("github", "https://github.com", "sha123456"),
        kind: ArtifactKind::Commit,
        title: "feat(auth): initial auth service implementation CU-123".to_string(),
        summary: None,
        body: "Resolves ClickUp #123 and fixes authentication issue".to_string(),
        provider: "github".to_string(),
        source_id: "owner/repo@sha123456".to_string(),
        source_url: "https://github.com/owner/repo/commit/sha123456".to_string(),
        repository: Some("owner/repo".to_string()),
        tags: vec![],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "sha123456checksum".to_string(),
        metadata: serde_json::json!({}),
    };

    storage.upsert_artifacts_batch(&[clickup_task, commit_artifact], None)?;

    let matches = storage.resolve_artifact_by_alias("CU-123")?;
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].source_id, "CU-123");
    assert_eq!(matches[0].provider, "clickup");

    let rels = Storage::extract_automatic_linking_relationships(&matches[0]);
    assert!(rels.is_empty() || rels.iter().all(|r| r.source_id == "CU-123"));

    let related = storage.get_related_artifacts("CU-123")?;
    assert!(!related.is_empty(), "Should automatically discover link between Git commit and ClickUp CU-123");

    Ok(())
}
