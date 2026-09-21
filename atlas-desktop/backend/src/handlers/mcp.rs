use crate::state::AppState;
use atlas_core::{McpClient, McpServerConfig};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Response item for configured MCP servers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpServerResponse {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env_keys: Vec<String>,
    pub enabled: bool,
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools_count: Option<usize>,
    pub status: String,
}

/// Request payload for creating or updating an MCP server configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct SaveMcpServerPayload {
    pub name: String,
    pub command: String,
    #[serde(default, deserialize_with = "deserialize_args")]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub prefix: Option<String>,
}

/// Request payload for testing an MCP server connection (optional overrides).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TestMcpServerPayload {
    pub command: Option<String>,
    #[serde(default, deserialize_with = "deserialize_args")]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
    #[serde(default)]
    pub prefix: Option<String>,
}

/// Ready-to-copy client configuration snippets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSnippetResponse {
    pub claude_desktop: Value,
    pub cursor: Value,
    pub agy: Value,
}

/// Flexible deserializer allowing `args` to be specified as an array of strings,
/// or a space/comma-separated single string.
fn deserialize_args<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct ArgsVisitor;

    impl<'de> serde::de::Visitor<'de> for ArgsVisitor {
        type Value = Option<Vec<String>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a list of arguments, a space/comma-separated string, or null")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut vec = Vec::new();
            while let Some(elem) = seq.next_element::<String>()? {
                vec.push(elem);
            }
            Ok(Some(vec))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(Some(Vec::new()));
            }
            let parts: Vec<String> = if trimmed.contains(',') {
                trimmed
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            } else {
                trimmed
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect()
            };
            Ok(Some(parts))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }
    }

    deserializer.deserialize_any(ArgsVisitor)
}

/// `GET /api/mcp/servers`
/// Returns list of configured MCP servers with fields name, command, args,
/// env_keys (redacted / keys only), enabled, prefix, and status.
pub async fn list_mcp_servers(State(state): State<AppState>) -> impl IntoResponse {
    let cfg = match state.load_config() {
        Ok(c) => c,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to load config: {}", err) })),
            );
        }
    };

    let mut servers: Vec<McpServerResponse> = cfg
        .mcp_servers
        .iter()
        .map(|(name, server_cfg)| {
            let mut env_keys: Vec<String> = server_cfg.env.keys().cloned().collect();
            env_keys.sort();
            let enabled = server_cfg.enabled.unwrap_or(true);
            let status = if enabled {
                "enabled".to_string()
            } else {
                "disabled".to_string()
            };

            McpServerResponse {
                name: name.clone(),
                command: server_cfg.command.clone(),
                args: server_cfg.args.clone(),
                env_keys,
                enabled,
                prefix: server_cfg.prefix.clone(),
                tools_count: None,
                status,
            }
        })
        .collect();

    servers.sort_by(|a, b| a.name.cmp(&b.name));

    (StatusCode::OK, Json(json!(servers)))
}

/// `POST /api/mcp/servers`
/// Saves or updates an MCP server configuration and writes config to disk.
pub async fn save_mcp_server(
    State(state): State<AppState>,
    Json(payload): Json<SaveMcpServerPayload>,
) -> impl IntoResponse {
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Server name cannot be empty" })),
        );
    }

    let command = payload.command.trim().to_string();
    if command.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Server command cannot be empty" })),
        );
    }

    let mut cfg = match state.load_config() {
        Ok(c) => c,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to load config: {}", err) })),
            );
        }
    };

    let existing = cfg.mcp_servers.get(&name).cloned();

    let final_args = payload.args.unwrap_or_else(|| {
        existing.as_ref().map(|e| e.args.clone()).unwrap_or_default()
    });

    let final_prefix = match payload.prefix {
        Some(p) => {
            let trimmed = p.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }
        None => existing.as_ref().and_then(|e| e.prefix.clone()),
    };

    let final_enabled = payload
        .enabled
        .or_else(|| existing.as_ref().and_then(|e| e.enabled))
        .or(Some(true));

    let mut final_env = existing.as_ref().map(|e| e.env.clone()).unwrap_or_default();
    if let Some(incoming_env) = payload.env {
        for (k, v) in incoming_env {
            let trimmed_k = k.trim().to_string();
            if trimmed_k.is_empty() {
                continue;
            }
            if v.trim().is_empty() {
                // Keep existing non-empty value if any, avoid wiping with blank
            } else {
                final_env.insert(trimmed_k, v);
            }
        }
    }

    let existing_aliases = cfg
        .mcp_servers
        .get(&name)
        .map(|s| s.aliases.clone())
        .unwrap_or_default();

    let new_server = McpServerConfig {
        command,
        args: final_args,
        env: final_env,
        enabled: final_enabled,
        prefix: final_prefix,
        aliases: existing_aliases,
    };

    cfg.mcp_servers.insert(name.clone(), new_server);

    if let Err(err) = cfg.save_to_path(&state.config_path) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to write config to disk: {}", err) })),
        );
    }

    (
        StatusCode::OK,
        Json(json!({ "success": true, "name": name })),
    )
}

/// `DELETE /api/mcp/servers/:name`
/// Removes an MCP server from configuration and saves to disk.
pub async fn delete_mcp_server(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let mut cfg = match state.load_config() {
        Ok(c) => c,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to load config: {}", err) })),
            );
        }
    };

    if cfg.mcp_servers.remove(&name).is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("MCP server '{}' not found", name) })),
        );
    }

    if let Err(err) = cfg.save_to_path(&state.config_path) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to write config to disk: {}", err) })),
        );
    }

    (
        StatusCode::OK,
        Json(json!({ "success": true, "name": name })),
    )
}

/// `POST /api/mcp/servers/:name/test`
/// Connects to upstream server with `atlas_core::mcp::McpClient::start`,
/// performs initialization handshake, calls `list_tools`, and returns
/// `{ success: true, tools: Vec<Value>, message: String }` or `{ success: false, error: String }`.
pub async fn test_mcp_server(
    State(state): State<AppState>,
    Path(name): Path<String>,
    payload: Option<Json<TestMcpServerPayload>>,
) -> impl IntoResponse {
    let cfg = match state.load_config() {
        Ok(c) => c,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": format!("Failed to load config: {}", err)
                })),
            );
        }
    };

    let existing = cfg.mcp_servers.get(&name).cloned();

    let server_cfg = if let Some(Json(p)) = payload {
        if let Some(cmd) = p.command.filter(|c| !c.trim().is_empty()) {
            let args = p.args.unwrap_or_else(|| {
                existing.as_ref().map(|e| e.args.clone()).unwrap_or_default()
            });
            let mut env = existing.as_ref().map(|e| e.env.clone()).unwrap_or_default();
            if let Some(incoming_env) = p.env {
                for (k, v) in incoming_env {
                    let tk = k.trim().to_string();
                    if !tk.is_empty() && !v.trim().is_empty() {
                        env.insert(tk, v);
                    }
                }
            }
            let prefix = p.prefix.or_else(|| existing.as_ref().and_then(|e| e.prefix.clone()));
            McpServerConfig {
                command: cmd,
                args,
                env,
                enabled: Some(true),
                prefix,
                aliases: existing.as_ref().map(|e| e.aliases.clone()).unwrap_or_default(),
            }
        } else if let Some(e) = existing {
            e
        } else {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "error": format!("MCP server '{}' not found in configuration", name)
                })),
            );
        }
    } else if let Some(e) = existing {
        e
    } else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "error": format!("MCP server '{}' not found in configuration", name)
            })),
        );
    };

    let mut client = match McpClient::start(&name, &server_cfg).await {
        Ok(c) => c,
        Err(err) => {
            return (
                StatusCode::OK,
                Json(json!({
                    "success": false,
                    "error": format!("Failed to spawn MCP server '{}': {:#}", name, err)
                })),
            );
        }
    };

    if let Err(err) = client.initialize().await {
        let _ = client.close().await;
        return (
            StatusCode::OK,
            Json(json!({
                "success": false,
                "error": format!("Protocol initialization failed for '{}': {:#}", name, err)
            })),
        );
    }

    let tools = match client.list_tools().await {
        Ok(t) => t,
        Err(err) => {
            let _ = client.close().await;
            return (
                StatusCode::OK,
                Json(json!({
                    "success": false,
                    "error": format!("Failed to query tools from '{}': {:#}", name, err)
                })),
            );
        }
    };

    let _ = client.close().await;

    let count = tools.len();
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "tools": tools,
            "message": format!("Successfully connected to '{}' and discovered {} tools.", name, count)
        })),
    )
}

/// `GET /api/mcp/snippet`
/// Returns JSON with configuration snippets for Claude Desktop, Cursor, and Antigravity (agy).
pub async fn get_mcp_snippets() -> impl IntoResponse {
    let snippets = McpSnippetResponse {
        claude_desktop: json!({
            "mcpServers": {
                "atlas": {
                    "command": "atx",
                    "args": ["mcp"]
                }
            }
        }),
        cursor: json!({
            "mcpServers": {
                "atlas": {
                    "command": "atx",
                    "args": ["mcp"]
                }
            }
        }),
        agy: json!({
            "mcp": {
                "servers": {
                    "atlas": {
                        "command": "atx",
                        "args": ["mcp"]
                    }
                }
            }
        }),
    };

    (StatusCode::OK, Json(snippets))
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::Config;
    use axum::extract::State;
    use std::fs;

    #[test]
    fn test_deserialize_args_various_formats() {
        let json_seq = r#"["arg1", "arg2"]"#;
        let res: Option<Vec<String>> = serde_json::from_str(json_seq).unwrap();
        assert_eq!(res, Some(vec!["arg1".to_string(), "arg2".to_string()]));

        #[derive(Deserialize)]
        struct Wrapper {
            #[serde(default, deserialize_with = "deserialize_args")]
            args: Option<Vec<String>>,
        }

        let w1: Wrapper = serde_json::from_str(r#"{"args": ["-y", "pkg"]}"#).unwrap();
        assert_eq!(w1.args, Some(vec!["-y".to_string(), "pkg".to_string()]));

        let w2: Wrapper = serde_json::from_str(r#"{"args": "-y pkg"}"#).unwrap();
        assert_eq!(w2.args, Some(vec!["-y".to_string(), "pkg".to_string()]));

        let w3: Wrapper = serde_json::from_str(r#"{"args": "foo, bar,baz"}"#).unwrap();
        assert_eq!(
            w3.args,
            Some(vec!["foo".to_string(), "bar".to_string(), "baz".to_string()])
        );

        let w4: Wrapper = serde_json::from_str(r#"{"args": null}"#).unwrap();
        assert_eq!(w4.args, None);

        let w5: Wrapper = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(w5.args, None);
    }

    #[tokio::test]
    async fn test_mcp_snippets() {
        let resp = get_mcp_snippets().await.into_response();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_mcp_servers_crud_flow() {
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let test_dir = std::env::temp_dir().join(format!("atlas_test_mcp_{}", unique_id));
        fs::create_dir_all(&test_dir).unwrap();
        let config_file = test_dir.join("config.toml");
        let initial_cfg = Config::default();
        initial_cfg.save_to_path(&config_file).unwrap();

        let state = AppState::new(config_file.clone());

        // 1. List initially empty
        let list_resp = list_mcp_servers(State(state.clone())).await.into_response();
        assert_eq!(list_resp.status(), StatusCode::OK);

        // 2. Save server
        let mut env = HashMap::new();
        env.insert("FIGMA_TOKEN".to_string(), "secret_token_123".to_string());
        let payload = SaveMcpServerPayload {
            name: "figma".to_string(),
            command: "npx".to_string(),
            args: Some(vec!["-y".to_string(), "@figma/mcp".to_string()]),
            env: Some(env),
            enabled: Some(true),
            prefix: Some("figma".to_string()),
        };

        let save_resp = save_mcp_server(State(state.clone()), Json(payload)).await.into_response();
        assert_eq!(save_resp.status(), StatusCode::OK);

        // Verify saved to disk
        let disk_cfg = Config::load_from_path(&config_file).unwrap();
        assert!(disk_cfg.mcp_servers.contains_key("figma"));
        let figma = &disk_cfg.mcp_servers["figma"];
        assert_eq!(figma.command, "npx");
        assert_eq!(figma.prefix.as_deref(), Some("figma"));
        assert_eq!(figma.env.get("FIGMA_TOKEN").unwrap(), "secret_token_123");

        // 3. List contains figma with env_keys redacted
        let list_resp2 = list_mcp_servers(State(state.clone())).await.into_response();
        assert_eq!(list_resp2.status(), StatusCode::OK);

        // 4. Delete server
        let del_resp = delete_mcp_server(State(state.clone()), Path("figma".to_string())).await.into_response();
        assert_eq!(del_resp.status(), StatusCode::OK);

        let disk_cfg2 = Config::load_from_path(&config_file).unwrap();
        assert!(!disk_cfg2.mcp_servers.contains_key("figma"));

        // 5. Delete non-existent returns 404
        let del_404 = delete_mcp_server(State(state.clone()), Path("figma".to_string())).await.into_response();
        assert_eq!(del_404.status(), StatusCode::NOT_FOUND);

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[tokio::test]
    async fn test_mcp_test_nonexistent_returns_not_found() {
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let test_dir = std::env::temp_dir().join(format!("atlas_test_mcp_{}", unique_id));
        fs::create_dir_all(&test_dir).unwrap();
        let config_file = test_dir.join("config.toml");
        let initial_cfg = Config::default();
        initial_cfg.save_to_path(&config_file).unwrap();

        let state = AppState::new(config_file.clone());

        let resp = test_mcp_server(State(state), Path("nonexistent".to_string()), None).await.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[tokio::test]
    async fn test_mcp_snippets_structure() {
        let resp = get_mcp_snippets().await.into_response();
        assert_eq!(resp.status(), StatusCode::OK);

        use axum::body::to_bytes;
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let val: serde_json::Value = serde_json::from_slice(&body).unwrap();

        // Claude Desktop
        let claude = val.get("claude_desktop").expect("claude_desktop field");
        assert_eq!(claude["mcpServers"]["atlas"]["command"], "atx");
        assert_eq!(claude["mcpServers"]["atlas"]["args"][0], "mcp");

        // Cursor
        let cursor = val.get("cursor").expect("cursor field");
        assert_eq!(cursor["mcpServers"]["atlas"]["command"], "atx");
        assert_eq!(cursor["mcpServers"]["atlas"]["args"][0], "mcp");

        // Agy
        let agy = val.get("agy").expect("agy field");
        assert_eq!(agy["mcp"]["servers"]["atlas"]["command"], "atx");
        assert_eq!(agy["mcp"]["servers"]["atlas"]["args"][0], "mcp");
    }

    #[tokio::test]
    async fn test_mcp_test_live_mock_server() {
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let test_dir = std::env::temp_dir().join(format!("atlas_test_mcp_{}", unique_id));
        fs::create_dir_all(&test_dir).unwrap();

        let script_path = test_dir.join("mock_mcp.py");
        let script_content = r#"
import sys, json
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    req = json.loads(line)
    req_id = req.get("id")
    method = req.get("method")
    if method == "initialize":
        res = {
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "mock-mcp", "version": "1.0.0"}
            }
        }
        sys.stdout.write(json.dumps(res) + "\n")
        sys.stdout.flush()
    elif method == "tools/list":
        res = {
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "tools": [
                    {"name": "mock_echo", "description": "Echo tool for testing"}
                ]
            }
        }
        sys.stdout.write(json.dumps(res) + "\n")
        sys.stdout.flush()
"#;
        fs::write(&script_path, script_content).unwrap();

        let config_file = test_dir.join("config.toml");
        let mut cfg = Config::default();
        cfg.mcp_servers.insert(
            "mock".to_string(),
            McpServerConfig {
                command: "python3".to_string(),
                args: vec![script_path.to_string_lossy().to_string()],
                env: HashMap::new(),
                enabled: Some(true),
                prefix: Some("mock".to_string()),
                aliases: HashMap::new(),
            },
        );
        cfg.save_to_path(&config_file).unwrap();

        let state = AppState::new(config_file);

        let resp = test_mcp_server(
            State(state),
            Path("mock".to_string()),
            None,
        )
        .await
        .into_response();

        assert_eq!(resp.status(), StatusCode::OK);
        use axum::body::to_bytes;
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let val: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(val["success"], true);
        assert_eq!(val["tools"].as_array().unwrap().len(), 1);
        assert_eq!(val["tools"][0]["name"], "mock_echo");
        assert!(val["message"].as_str().unwrap().contains("discovered 1 tools"));

        let _ = fs::remove_dir_all(&test_dir);
    }
}
