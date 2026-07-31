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


    Ok(())
}
