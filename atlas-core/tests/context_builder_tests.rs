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

#[test]
fn test_context_builder_telemetry_and_depth() -> anyhow::Result<()> {
    let tmp_file = NamedTempFile::new()?;
    let storage = Storage::new(tmp_file.path())?;
    let now = Utc::now();

    let issue = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("jira", "https://jira.example.com", "INIT-488"),
        kind: ArtifactKind::Issue,
        title: "Scale context engine for 100k artifacts".to_string(),
        summary: Some("Optimize retrieval latency".to_string()),
        body: "Full details of optimization plan".to_string(),
        provider: "jira".to_string(),
        source_id: "INIT-488".to_string(),
        source_url: "https://jira.example.com/browse/INIT-488".to_string(),
        repository: Some("atlas".to_string()),
        tags: vec!["performance".to_string()],
        relationships: vec![],
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "cs_init".to_string(),
        metadata: serde_json::Value::Null,
    };

    let pr = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("github", "https://api.github.com", "atlas#100"),
        kind: ArtifactKind::PullRequest,
        title: "perf: optimize context graph traversal".to_string(),
        summary: Some("ID-first traversal".to_string()),
        body: "PR details".to_string(),
        provider: "github".to_string(),
        source_id: "atlas#100".to_string(),
        source_url: "https://github.com/org/atlas/pull/100".to_string(),
        repository: Some("atlas".to_string()),
        tags: vec![],
        relationships: vec![ArtifactRelationship {
            source_id: "atlas#100".to_string(),
            target_id: "INIT-488".to_string(),
            relationship_type: "implements".to_string(),
        }],
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "cs_pr_init".to_string(),
        metadata: serde_json::Value::Null,
    };
    storage.upsert_artifact(&issue)?;
    storage.upsert_artifact(&pr)?;

    let builder = ContextBuilder::new(&storage);
    let mut options = ContextOptions::default();
    options.profile = true;
    options.depth = 1;

    let pkg = builder.build(Some("issue"), "INIT-488", &options)?;
    assert!(pkg.telemetry.is_some());
    let t = pkg.telemetry.unwrap();
    assert!(t.candidate_headers_count >= 1);
    assert_eq!(t.hydrated_artifacts_count, 1);
    assert_eq!(pkg.related_pull_requests.len(), 1);
    Ok(())
}

#[test]
fn test_context_builder_rejects_unrelated_promotion_placeholders() -> anyhow::Result<()> {
    let tmp_file = NamedTempFile::new()?;
    let storage = Storage::new(tmp_file.path())?;
    let now = Utc::now();

    let auth_epic = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("jira", "https://tayoai.atlassian.net", "PC-5"),
        kind: ArtifactKind::Issue,
        title: "EPIC-1: User Authentication & Workspace Access".to_string(),
        summary: Some("User authentication and multi-tenant workspace management".to_string()),
        body: "High-level overview of OAuth authentication flow and RBAC workspace access.".to_string(),
        provider: "jira".to_string(),
        source_id: "PC-5".to_string(),
        source_url: "https://tayoai.atlassian.net/browse/PC-5".to_string(),
        repository: Some("PC".to_string()),
        tags: vec!["auth".to_string(), "epic".to_string()],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "cs_pc5".to_string(),
        metadata: serde_json::json!({ "status": "To Do" }),
    };

    storage.upsert_artifact(&auth_epic)?;

    let builder = ContextBuilder::new(&storage);
    let options = ContextOptions::default();
    let pkg = builder.build(Some("issue"), "PC-5", &options)?;

    // Check possible implementation areas
    let possible_areas = pkg.implementation_areas.as_ref().expect("implementation_areas present");
    for rule in &possible_areas.business_rules {
        assert!(
            !rule.to_lowercase().contains("promotion") && !rule.to_lowercase().contains("voucher") && !rule.to_lowercase().contains("campaign"),
            "Business rule should not hallucinate unrelated promotions: {}", rule
        );
    }
    for comp in &possible_areas.potential_components {
        assert!(
            !comp.to_lowercase().contains("promotion") && !comp.to_lowercase().contains("campaign"),
            "Potential component should not hallucinate promotion engine: {}", comp
        );
    }

    let json_str = serde_json::to_string(&pkg)?;
    assert!(!json_str.to_lowercase().contains("promotion eligibility"));
    assert!(!json_str.to_lowercase().contains("voucher redemption"));
    assert!(!json_str.to_lowercase().contains("promotion engine"));

    Ok(())
}

#[test]
fn test_context_builder_does_not_promote_api_tickets_to_high_confidence_contracts() -> anyhow::Result<()> {
    let tmp_file = NamedTempFile::new()?;
    let storage = Storage::new(tmp_file.path())?;
    let now = Utc::now();

    let epic = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("jira", "https://tayoai.atlassian.net", "PC-5"),
        kind: ArtifactKind::Issue,
        title: "EPIC-1: User Authentication & Workspace Access".to_string(),
        summary: Some("Authentication and workspace access".to_string()),
        body: "OAuth, membership, and RBAC workspace access.".to_string(),
        provider: "jira".to_string(),
        source_id: "PC-5".to_string(),
        source_url: "https://tayoai.atlassian.net/browse/PC-5".to_string(),
        repository: Some("PC".to_string()),
        tags: vec!["epic".to_string(), "auth".to_string()],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "cs_pc5_api_ticket_guard".to_string(),
        metadata: serde_json::json!({ "status": "To Do" }),
    };

    let make_ticket = |source_id: &str, title: &str| KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("jira", "https://tayoai.atlassian.net", source_id),
        kind: ArtifactKind::Ticket,
        title: title.to_string(),
        summary: Some("Backend work item".to_string()),
        body: "Implementation task for the product workspace.".to_string(),
        provider: "jira".to_string(),
        source_id: source_id.to_string(),
        source_url: format!("https://tayoai.atlassian.net/browse/{}", source_id),
        repository: Some("PC".to_string()),
        tags: vec!["backend".to_string()],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: format!("cs_{}", source_id),
        metadata: serde_json::json!({ "status": "To Do" }),
    };

    storage.upsert_artifact(&epic)?;
    storage.upsert_artifact(&make_ticket(
        "PC-50",
        "[BE] Sprint Entity CRUD & Task Backlog Allocation API in Go",
    ))?;
    storage.upsert_artifact(&make_ticket(
        "PC-60",
        "[BE] Member Workload Task Breakdown API in Go",
    ))?;

    let package = ContextBuilder::new(&storage).build(
        Some("issue"),
        "PC-5",
        &ContextOptions::default(),
    )?;

    assert!(package.apis.is_empty(), "Jira work tickets must not become API contracts");
    for evidence in &package.evidence_ranking {
        if ["PC-50", "PC-60"].contains(&evidence.artifact_id.as_str()) {
            assert_ne!(evidence.kind, "API Contract");
            assert_ne!(evidence.confidence_level, "High Confidence");
        }
    }

    Ok(())
}
