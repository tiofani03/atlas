use crate::config::McpServerConfig;
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

/// Client for communicating with an external upstream MCP server over stdio.
pub struct McpClient {
    server_name: String,
    child: Child,
    stdin: ChildStdin,
    reader: Lines<BufReader<ChildStdout>>,
    next_id: u64,
}

impl McpClient {
    /// Launch the MCP server child process using the provided configuration.
    pub async fn start(server_name: &str, config: &McpServerConfig) -> Result<Self> {
        let mut cmd = Command::new(&config.command);
        cmd.args(&config.args);
        cmd.envs(&config.env);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.kill_on_drop(true);

        let mut child = cmd.spawn().with_context(|| {
            format!(
                "Failed to spawn MCP server '{}' with command '{}'",
                server_name, config.command
            )
        })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            anyhow::anyhow!("Failed to open stdin for MCP server '{}'", server_name)
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            anyhow::anyhow!("Failed to open stdout for MCP server '{}'", server_name)
        })?;

        if let Some(stderr) = child.stderr.take() {
            let s_name = server_name.to_string();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tracing::debug!(server = %s_name, "[mcp stderr] {}", line);
                }
            });
        }

        let reader = BufReader::new(stdout).lines();

        Ok(Self {
            server_name: server_name.to_string(),
            child,
            stdin,
            reader,
            next_id: 1,
        })
    }

    /// Server name identifier.
    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    /// Format a JSON-RPC 2.0 request payload.
    pub fn format_request(id: u64, method: &str, params: Option<Value>) -> Value {
        let mut req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
        });
        if let Some(p) = params {
            req["params"] = p;
        }
        req
    }

    /// Parse a single response line, checking if it matches the expected request ID.
    /// Returns:
    /// - `Ok(Some(result))` if response matches `expected_id` and is successful
    /// - `Ok(None)` if line is non-JSON, a notification, or matches a different ID
    /// - `Err(...)` if response matches `expected_id` and contains a JSON-RPC error
    pub fn parse_response_line(line: &str, expected_id: u64) -> Result<Option<Value>> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }

        let resp: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => {
                // Ignore non-JSON stdout lines (e.g. child logging)
                return Ok(None);
            }
        };

        if let Some(resp_id) = resp.get("id") {
            let is_match = match resp_id {
                Value::Number(n) => n.as_u64() == Some(expected_id),
                Value::String(s) => s == &expected_id.to_string(),
                _ => false,
            };

            if is_match {
                if let Some(err) = resp.get("error") {
                    if !err.is_null() {
                        let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
                        let msg = err
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("Unknown MCP error");
                        anyhow::bail!("MCP server error ({}): {}", code, msg);
                    }
                }

                if let Some(result) = resp.get("result") {
                    return Ok(Some(result.clone()));
                }
                return Ok(Some(Value::Null));
            }
        }

        Ok(None)
    }

    /// Send a JSON-RPC request and wait for the response matching its ID.
    pub async fn send_request(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;

        let req = Self::format_request(id, method, params);
        let mut line = serde_json::to_string(&req)?;
        line.push('\n');

        self.stdin
            .write_all(line.as_bytes())
            .await
            .with_context(|| format!("Failed to write request to MCP server '{}'", self.server_name))?;
        self.stdin
            .flush()
            .await
            .with_context(|| format!("Failed to flush request to MCP server '{}'", self.server_name))?;

        while let Some(line) = self
            .reader
            .next_line()
            .await
            .with_context(|| format!("Failed to read response line from MCP server '{}'", self.server_name))?
        {
            if let Some(result) = Self::parse_response_line(&line, id)? {
                return Ok(result);
            }
        }

        anyhow::bail!(
            "MCP server '{}' closed connection before responding to request ID {}",
            self.server_name,
            id
        )
    }

    /// Send a JSON-RPC notification (no ID, no response expected).
    pub async fn send_notification(&mut self, method: &str, params: Option<Value>) -> Result<()> {
        let mut notif = json!({
            "jsonrpc": "2.0",
            "method": method,
        });
        if let Some(p) = params {
            notif["params"] = p;
        }
        let mut line = serde_json::to_string(&notif)?;
        line.push('\n');

        self.stdin
            .write_all(line.as_bytes())
            .await
            .with_context(|| format!("Failed to write notification to MCP server '{}'", self.server_name))?;
        self.stdin
            .flush()
            .await
            .with_context(|| format!("Failed to flush notification to MCP server '{}'", self.server_name))?;

        Ok(())
    }

    /// Perform protocol initialization handshake.
    pub async fn initialize(&mut self) -> Result<Value> {
        let params = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "clientInfo": {
                "name": "atlas",
                "version": env!("CARGO_PKG_VERSION")
            }
        });
        let result = self.send_request("initialize", Some(params)).await?;
        // Send notifications/initialized per MCP 2024-11-05 specification
        let _ = self.send_notification("notifications/initialized", Some(json!({}))).await;
        Ok(result)
    }

    /// Query available tools from the upstream MCP server.
    pub async fn list_tools(&mut self) -> Result<Vec<Value>> {
        let result = self.send_request("tools/list", None).await?;
        let tools = if let Some(arr) = result.get("tools").and_then(|t| t.as_array()) {
            arr.clone()
        } else if let Some(arr) = result.as_array() {
            arr.clone()
        } else {
            Vec::new()
        };
        Ok(tools)
    }

    /// Execute a tool on the upstream MCP server.
    pub async fn call_tool(&mut self, tool_name: &str, arguments: Value) -> Result<Value> {
        let params = json!({
            "name": tool_name,
            "arguments": arguments,
        });
        self.send_request("tools/call", Some(params)).await
    }

    /// Gracefully terminate the child process.
    pub async fn close(&mut self) -> Result<()> {
        let _ = self.child.kill().await;
        Ok(())
    }
}
