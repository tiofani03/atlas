use atlas_core::{
    ArtifactKind, ConnectorConfig, GithubConnector, Connector,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_github_connector_sync_all_artifact_types() -> anyhow::Result<()> {
    let mock_server = MockServer::start().await;

    // 1. Mock Repo Info
    let repo_json = serde_json::json!({
        "name": "atlas",
        "description": "Universal engineering context engine",
        "html_url": format!("{}/owner/atlas", mock_server.uri()),
        "visibility": "public",
        "default_branch": "main",
        "topics": ["context", "rust"],
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-07-31T00:00:00Z"
    });
    Mock::given(method("GET"))
        .and(path("/repos/owner/atlas"))
        .respond_with(ResponseTemplate::new(200).set_body_json(repo_json))
        .mount(&mock_server)
        .await;

    // 2. Mock Issues (1 issue, 0 PRs)
    let issues_json = serde_json::json!([
        {
            "number": 1,
            "title": "Issue: Architecture Redesign",
            "body": "See #2 for details and PAY-100",
            "html_url": format!("{}/owner/atlas/issues/1", mock_server.uri()),
            "state": "open",
            "labels": [{"name": "architecture"}],
            "created_at": "2026-07-30T10:00:00Z",
            "updated_at": "2026-07-31T10:00:00Z"
        }
    ]);
    Mock::given(method("GET"))
        .and(path("/repos/owner/atlas/issues"))
        .respond_with(ResponseTemplate::new(200).set_body_json(issues_json))
        .mount(&mock_server)
        .await;

    // 3. Mock PRs (1 PR)
    let prs_json = serde_json::json!([
        {
            "number": 2,
            "title": "PR: KnowledgeArtifact Migration",
            "body": "Closes #1",
            "html_url": format!("{}/owner/atlas/pull/2", mock_server.uri()),
            "state": "open",
            "draft": false,
            "merged_at": null,
            "created_at": "2026-07-30T11:00:00Z",
            "updated_at": "2026-07-31T11:00:00Z"
        }
    ]);
    Mock::given(method("GET"))
        .and(path("/repos/owner/atlas/pulls"))
        .respond_with(ResponseTemplate::new(200).set_body_json(prs_json))
        .mount(&mock_server)
        .await;

    // 4. Mock PR Reviews
    let reviews_json = serde_json::json!([
        {
            "id": 101,
            "user": { "login": "reviewer1" },
            "state": "APPROVED",
            "body": "LGTM! Artifact model is clean.",
            "html_url": format!("{}/owner/atlas/pull/2#review-101", mock_server.uri()),
            "submitted_at": "2026-07-31T11:30:00Z"
        }
    ]);
    Mock::given(method("GET"))
        .and(path("/repos/owner/atlas/pulls/2/reviews"))
        .respond_with(ResponseTemplate::new(200).set_body_json(reviews_json))
        .mount(&mock_server)
        .await;

    // 5. Mock Review Comments
    let comments_json = serde_json::json!([
        {
            "id": 201,
            "body": "Ensure relationship table indexed",
            "path": "src/domain.rs",
            "line": 42,
            "user": { "login": "reviewer1" },
            "html_url": format!("{}/owner/atlas/pull/2#discussion-201", mock_server.uri()),
            "pull_request_url": format!("{}/repos/owner/atlas/pulls/2", mock_server.uri()),
            "created_at": "2026-07-31T11:40:00Z",
            "updated_at": "2026-07-31T11:40:00Z"
        }
    ]);
    Mock::given(method("GET"))
        .and(path("/repos/owner/atlas/pulls/comments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(comments_json))
        .mount(&mock_server)
        .await;

    // 6. Mock Commits (Metadata ONLY)
    let commits_json = serde_json::json!([
        {
            "sha": "a1b2c3d4e5f67890",
            "html_url": format!("{}/owner/atlas/commit/a1b2c3d4e5f67890", mock_server.uri()),
            "commit": {
                "message": "feat: introduce KnowledgeArtifact domain model\n\nFull details here",
                "author": { "name": "Dev", "date": "2026-07-31T12:00:00Z" },
                "committer": { "name": "Dev", "date": "2026-07-31T12:00:00Z" }
            },
            "parents": []
        }
    ]);
    Mock::given(method("GET"))
        .and(path("/repos/owner/atlas/commits"))
        .respond_with(ResponseTemplate::new(200).set_body_json(commits_json))
        .mount(&mock_server)
        .await;

    // 7. Mock Releases
    let releases_json = serde_json::json!([
        {
            "tag_name": "v0.2.0",
            "name": "Atlas v0.2.0 - Unified Context Engine",
            "body": "Release notes for v0.2.0",
            "html_url": format!("{}/owner/atlas/releases/tag/v0.2.0", mock_server.uri()),
            "target_commitish": "a1b2c3d4e5f67890",
            "published_at": "2026-07-31T13:00:00Z"
        }
    ]);
    Mock::given(method("GET"))
        .and(path("/repos/owner/atlas/releases"))
        .respond_with(ResponseTemplate::new(200).set_body_json(releases_json))
        .mount(&mock_server)
        .await;

    let config = ConnectorConfig {
        provider: "github".to_string(),
        instance_url: mock_server.uri(),
        email: "".to_string(),
        api_token: None,
        api_token_env: None,
        projects: vec![],
        spaces: vec![],
        repos: vec!["owner/atlas".to_string()],
        path: None,
        paths: vec![],
        glob_patterns: vec![],
        ..Default::default()
    };

    let connector = GithubConnector::new("github-test".to_string(), config)?;
    let artifacts = connector.fetch_modified(None).await?;

    assert!(!artifacts.is_empty());

    let kinds: Vec<ArtifactKind> = artifacts.iter().map(|a| a.kind.clone()).collect();
    assert!(kinds.contains(&ArtifactKind::Repository));
    assert!(kinds.contains(&ArtifactKind::Issue));
    assert!(kinds.contains(&ArtifactKind::PullRequest));
    assert!(kinds.contains(&ArtifactKind::PullRequestReview));
    assert!(kinds.contains(&ArtifactKind::ReviewComment));
    assert!(kinds.contains(&ArtifactKind::Commit));
    assert!(kinds.contains(&ArtifactKind::Release));

    // Verify commit metadata only (no source code indexed)
    let commit_art = artifacts.iter().find(|a| a.kind == ArtifactKind::Commit).unwrap();
    assert_eq!(commit_art.source_id, "owner/atlas@a1b2c3d4e5f67890");
    assert!(!commit_art.body.contains("diff --git"));

    Ok(())
}
