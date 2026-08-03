use atlas_core::{config::ConnectorConfig, connectors::Connector, JiraConnector};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn cfg(instance_url: &str, token: &str) -> ConnectorConfig {
    ConnectorConfig {
        provider: "jira".to_string(),
        instance_url: instance_url.to_string(),
        email: "dev@acme.com".to_string(),
        api_token: Some(token.to_string()),
        ..Default::default()
    }
}

#[tokio::test]
async fn verify_success_returns_connected_message() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/rest/api/3/myself"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({ "displayName": "Ada Lovelace" })),
        )
        .mount(&mock_server)
        .await;

    let conn = JiraConnector::new("jira-test".to_string(), cfg(&mock_server.uri(), "tok")).unwrap();
    let msg = conn.verify().await.unwrap();
    assert!(msg.contains("Ada Lovelace"), "unexpected message: {}", msg);
}

#[tokio::test]
async fn verify_invalid_credentials_reports_401() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/rest/api/3/myself"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
        .mount(&mock_server)
        .await;

    let conn = JiraConnector::new("jira-test".to_string(), cfg(&mock_server.uri(), "bad")).unwrap();
    let err = conn.verify().await.unwrap_err();
    assert!(
        err.to_string().contains("401"),
        "expected 401 in error, got: {}",
        err
    );
}

#[tokio::test]
async fn verify_falls_back_to_api_2() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/rest/api/3/myself"))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/rest/api/2/myself"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({ "displayName": "Grace Hopper" })),
        )
        .mount(&mock_server)
        .await;

    let conn = JiraConnector::new("jira-test".to_string(), cfg(&mock_server.uri(), "tok")).unwrap();
    let msg = conn.verify().await.unwrap();
    assert!(msg.contains("Grace Hopper"), "unexpected message: {}", msg);
}

#[tokio::test]
async fn verify_network_error_surfaces() {
    // Point at a dead port — reqwest errors out (no response).
    let conn = JiraConnector::new(
        "jira-test".to_string(),
        cfg("http://127.0.0.1:1", "tok"),
    )
    .unwrap();
    assert!(conn.verify().await.is_err());
}
