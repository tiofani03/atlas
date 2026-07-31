use atlas_core::{
    ArtifactKind, ArtifactRelationship, ContextBuilder, ContextOptions, KnowledgeArtifact, Storage,
};
use chrono::Utc;
use tempfile::NamedTempFile;

#[test]
fn test_context_builder_issue_flow() -> anyhow::Result<()> {
    let tmp_file = NamedTempFile::new()?;
    let storage = Storage::new(tmp_file.path())?;
    let now = Utc::now();

    let issue = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("jira", "https://jira.example.com", "PAY-123"),
        kind: ArtifactKind::Issue,
        title: "Implement Payment Gateway Timeout Handling".to_string(),
        summary: Some("Handle timeouts gracefully in payment service".to_string()),
        body: "Full details of payment gateway timeout handling...".to_string(),
        provider: "jira".to_string(),
        source_id: "PAY-123".to_string(),
        source_url: "https://jira.example.com/browse/PAY-123".to_string(),
        repository: Some("payment-service".to_string()),
        tags: vec!["payment".to_string(), "backend".to_string()],
        relationships: vec![
            ArtifactRelationship {
                source_id: "PAY-123".to_string(),
                target_id: "payment-service#456".to_string(),
                relationship_type: "implemented_by".to_string(),
            },
            ArtifactRelationship {
                source_id: "PAY-123".to_string(),
                target_id: "ADR-001".to_string(),
                relationship_type: "documented_by".to_string(),
            },
        ],
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "cs_issue".to_string(),
        metadata: serde_json::json!({ "status": "In Progress" }),
    };

    let pr = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("github", "https://api.github.com", "payment-service#456"),
        kind: ArtifactKind::PullRequest,
        title: "Add timeout handler to payment client".to_string(),
        summary: Some("Adds timeout handling".to_string()),
        body: "PR body content".to_string(),
        provider: "github".to_string(),
        source_id: "payment-service#456".to_string(),
        source_url: "https://github.com/org/payment-service/pull/456".to_string(),
        repository: Some("payment-service".to_string()),
        tags: vec!["payment".to_string()],
        relationships: vec![ArtifactRelationship {
            source_id: "payment-service#456".to_string(),
            target_id: "c1a2b3c4d5e6".to_string(),
            relationship_type: "commit".to_string(),
        }],
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "cs_pr".to_string(),
        metadata: serde_json::json!({ "state": "open" }),
    };

    let commit = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("github", "https://api.github.com", "c1a2b3c4d5e6"),
        kind: ArtifactKind::Commit,
        title: "fix(payment): timeout retries logic".to_string(),
        summary: None,
        body: "Commit details".to_string(),
        provider: "github".to_string(),
        source_id: "c1a2b3c4d5e6".to_string(),
        source_url: "https://github.com/org/payment-service/commit/c1a2b3c4d5e6".to_string(),
        repository: Some("payment-service".to_string()),
        tags: vec![],
        relationships: vec![],
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "cs_commit".to_string(),
        metadata: serde_json::Value::Null,
    };

    let adr = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("confluence", "https://confluence.example.com", "ADR-001"),
        kind: ArtifactKind::Document,
        title: "ADR-001: Payment Resilience and Circuit Breaking".to_string(),
        summary: Some("Architecture decision record for payment resilience".to_string()),
        body: "ADR body content".to_string(),
        provider: "confluence".to_string(),
        source_id: "ADR-001".to_string(),
        source_url: "https://confluence.example.com/pages/ADR-001".to_string(),
        repository: None,
        tags: vec!["adr".to_string(), "architecture".to_string()],
        relationships: vec![],
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "cs_adr".to_string(),
        metadata: serde_json::Value::Null,
    };

    let api_comp = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("confluence", "https://confluence.example.com", "PaymentAPI"),
        kind: ArtifactKind::Component,
        title: "Payment Gateway REST API Specification".to_string(),
        summary: Some("API definition".to_string()),
        body: "OpenAPI spec".to_string(),
        provider: "confluence".to_string(),
        source_id: "PaymentAPI".to_string(),
        source_url: "https://confluence.example.com/pages/PaymentAPI".to_string(),
        repository: Some("payment-service".to_string()),
        tags: vec!["openapi".to_string(), "api".to_string()],
        relationships: vec![],
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "cs_api".to_string(),
        metadata: serde_json::Value::Null,
    };

    storage.upsert_artifact(&issue)?;
    storage.upsert_artifact(&pr)?;
    storage.upsert_artifact(&commit)?;
    storage.upsert_artifact(&adr)?;
    storage.upsert_artifact(&api_comp)?;

    let builder = ContextBuilder::new(&storage);
    let options = ContextOptions::default();

    // 1. Build Issue Context
    let pkg = builder.build(Some("issue"), "PAY-123", &options)?;
    assert_eq!(pkg.target_kind, "issue");
    assert_eq!(pkg.target_id, "PAY-123");
    assert!(pkg.primary_artifact.is_some());
    assert_eq!(pkg.primary_artifact.as_ref().unwrap().source_id, "PAY-123");
    assert_eq!(pkg.architecture_decisions.len(), 1);
    assert_eq!(pkg.architecture_decisions[0].artifact.source_id, "ADR-001");
    assert_eq!(pkg.related_pull_requests[0].artifact.source_id, "payment-service#456");
    assert_eq!(pkg.related_commits[0].artifact.source_id, "c1a2b3c4d5e6");
    assert_eq!(pkg.apis.len(), 1);
    assert_eq!(pkg.apis[0].artifact.source_id, "PaymentAPI");
    assert!(pkg.affected_repositories.contains(&"payment-service".to_string()));
    assert!(!pkg.dependency_graph.is_empty());
    assert!(!pkg.implementation_hints.is_empty());

    // 2. Build PR Context
    let pr_pkg = builder.build(Some("pr"), "456", &options)?;
    assert_eq!(pr_pkg.target_id, "456");
    assert!(pr_pkg.primary_artifact.is_some());

    // 3. Build Repository Context
    let repo_pkg = builder.build(Some("repository"), "payment-service", &options)?;
    assert_eq!(repo_pkg.target_kind, "repository");
    assert_eq!(repo_pkg.target_id, "payment-service");
    assert!(repo_pkg.affected_repositories.contains(&"payment-service".to_string()));

    // 4. Build ADR Context
    let adr_pkg = builder.build(Some("adr"), "ADR-001", &options)?;
    assert_eq!(adr_pkg.target_kind, "adr");
    assert_eq!(adr_pkg.target_id, "ADR-001");
    assert!(adr_pkg.primary_artifact.is_some());

    Ok(())
}
