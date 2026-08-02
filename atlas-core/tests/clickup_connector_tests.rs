use atlas_core::{ArtifactKind, ClickUpConnector, Connector, ConnectorConfig};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_clickup_connector_maps_tasks_to_ticket_artifacts() -> anyhow::Result<()> {
    let mock_server = MockServer::start().await;

    let tasks_json = serde_json::json!({
        "tasks": [
            {
                "id": "abc123",
                "name": "Build ClickUp connector",
                "markdown_description": "Implement sync for INIT-488 and link #42",
                "url": "https://app.clickup.com/t/abc123",
                "date_created": "1782979200000",
                "date_updated": "1783065600000",
                "status": { "status": "in progress" },
                "priority": { "priority": "high" },
                "tags": [{ "name": "integration" }],
                "list": { "id": "list-1", "name": "Engineering" },
                "space": { "id": "space-1", "name": "Product" },
                "parent": "parent-1",
                "dependencies": [{ "task_id": "dep-1" }]
            }
        ]
    });

    Mock::given(method("GET"))
        .and(path("/team/123/task"))
        .and(query_param("include_markdown_description", "true"))
        .and(query_param("include_closed", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(tasks_json))
        .mount(&mock_server)
        .await;

    let config = ConnectorConfig {
        provider: "clickup".to_string(),
        instance_url: mock_server.uri(),
        email: String::new(),
        api_token: Some("pk_test".to_string()),
        api_token_env: None,
        projects: vec!["123".to_string()],
        spaces: vec![],
        repos: vec![],
        path: None,
        paths: vec![],
        glob_patterns: vec![],
    };

    let connector = ClickUpConnector::new("clickup-test".to_string(), config)?;
    let artifacts = connector.fetch_modified(None).await?;

    assert_eq!(artifacts.len(), 1);
    let artifact = &artifacts[0];
    assert_eq!(artifact.kind, ArtifactKind::Ticket);
    assert_eq!(artifact.provider, "clickup");
    assert_eq!(artifact.source_id, "abc123");
    assert_eq!(artifact.title, "Build ClickUp connector");
    assert_eq!(artifact.summary.as_deref(), Some("Status: in progress"));
    assert_eq!(artifact.repository.as_deref(), Some("Engineering"));
    assert!(artifact.tags.contains(&"workspace:123".to_string()));
    assert!(artifact.tags.contains(&"status:in progress".to_string()));
    assert!(artifact.tags.contains(&"integration".to_string()));
    assert!(artifact
        .relationships
        .iter()
        .any(|rel| rel.relationship_type == "subtask_of" && rel.target_id == "parent-1"));
    assert!(artifact
        .relationships
        .iter()
        .any(|rel| rel.relationship_type == "depends_on" && rel.target_id == "dep-1"));

    Ok(())
}
