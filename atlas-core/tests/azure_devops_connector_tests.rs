use atlas_core::{
    ArtifactKind, ConnectorConfig, KnowledgeArtifact, Storage, AzureDevopsConnector, Connector,
};
use tempfile::tempdir;

#[test]
fn test_azure_devops_config_deserialization() {
    let toml_content = r#"
        [[connectors]]
        type = "azure_devops"
        token = "ado_pat_test123"
        organization = "acme-devops"
        enabled = true
        projects = ["CorePlatform", "MobileApp"]
    "#;

    #[derive(serde::Deserialize)]
    struct Wrapper {
        connectors: Vec<ConnectorConfig>,
    }

    let parsed: Wrapper = toml::from_str(toml_content).unwrap();
    assert_eq!(parsed.connectors.len(), 1);
    let conn = &parsed.connectors[0];
    assert_eq!(conn.provider, "azure_devops");
    assert_eq!(conn.api_token.as_deref(), Some("ado_pat_test123"));
    assert_eq!(conn.organization.as_deref(), Some("acme-devops"));
    assert_eq!(conn.enabled, Some(true));
    assert_eq!(conn.projects, vec!["CorePlatform", "MobileApp"]);
}

#[test]
fn test_azure_devops_work_item_storage_and_alias_resolution() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("test_ado.db");
    let storage = Storage::new(&db_path)?;

    let now = chrono::Utc::now();

    let work_item = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("azure_devops", "https://dev.azure.com/acme", "CorePlatform/workitem/9021"),
        kind: ArtifactKind::Ticket,
        title: "Work Item #9021: Upgrade TLS 1.3 cipher suite".to_string(),
        summary: Some("State: Active | Type: User Story".to_string()),
        body: "Detailed criteria for TLS 1.3 cipher suite migration.".to_string(),
        provider: "azure_devops".to_string(),
        source_id: "CorePlatform/workitem/9021".to_string(),
        source_url: "https://dev.azure.com/acme/CorePlatform/_workitems/edit/9021".to_string(),
        repository: Some("CorePlatform".to_string()),
        tags: vec!["ado:workitem".to_string()],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "checksum_ado_9021".to_string(),
        metadata: serde_json::json!({ "id": 9021 }),
    };

    storage.upsert_artifacts_batch(&[work_item])?;

    let matches = storage.resolve_artifact_by_alias("CorePlatform/workitem/9021")?;
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].provider, "azure_devops");

    let mut cfg = ConnectorConfig::default();
    cfg.provider = "azure_devops".to_string();
    cfg.api_token = Some("ado_pat_test".to_string());
    let conn = AzureDevopsConnector::new("ado-inst".to_string(), cfg)?;
    assert_eq!(conn.provider(), "azure_devops");

    Ok(())
}
