use crate::config::Config;
use crate::domain::KnowledgeArtifact;
use crate::mcp::client::McpClient;
use crate::mcp::resolver::{
    parse_figma_target, resolve_figma_target_with_candidates, FigmaFileCandidate,
};
use crate::storage::Storage;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

/// Represents a connected upstream MCP server managed by McpHub.
pub struct UpstreamServer {
    pub name: String,
    pub prefix: String,
    pub client: Arc<Mutex<McpClient>>,
    pub aliases: std::collections::HashMap<String, String>,
}

/// McpHub aggregates native Atlas tools and upstream external MCP servers.
pub struct McpHub {
    storage: Storage,
    servers: Vec<UpstreamServer>,
    figma_aliases: HashMap<String, String>,
}

impl From<Storage> for McpHub {
    fn from(storage: Storage) -> Self {
        Self::new(storage)
    }
}

impl McpHub {
    /// Create a new McpHub wrapping the local storage with no upstream servers.
    pub fn new(storage: Storage) -> Self {
        Self {
            storage,
            servers: Vec::new(),
            figma_aliases: HashMap::new(),
        }
    }

    /// Access reference to underlying Storage.
    pub fn storage(&self) -> &Storage {
        &self.storage
    }

    /// Access reference to registered upstream servers.
    pub fn servers(&self) -> &[UpstreamServer] {
        &self.servers
    }

    /// Register an upstream McpClient with custom aliases.
    pub fn add_server_with_aliases(
        &mut self,
        name: impl Into<String>,
        prefix: Option<String>,
        aliases: std::collections::HashMap<String, String>,
        client: McpClient,
    ) {
        let name = name.into();
        let prefix = prefix
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| name.clone());

        self.servers.push(UpstreamServer {
            name,
            prefix,
            client: Arc::new(Mutex::new(client)),
            aliases,
        });
    }

    /// Register an upstream McpClient.
    /// If prefix is None or empty, defaults to server name.
    pub fn add_server(
        &mut self,
        name: impl Into<String>,
        prefix: Option<String>,
        client: McpClient,
    ) {
        self.add_server_with_aliases(name, prefix, std::collections::HashMap::new(), client);
    }

    /// Alias for add_server.
    pub fn add_client(
        &mut self,
        name: impl Into<String>,
        prefix: Option<String>,
        client: McpClient,
    ) {
        self.add_server(name, prefix, client);
    }

    /// Discover enabled MCP servers from Config and initialize McpHub.
    /// Servers that fail to spawn or initialize log a warning rather than failing the entire hub.
    pub async fn from_config(config: &Config, storage: Storage) -> Result<Self> {
        let mut hub = Self::new(storage);

        if let Some((_, figma_server)) = config.mcp_servers.iter().find(|(name, server)| {
            name.eq_ignore_ascii_case("figma")
                || server.prefix.as_deref().is_some_and(|prefix| prefix.eq_ignore_ascii_case("figma"))
        }) {
            hub.figma_aliases = figma_server.aliases.clone();
        }

        let mut server_names: Vec<&String> = config.mcp_servers.keys().collect();
        server_names.sort();

        for server_name in server_names {
            let server_cfg = &config.mcp_servers[server_name];
            if server_cfg.enabled == Some(false) {
                tracing::debug!(server = %server_name, "MCP server disabled in config, skipping");
                continue;
            }

            let prefix = server_cfg
                .prefix
                .as_deref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .unwrap_or(server_name)
                .to_string();

            match McpClient::start(server_name, server_cfg).await {
                Ok(mut client) => {
                    if let Err(e) = client.initialize().await {
                        tracing::warn!(
                            server = %server_name,
                            "Failed to initialize MCP server handshake: {}",
                            e
                        );
                        continue;
                    }
                    hub.add_server_with_aliases(
                        server_name.clone(),
                        Some(prefix),
                        server_cfg.aliases.clone(),
                        client,
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        server = %server_name,
                        "Failed to start upstream MCP server: {}",
                        e
                    );
                }
            }
        }

        Ok(hub)
    }

    /// Returns list of all native Atlas MCP tools.
    pub fn native_tools() -> Vec<Value> {
        vec![
            json!({
                "name": "atx_search",
                "description": "Perform full-text BM25 search across unified engineering context graph with optional filters",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search query terms" },
                        "kind": { "type": "string", "description": "Optional artifact kind filter (e.g., repository, issue, pull_request, commit, release, ticket, document)" },
                        "tag": { "type": "string", "description": "Optional tag filter" },
                        "repository": { "type": "string", "description": "Optional repository filter (e.g. owner/repo)" },
                        "limit": { "type": "integer", "description": "Max results to return (default 10)" }
                    }
                }
            }),
            json!({
                "name": "atx_artifact",
                "description": "Get detailed canonical information for a specific artifact by ID or source_id",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Artifact ID or source_id (e.g., octocat/hello-world#42 or commit SHA)" }
                    },
                    "required": ["id"]
                }
            }),
            json!({
                "name": "atx_related",
                "description": "Get connected engineering graph artifacts for a given artifact ID or source_id",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Artifact ID or source_id" }
                    },
                    "required": ["id"]
                }
            }),
            json!({
                "name": "atx_context",
                "description": "Build concise, deterministic, AI-ready engineering context for an issue, PR, repository, ADR, or artifact ID",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Artifact ID, source_id, repository name, or ADR ID (e.g., PAY-123, 456, payment-service, ADR-001)" },
                        "kind": { "type": "string", "description": "Optional context target kind (e.g., issue, pr, repository, adr)" }
                    },
                    "required": ["id"]
                }
            }),
            json!({
                "name": "atx_status",
                "description": "Get current status and statistics of local Atlas context graph",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }),
            json!({
                "name": "atx_explain",
                "description": "Explain relationship graph context for an artifact",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Artifact ID or source_id" }
                    },
                    "required": ["id"]
                }
            }),
        ]
    }

    /// Query and aggregate tools from both native storage and all upstream MCP servers.
    /// Upstream tools are prefixed with `<prefix>__<tool_name>` and labeled with `[Server: <name>]`.
    pub async fn get_aggregated_tools(&self) -> Result<Vec<Value>> {
        let mut tools = Self::native_tools();

        for server in &self.servers {
            let list_res = {
                let mut client = server.client.lock().await;
                client.list_tools().await
            };

            match list_res {
                Ok(upstream_tools) => {
                    for tool in upstream_tools {
                        let mut aggregated = tool.clone();
                        if let Some(orig_name) = tool.get("name").and_then(|v| v.as_str()) {
                            let prefixed_name = format!("{}__", server.prefix) + orig_name;
                            aggregated["name"] = json!(prefixed_name);
                        }
                        let orig_desc = tool
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let new_desc = if orig_desc.is_empty() {
                            format!("[Server: {}]", server.name)
                        } else if orig_desc.starts_with(&format!("[Server: {}]", server.name)) {
                            orig_desc.to_string()
                        } else {
                            format!("[Server: {}] {}", server.name, orig_desc)
                        };
                        aggregated["description"] = json!(new_desc);
                        tools.push(aggregated);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        server = %server.name,
                        "Failed to list tools from MCP server: {}",
                        e
                    );
                }
            }
        }

        Ok(tools)
    }

    /// Route a tool invocation to either local storage or the matching upstream MCP server.
    pub async fn route_tool_call(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        if is_native_tool(tool_name) {
            return handle_native_tool_with_figma_context(
                &self.storage,
                tool_name,
                arguments,
                &self.figma_aliases,
            )
            .await;
        }

        let mut sorted_servers: Vec<&UpstreamServer> = self.servers.iter().collect();
        sorted_servers.sort_by_key(|b| std::cmp::Reverse(b.prefix.len()));

        for server in sorted_servers {
            let marker = format!("{}__", server.prefix);
            if let Some(orig_name) = tool_name.strip_prefix(&marker) {
                let mut final_arguments = arguments.clone();
                if server.name == "figma"
                    || !server.aliases.is_empty()
                    || orig_name.contains("figma")
                    || orig_name.contains("file")
                {
                    let figma_candidates =
                        figma_candidates_for_arguments(&self.storage, &final_arguments).await;
                    sanitize_and_resolve_arguments(
                        &server.name,
                        &server.aliases,
                        &mut final_arguments,
                        &figma_candidates,
                    );
                }
                let mut client = server.client.lock().await;
                return client.call_tool(orig_name, final_arguments).await;
            }
        }

        anyhow::bail!("Unknown tool: {}", tool_name)
    }

    /// Handle a single incoming JSON-RPC method request.
    pub async fn handle_request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        match method {
            "initialize" => Ok(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "atlas-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
            "tools/list" => {
                let tools = self.get_aggregated_tools().await?;
                Ok(json!({ "tools": tools }))
            }
            "tools/call" => {
                let params = params.unwrap_or(Value::Null);
                let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(Value::Null);
                self.route_tool_call(name, args).await
            }
            _ => anyhow::bail!("Unsupported MCP method: {}", method),
        }
    }

    /// Run the JSON-RPC server loop reading from `reader` and writing to `writer`.
    pub async fn run_server<R, W>(&self, reader: R, mut writer: W) -> Result<()>
    where
        R: tokio::io::AsyncRead + Unpin,
        W: tokio::io::AsyncWrite + Unpin,
    {
        let mut lines = BufReader::new(reader).lines();

        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }

            let req: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(_) => continue,
            };

            if req.id.is_none() {
                continue;
            }

            let id = req.id.unwrap_or(Value::Null);
            let resp = match self.handle_request(&req.method, req.params).await {
                Ok(result) => JsonRpcResponse {
                    jsonrpc: "2.0",
                    id,
                    result: Some(result),
                    error: None,
                },
                Err(err) => JsonRpcResponse {
                    jsonrpc: "2.0",
                    id,
                    result: None,
                    error: Some(json!({
                        "code": -32603,
                        "message": err.to_string()
                    })),
                },
            };

            let mut bytes = serde_json::to_vec(&resp)?;
            bytes.push(b'\n');
            writer.write_all(&bytes).await?;
            writer.flush().await?;
        }

        Ok(())
    }

    /// Run stdio MCP server loop.
    pub async fn run_stdio_mcp_server(hub: impl Into<McpHub>) -> Result<()> {
        run_stdio_mcp_server(hub).await
    }
}

/// Run the stdio JSON-RPC loop using the aggregated tools from McpHub.
pub async fn run_stdio_mcp_server(hub: impl Into<McpHub>) -> Result<()> {
    let hub = hub.into();
    hub.run_server(tokio::io::stdin(), tokio::io::stdout())
        .await
}

fn is_native_tool(name: &str) -> bool {
    matches!(
        name,
        "atx_search"
            | "atlas_search"
            | "atlas_query"
            | "atx_artifact"
            | "atx_related"
            | "atx_context"
            | "atlas_context"
            | "atx_status"
            | "atlas_status"
            | "atx_explain"
            | "atlas_explain"
    )
}

pub async fn handle_native_tool(
    storage: &Storage,
    name: &str,
    args: Value,
) -> Result<Value> {
    handle_native_tool_with_figma_context(storage, name, args, &HashMap::new()).await
}

pub async fn handle_native_tool_with_figma_context(
    storage: &Storage,
    name: &str,
    args: Value,
    figma_aliases: &HashMap<String, String>,
) -> Result<Value> {
    match name {
        "atx_search" | "atlas_search" | "atlas_query" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let kind = args
                .get("kind")
                .or_else(|| args.get("object_type"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let tag = args.get("tag").and_then(|v| v.as_str()).map(|s| s.to_string());
            let repository = args
                .get("repository")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

            let storage_clone = storage.clone();
            let query_owned = query.to_string();

            let results = tokio::task::spawn_blocking(move || {
                if !query_owned.is_empty() {
                    storage_clone.search_fts(
                        &query_owned,
                        kind.as_deref(),
                        tag.as_deref(),
                        repository.as_deref(),
                        limit,
                    )
                } else {
                    storage_clone.query_structured(
                        kind.as_deref(),
                        tag.as_deref(),
                        repository.as_deref(),
                        limit,
                    )
                }
            })
            .await??;

            let formatted = format_results_as_markdown(&results);

            Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": formatted
                    }
                ]
            }))
        }
        "atx_artifact" => {
            let id_param = args
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let storage_clone = storage.clone();

            let artifact = tokio::task::spawn_blocking(move || {
                storage_clone.get_artifact_by_id(&id_param)
            })
            .await??;

            let text = match artifact {
                Some(art) => format_results_as_markdown(&[art]),
                None => format!(
                    "Artifact with ID '{}' not found.",
                    args.get("id").and_then(|v| v.as_str()).unwrap_or("")
                ),
            };

            Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": text
                    }
                ]
            }))
        }
        "atx_related" => {
            let id_param = args
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let storage_clone = storage.clone();

            let related = tokio::task::spawn_blocking(move || {
                storage_clone.get_related_artifacts(&id_param)
            })
            .await??;

            let mut out = String::new();
            out.push_str(&format!("Found {} related artifact(s):\n\n", related.len()));
            for (rel, art) in related {
                out.push_str(&format!(
                    "- [{}] -> [{}] {}\n  (ID: {})\n",
                    rel.relationship_type,
                    art.kind.to_string().to_uppercase(),
                    art.title,
                    art.source_id
                ));
            }

            Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": out
                    }
                ]
            }))
        }
        "atx_context" | "atlas_context" | "atx_explain" | "atlas_explain" => {
            let id_param = args
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let kind_param = args
                .get("kind")
                .and_then(|v| v.as_str()).map(|s| s.to_string());
            let storage_clone = storage.clone();
            let lookup_id = id_param.clone();
            let context_id = id_param.clone();

            let (pkg, figma_candidates) = tokio::task::spawn_blocking(move || {
                let builder = crate::context::ContextBuilder::new(&storage_clone);
                let options = crate::context::ContextOptions::default();
                let pkg = builder.build(kind_param.as_deref(), &context_id, &options)?;
                let candidates = storage_clone
                    .search_fts_paginated(
                        &lookup_id,
                        None,
                        Some("figma"),
                        None,
                        None,
                        50,
                        0,
                    )
                    .map(|(items, _)| {
                        items
                            .into_iter()
                            .filter_map(figma_artifact_candidate)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                Ok::<_, anyhow::Error>((pkg, candidates))
            })
            .await??;

            let (resolved_key, resolved_node) = resolve_figma_target_with_candidates(
                &id_param,
                figma_aliases,
                None,
                None,
                &figma_candidates,
            );
            let mut context_value = serde_json::to_value(&pkg)?;
            if resolved_key != id_param || resolved_node.is_some() {
                context_value["figma"] = json!({
                    "fileKey": resolved_key,
                    "nodeId": resolved_node,
                    "url": format!("https://www.figma.com/design/{}", resolved_key),
                    "status": "resolved"
                });
            }
            let json_str = serde_json::to_string_pretty(&context_value)?;

            Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": json_str
                    }
                ]
            }))
        }
        "atx_status" | "atlas_status" => {
            let storage_clone = storage.clone();
            let stats =
                tokio::task::spawn_blocking(move || storage_clone.get_stats()).await??;

            let summary = format!(
                "### Atlas Engineering Context Graph Status\n- **Total Artifacts**: {}\n- **Connectors Synced**: {}\n- **Database File Size**: {:.2} MB",
                stats.total_artifacts,
                stats.connectors_count,
                stats.db_size_bytes as f64 / (1024.0 * 1024.0)
            );

            Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": summary
                    }
                ]
            }))
        }
        _ => anyhow::bail!("Unknown tool: {}", name),
    }
}

fn figma_artifact_candidate(artifact: KnowledgeArtifact) -> Option<FigmaFileCandidate> {
    if !artifact.provider.eq_ignore_ascii_case("figma") {
        return None;
    }

    let (url_key, url_node) = parse_figma_target(&artifact.source_url);
    let (file_key, source_node) = if !url_key.is_empty() {
        (url_key, url_node)
    } else if let Some(raw_key) = artifact.source_id.strip_prefix("file:") {
        (raw_key.to_string(), None)
    } else {
        parse_figma_target(&artifact.source_id)
    };

    if file_key.is_empty() {
        return None;
    }

    Some(FigmaFileCandidate {
        file_key,
        name: artifact.title,
        body: artifact.body,
        node_id: source_node,
    })
}

fn format_results_as_markdown(artifacts: &[KnowledgeArtifact]) -> String {
    if artifacts.is_empty() {
        return "No matching engineering artifacts found.".to_string();
    }

    let mut out = String::new();
    out.push_str(&format!(
        "Found {} matching artifact(s):\n\n",
        artifacts.len()
    ));

    for (idx, art) in artifacts.iter().enumerate() {
        out.push_str(&format!(
            "### {}. [{}] {}\n",
            idx + 1,
            art.kind.to_string().to_uppercase(),
            art.title
        ));
        out.push_str(&format!("- **ID**: `{}`\n", art.id));
        out.push_str(&format!(
            "- **Source**: [{}]({})\n",
            art.source_id, art.source_url
        ));
        if let Some(ref repo) = art.repository {
            out.push_str(&format!("- **Repository**: {}\n", repo));
        }
        if let Some(ref sum) = art.summary {
            out.push_str(&format!("- **Summary**: {}\n", sum));
        }
        if !art.tags.is_empty() {
            out.push_str(&format!("- **Tags**: {}\n", art.tags.join(", ")));
        }
        if !art.relationships.is_empty() {
            let rel_strs: Vec<String> = art
                .relationships
                .iter()
                .map(|r| format!("{} {}", r.relationship_type, r.target_id))
                .collect();
            out.push_str(&format!("- **Relationships**: {}\n", rel_strs.join("; ")));
        }
        out.push_str("\n**Body**:\n");
        let body_snippet = if art.body.chars().count() > 500 {
            format!("{}...", art.body.chars().take(500).collect::<String>())
        } else {
            art.body.clone()
        };
        out.push_str(&body_snippet);
        out.push_str("\n\n---\n\n");
    }

    out
}

async fn figma_candidates_for_arguments(
    storage: &Storage,
    arguments: &Value,
) -> Vec<FigmaFileCandidate> {
    let lookup = ["fileKey", "file_key", "key"]
        .iter()
        .find_map(|key| arguments.get(*key).and_then(Value::as_str))
        .map(|value| parse_figma_target(value).0)
        .filter(|key| !key.is_empty() && !key.contains("figma.com"));

    let Some(lookup) = lookup else {
        return Vec::new();
    };

    let storage = storage.clone();
    tokio::task::spawn_blocking(move || {
        storage
            .search_fts_paginated(&lookup, None, Some("figma"), None, None, 50, 0)
            .map(|(items, _)| items.into_iter().filter_map(figma_artifact_candidate).collect())
            .unwrap_or_default()
    })
    .await
    .unwrap_or_default()
}

fn sanitize_and_resolve_arguments(
    _server_name: &str,
    aliases: &std::collections::HashMap<String, String>,
    arguments: &mut Value,
    candidates: &[FigmaFileCandidate],
) {
    if let Some(obj) = arguments.as_object_mut() {
        let mut extracted_node = None;

        for key in ["fileKey", "file_key", "key"] {
            if let Some(val) = obj.get(key).and_then(|v| v.as_str()).map(|s| s.to_string()) {
                let (resolved_key, node_id) =
                    resolve_figma_target_with_candidates(&val, aliases, None, None, candidates);
                obj.insert(key.to_string(), json!(resolved_key));
                if node_id.is_some() {
                    extracted_node = node_id;
                }
            }
        }

        if let Some(node) = extracted_node {
            if !obj.contains_key("node_id") && !obj.contains_key("nodeId") && !obj.contains_key("nodeIds") {
                obj.insert("node_id".to_string(), json!(node));
            }
        }
    }
}
