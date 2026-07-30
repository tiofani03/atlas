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
                    "description": "Perform full-text BM25 search across unified engineering knowledge with optional filters",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": { "type": "string", "description": "Search query terms" },
                            "object_type": { "type": "string", "description": "Optional type filter: ticket, document, specification" },
                            "tag": { "type": "string", "description": "Optional tag filter" },
                            "limit": { "type": "integer", "description": "Max results to return (default 10)" }
                        }
                    }
                },
                {
                    "name": "atx_status",
                    "description": "Get current status and statistics of local Atlas database",
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
                    let object_type = args.get("object_type").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let tag = args.get("tag").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

                    let storage_clone = storage.clone();
                    let query_owned = query.to_string();

                    let results = tokio::task::spawn_blocking(move || {
                        if !query_owned.is_empty() {
                            storage_clone.search_fts(&query_owned, limit)
                        } else {
                            storage_clone.query_structured(object_type.as_deref(), tag.as_deref(), limit)
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
                "atx_status" | "atlas_status" => {
                    let storage_clone = storage.clone();
                    let stats = tokio::task::spawn_blocking(move || storage_clone.get_stats()).await??;

                    let summary = format!(
                        "### Atlas Storage Status\n- **Total Knowledge Objects**: {}\n- **Connectors Synced**: {}\n- **Database File Size**: {:.2} MB",
                        stats.total_objects,
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

fn format_results_as_markdown(objects: &[crate::domain::KnowledgeObject]) -> String {
    if objects.is_empty() {
        return "No matching engineering knowledge objects found.".to_string();
    }

    let mut out = String::new();
    out.push_str(&format!("Found {} matching object(s):\n\n", objects.len()));

    for (idx, obj) in objects.iter().enumerate() {
        out.push_str(&format!(
            "### {}. [{}] {}\n",
            idx + 1,
            obj.object_type.to_string().to_uppercase(),
            obj.title
        ));
        out.push_str(&format!("- **ID**: `{}`\n", obj.id));
        out.push_str(&format!("- **Source**: [{}]({})\n", obj.source.original_id, obj.source.web_url));
        if let Some(ref sum) = obj.summary {
            out.push_str(&format!("- **Summary**: {}\n", sum));
        }
        if !obj.tags.is_empty() {
            out.push_str(&format!("- **Tags**: {}\n", obj.tags.join(", ")));
        }
        out.push_str("\n**Content**:\n");
        let content_snippet = if obj.content.len() > 500 {
            format!("{}...", &obj.content[..500])
        } else {
            obj.content.clone()
        };
        out.push_str(&content_snippet);
        out.push_str("\n\n---\n\n");
    }

    out
}
