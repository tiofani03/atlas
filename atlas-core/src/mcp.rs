use crate::domain::KnowledgeArtifact;
use crate::storage::Storage;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

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

pub async fn run_stdio_mcp_server(storage: Storage) -> Result<()> {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin).lines();

    while let Some(line) = reader.next_line().await? {
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
        let resp = match handle_request(&storage, &req.method, req.params).await {
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
        stdout.write_all(&bytes).await?;
        stdout.flush().await?;
    }

    Ok(())
}

async fn handle_request(
    storage: &Storage,
    method: &str,
    params: Option<Value>,
) -> Result<Value> {
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
        "tools/list" => Ok(json!({
            "tools": [
                {
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
                },
                {
                    "name": "atx_artifact",
                    "description": "Get detailed canonical information for a specific artifact by ID or source_id",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string", "description": "Artifact ID or source_id (e.g., octocat/hello-world#42 or commit SHA)" }
                        },
                        "required": ["id"]
                    }
                },
                {
                    "name": "atx_related",
                    "description": "Get connected engineering graph artifacts for a given artifact ID or source_id",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string", "description": "Artifact ID or source_id" }
                        },
                        "required": ["id"]
                    }
                },
                {
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
                },
                {
                    "name": "atx_status",
                    "description": "Get current status and statistics of local Atlas context graph",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                }
            ]
        })),
        "tools/call" => {
            let params = params.unwrap_or(Value::Null);
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(Value::Null);

            match name {
                "atx_search" | "atlas_search" | "atlas_query" => {
                    let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                    let kind = args
                        .get("kind")
                        .or_else(|| args.get("object_type"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let tag = args.get("tag").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let repository = args.get("repository").and_then(|v| v.as_str()).map(|s| s.to_string());
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
                    let id_param = args.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let storage_clone = storage.clone();

                    let artifact = tokio::task::spawn_blocking(move || {
                        storage_clone.get_artifact_by_id(&id_param)
                    })
                    .await??;

                    let text = match artifact {
                        Some(art) => format_results_as_markdown(&[art]),
                        None => format!("Artifact with ID '{}' not found.", args.get("id").and_then(|v| v.as_str()).unwrap_or("")),
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
                    let id_param = args.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let storage_clone = storage.clone();

                    let related = tokio::task::spawn_blocking(move || {
                        storage_clone.get_related_artifacts(&id_param)
                    })
                    .await??;

                    let mut out = String::new();
                    out.push_str(&format!("Found {} related artifact(s):\n\n", related.len()));
                    for (rel, art) in related {
                        out.push_str(&format!("- [{}] -> [{}] {}\n  (ID: {})\n", rel.relationship_type, art.kind.to_string().to_uppercase(), art.title, art.source_id));
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
                "atx_context" | "atlas_context" => {
                    let id_param = args.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let kind_param = args.get("kind").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let storage_clone = storage.clone();

                    let pkg = tokio::task::spawn_blocking(move || {
                        let builder = crate::context::ContextBuilder::new(&storage_clone);
                        let options = crate::context::ContextOptions::default();
                        builder.build(kind_param.as_deref(), &id_param, &options)
                    })
                    .await??;

                    let json_str = serde_json::to_string_pretty(&pkg)?;

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
                    let stats = tokio::task::spawn_blocking(move || storage_clone.get_stats()).await??;

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
        _ => anyhow::bail!("Unsupported MCP method: {}", method),
    }
}

fn format_results_as_markdown(artifacts: &[KnowledgeArtifact]) -> String {
    if artifacts.is_empty() {
        return "No matching engineering artifacts found.".to_string();
    }

    let mut out = String::new();
    out.push_str(&format!("Found {} matching artifact(s):\n\n", artifacts.len()));

    for (idx, art) in artifacts.iter().enumerate() {
        out.push_str(&format!(
            "### {}. [{}] {}\n",
            idx + 1,
            art.kind.to_string().to_uppercase(),
            art.title
        ));
        out.push_str(&format!("- **ID**: `{}`\n", art.id));
        out.push_str(&format!("- **Source**: [{}]({})\n", art.source_id, art.source_url));
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
        let body_snippet = if art.body.len() > 500 {
            format!("{}...", &art.body[..500])
        } else {
            art.body.clone()
        };
        out.push_str(&body_snippet);
        out.push_str("\n\n---\n\n");
    }

    out
}

