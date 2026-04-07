use crate::index::SymbolStore;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Minimal MCP (Model Context Protocol) server over stdio.
/// Implements just enough of the protocol: initialize, tools/list, tools/call.
pub async fn run_mcp_server<S: SymbolStore + 'static>(
    store: Arc<S>,
    indexed: Arc<AtomicBool>,
    workspace: String,
) -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break; // EOF
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "error": {"code": -32700, "message": format!("Parse error: {}", e)},
                    "id": null
                });
                write_response(&mut stdout, &err_resp).await?;
                continue;
            }
        };

        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("");
        let params = request.get("params").cloned().unwrap_or(json!({}));

        let response = match method {
            "initialize" => handle_initialize(&id),
            "initialized" => continue, // notification, no response
            "tools/list" => handle_tools_list(&id),
            "tools/call" => handle_tools_call(&id, &params, &store, &indexed, &workspace).await,
            _ => json!({
                "jsonrpc": "2.0",
                "error": {"code": -32601, "message": format!("Method not found: {}", method)},
                "id": id
            }),
        };

        write_response(&mut stdout, &response).await?;
    }

    Ok(())
}

fn handle_initialize(id: &Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "gpsource",
                "version": env!("CARGO_PKG_VERSION")
            }
        },
        "id": id
    })
}

fn handle_tools_list(id: &Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "result": {
            "tools": [
                {
                    "name": "gps_status",
                    "description": "Check if the gpsource indexing engine is running and whether initial indexation is complete. Call this first at the start of any session before using other GPS tools.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "gps_locate",
                    "description": "Find the exact location of a symbol (function, class, struct, interface, trait, enum, method) in the indexed workspace. Returns file path, line, column, kind, and qualified name. Use this instead of grep/ripgrep when looking for definitions — it is faster and structurally aware.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "symbol_name": {
                                "type": "string",
                                "description": "The name or partial name of the symbol to locate. Prefix matching is used."
                            }
                        },
                        "required": ["symbol_name"]
                    }
                },
                {
                    "name": "gps_neighborhood",
                    "description": "Get the dependency neighborhood of a file: its imports, which files import it, and symbols in related files. Use this to understand how a file fits into the codebase architecture before making changes.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "file_path": {
                                "type": "string",
                                "description": "The workspace-relative path of the file to analyze."
                            },
                            "depth": {
                                "type": "integer",
                                "description": "How many hops to traverse in the dependency graph (default: 1)."
                            }
                        },
                        "required": ["file_path"]
                    }
                },
                {
                    "name": "gps_summary",
                    "description": "Get a condensed workspace map: total file count, files by language, module structure, and public API surface. Use this to orient yourself in an unfamiliar codebase before diving into specific files.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                }
            ]
        },
        "id": id
    })
}

async fn handle_tools_call<S: SymbolStore>(
    id: &Value,
    params: &Value,
    store: &Arc<S>,
    indexed: &Arc<AtomicBool>,
    workspace: &str,
) -> Value {
    let tool_name = params
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("");
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    let result = match tool_name {
        "gps_status" => tool_status(indexed, workspace),
        "gps_locate" => tool_locate(store, &arguments),
        "gps_neighborhood" => tool_neighborhood(store, &arguments),
        "gps_summary" => tool_summary(),
        _ => Err(format!("Unknown tool: {}", tool_name)),
    };

    match result {
        Ok(content) => json!({
            "jsonrpc": "2.0",
            "result": {
                "content": [{"type": "text", "text": content}]
            },
            "id": id
        }),
        Err(e) => json!({
            "jsonrpc": "2.0",
            "result": {
                "content": [{"type": "text", "text": e}],
                "isError": true
            },
            "id": id
        }),
    }
}

fn tool_status(indexed: &Arc<AtomicBool>, workspace: &str) -> Result<String, String> {
    let status = if indexed.load(Ordering::Relaxed) {
        "ready"
    } else {
        "indexing"
    };
    Ok(serde_json::to_string_pretty(&json!({
        "status": status,
        "indexed": indexed.load(Ordering::Relaxed),
        "workspace": workspace,
        "pid": std::process::id()
    }))
    .unwrap())
}

fn tool_locate<S: SymbolStore>(store: &Arc<S>, args: &Value) -> Result<String, String> {
    let symbol_name = args
        .get("symbol_name")
        .and_then(|s| s.as_str())
        .ok_or("Missing required parameter: symbol_name")?;

    let records = store
        .locate(symbol_name)
        .map_err(|e| format!("Locate error: {}", e))?;

    if records.is_empty() {
        return Ok(format!("No symbols found matching '{}'", symbol_name));
    }

    let results: Vec<Value> = records
        .into_iter()
        .map(|r| {
            json!({
                "file": r.file,
                "line": r.start_line,
                "col": r.start_col,
                "kind": r.kind
            })
        })
        .collect();

    Ok(serde_json::to_string_pretty(&results).unwrap())
}

fn tool_neighborhood<S: SymbolStore>(store: &Arc<S>, args: &Value) -> Result<String, String> {
    let file_path = args
        .get("file_path")
        .and_then(|s| s.as_str())
        .ok_or("Missing required parameter: file_path")?;

    let imports = store
        .get_imports(file_path)
        .map_err(|e| format!("Error: {}", e))?;
    let imported_by = store
        .get_importers(file_path)
        .map_err(|e| format!("Error: {}", e))?;

    Ok(serde_json::to_string_pretty(&json!({
        "file": file_path,
        "imports": imports,
        "imported_by": imported_by
    }))
    .unwrap())
}

fn tool_summary() -> Result<String, String> {
    Ok(serde_json::to_string_pretty(&json!({
        "note": "Workspace summary - use the HTTP API for the full computed version"
    }))
    .unwrap())
}

async fn write_response(
    stdout: &mut tokio::io::Stdout,
    response: &Value,
) -> anyhow::Result<()> {
    let mut output = serde_json::to_string(response)?;
    output.push('\n');
    stdout.write_all(output.as_bytes()).await?;
    stdout.flush().await?;
    Ok(())
}
