use atlas_core::{
    ArtifactKind, ConnectorConfig, KnowledgeArtifact, Storage, BitbucketConnector, Connector,
};
use tempfile::tempdir;

#[test]
fn test_bitbucket_config_deserialization() {
    let toml_content = r#"
        [[connectors]]
        type = "bitbucket"
        token = "bb_app_pass_test"
        email = "dev@acme.com"
        workspace = "acme-devs"
        enabled = true
    "#;

    #[derive(serde::Deserialize)]
    struct Wrapper {
        connectors: Vec<ConnectorConfig>,
    }

    let parsed: Wrapper = toml::from_str(toml_content).unwrap();
    assert_eq!(parsed.connectors.len(), 1);
    let conn = &parsed.connectors[0];
    assert_eq!(conn.provider, "bitbucket");
    assert_eq!(conn.api_token.as_deref(), Some("bb_app_pass_test"));
    assert_eq!(conn.email, "dev@acme.com");
    assert_eq!(conn.workspace.as_deref(), Some("acme-devs"));
    assert_eq!(conn.enabled, Some(true));
}

#[test]
fn test_bitbucket_pr_storage_and_alias_resolution() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("test_bitbucket.db");
    let storage = Storage::new(&db_path)?;

    let now = chrono::Utc::now();

    let pr_artifact = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("bitbucket", "https://api.bitbucket.org/2.0", "acme-devs/repo/pr/105"),
        kind: ArtifactKind::PullRequest,
        title: "PR #105: Refactor Async Engine".to_string(),
        summary: Some("State: OPEN".to_string()),
        body: "Refactors background worker pool loop.".to_string(),
        provider: "bitbucket".to_string(),
        source_id: "acme-devs/repo/pr/105".to_string(),
        source_url: "https://bitbucket.org/acme-devs/repo/pull-requests/105".to_string(),
        repository: Some("acme-devs/repo".to_string()),
        tags: vec!["bitbucket:pr".to_string()],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "checksum_bb_105".to_string(),
        metadata: serde_json::json!({ "id": 105 }),
    };

    storage.upsert_artifacts_batch(&[pr_artifact])?;

    let matches = storage.resolve_artifact_by_alias("acme-devs/repo/pr/105")?;
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].provider, "bitbucket");

    let mut cfg = ConnectorConfig::default();
    cfg.provider = "bitbucket".to_string();
    cfg.api_token = Some("bb_pass".to_string());
    cfg.email = "dev@acme.com".to_string();
    let conn = BitbucketConnector::new("bitbucket-inst".to_string(), cfg)?;
    assert_eq!(conn.provider(), "bitbucket");

    Ok(())
}
