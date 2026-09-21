#!/usr/bin/env python3
import sys, json

def send_response(resp):
    sys.stdout.write(json.dumps(resp) + "\n")
    sys.stdout.flush()

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

    if method == "initialize":
        send_response({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "mock-figma-mcp",
                    "version": "1.0.0"
                }
            }
        })
    elif method == "notifications/initialized":
        pass
    elif method == "tools/list":
        send_response({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "tools": [
                    {
                        "name": "get_file",
                        "description": "Fetch a Figma file document by key",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "file_key": {"type": "string", "description": "Figma file key"}
                            },
                            "required": ["file_key"]
                        }
                    },
                    {
                        "name": "get_comments",
                        "description": "Get comments from a Figma design file",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "file_key": {"type": "string", "description": "Figma file key"}
                            },
                            "required": ["file_key"]
                        }
                    },
                    {
                        "name": "get_image",
                        "description": "Render and download images of frames or components from Figma",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "file_key": {"type": "string"},
                                "ids": {"type": "array", "items": {"type": "string"}}
                            },
                            "required": ["file_key", "ids"]
                        }
                    }
                ]
            }
        })
    elif method == "tools/call":
        params = req.get("params", {})
        name = params.get("name")
        args = params.get("arguments", {})
        send_response({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": f"Mock Figma MCP successfully executed tool '{name}' with args {json.dumps(args)}"
                    }
                ]
            }
        })
    elif req_id is not None:
        send_response({
            "jsonrpc": "2.0",
            "id": req_id,
            "error": {
                "code": -32601,
                "message": f"Method '{method}' not found"
            }
        })
