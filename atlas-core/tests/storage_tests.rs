use atlas_core::{
    ArtifactKind, ArtifactRelationship, KnowledgeArtifact, Storage,
};
use chrono::Utc;
use tempfile::NamedTempFile;

#[test]
fn test_artifact_id_and_checksum_computation() {
    let id1 = KnowledgeArtifact::generate_id("github", "https://api.github.com", "octocat/hello#1");
    let id2 = KnowledgeArtifact::generate_id("github", "https://api.github.com", "octocat/hello#1");
    let id3 = KnowledgeArtifact::generate_id("github", "https://api.github.com", "octocat/hello#2");

    assert_eq!(id1, id2);
    assert_ne!(id1, id3);

    let checksum1 = KnowledgeArtifact::compute_checksum(
        "Title",
        Some("Summary"),
        "Body content",
        &["tag1".to_string()],
    );
    let checksum2 = KnowledgeArtifact::compute_checksum(
        "Title",
        Some("Summary"),
        "Body content",
        &["tag1".to_string()],
    );
    assert_eq!(checksum1, checksum2);
}

#[test]
fn test_storage_upsert_and_graph_queries() -> anyhow::Result<()> {
    let tmp_file = NamedTempFile::new()?;
    let storage = Storage::new(tmp_file.path())?;

    let now = Utc::now();
    let repo_artifact = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("github", "https://api.github.com", "atlas-owner/atlas"),
        kind: ArtifactKind::Repository,
        title: "atlas-owner/atlas".to_string(),
        summary: Some("Universal engineering context engine".to_string()),
        body: "Repository description and metadata".to_string(),
        provider: "github".to_string(),
        source_id: "atlas-owner/atlas".to_string(),
        source_url: "https://github.com/atlas-owner/atlas".to_string(),
        repository: Some("atlas-owner/atlas".to_string()),
        tags: vec!["repo:atlas-owner/atlas".to_string()],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "cs1".to_string(),
        metadata: serde_json::json!({ "name": "atlas" }),
    };

    let issue_artifact = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("github", "https://api.github.com", "atlas-owner/atlas#10"),
        kind: ArtifactKind::Issue,
        title: "Fix Context Graph Query".to_string(),
        summary: Some("Context graph query issue description".to_string()),
        body: "Fixes #10 and connects to commit".to_string(),
        provider: "github".to_string(),
        source_id: "atlas-owner/atlas#10".to_string(),
        source_url: "https://github.com/atlas-owner/atlas/issues/10".to_string(),
        repository: Some("atlas-owner/atlas".to_string()),
        tags: vec!["repo:atlas-owner/atlas".to_string(), "state:open".to_string()],
        relationships: vec![
            ArtifactRelationship {
                source_id: "atlas-owner/atlas".to_string(),
                target_id: "atlas-owner/atlas#10".to_string(),
                relationship_type: "owns".to_string(),
            },
        ],
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "cs2".to_string(),
        metadata: serde_json::json!({ "number": 10 }),
    };

    storage.upsert_artifact(&repo_artifact)?;
    storage.upsert_artifact(&issue_artifact)?;

    let retrieved = storage.get_artifact_by_id("atlas-owner/atlas#10")?;
    assert!(retrieved.is_some());
    let retrieved_art = retrieved.unwrap();
    assert_eq!(retrieved_art.title, "Fix Context Graph Query");
    assert_eq!(retrieved_art.kind, ArtifactKind::Issue);

    let related = storage.get_related_artifacts("atlas-owner/atlas#10")?;
    assert!(!related.is_empty());
    assert_eq!(related[0].1.source_id, "atlas-owner/atlas");

    let repo_artifacts = storage.query_by_repository("atlas-owner/atlas", 10)?;
    assert_eq!(repo_artifacts.len(), 2);

    let fts_results = storage.search_fts("Graph", None, None, None, 10)?;
    assert!(!fts_results.is_empty());
    assert_eq!(fts_results[0].source_id, "atlas-owner/atlas#10");

    let fts_hyphen_results = storage.search_fts("realtime-call", None, None, None, 10);
    assert!(fts_hyphen_results.is_ok());

    Ok(())
}

#[test]
fn test_commit_automatic_knowledge_linking() -> anyhow::Result<()> {
    let tmp_file = NamedTempFile::new()?;
    let storage = Storage::new(tmp_file.path())?;
    let now = Utc::now();

    let commit_artifact = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("git", "atlas-owner/atlas", "a1b2c3d4e5f6"),
        kind: ArtifactKind::Commit,
        title: "feat(auth): implement jwt session security [INIT-488]".to_string(),
        summary: Some("Adds webhook handler and session validation (#142)".to_string()),
        body: "Refers to PR #142 and resolves INIT-488 ticket requirement.".to_string(),
        provider: "github".to_string(),
        source_id: "a1b2c3d4e5f6".to_string(),
        source_url: "https://github.com/atlas-owner/atlas/commit/a1b2c3d4e5f6".to_string(),
        repository: Some("atlas-owner/atlas".to_string()),
        tags: vec!["repo:atlas-owner/atlas".to_string()],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "commit_cs".to_string(),
        metadata: serde_json::json!({
            "author_name": "Erik",
            "author_email": "erik@example.com",
            "is_merge": false,
            "files": [
                { "filename": "src/auth/jwt.rs", "status": "ADDED", "additions": 120, "deletions": 0 },
                { "filename": "src/main.rs", "status": "MODIFIED", "additions": 5, "deletions": 2 }
            ]
        }),
    };

    storage.upsert_artifact(&commit_artifact)?;

    // Verify commit was linked to ticket INIT-488
    let ticket_commits = storage.get_commits_for_ticket("INIT-488")?;
    assert_eq!(ticket_commits.len(), 1);
    assert_eq!(ticket_commits[0].source_id, "a1b2c3d4e5f6");

    // Verify file hotspots query
    let hotspots = storage.get_file_hotspots("atlas-owner/atlas", 10)?;
    assert_eq!(hotspots.len(), 2);
    assert_eq!(hotspots[0].0, "src/auth/jwt.rs");
    assert_eq!(hotspots[0].1, 1);
    assert_eq!(hotspots[0].2, 120);

    Ok(())
}

#[test]
fn test_release_ancestry_and_pr_commit_queries() -> anyhow::Result<()> {
    let tmp_file = NamedTempFile::new()?;
    let storage = Storage::new(tmp_file.path())?;
    let now = Utc::now();

    let release_artifact = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("github", "https://api.github.com", "atlas-owner/atlas#v4.52.0"),
        kind: ArtifactKind::Release,
        title: "Release v4.52.0 - Security Update".to_string(),
        summary: Some("Contains security enhancements and jwt authentication".to_string()),
        body: "Release payload for v4.52.0".to_string(),
        provider: "github".to_string(),
        source_id: "atlas-owner/atlas#v4.52.0".to_string(),
        source_url: "https://github.com/atlas-owner/atlas/releases/tag/v4.52.0".to_string(),
        repository: Some("atlas-owner/atlas".to_string()),
        tags: vec!["tag:v4.52.0".to_string()],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "release_cs".to_string(),
        metadata: serde_json::json!({
            "target_commitish": "a1b2c3d4e5f6"
        }),
    };

    let commit_artifact = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("git", "atlas-owner/atlas", "a1b2c3d4e5f6"),
        kind: ArtifactKind::Commit,
        title: "Merge pull request #142 from feature/auth".to_string(),
        summary: Some("Adds jwt security (#142)".to_string()),
        body: "Merged pull request #142".to_string(),
        provider: "github".to_string(),
        source_id: "a1b2c3d4e5f6".to_string(),
        source_url: "https://github.com/atlas-owner/atlas/commit/a1b2c3d4e5f6".to_string(),
        repository: Some("atlas-owner/atlas".to_string()),
        tags: vec!["repo:atlas-owner/atlas".to_string()],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "commit_cs2".to_string(),
        metadata: serde_json::json!({ "is_merge": true }),
    };

    storage.upsert_artifact(&release_artifact)?;
    storage.upsert_artifact(&commit_artifact)?;

    // Verify commit is linked to PR #142
    let pr_commits = storage.get_commits_for_pr("atlas-owner/atlas#142")?;
    assert_eq!(pr_commits.len(), 1);
    assert_eq!(pr_commits[0].source_id, "a1b2c3d4e5f6");

    // Verify release reachability query
    let release_commits = storage.get_commits_for_release("atlas-owner/atlas#v4.52.0")?;
    assert_eq!(release_commits.len(), 1);
    assert_eq!(release_commits[0].source_id, "a1b2c3d4e5f6");

    let releases = storage.get_releases_for_commit("a1b2c3d4e5f6")?;
    assert_eq!(releases.len(), 1);
    assert_eq!(releases[0].source_id, "atlas-owner/atlas#v4.52.0");

    Ok(())
}

#[test]
fn test_artifact_alias_resolution_and_bidirectional_graph() -> anyhow::Result<()> {
    let tmp_file = NamedTempFile::new()?;
    let storage = Storage::new(tmp_file.path())?;
    let now = Utc::now();

    let pr_artifact = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("github", "https://api.github.com", "Alfagift/alfagift-android#23"),
        kind: ArtifactKind::PullRequest,
        title: "In-House In-App-Chat & In-App-Call".to_string(),
        summary: Some("INIT-488 - In-House In-App-Chat".to_string()),
        body: "Implements INIT-488 feature".to_string(),
        provider: "github".to_string(),
        source_id: "Alfagift/alfagift-android#23".to_string(),
        source_url: "https://github.com/Alfagift/alfagift-android/pull/23".to_string(),
        repository: Some("Alfagift/alfagift-android".to_string()),
        tags: vec!["repo:Alfagift/alfagift-android".to_string()],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "pr23_cs".to_string(),
        metadata: serde_json::Value::Null,
    };

    let commit_artifact = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("github", "https://api.github.com", "Alfagift/alfagift-android@01742e72a1a0fe33358840c7cad38560055bc271"),
        kind: ArtifactKind::Commit,
        title: "[DONE][INIT-488] In-House In-App-Chat & In-App-Call (#23)".to_string(),
        summary: Some("Commit for INIT-488".to_string()),
        body: "Merge pull request #23 from feature/INIT-488".to_string(),
        provider: "github".to_string(),
        source_id: "Alfagift/alfagift-android@01742e72a1a0fe33358840c7cad38560055bc271".to_string(),
        source_url: "https://github.com/Alfagift/alfagift-android/commit/01742e72a1a0fe33358840c7cad38560055bc271".to_string(),
        repository: Some("Alfagift/alfagift-android".to_string()),
        tags: vec!["repo:Alfagift/alfagift-android".to_string()],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "c0174_cs".to_string(),
        metadata: serde_json::Value::Null,
    };

    storage.upsert_artifact(&pr_artifact)?;
    storage.upsert_artifact(&commit_artifact)?;

    // 1. Alias Resolution Tests
    let pr_aliases = vec!["PR#23", "pr#23", "#23", "23", "Alfagift/alfagift-android#23"];
    for alias in pr_aliases {
        let matches = storage.resolve_artifact_by_alias(alias)?;
        assert!(!matches.is_empty(), "Failed to resolve alias: {}", alias);
        assert_eq!(matches[0].source_id, "Alfagift/alfagift-android#23");
    }

    let commit_aliases = vec!["01742e72", "01742e72a1a0fe33358840c7cad38560055bc271", "Alfagift/alfagift-android@01742e72"];
    for alias in commit_aliases {
        let matches = storage.resolve_artifact_by_alias(alias)?;
        assert!(!matches.is_empty(), "Failed to resolve commit alias: {}", alias);
        assert_eq!(matches[0].source_id, "Alfagift/alfagift-android@01742e72a1a0fe33358840c7cad38560055bc271");
    }

    // 2. Bidirectional Relationship Verification
    let related_to_ticket = storage.get_related_artifacts("INIT-488")?;
    assert!(!related_to_ticket.is_empty(), "Graph should link INIT-488 bidirectionally");

    // 3. Deduplication Verification
    let init488_count = related_to_ticket
        .iter()
        .filter(|(_, a)| a.source_id == "Alfagift/alfagift-android#23")
        .count();
    assert_eq!(init488_count, 1, "PR #23 must be linked to INIT-488 exactly once without duplicate edges");

    Ok(())
}

#[test]
fn test_graph_relationship_deduplication_and_reverse_edges() -> anyhow::Result<()> {
    let tmp_file = NamedTempFile::new()?;
    let storage = Storage::new(tmp_file.path())?;
    let now = Utc::now();

    let commit_art = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("github", "https://api.github.com", "Alfagift/alfagift-android@c1111111"),
        kind: ArtifactKind::Commit,
        title: "feat(chat): [INIT-488] Add chat interface (#42)".to_string(),
        summary: Some("Adds chat interface (#42)".to_string()),
        body: "Implements INIT-488 and merged into PR #42".to_string(),
        provider: "github".to_string(),
        source_id: "Alfagift/alfagift-android@c1111111".to_string(),
        source_url: "https://github.com/Alfagift/alfagift-android/commit/c1111111".to_string(),
        repository: Some("Alfagift/alfagift-android".to_string()),
        tags: vec!["repo:Alfagift/alfagift-android".to_string()],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "c111_cs".to_string(),
        metadata: serde_json::Value::Null,
    };

    // Upsert multiple times to test idempotency & deduplication
    storage.upsert_artifact(&commit_art)?;
    storage.upsert_artifact(&commit_art)?;

    // 1. Verify Ticket → Commit implements relationship is unique (no duplicate edges)
    let related_ticket = storage.get_related_artifacts("INIT-488")?;
    let commit_matches: Vec<_> = related_ticket
        .iter()
        .filter(|(_, a)| a.source_id == "Alfagift/alfagift-android@c1111111")
        .collect();
    assert_eq!(commit_matches.len(), 1, "Commit must be linked to INIT-488 exactly once");

    // 2. Verify Commit → PR merged_into and PR → Commit contains reverse edges
    let pr_commits = storage.get_commits_for_pr("Alfagift/alfagift-android#42")?;
    assert_eq!(pr_commits.len(), 1, "PR #42 must contain commit c1111111 via reverse edge");
    assert_eq!(pr_commits[0].source_id, "Alfagift/alfagift-android@c1111111");

    Ok(())
}

#[test]
fn test_exact_pr_number_resolution_and_no_substring_matches() -> anyhow::Result<()> {
    let tmp_file = NamedTempFile::new()?;
    let storage = Storage::new(tmp_file.path())?;
    let now = Utc::now();

    let pr23 = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("github", "https://api.github.com", "Alfagift/alfagift-android#23"),
        kind: ArtifactKind::PullRequest,
        title: "In-House Chat PR #23".to_string(),
        summary: None,
        body: "".to_string(),
        provider: "github".to_string(),
        source_id: "Alfagift/alfagift-android#23".to_string(),
        source_url: "".to_string(),
        repository: Some("Alfagift/alfagift-android".to_string()),
        tags: vec![],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "pr23_cs".to_string(),
        metadata: serde_json::Value::Null,
    };

    let pr233 = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("github", "https://api.github.com", "Alfagift/alfagift-android#233"),
        kind: ArtifactKind::PullRequest,
        title: "Top Spender Sheet PR #233".to_string(),
        summary: None,
        body: "".to_string(),
        provider: "github".to_string(),
        source_id: "Alfagift/alfagift-android#233".to_string(),
        source_url: "".to_string(),
        repository: Some("Alfagift/alfagift-android".to_string()),
        tags: vec![],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "pr233_cs".to_string(),
        metadata: serde_json::Value::Null,
    };

    storage.upsert_artifact(&pr23)?;
    storage.upsert_artifact(&pr233)?;

    // 1. Exact PR #23 resolution must NOT return PR #233
    let matches_23 = storage.resolve_artifact_by_alias("PR#23")?;
    assert_eq!(matches_23.len(), 1, "PR#23 must match PR #23 exactly");
    assert_eq!(matches_23[0].source_id, "Alfagift/alfagift-android#23");

    // 2. Exact PR #233 resolution must NOT return PR #23
    let matches_233 = storage.resolve_artifact_by_alias("PR#233")?;
    assert_eq!(matches_233.len(), 1, "PR#233 must match PR #233 exactly");
    assert_eq!(matches_233[0].source_id, "Alfagift/alfagift-android#233");

    // 3. Verify PR regex in automatic relationship extraction for commit with (#23)
    let commit_23 = KnowledgeArtifact {
        id: KnowledgeArtifact::generate_id("github", "https://api.github.com", "Alfagift/alfagift-android@c23"),
        kind: ArtifactKind::Commit,
        title: "[DONE][INIT-488] In-House Chat (#23)".to_string(),
        summary: None,
        body: "".to_string(),
        provider: "github".to_string(),
        source_id: "Alfagift/alfagift-android@c23".to_string(),
        source_url: "".to_string(),
        repository: Some("Alfagift/alfagift-android".to_string()),
        tags: vec![],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: "c23_cs".to_string(),
        metadata: serde_json::Value::Null,
    };

    storage.upsert_artifact(&commit_23)?;

    let pr23_commits = storage.get_commits_for_pr("Alfagift/alfagift-android#23")?;
    assert_eq!(pr23_commits.len(), 1, "PR #23 must link commit c23");

    let pr233_commits = storage.get_commits_for_pr("Alfagift/alfagift-android#233")?;
    assert_eq!(pr233_commits.len(), 0, "PR #233 must NOT link commit c23 via substring match");

    Ok(())
}
