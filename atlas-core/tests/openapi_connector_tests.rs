use atlas_core::{
    ArtifactKind, ConnectorConfig, KnowledgeArtifact, Storage, OpenapiConnector, Connector,
};
use tempfile::tempdir;
use std::io::Write;

#[test]
fn test_openapi_config_deserialization() {
    let toml_content = r#"
        [[connectors]]
        type = "openapi"
        path = "specs/petstore.json"
        enabled = true
        paths = ["specs/petstore.json", "specs/payment.json"]
    "#;

    #[derive(serde::Deserialize)]
    struct Wrapper {
        connectors: Vec<ConnectorConfig>,
    }

    let parsed: Wrapper = toml::from_str(toml_content).unwrap();
    assert_eq!(parsed.connectors.len(), 1);
    let conn = &parsed.connectors[0];
    assert_eq!(conn.provider, "openapi");
    assert_eq!(conn.path.as_deref(), Some("specs/petstore.json"));
    assert_eq!(conn.enabled, Some(true));
    assert_eq!(conn.paths, vec!["specs/petstore.json", "specs/payment.json"]);
}

#[tokio::test]
async fn test_openapi_parsing_and_storage_pipeline() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let spec_path = dir.path().join("openapi.json");
    let spec_json = serde_json::json!({
        "openapi": "3.0.0",
        "info": { "title": "Payment API", "version": "2.1.0" },
        "paths": {
            "/v2/charge": {
                "post": {
                    "summary": "Process credit card charge",
                    "description": "Submits transaction payload to gateway"
                }
            }
        },
        "components": {
            "schemas": {
                "ChargeRequest": {
                    "type": "object",
                    "properties": {
                        "amount": { "type": "integer" }
                    }
                }
            }
        }
    });

    let mut f = std::fs::File::create(&spec_path)?;
    f.write_all(spec_json.to_string().as_bytes())?;

    let mut cfg = ConnectorConfig::default();
    cfg.provider = "openapi".to_string();
    cfg.path = Some(spec_path.to_str().unwrap().to_string());

    let conn = OpenapiConnector::new("openapi-inst".to_string(), cfg)?;
    let artifacts = conn.fetch_modified(None).await?;

    assert!(!artifacts.is_empty());
    assert!(artifacts.iter().any(|a| a.title.contains("POST /v2/charge")));
    assert!(artifacts.iter().any(|a| a.title.contains("Schema: ChargeRequest")));

    let db_path = dir.path().join("test_openapi.db");
    let storage = Storage::new(&db_path)?;
    storage.upsert_artifacts_batch(&artifacts, None)?;

    let search_results = storage.search_fts("charge", None, None, None, 10)?;
    assert!(!search_results.is_empty(), "FTS5 search should find OpenAPI endpoint specs");

    Ok(())
}
