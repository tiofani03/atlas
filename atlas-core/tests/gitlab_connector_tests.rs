use atlas_core::{
    ArtifactKind, ConnectorConfig, KnowledgeArtifact, Storage, GitlabConnector, Connector,
};
use tempfile::tempdir;

#[test]
fn test_gitlab_config_deserialization() {
    let toml_content = r#"
        [[connectors]]
        type = "gitlab"
        token = "glpat-test998877"
        instance_url = "https://gitlab.internal.co"
        enabled = true
        projects = ["infrastructure/atlas", "backend/core"]
    "#;

    #[derive(serde::Deserialize)]
    struct Wrapper {
        connectors: Vec<ConnectorConfig>,
    }

    let parsed: Wrapper = toml::from_str(toml_content).unwrap();
    assert_eq!(parsed.connectors.len(), 1);
    let conn = &parsed.connectors[0];
    assert_eq!(conn.provider, "gitlab");
    assert_eq!(conn.api_token.as_deref(), Some("glpat-test998877"));
    assert_eq!(conn.instance_url, "https://gitlab.internal.co");
    assert_eq!(conn.enabled, Some(true));
    assert_eq!(conn.projects, vec!["infrastructure/atlas", "backend/core"]);
}

#[test]
fn test_gitlab_mr_storage_and_alias_resolution() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("test_gitlab.db");
    let storage = Storage::new(&db_path)?;

    let now = chrono::Utc::now();

    let gitlab_mr = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("gitlab", "https://gitlab.com", "group/repo/mr/42"),
        kind: ArtifactKind::PullRequest,
        title: "Merge Request !42: Implement mTLS authentication".to_string(),
        summary: Some("State: merged".to_string()),
        body: "Detailed MR notes for mTLS implementation.".to_string(),
        provider: "gitlab".to_string(),
        source_id: "group/repo/mr/42".to_string(),
        source_url: "https://gitlab.com/group/repo/-/merge_requests/42".to_string(),
        repository: Some("group/repo".to_string()),
        tags: vec!["gitlab:mr".to_string()],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "checksum_gitlab_mr_42".to_string(),
        metadata: serde_json::json!({ "iid": 42 }),
    };

    storage.upsert_artifacts_batch(&[gitlab_mr], None)?;

    let matches = storage.resolve_artifact_by_alias("group/repo/mr/42")?;
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].provider, "gitlab");

    let mut cfg = ConnectorConfig::default();
    cfg.provider = "gitlab".to_string();
    cfg.api_token = Some("glpat-12345".to_string());
    cfg.projects = vec!["group/repo".to_string()];
    let conn = GitlabConnector::new("gitlab-inst".to_string(), cfg)?;
    assert_eq!(conn.provider(), "gitlab");

    Ok(())
}
