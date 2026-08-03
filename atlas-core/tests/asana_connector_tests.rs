use atlas_core::{
    ArtifactKind, ConnectorConfig, KnowledgeArtifact, Storage, AsanaConnector, Connector,
};
use tempfile::tempdir;

#[test]
fn test_asana_config_deserialization() {
    let toml_content = r#"
        [[connectors]]
        type = "asana"
        token = "1/120938:asana_pat_test"
        workspace = "9182390123"
        enabled = true
        projects = ["12093810293", "12093810294"]
    "#;

    #[derive(serde::Deserialize)]
    struct Wrapper {
        connectors: Vec<ConnectorConfig>,
    }

    let parsed: Wrapper = toml::from_str(toml_content).unwrap();
    assert_eq!(parsed.connectors.len(), 1);
    let conn = &parsed.connectors[0];
    assert_eq!(conn.provider, "asana");
    assert_eq!(conn.api_token.as_deref(), Some("1/120938:asana_pat_test"));
    assert_eq!(conn.workspace.as_deref(), Some("9182390123"));
    assert_eq!(conn.enabled, Some(true));
    assert_eq!(conn.projects, vec!["12093810293", "12093810294"]);
}

#[test]
fn test_asana_task_storage_and_alias_resolution() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("test_asana.db");
    let storage = Storage::new(&db_path)?;

    let now = chrono::Utc::now();

    let asana_task = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("asana", "https://app.asana.com/api/1.0", "task:12093810293"),
        kind: ArtifactKind::Ticket,
        title: "Migrate auth service to OAuth2 PKCE".to_string(),
        summary: Some("Workspace: 9182390123 | Completed: false".to_string()),
        body: "Task details for OAuth2 PKCE migration.".to_string(),
        provider: "asana".to_string(),
        source_id: "task:12093810293".to_string(),
        source_url: "https://app.asana.com/0/9182390123/12093810293".to_string(),
        repository: None,
        tags: vec!["asana:task".to_string()],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "checksum_asana_1209".to_string(),
        metadata: serde_json::json!({ "gid": "12093810293" }),
    };

    storage.upsert_artifacts_batch(&[asana_task], None)?;

    let matches = storage.resolve_artifact_by_alias("task:12093810293")?;
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].provider, "asana");

    let mut cfg = ConnectorConfig::default();
    cfg.provider = "asana".to_string();
    cfg.api_token = Some("1/123:pat".to_string());
    let conn = AsanaConnector::new("asana-inst".to_string(), cfg)?;
    assert_eq!(conn.provider(), "asana");

    Ok(())
}
