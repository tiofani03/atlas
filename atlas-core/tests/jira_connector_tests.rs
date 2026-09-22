use atlas_core::{connectors::jira::JiraConnector, Storage};
use serde_json::json;
use tempfile::NamedTempFile;

#[test]
fn test_jira_parse_issue_relationships_parent_subtasks_links() {
    let issue_json = json!({
        "key": "PC-11",
        "fields": {
            "summary": "Google OAuth authentication",
            "status": { "name": "To Do" },
            "description": "Implement Google OAuth 2.0 flow.",
            "project": { "key": "PC" },
            "labels": ["auth", "backend"],
            "parent": {
                "key": "PC-5"
            },
            "subtasks": [
                { "key": "PC-30" },
                { "key": "PC-31" }
            ],
            "issuelinks": [
                {
                    "type": { "name": "Relates", "inward": "is related to", "outward": "relates to" },
                    "outwardIssue": { "key": "PC-12" }
                },
                {
                    "type": { "name": "Blocks", "inward": "is blocked by", "outward": "blocks" },
                    "inwardIssue": { "key": "PC-13" }
                }
            ]
        }
    });

    let parsed = JiraConnector::parse_issue_json(&issue_json, "https://tayoai.atlassian.net")
        .expect("issue parsed successfully");

    assert_eq!(parsed.source_id, "PC-11");
    assert_eq!(parsed.title, "Google OAuth authentication");

    // Must capture parent PC-5
    let has_parent = parsed.relationships.iter().any(|r| {
        r.target_id == "PC-5" && (r.relationship_type == "parent" || r.relationship_type == "child_of")
    });
    assert!(has_parent, "Missing parent PC-5 relationship in parsed issue");

    // Must capture subtasks PC-30 and PC-31
    let has_subtask_30 = parsed.relationships.iter().any(|r| {
        r.target_id == "PC-30" && r.relationship_type == "subtask"
    });
    assert!(has_subtask_30, "Missing subtask PC-30 relationship");

    let has_subtask_31 = parsed.relationships.iter().any(|r| {
        r.target_id == "PC-31" && r.relationship_type == "subtask"
    });
    assert!(has_subtask_31, "Missing subtask PC-31 relationship");

    // Must capture outward link PC-12
    let has_outward = parsed.relationships.iter().any(|r| {
        r.target_id == "PC-12"
    });
    assert!(has_outward, "Missing outward link PC-12");

    // Must capture inward link PC-13
    let has_inward = parsed.relationships.iter().any(|r| {
        r.target_id == "PC-13"
    });
    assert!(has_inward, "Missing inward link PC-13");
}

#[test]
fn test_jira_epic_child_bidirectional_discovery_in_storage() -> anyhow::Result<()> {
    let tmp_file = NamedTempFile::new()?;
    let storage = Storage::new(tmp_file.path())?;

    // Epic PC-5
    let epic_json = json!({
        "key": "PC-5",
        "fields": {
            "summary": "EPIC-1: User Authentication & Workspace Access",
            "status": { "name": "To Do" },
            "description": "PRD for authentication and workspace management.",
            "project": { "key": "PC" }
        }
    });

    // Story PC-11 whose parent is PC-5
    let story_json = json!({
        "key": "PC-11",
        "fields": {
            "summary": "Google OAuth authentication",
            "status": { "name": "To Do" },
            "description": "Auth story description",
            "project": { "key": "PC" },
            "parent": { "key": "PC-5" }
        }
    });

    let epic_art = JiraConnector::parse_issue_json(&epic_json, "https://tayoai.atlassian.net")
        .expect("epic parsed");
    let story_art = JiraConnector::parse_issue_json(&story_json, "https://tayoai.atlassian.net")
        .expect("story parsed");

    storage.upsert_artifact(&epic_art)?;
    storage.upsert_artifact(&story_art)?;

    // Querying related artifacts for PC-5 should discover child PC-11
    let related_to_epic = storage.get_related_artifacts("PC-5")?;
    let found_child = related_to_epic.iter().any(|a| a.1.source_id == "PC-11");
    assert!(
        found_child,
        "Storage graph should link parent PC-5 to child PC-11 bidirectionally"
    );

    Ok(())
}
