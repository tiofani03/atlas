use atlas_core::config::McpServerConfig;
use atlas_core::mcp::client::McpClient;
use serde_json::json;

#[test]
fn test_mcp_client_request_formatting() {
    let req1 = McpClient::format_request(1, "initialize", Some(json!({"protocolVersion": "2024-11-05"})));
    assert_eq!(req1["jsonrpc"], "2.0");
    assert_eq!(req1["id"], 1);
    assert_eq!(req1["method"], "initialize");
    assert_eq!(req1["params"]["protocolVersion"], "2024-11-05");

    let req2 = McpClient::format_request(2, "tools/list", None);
    assert_eq!(req2["jsonrpc"], "2.0");
    assert_eq!(req2["id"], 2);
    assert_eq!(req2["method"], "tools/list");
    assert!(req2.get("params").is_none());
}

#[test]
fn test_mcp_client_response_parsing() {
    // Matching successful response
    let line = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[{"name":"test_tool"}]}}"#;
    let res = McpClient::parse_response_line(line, 1).expect("parse response");
    assert!(res.is_some());
    let val = res.unwrap();
    assert_eq!(val["tools"][0]["name"], "test_tool");

    // Mismatched id should return None (ignored)
    let mismatched = McpClient::parse_response_line(line, 2).expect("parse mismatched id");
    assert!(mismatched.is_none());

    // Non-JSON or notification line should return None
    let notif = r#"{"jsonrpc":"2.0","method":"notifications/message","params":{}}"#;
    let notif_res = McpClient::parse_response_line(notif, 1).expect("parse notification");
    assert!(notif_res.is_none());

    let garbage = "some random log message";
    let garbage_res = McpClient::parse_response_line(garbage, 1).expect("parse garbage");
    assert!(garbage_res.is_none());

    // Error response should return Err
    let err_line = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#;
    let err_res = McpClient::parse_response_line(err_line, 1);
    assert!(err_res.is_err());
    let err_str = err_res.unwrap_err().to_string();
    assert!(err_str.contains("-32601"));
    assert!(err_str.contains("Method not found"));
}

#[tokio::test]
async fn test_mcp_client_mock_process_lifecycle() {
    // Mock MCP server python script that handles initialize, tools/list, and tools/call
    let script = r#"
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
        # notification, e.g. notifications/initialized
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
"#;

    let config = McpServerConfig {
        command: "python3".to_string(),
        args: vec!["-u".to_string(), "-c".to_string(), script.to_string()],
        env: std::collections::HashMap::new(),
        enabled: Some(true),
        prefix: Some("mock".to_string()),
        aliases: std::collections::HashMap::new(),
    };

    let mut client = McpClient::start("mock-server", &config).await.expect("start mock client");
    assert_eq!(client.server_name(), "mock-server");

    // 1. initialize
    let init_res = client.initialize().await.expect("initialize");
    assert_eq!(init_res["protocolVersion"], "2024-11-05");
    assert_eq!(init_res["serverInfo"]["name"], "mock-mcp-server");

    // 2. list_tools
    let tools = client.list_tools().await.expect("list_tools");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "echo_tool");

    // 3. call_tool
    let call_res = client.call_tool("echo_tool", json!({"msg": "hello atlas"})).await.expect("call_tool");
    assert_eq!(call_res["content"][0]["text"], "echo: hello atlas");

    // 4. close
    client.close().await.expect("close client");
}

#[tokio::test]
async fn test_mcp_client_spawn_failure() {
    let config = McpServerConfig {
        command: "non_existent_command_xyz_123".to_string(),
        args: vec![],
        env: std::collections::HashMap::new(),
        enabled: Some(true),
        prefix: None,
        aliases: std::collections::HashMap::new(),
    };

    let result = McpClient::start("invalid-server", &config).await;
    assert!(result.is_err());
}
