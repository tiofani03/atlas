use atlas_core::{
    ArtifactKind, ConnectorConfig, Storage, SpreadsheetConnector, Connector,
};
use tempfile::tempdir;
use std::io::Write;

#[test]
fn test_spreadsheet_config_deserialization() {
    let toml_content = r#"
        [[connectors]]
        type = "spreadsheet"
        path = "data/requirements.csv"
        token = "ya29.a0_google_token"
        enabled = true
        has_header_row = true
        max_rows_per_sheet = 5000
    "#;

    #[derive(serde::Deserialize)]
    struct Wrapper {
        connectors: Vec<ConnectorConfig>,
    }

    let parsed: Wrapper = toml::from_str(toml_content).unwrap();
    assert_eq!(parsed.connectors.len(), 1);
    let conn = &parsed.connectors[0];
    assert_eq!(conn.provider, "spreadsheet");
    assert_eq!(conn.path.as_deref(), Some("data/requirements.csv"));
    assert_eq!(conn.api_token.as_deref(), Some("ya29.a0_google_token"));
    assert_eq!(conn.has_header_row, Some(true));
    assert_eq!(conn.max_rows_per_sheet, Some(5000));
}

#[tokio::test]
async fn test_spreadsheet_parsing_and_storage_pipeline() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let csv_path = dir.path().join("matrix.csv");
    let csv_content = "Requirement,Owner,Status\nREQ-01,Alice,Approved\nREQ-02,Bob,Pending";

    let mut f = std::fs::File::create(&csv_path)?;
    f.write_all(csv_content.as_bytes())?;

    let mut cfg = ConnectorConfig::default();
    cfg.provider = "spreadsheet".to_string();
    cfg.path = Some(csv_path.to_str().unwrap().to_string());
    cfg.has_header_row = Some(true);

    let conn = SpreadsheetConnector::new("sheet-inst".to_string(), cfg)?;
    let artifacts = conn.fetch_modified(None).await?;

    assert_eq!(artifacts.len(), 3); // 1 Workbook + 2 Rows
    assert!(artifacts.iter().any(|a| a.kind == ArtifactKind::Document));
    assert!(artifacts.iter().any(|a| a.body.contains("Requirement: REQ-01")));

    let db_path = dir.path().join("test_sheet.db");
    let storage = Storage::new(&db_path)?;
    storage.upsert_artifacts_batch(&artifacts)?;

    let search_results = storage.search_fts("Alice", None, None, None, 10)?;
    assert!(!search_results.is_empty(), "FTS5 search should find spreadsheet row artifacts");

    Ok(())
}
