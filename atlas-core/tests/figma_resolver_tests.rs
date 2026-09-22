use atlas_core::mcp::resolver::{
    find_cached_figma_file_for_ticket_in_dir, find_figma_file_for_ticket_in_candidates,
    load_local_project_aliases, load_local_project_config, parse_figma_target,
    resolve_figma_target, FigmaFileCandidate,
};
use std::collections::HashMap;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_parse_figma_target_full_urls() {
    // 1. New design URL with dash node-id and dev query param
    let url1 = "https://www.figma.com/design/wOeG8ZbAQwzyrtZbWpAmIB/-PROJ-123----Checkout-Flow-Redesign--Copy---Copy-?node-id=6236-33268&m=dev";
    let (key1, node1) = parse_figma_target(url1);
    assert_eq!(key1, "wOeG8ZbAQwzyrtZbWpAmIB");
    assert_eq!(node1, Some("6236:33268".to_string()));

    // 2. Legacy /file/ URL with encoded colon node-id (%3A)
    let url2 = "https://www.figma.com/file/abc123DEF456ghi/Project-Design?node-id=101%3A202";
    let (key2, node2) = parse_figma_target(url2);
    assert_eq!(key2, "abc123DEF456ghi");
    assert_eq!(node2, Some("101:202".to_string()));

    // 3. Proto URL without node-id
    let url3 = "https://www.figma.com/proto/testProtoKey789/Prototype-Flow";
    let (key3, node3) = parse_figma_target(url3);
    assert_eq!(key3, "testProtoKey789");
    assert_eq!(node3, None);

    // 4. Raw key with leading/trailing whitespace
    let (key4, node4) = parse_figma_target("  wOeG8ZbAQwzyrtZbWpAmIB  ");
    assert_eq!(key4, "wOeG8ZbAQwzyrtZbWpAmIB");
    assert_eq!(node4, None);

    // 5. Raw key with colon node
    let (key5, node5) = parse_figma_target("wOeG8ZbAQwzyrtZbWpAmIB:6236:33268");
    assert_eq!(key5, "wOeG8ZbAQwzyrtZbWpAmIB");
    assert_eq!(node5, Some("6236:33268".to_string()));
}

#[test]
fn test_resolve_figma_target_with_server_aliases() {
    let mut aliases = HashMap::new();
    aliases.insert("ORIGINAL_KEY_111".to_string(), "CLONED_KEY_222".to_string());
    aliases.insert("PROJ-123".to_string(), "wOeG8ZbAQwzyrtZbWpAmIB".to_string());

    // 1. Rewrite key directly
    let (res_key1, node1) = resolve_figma_target("ORIGINAL_KEY_111", &aliases, None, None);
    assert_eq!(res_key1, "CLONED_KEY_222");
    assert_eq!(node1, None);

    // 2. Rewrite URL containing original key
    let url = "https://www.figma.com/design/ORIGINAL_KEY_111/Canonical-Design?node-id=42-99";
    let (res_key2, node2) = resolve_figma_target(url, &aliases, None, None);
    assert_eq!(res_key2, "CLONED_KEY_222");
    assert_eq!(node2, Some("42:99".to_string()));

    // 3. Resolve ticket code (case-insensitive)
    let (res_key3, _) = resolve_figma_target("proj-123", &aliases, None, None);
    assert_eq!(res_key3, "wOeG8ZbAQwzyrtZbWpAmIB");
}

#[test]
fn test_resolve_figma_target_with_local_project_file() {
    let dir = tempdir().expect("temp dir");
    let atlas_dir = dir.path().join(".atlas");
    fs::create_dir_all(&atlas_dir).expect("create .atlas dir");

    let toml_content = r#"
        [aliases]
        "INIT-404" = "PROJECT_CLONE_KEY_999"
        "CANONICAL_SHARED" = "PERSONAL_FORK_KEY"
    "#;
    fs::write(atlas_dir.join("figma.toml"), toml_content).expect("write figma.toml");

    let local_aliases = load_local_project_aliases(dir.path());
    assert_eq!(local_aliases.get("INIT-404").map(|s| s.as_str()), Some("PROJECT_CLONE_KEY_999"));

    let empty_server_aliases = HashMap::new();
    let (res_key, _) = resolve_figma_target("INIT-404", &empty_server_aliases, Some(dir.path()), None);
    assert_eq!(res_key, "PROJECT_CLONE_KEY_999");
}

#[test]
fn test_resolve_figma_target_auto_detection_from_cache() {
    let dir = tempdir().expect("temp cache dir");
    let cache_dir = dir.path();

    // Create a mock cached figma file
    let file_content = r#"{
        "name": "[PROJ 123] - Checkout Flow Redesign (Copy) (Copy)",
        "document": { "id": "0:0", "name": "Document" }
    }"#;
    fs::write(
        cache_dir.join("file_wOeG8ZbAQwzyrtZbWpAmIB_1789977151226.json"),
        file_content,
    )
    .expect("write cache file");

    // Also write a nodes file which should be ignored
    fs::write(
        cache_dir.join("file_nodes_wOeG8ZbAQwzyrtZbWpAmIB_1789977200555.json"),
        file_content,
    )
    .expect("write nodes file");

    let detected = find_cached_figma_file_for_ticket_in_dir("PROJ-123", cache_dir);
    assert_eq!(detected.as_deref(), Some("wOeG8ZbAQwzyrtZbWpAmIB"));

    // Also test with spaces format: "PROJ 123"
    let detected2 = find_cached_figma_file_for_ticket_in_dir("PROJ 123", cache_dir);
    assert_eq!(detected2.as_deref(), Some("wOeG8ZbAQwzyrtZbWpAmIB"));

    // Resolve target using auto-detection
    let empty_aliases = HashMap::new();
    let (res_key, _) = resolve_figma_target("PROJ-123", &empty_aliases, None, Some(cache_dir));
    assert_eq!(res_key, "wOeG8ZbAQwzyrtZbWpAmIB");
}

#[test]
fn test_resolve_figma_target_from_plan_project_config() {
    let dir = tempdir().expect("temp dir");
    let atlas_dir = dir.path().join(".atlas");
    fs::create_dir_all(&atlas_dir).expect("create .atlas dir");
    fs::write(
        atlas_dir.join("figma.toml"),
        r#"
            [figma]
            file_key = "wOeG8ZbAQwzyrtZbWpAmIB"
            node_id = "6236-33268"
        "#,
    )
    .expect("write figma.toml");

    let project = load_local_project_config(dir.path());
    assert_eq!(project.file_key.as_deref(), Some("wOeG8ZbAQwzyrtZbWpAmIB"));
    assert_eq!(project.node_id.as_deref(), Some("6236:33268"));

    let empty_aliases = HashMap::new();
    let (resolved_key, resolved_node) =
        resolve_figma_target("PROJ-123", &empty_aliases, Some(dir.path()), None);
    assert_eq!(resolved_key, "wOeG8ZbAQwzyrtZbWpAmIB");
    assert_eq!(resolved_node.as_deref(), Some("6236:33268"));
}

#[test]
fn test_auto_detection_from_indexed_figma_candidates() {
    let candidates = vec![
        FigmaFileCandidate {
            file_key: "unrelated-key".to_string(),
            name: "PROJ 120 - Other screen".to_string(),
            body: "Other design".to_string(),
            node_id: None,
        },
        FigmaFileCandidate {
            file_key: "clone-key-123".to_string(),
            name: "[PROJ 123] - Checkout Flow Redesign (Copy)".to_string(),
            body: "Working clone".to_string(),
            node_id: Some("6236:33268".to_string()),
        },
    ];

    let resolved = find_figma_file_for_ticket_in_candidates("PROJ-123", &candidates);
    assert_eq!(
        resolved,
        Some((
            "clone-key-123".to_string(),
            Some("6236:33268".to_string())
        ))
    );
}
