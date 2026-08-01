use atlas_core::{
    ArtifactKind, ArtifactRelationship, KnowledgeArtifact, Storage,
};
use chrono::Utc;
use tempfile::NamedTempFile;

fn create_test_pr_artifact(repo: &str, number: u64, title: &str) -> KnowledgeArtifact {
    let now = Utc::now();
    let source_id = format!("{}#{}", repo, number);
    let id = KnowledgeArtifact::generate_id("github", "https://api.github.com", &source_id);

    KnowledgeArtifact {
        id,
        kind: ArtifactKind::PullRequest,
        title: title.to_string(),
        summary: Some(format!("PR #{}", number)),
        body: format!("Body of PR #{}", number),
        provider: "github".to_string(),
        source_id: source_id.clone(),
        source_url: format!("https://github.com/{}/pull/{}", repo, number),
        repository: Some(repo.to_string()),
        tags: vec![format!("repo:{}", repo), "type:pull_request".to_string()],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: format!("checksum_{}_{}", repo, number),
        metadata: serde_json::json!({
            "artifact_type": "pull_request",
            "repository": repo,
            "provider": "github",
            "number": number,
            "title": title,
            "state": "closed",
        }),
    }
}

fn create_test_commit_artifact(repo: &str, sha: &str, message: &str) -> KnowledgeArtifact {
    let now = Utc::now();
    let source_id = format!("{}@{}", repo, sha);
    let id = KnowledgeArtifact::generate_id("github", "https://api.github.com", &source_id);

    KnowledgeArtifact {
        id,
        kind: ArtifactKind::Commit,
        title: message.to_string(),
        summary: Some(message.to_string()),
        body: message.to_string(),
        provider: "github".to_string(),
        source_id: source_id.clone(),
        source_url: format!("https://github.com/{}/commit/{}", repo, sha),
        repository: Some(repo.to_string()),
        tags: vec![format!("repo:{}", repo), "type:commit".to_string()],
        relationships: Vec::new(),
        created_at: Some(now),
        updated_at: now,
        synced_at: now,
        checksum: format!("checksum_{}_{}", repo, sha),
        metadata: serde_json::json!({
            "sha": sha,
            "message": message,
        }),
    }
}

#[test]
fn test_exact_pr_lookup_not_prefix_matching() -> anyhow::Result<()> {
    let tmp_file = NamedTempFile::new()?;
    let storage = Storage::new(tmp_file.path())?;

    let pr23 = create_test_pr_artifact("Alfagift/alfagift-android", 23, "[DONE][INIT-488][INIT-573] In-House In-App-Chat & In-App-Call");
    let pr233 = create_test_pr_artifact("Alfagift/alfagift-android", 233, "Refactor payment flow");
    let pr12 = create_test_pr_artifact("Alfagift/alfagift-android", 12, "Fix login bug");
    let pr120 = create_test_pr_artifact("Alfagift/alfagift-android", 120, "Update dependencies");
    let pr7 = create_test_pr_artifact("Alfagift/alfagift-android", 7, "Old feature 7");
    let pr71 = create_test_pr_artifact("Alfagift/alfagift-android", 71, "New feature 71");
    let pr101 = create_test_pr_artifact("Alfagift/alfagift-android", 101, "PR 101");
    let pr1010 = create_test_pr_artifact("Alfagift/alfagift-android", 1010, "PR 1010");
    let pr307 = create_test_pr_artifact("Alfagift/alfagift-android", 307, "Merge pull request #307");

    storage.upsert_artifact(&pr23)?;
    storage.upsert_artifact(&pr233)?;
    storage.upsert_artifact(&pr12)?;
    storage.upsert_artifact(&pr120)?;
    storage.upsert_artifact(&pr7)?;
    storage.upsert_artifact(&pr71)?;
    storage.upsert_artifact(&pr101)?;
    storage.upsert_artifact(&pr1010)?;
    storage.upsert_artifact(&pr307)?;

    // Test resolve_pr
    let resolved_23 = storage.resolve_pr("Alfagift/alfagift-android", 23)?;
    assert!(resolved_23.is_some());
    assert_eq!(resolved_23.unwrap().source_id, "Alfagift/alfagift-android#23");

    let resolved_233 = storage.resolve_pr("Alfagift/alfagift-android", 233)?;
    assert!(resolved_233.is_some());
    assert_eq!(resolved_233.unwrap().source_id, "Alfagift/alfagift-android#233");

    // PR #23 must NEVER match PR #233
    let res_alias_23 = storage.resolve_artifact_by_alias("Alfagift/alfagift-android#23")?;
    assert_eq!(res_alias_23.len(), 1);
    assert_eq!(res_alias_23[0].source_id, "Alfagift/alfagift-android#23");

    // PR #12 must NEVER match PR #120
    let res_alias_12 = storage.resolve_artifact_by_alias("Alfagift/alfagift-android#12")?;
    assert_eq!(res_alias_12.len(), 1);
    assert_eq!(res_alias_12[0].source_id, "Alfagift/alfagift-android#12");

    // PR #7 must NEVER match PR #71
    let res_alias_7 = storage.resolve_artifact_by_alias("Alfagift/alfagift-android#7")?;
    assert_eq!(res_alias_7.len(), 1);
    assert_eq!(res_alias_7[0].source_id, "Alfagift/alfagift-android#7");

    // PR #101 must NEVER match PR #1010
    let res_alias_101 = storage.resolve_artifact_by_alias("Alfagift/alfagift-android#101")?;
    assert_eq!(res_alias_101.len(), 1);
    assert_eq!(res_alias_101[0].source_id, "Alfagift/alfagift-android#101");

    Ok(())

}

#[test]
fn test_commit_pr_relationship_linking_and_repair() -> anyhow::Result<()> {
    let tmp_file = NamedTempFile::new()?;
    let storage = Storage::new(tmp_file.path())?;

    let repo = "Alfagift/alfagift-android";
    let pr23 = create_test_pr_artifact(repo, 23, "[DONE][INIT-488][INIT-573] In-House In-App-Chat & In-App-Call");
    let pr233 = create_test_pr_artifact(repo, 233, "Unrelated feature PR");
    let commit = create_test_commit_artifact(repo, "d18bdfb", "[DONE][INIT-488][INIT-573] In-House In-App-Chat & In-App-Call (#23)");

    storage.upsert_artifact(&pr23)?;
    storage.upsert_artifact(&pr233)?;
    storage.upsert_artifact(&commit)?;

    // Manually insert an incorrect relationship to simulate corrupt/old graph
    let conn = storage.get_connection()?;
    conn.execute(
        "INSERT INTO artifact_relationships (source_id, target_id, relationship_type) VALUES (?1, ?2, ?3)",
        rusqlite::params![commit.source_id, pr233.source_id, "merged_into"],
    )?;

    // Rebuild relationships
    storage.rebuild_all_relationships()?;

    // Fetch related artifacts for the commit
    let related = storage.get_related_artifacts(&commit.source_id)?;
    let pr_targets: Vec<String> = related
        .iter()
        .filter(|(rel, art)| art.kind == ArtifactKind::PullRequest && rel.relationship_type == "merged_into")
        .map(|(_, art)| art.source_id.clone())
        .collect();

    // Must link ONLY to PR #23 and NOT PR #233
    assert_eq!(pr_targets, vec!["Alfagift/alfagift-android#23"]);

    Ok(())
}

#[test]
fn test_multiple_repositories_pr_scoping_and_ambiguity() -> anyhow::Result<()> {
    let tmp_file = NamedTempFile::new()?;
    let storage = Storage::new(tmp_file.path())?;

    let repo1_pr23 = create_test_pr_artifact("Alfagift/alfagift-android", 23, "Android PR #23");
    let repo2_pr23 = create_test_pr_artifact("Alfagift/alfagift-ios", 23, "iOS PR #23");

    storage.upsert_artifact(&repo1_pr23)?;
    storage.upsert_artifact(&repo2_pr23)?;

    // Fully qualified lookup returns exact repo match
    let res_fq1 = storage.resolve_artifact_by_alias("Alfagift/alfagift-android#23")?;
    assert_eq!(res_fq1.len(), 1);
    assert_eq!(res_fq1[0].source_id, "Alfagift/alfagift-android#23");

    let res_fq2 = storage.resolve_artifact_by_alias("Alfagift/alfagift-ios#23")?;
    assert_eq!(res_fq2.len(), 1);
    assert_eq!(res_fq2[0].source_id, "Alfagift/alfagift-ios#23");

    // Unqualified PR#23 across multiple repos produces ambiguous list (both returned)
    let res_ambiguous = storage.resolve_artifact_by_alias("PR#23")?;
    assert_eq!(res_ambiguous.len(), 2);

    Ok(())
}

#[test]
fn test_cli_alias_variants() -> anyhow::Result<()> {
    let tmp_file = NamedTempFile::new()?;
    let storage = Storage::new(tmp_file.path())?;

    let pr23 = create_test_pr_artifact("Alfagift/alfagift-android", 23, "Feature 23");
    storage.upsert_artifact(&pr23)?;

    // Test alias formats
    for alias in &["PR#23", "#23", "23", "pr#23", "PR23", "pr-23", "Alfagift/alfagift-android#23"] {
        let res = storage.resolve_artifact_by_alias(alias)?;
        assert_eq!(res.len(), 1, "Failed for alias: {}", alias);
        assert_eq!(res[0].source_id, "Alfagift/alfagift-android#23");
    }

    Ok(())
}

#[test]
fn test_self_parent_commit_rejection() -> anyhow::Result<()> {
    let tmp_file = NamedTempFile::new()?;
    let storage = Storage::new(tmp_file.path())?;
    let repo = "Alfagift/alfagift-android";

    let mut commit = create_test_commit_artifact(repo, "aaaaaa1", "Commit A");
    // Manually push self-parent relationship (child_sha == parent_sha)
    commit.relationships.push(ArtifactRelationship {
        source_id: commit.source_id.clone(),
        target_id: commit.source_id.clone(),
        relationship_type: "parent_commit".to_string(),
    });

    storage.upsert_artifact(&commit)?;

    let related = storage.get_related_artifacts(&commit.source_id)?;
    // Self parent relationship must be rejected
    for (rel, art) in &related {
        assert_ne!(rel.source_id, rel.target_id, "Self-relationship edge found: {:?}", rel);
        assert_ne!(art.source_id, commit.source_id, "Self-target artifact returned in related list");
    }

    let issues = storage.validate_graph_integrity()?;
    assert!(issues.is_empty(), "Graph integrity issues found: {:?}", issues);

    Ok(())
}

#[test]
fn test_merge_commit_parents_and_directionality() -> anyhow::Result<()> {
    let tmp_file = NamedTempFile::new()?;
    let storage = Storage::new(tmp_file.path())?;
    let repo = "owner/repo";

    let parent_a = create_test_commit_artifact(repo, "aaaaaa1", "Parent Commit A");
    let parent_b = create_test_commit_artifact(repo, "bbbbbb2", "Parent Commit B");
    let mut merge_c = create_test_commit_artifact(repo, "cccccc3", "Merge Commit C");

    merge_c.relationships.push(ArtifactRelationship {
        source_id: merge_c.source_id.clone(),
        target_id: parent_a.source_id.clone(),
        relationship_type: "parent_commit".to_string(),
    });
    merge_c.relationships.push(ArtifactRelationship {
        source_id: merge_c.source_id.clone(),
        target_id: parent_b.source_id.clone(),
        relationship_type: "parent_commit".to_string(),
    });

    storage.upsert_artifact(&parent_a)?;
    storage.upsert_artifact(&parent_b)?;
    storage.upsert_artifact(&merge_c)?;

    // Inspecting merge_c (outgoing relationships)
    let related_c = storage.get_related_artifacts(&merge_c.source_id)?;
    let parent_rels: Vec<(String, String)> = related_c
        .iter()
        .filter(|(r, _)| r.relationship_type == "parent_commit")
        .map(|(r, a)| (r.target_id.clone(), a.title.clone()))
        .collect();

    assert_eq!(parent_rels.len(), 2);
    assert!(parent_rels.iter().any(|(id, _)| id == &parent_a.source_id));
    assert!(parent_rels.iter().any(|(id, _)| id == &parent_b.source_id));

    // Inspecting parent_a (incoming relationship from child merge_c)
    let related_a = storage.get_related_artifacts(&parent_a.source_id)?;
    let child_rels: Vec<(String, String)> = related_a
        .iter()
        .filter(|(r, _)| r.relationship_type == "child_commit")
        .map(|(r, a)| (r.target_id.clone(), a.title.clone()))
        .collect();

    assert_eq!(child_rels.len(), 1);
    assert_eq!(child_rels[0].0, merge_c.source_id);
    assert_eq!(child_rels[0].1, "Merge Commit C");

    Ok(())
}

#[test]
fn test_utf8_safe_truncation_emojis_and_cjk() {
    let input = "🚀✨⚡→こんにちは世界😀";
    
    // Helper char boundary safe truncation test
    let safe_truncate = |text: &str, max_chars: usize| -> String {
        text.chars().take(max_chars).collect()
    };

    for limit in 0..=input.chars().count() + 2 {
        let truncated = safe_truncate(input, limit);
        assert!(std::str::from_utf8(truncated.as_bytes()).is_ok(), "Invalid UTF-8 at limit {}", limit);
    }
}

#[test]
fn test_long_release_notes_rendering() -> anyhow::Result<()> {
    let tmp_file = NamedTempFile::new()?;
    let storage = Storage::new(tmp_file.path())?;
    let repo = "Alfagift/alfagift-android";

    // Build >5000 character release note with emojis and arrows
    let mut long_body = String::from("🚀 Release Notes 4.52.0 ✨\n\n");
    for i in 0..200 {
        long_body.push_str(&format!("* [DONE][INIT-{}] Feature {} → https://github.com/{}/pull/{}\n", i, i, repo, i));
    }

    let mut release = create_test_pr_artifact(repo, 4520, "Release 4.52.0");
    release.kind = ArtifactKind::Release;
    release.source_id = format!("{}/releases/4.52.0", repo);
    release.body = long_body;

    storage.upsert_artifact(&release)?;

    let fetched = storage.get_artifact_by_id(&release.source_id)?.expect("Release artifact not found");
    assert!(fetched.body.chars().count() > 5000);

    Ok(())
}

#[test]
fn test_release_to_commit_graph_traversal() -> anyhow::Result<()> {
    let tmp_file = NamedTempFile::new()?;
    let storage = Storage::new(tmp_file.path())?;
    let repo = "Alfagift/alfagift-android";

    let commit_a = create_test_commit_artifact(repo, "d18bdfb4821bcea8581e6422c68c505433bf9c49", "Commit A");
    let pr23 = create_test_pr_artifact(repo, 23, "PR #23");

    let release_meta = serde_json::json!({
        "target_commitish": "d18bdfb4821bcea8581e6422c68c505433bf9c49",
        "tag_name": "4.52.0"
    });

    let release = KnowledgeArtifact {
        id: "rel_4520".to_string(),
        kind: ArtifactKind::Release,
        title: "Release 4.52.0".to_string(),
        summary: Some("Release 4.52.0".to_string()),
        body: format!("Included PR: https://github.com/{}/pull/23 and ticket INIT-488", repo),
        provider: "github".to_string(),
        source_id: format!("{}/releases/4.52.0", repo),
        source_url: format!("https://github.com/{}/releases/tag/4.52.0", repo),
        repository: Some(repo.to_string()),
        tags: vec!["type:release".to_string()],
        relationships: vec![],
        created_at: Some(Utc::now()),
        updated_at: Utc::now(),
        synced_at: Utc::now(),
        checksum: "rel_checksum".to_string(),
        metadata: release_meta,
    };

    storage.upsert_artifact(&commit_a)?;
    storage.upsert_artifact(&pr23)?;
    storage.upsert_artifact(&release)?;

    let related = storage.get_related_artifacts(&release.source_id)?;
    let target_ids: Vec<String> = related.iter().map(|(_, a)| a.source_id.clone()).collect();

    assert!(target_ids.contains(&commit_a.source_id), "Release should link to target commit SHA");
    assert!(target_ids.contains(&pr23.source_id), "Release should link to PR #23");

    Ok(())
}


