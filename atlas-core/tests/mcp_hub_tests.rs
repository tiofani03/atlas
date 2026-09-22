use atlas_core::config::{Config, McpServerConfig};
use atlas_core::domain::{ArtifactKind, KnowledgeArtifact};
use atlas_core::mcp::client::McpClient;
use atlas_core::mcp::hub::{handle_native_tool_with_figma_context, McpHub};
use atlas_core::storage::Storage;
use chrono::Utc;
use serde_json::json;
use std::collections::HashMap;
use tempfile::NamedTempFile;

fn mock_python_script() -> &'static str {
    r#"
import sys, json

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        req = json.loads(line)
    except Exception:
        continue

    req_id = req.get("id")
    method = req.get("method")

    if req_id is None:
        continue

    if method == "initialize":
        resp = {
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "mock-mcp-server", "version": "1.0.0"}
            }
        }
    elif method == "tools/list":
        resp = {
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "tools": [
                    {
                        "name": "echo_tool",
                        "description": "Echo input message",
                        "inputSchema": {
                            "type": "object",
                            "properties": {"msg": {"type": "string"}},
                            "required": ["msg"]
                        }
                    }
                ]
            }
        }
    elif method == "tools/call":
        args = req.get("params", {}).get("arguments", {})
        msg = args.get("msg", "")
        resp = {
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "content": [{"type": "text", "text": f"echo: {msg}"}]
            }
        }
    else:
        resp = {
            "jsonrpc": "2.0",
            "id": req_id,
            "error": {"code": -32601, "message": f"Unknown method: {method}"}
        }
    sys.stdout.write(json.dumps(resp) + "\n")
    sys.stdout.flush()
"#
}

#[tokio::test]
async fn test_mcp_hub_aggregation() {
    let tmp_file = NamedTempFile::new().expect("temp file");
    let storage = Storage::new(tmp_file.path()).expect("init storage");

    let config = McpServerConfig {
        command: "python3".to_string(),
        args: vec!["-u".to_string(), "-c".to_string(), mock_python_script().to_string()],
        env: HashMap::new(),
        enabled: Some(true),
        prefix: Some("mock".to_string()),
        aliases: HashMap::new(),
    };

    let mut client = McpClient::start("mock-server", &config).await.expect("start client");
    client.initialize().await.expect("initialize client");

    let mut hub = McpHub::new(storage);
    hub.add_server("mock-server", Some("mock".to_string()), client);

    // 1. Verify get_aggregated_tools()
    let tools = hub.get_aggregated_tools().await.expect("get aggregated tools");

    // Native tools must be present and un-prefixed
    let native_tool_names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .filter(|n| n.starts_with("atx_"))
        .collect();
    assert!(native_tool_names.contains(&"atx_search"));
    assert!(native_tool_names.contains(&"atx_artifact"));
    assert!(native_tool_names.contains(&"atx_related"));
    assert!(native_tool_names.contains(&"atx_context"));
    assert!(native_tool_names.contains(&"atx_status"));

    // Upstream tool must be prefixed with <prefix>__<tool_name>
    let echo_tool = tools
        .iter()
        .find(|t| t.get("name").and_then(|n| n.as_str()) == Some("mock__echo_tool"))
        .expect("mock__echo_tool not found in aggregated tools");

    let desc = echo_tool
        .get("description")
        .and_then(|d| d.as_str())
        .expect("echo_tool description");
    assert!(desc.contains("[Server: mock-server]"));
    assert!(desc.contains("Echo input message"));

    assert_eq!(
        echo_tool["inputSchema"]["properties"]["msg"]["type"],
        "string"
    );

    // 2. Verify route_tool_call()
    // 2a. Route native tool
    let status_res = hub
        .route_tool_call("atx_status", json!({}))
        .await
        .expect("route atx_status");
    assert!(status_res.to_string().contains("Atlas Engineering Context Graph Status"));

    // 2b. Route upstream tool with prefix
    let echo_res = hub
        .route_tool_call("mock__echo_tool", json!({"msg": "hello from hub router"}))
        .await
        .expect("route mock__echo_tool");
    assert_eq!(echo_res["content"][0]["text"], "echo: hello from hub router");

    // 2c. Un-prefixed upstream tool name should fail routing
    let unprefixed_err = hub.route_tool_call("echo_tool", json!({"msg": "test"})).await;
    assert!(unprefixed_err.is_err());

    // 2d. Unknown tool should fail routing
    let unknown_err = hub.route_tool_call("unknown_tool_xyz", json!({})).await;
    assert!(unknown_err.is_err());
}

#[tokio::test]
async fn test_mcp_hub_from_config() {
    let tmp_file = NamedTempFile::new().expect("temp file");
    let storage = Storage::new(tmp_file.path()).expect("init storage");

    let mut config = Config::default();

    // Enabled server with custom prefix
    config.mcp_servers.insert(
        "mock_srv".to_string(),
        McpServerConfig {
            command: "python3".to_string(),
            args: vec!["-u".to_string(), "-c".to_string(), mock_python_script().to_string()],
            env: HashMap::new(),
            enabled: Some(true),
            prefix: Some("custom_pfx".to_string()),
            aliases: HashMap::new(),
        },
    );

    // Disabled server should not be spawned
    config.mcp_servers.insert(
        "disabled_srv".to_string(),
        McpServerConfig {
            command: "non_existent_disabled_command".to_string(),
            args: vec![],
            env: HashMap::new(),
            enabled: Some(false),
            prefix: Some("disabled".to_string()),
            aliases: HashMap::new(),
        },
    );

    // Server that fails to spawn should log warning and not crash from_config
    config.mcp_servers.insert(
        "failed_srv".to_string(),
        McpServerConfig {
            command: "non_existent_command_xyz_12345".to_string(),
            args: vec![],
            env: HashMap::new(),
            enabled: Some(true),
            prefix: Some("failed".to_string()),
            aliases: HashMap::new(),
        },
    );

    let hub = McpHub::from_config(&config, storage)
        .await
        .expect("McpHub::from_config should succeed gracefully");

    let tools = hub.get_aggregated_tools().await.expect("get tools");
    assert!(tools.iter().any(|t| t.get("name").and_then(|n| n.as_str()) == Some("custom_pfx__echo_tool")));
    assert!(!tools.iter().any(|t| t.get("name").and_then(|n| n.as_str()) == Some("disabled__echo_tool")));

    let call_res = hub
        .route_tool_call("custom_pfx__echo_tool", json!({"msg": "from config"}))
        .await
        .expect("call custom_pfx__echo_tool");
    assert_eq!(call_res["content"][0]["text"], "echo: from config");
}

#[tokio::test]
async fn test_mcp_hub_default_prefix() {
    let tmp_file = NamedTempFile::new().expect("temp file");
    let storage = Storage::new(tmp_file.path()).expect("init storage");

    let config = McpServerConfig {
        command: "python3".to_string(),
        args: vec!["-u".to_string(), "-c".to_string(), mock_python_script().to_string()],
        env: HashMap::new(),
        enabled: Some(true),
        prefix: None, // Omitting prefix should default to server name
        aliases: HashMap::new(),
    };

    let mut client = McpClient::start("my_server", &config).await.expect("start client");
    client.initialize().await.expect("initialize client");

    let mut hub = McpHub::new(storage);
    hub.add_server("my_server", None, client);

    let tools = hub.get_aggregated_tools().await.expect("get tools");
    assert!(tools.iter().any(|t| t.get("name").and_then(|n| n.as_str()) == Some("my_server__echo_tool")));

    let call_res = hub
        .route_tool_call("my_server__echo_tool", json!({"msg": "default prefix test"}))
        .await
        .expect("call my_server__echo_tool");
    assert_eq!(call_res["content"][0]["text"], "echo: default prefix test");
}

#[tokio::test]
async fn test_mcp_hub_from_storage_and_json_rpc_loop() {
    let tmp_file = NamedTempFile::new().expect("temp file");
    let storage = Storage::new(tmp_file.path()).expect("init storage");

    // From<Storage> for McpHub
    let hub: McpHub = storage.into();

    let tools = hub.get_aggregated_tools().await.expect("get tools");
    assert!(!tools.is_empty());
    assert!(tools.iter().all(|t| t.get("name").and_then(|n| n.as_str()).unwrap().starts_with("atx_")));

    // Test JSON-RPC server loop over duplex stream
    let (client_io, server_io) = tokio::io::duplex(4096);
    let (server_reader, server_writer) = tokio::io::split(server_io);

    let server_task = tokio::spawn(async move {
        hub.run_server(server_reader, server_writer).await
    });

    let (client_reader, mut client_writer) = tokio::io::split(client_io);
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let mut client_lines = BufReader::new(client_reader).lines();

    // Send initialize
    let init_req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    client_writer.write_all(format!("{}\n", init_req).as_bytes()).await.unwrap();
    client_writer.flush().await.unwrap();

    let init_line = client_lines.next_line().await.unwrap().expect("init response");
    let init_val: serde_json::Value = serde_json::from_str(&init_line).unwrap();
    assert_eq!(init_val["id"], 1);
    assert_eq!(init_val["result"]["protocolVersion"], "2024-11-05");

    // Send tools/list
    let list_req = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
    client_writer.write_all(format!("{}\n", list_req).as_bytes()).await.unwrap();
    client_writer.flush().await.unwrap();

    let list_line = client_lines.next_line().await.unwrap().expect("list response");
    let list_val: serde_json::Value = serde_json::from_str(&list_line).unwrap();
    assert_eq!(list_val["id"], 2);
    assert!(list_val["result"]["tools"].as_array().unwrap().len() >= 5);

    // Send tools/call for atx_status
    let call_req = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"atx_status","arguments":{}}}"#;
    client_writer.write_all(format!("{}\n", call_req).as_bytes()).await.unwrap();
    client_writer.flush().await.unwrap();

    let call_line = client_lines.next_line().await.unwrap().expect("call response");
    let call_val: serde_json::Value = serde_json::from_str(&call_line).unwrap();
    assert_eq!(call_val["id"], 3);
    assert!(call_val["result"]["content"][0]["text"].as_str().unwrap().contains("Atlas Engineering Context Graph Status"));

    // Close client streams and terminate server loop
    drop(client_lines);
    drop(client_writer);
    server_task.abort();
    let _ = server_task.await;
}

fn mock_figma_python_script() -> &'static str {
    r#"
import sys, json

for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        req = json.loads(line)
    except Exception:
        continue

    req_id = req.get("id")
    method = req.get("method")

    if req_id is None:
        continue

    if method == "initialize":
        resp = {
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "mock-figma-server", "version": "1.0.0"}
            }
        }
    elif method == "tools/list":
        resp = {
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "tools": [
                    {"name": "get_file", "description": "Get file", "inputSchema": {"type": "object"}},
                    {"name": "get_file_nodes", "description": "Get file nodes", "inputSchema": {"type": "object"}}
                ]
            }
        }
    elif method == "tools/call":
        args = req.get("params", {}).get("arguments", {})
        resp = {
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "content": [{"type": "text", "text": json.dumps(args)}]
            }
        }
    else:
        resp = {"jsonrpc": "2.0", "id": req_id, "error": {"code": -32601, "message": f"Unknown method: {method}"}}
    sys.stdout.write(json.dumps(resp) + "\n")
    sys.stdout.flush()
"#
}

#[tokio::test]
async fn test_mcp_hub_figma_transparent_rewriting() {
    let tmp_file = NamedTempFile::new().expect("temp file");
    let storage = Storage::new(tmp_file.path()).expect("init storage");
    let now = Utc::now();
    storage
        .upsert_artifact(&KnowledgeArtifact {
            id: KnowledgeArtifact::generate_id(
                "figma",
                "https://api.figma.com",
                "file:INDEXED_CLONE_KEY",
            ),
            kind: ArtifactKind::Design,
            title: "[PROJ 124] - Checkout Flow Redesign (Copy)".to_string(),
            summary: None,
            body: "Working Figma clone for PROJ-124".to_string(),
            provider: "figma".to_string(),
            source_id: "file:INDEXED_CLONE_KEY".to_string(),
            source_url: "https://www.figma.com/design/INDEXED_CLONE_KEY/Checkout".to_string(),
            repository: None,
            tags: vec!["figma:file".to_string()],
            relationships: Vec::new(),
            created_at: None,
            updated_at: now,
            synced_at: now,
            checksum: "figma-indexed-clone".to_string(),
            metadata: serde_json::json!({}),
        })
        .expect("index Figma candidate");

    let mut aliases = HashMap::new();
    aliases.insert("PROJ-123".to_string(), "wOeG8ZbAQwzyrtZbWpAmIB".to_string());
    aliases.insert("CANONICAL_SHARED".to_string(), "PERSONAL_CLONE_KEY".to_string());

    let mut config = Config::default();
    config.mcp_servers.insert(
        "figma".to_string(),
        McpServerConfig {
            command: "python3".to_string(),
            args: vec!["-u".to_string(), "-c".to_string(), mock_figma_python_script().to_string()],
            env: HashMap::new(),
            enabled: Some(true),
            prefix: Some("figma".to_string()),
            aliases,
        },
    );

    let hub = McpHub::from_config(&config, storage)
        .await
        .expect("from_config");

    // 1. Test ticket alias rewriting: "PROJ-123" -> "wOeG8ZbAQwzyrtZbWpAmIB"
    let res1 = hub
        .route_tool_call(
            "figma__get_file_nodes",
            json!({"fileKey": "PROJ-123", "node_id": "1:2"}),
        )
        .await
        .expect("route ticket call");
    let args1: serde_json::Value =
        serde_json::from_str(res1["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(args1["fileKey"], "wOeG8ZbAQwzyrtZbWpAmIB");
    assert_eq!(args1["node_id"], "1:2");

    // 2. Test full URL sanitization + alias rewriting + node-id extraction
    let url = "https://www.figma.com/design/CANONICAL_SHARED/-Checkout-Flow-?node-id=6236-33268&m=dev";
    let res2 = hub
        .route_tool_call("figma__get_file_nodes", json!({"fileKey": url}))
        .await
        .expect("route url call");
    let args2: serde_json::Value =
        serde_json::from_str(res2["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(args2["fileKey"], "PERSONAL_CLONE_KEY");
    assert_eq!(args2["node_id"], "6236:33268");

    // 3. Test raw URL sanitization for direct key
    let url_raw = "https://www.figma.com/design/wOeG8ZbAQwzyrtZbWpAmIB/Title";
    let res3 = hub
        .route_tool_call("figma__get_file", json!({"fileKey": url_raw}))
        .await
        .expect("route url call");
    let args3: serde_json::Value =
        serde_json::from_str(res3["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(args3["fileKey"], "wOeG8ZbAQwzyrtZbWpAmIB");

    // 4. Gateway auto-detects a ticket from indexed Figma metadata.
    let res4 = hub
        .route_tool_call("figma__get_file", json!({"fileKey": "PROJ-124"}))
        .await
        .expect("route indexed ticket call");
    let args4: serde_json::Value =
        serde_json::from_str(res4["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(args4["fileKey"], "INDEXED_CLONE_KEY");
}

#[tokio::test]
async fn test_native_context_includes_resolved_figma_design() {
    let tmp_file = NamedTempFile::new().expect("temp file");
    let storage = Storage::new(tmp_file.path()).expect("init storage");

    let mut aliases = HashMap::new();
    aliases.insert("PROJ-123".to_string(), "wOeG8ZbAQwzyrtZbWpAmIB:6236:33268".to_string());

    let result = handle_native_tool_with_figma_context(
        &storage,
        "atx_context",
        json!({"id": "PROJ-123", "kind": "issue"}),
        &aliases,
    )
    .await
    .expect("build native context");

    let text = result["content"][0]["text"].as_str().expect("context text");
    let context: serde_json::Value = serde_json::from_str(text).expect("context JSON");
    assert_eq!(context["figma"]["fileKey"], "wOeG8ZbAQwzyrtZbWpAmIB");
    assert_eq!(context["figma"]["nodeId"], "6236:33268");
}
