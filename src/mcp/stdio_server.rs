use crate::index::SymbolStore;
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Maximum number of matching lines to return per file in search results.
const MAX_MATCHES_PER_FILE: usize = 5;

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
        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
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
                },
                {
                    "name": "gps_tree",
                    "description": "Get the full file tree of the indexed workspace with language, line count, and symbol count per file. Use this to understand the project structure and find relevant files.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "gps_read",
                    "description": "Read the content of a source file, optionally restricted to a line range. Sensitive files (.env, credentials, keys) are blocked. Use this to inspect code without opening files in an editor.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Absolute or workspace-relative path to the file to read."
                            },
                            "start_line": {
                                "type": "integer",
                                "description": "First line to return (1-based, default: 1)."
                            },
                            "end_line": {
                                "type": "integer",
                                "description": "Last line to return (1-based, default: last line)."
                            }
                        },
                        "required": ["path"]
                    }
                },
                {
                    "name": "gps_hot",
                    "description": "Get the most recently modified files in the workspace. Use this to understand what's actively being worked on.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "limit": {
                                "type": "integer",
                                "description": "Maximum number of files to return (default: 20)."
                            }
                        }
                    }
                },
                {
                    "name": "gps_search",
                    "description": "Full-text search across the indexed workspace using trigram-accelerated matching. Returns matching files with line numbers and content. Much faster than grep for pre-indexed codebases.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "The text to search for (case-insensitive)."
                            },
                            "max_results": {
                                "type": "integer",
                                "description": "Maximum number of files to return (default: 20)."
                            }
                        },
                        "required": ["query"]
                    }
                },
                {
                    "name": "gps_word",
                    "description": "O(1) exact word lookup in the inverted index. Find all files and line numbers where a specific identifier (function name, class name, variable) is defined. Faster than search for exact symbol names.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "word": {
                                "type": "string",
                                "description": "The exact identifier to look up."
                            }
                        },
                        "required": ["word"]
                    }
                },
                {
                    "name": "gps_changes",
                    "description": "Get files that changed since a given sequence number. Use this for incremental polling — call gps_status to get the current sequence, then later call gps_changes with that sequence to see what changed.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "since": {
                                "type": "integer",
                                "description": "Sequence number to query changes from. Use 0 for all changes."
                            }
                        },
                        "required": ["since"]
                    }
                },
                {
                    "name": "gps_bundle",
                    "description": "Execute multiple read-only GPS queries in a single call. Reduces round-trip overhead when you need several pieces of information at once. Supports up to 20 operations per bundle.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "operations": {
                                "type": "array",
                                "description": "Array of operations to execute. Each has a 'tool' name and 'arguments' object.",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "tool": { "type": "string" },
                                        "arguments": { "type": "object" }
                                    },
                                    "required": ["tool"]
                                },
                                "maxItems": 20
                            }
                        },
                        "required": ["operations"]
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
    let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    let result = dispatch_tool(tool_name, &arguments, store, indexed, workspace);

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

fn dispatch_tool<S: SymbolStore>(
    tool_name: &str,
    arguments: &Value,
    store: &Arc<S>,
    indexed: &Arc<AtomicBool>,
    workspace: &str,
) -> Result<String, String> {
    match tool_name {
        "gps_status" => tool_status(indexed, workspace, store),
        "gps_locate" => tool_locate(store, arguments),
        "gps_neighborhood" => tool_neighborhood(store, arguments),
        "gps_summary" => tool_summary(store),
        "gps_tree" => tool_tree(store),
        "gps_read" => tool_read(arguments),
        "gps_hot" => tool_hot(store, arguments),
        "gps_search" => tool_search(store, arguments),
        "gps_word" => tool_word(store, arguments),
        "gps_changes" => tool_changes(store, arguments),
        "gps_bundle" => tool_bundle(store, indexed, workspace, arguments),
        _ => Err(format!("Unknown tool: {}", tool_name)),
    }
}

fn tool_status<S: SymbolStore>(
    indexed: &Arc<AtomicBool>,
    workspace: &str,
    store: &Arc<S>,
) -> Result<String, String> {
    let status = if indexed.load(Ordering::Relaxed) {
        "ready"
    } else {
        "indexing"
    };
    let seq = store.current_seq().unwrap_or(0);
    Ok(serde_json::to_string_pretty(&json!({
        "status": status,
        "indexed": indexed.load(Ordering::Relaxed),
        "workspace": workspace,
        "pid": std::process::id(),
        "current_seq": seq
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

fn tool_summary<S: SymbolStore>(store: &Arc<S>) -> Result<String, String> {
    let all_files = store
        .list_all_files()
        .map_err(|e| format!("Error: {}", e))?;
    let total_files = all_files.len();
    let mut files_by_language = std::collections::HashMap::new();
    let mut total_lines = 0u64;
    let mut total_symbols = 0u64;
    for (_, meta) in &all_files {
        if !meta.language.is_empty() {
            *files_by_language.entry(meta.language.clone()).or_insert(0u64) += 1;
        }
        total_lines += meta.line_count as u64;
        total_symbols += meta.symbol_count as u64;
    }

    Ok(serde_json::to_string_pretty(&json!({
        "total_files": total_files,
        "total_lines": total_lines,
        "total_symbols": total_symbols,
        "files_by_language": files_by_language
    }))
    .unwrap())
}

fn tool_tree<S: SymbolStore>(store: &Arc<S>) -> Result<String, String> {
    let all_files = store
        .list_all_files()
        .map_err(|e| format!("Error: {}", e))?;
    let mut entries: Vec<Value> = all_files
        .into_iter()
        .map(|(path, meta)| {
            json!({
                "path": path,
                "language": meta.language,
                "lines": meta.line_count,
                "symbols": meta.symbol_count
            })
        })
        .collect();
    entries.sort_by(|a, b| {
        a.get("path")
            .and_then(|p| p.as_str())
            .cmp(&b.get("path").and_then(|p| p.as_str()))
    });
    Ok(serde_json::to_string_pretty(&entries).unwrap())
}

fn tool_read(args: &Value) -> Result<String, String> {
    let path = args
        .get("path")
        .and_then(|s| s.as_str())
        .ok_or("Missing required parameter: path")?;

    // Block sensitive files
    if crate::sensitive::is_sensitive_file(std::path::Path::new(path)) {
        return Err("Access denied: sensitive file".to_string());
    }

    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len() as u32;
    let start = args
        .get("start_line")
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .max(1) as usize;
    let end = args
        .get("end_line")
        .and_then(|v| v.as_u64())
        .unwrap_or(total_lines as u64)
        .min(total_lines as u64) as usize;

    let selected: String = lines
        .get(start.saturating_sub(1)..end.min(lines.len()))
        .unwrap_or(&[])
        .join("\n");

    Ok(serde_json::to_string_pretty(&json!({
        "path": path,
        "content": selected,
        "start_line": start,
        "end_line": end,
        "total_lines": total_lines
    }))
    .unwrap())
}

fn tool_hot<S: SymbolStore>(store: &Arc<S>, args: &Value) -> Result<String, String> {
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;

    let files = store.hot_files(limit).map_err(|e| format!("Error: {}", e))?;
    let entries: Vec<Value> = files
        .into_iter()
        .map(|(path, meta)| {
            json!({
                "path": path,
                "language": meta.language,
                "mtime": meta.mtime,
                "symbols": meta.symbol_count
            })
        })
        .collect();

    Ok(serde_json::to_string_pretty(&entries).unwrap())
}

fn tool_search<S: SymbolStore>(store: &Arc<S>, args: &Value) -> Result<String, String> {
    let query = args
        .get("query")
        .and_then(|s| s.as_str())
        .ok_or("Missing required parameter: query")?;
    let max = args
        .get("max_results")
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;

    // Extract trigrams for index lookup
    let query_lower = query.to_lowercase();
    let chars: Vec<char> = query_lower.chars().collect();
    let trigrams: Vec<String> = if chars.len() >= 3 {
        chars
            .windows(3)
            .map(|w| w.iter().collect::<String>())
            .collect()
    } else {
        Vec::new()
    };

    let candidate_files = if trigrams.is_empty() {
        store
            .list_all_files()
            .map_err(|e| format!("Error: {}", e))?
            .into_iter()
            .map(|(path, _)| path)
            .collect()
    } else {
        store
            .search_trigrams(&trigrams)
            .map_err(|e| format!("Error: {}", e))?
    };

    let mut results = Vec::new();
    for file_path in candidate_files {
        if results.len() >= max {
            break;
        }
        if let Ok(content) = std::fs::read_to_string(&file_path) {
            let mut matches = Vec::new();
            for (i, line) in content.lines().enumerate() {
                if line.to_lowercase().contains(&query_lower) {
                    matches.push(json!({
                        "line": i + 1,
                        "content": line
                    }));
                    if matches.len() >= MAX_MATCHES_PER_FILE {
                        break;
                    }
                }
            }
            if !matches.is_empty() {
                results.push(json!({
                    "file": file_path,
                    "matches": matches
                }));
            }
        }
    }

    Ok(serde_json::to_string_pretty(&results).unwrap())
}

fn tool_word<S: SymbolStore>(store: &Arc<S>, args: &Value) -> Result<String, String> {
    let word = args
        .get("word")
        .and_then(|s| s.as_str())
        .ok_or("Missing required parameter: word")?;

    let locations = store
        .lookup_word(word)
        .map_err(|e| format!("Error: {}", e))?;

    if locations.is_empty() {
        return Ok(format!("No occurrences found for '{}'", word));
    }

    let results: Vec<Value> = locations
        .into_iter()
        .map(|wl| {
            json!({
                "file": wl.file,
                "line": wl.line
            })
        })
        .collect();

    Ok(serde_json::to_string_pretty(&results).unwrap())
}

fn tool_changes<S: SymbolStore>(store: &Arc<S>, args: &Value) -> Result<String, String> {
    let since = args
        .get("since")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let entries = store
        .changes_since(since)
        .map_err(|e| format!("Error: {}", e))?;

    let results: Vec<Value> = entries
        .into_iter()
        .map(|e| {
            json!({
                "seq": e.seq,
                "file": e.file_path,
                "operation": format!("{:?}", e.operation),
                "timestamp": e.timestamp
            })
        })
        .collect();

    Ok(serde_json::to_string_pretty(&json!({
        "current_seq": store.current_seq().unwrap_or(0),
        "changes": results
    }))
    .unwrap())
}

fn tool_bundle<S: SymbolStore>(
    store: &Arc<S>,
    indexed: &Arc<AtomicBool>,
    workspace: &str,
    args: &Value,
) -> Result<String, String> {
    let operations = args
        .get("operations")
        .and_then(|v| v.as_array())
        .ok_or("Missing required parameter: operations")?;

    if operations.len() > 20 {
        return Err("Maximum 20 operations per bundle".to_string());
    }

    let mut results = Vec::new();
    for op in operations {
        let tool = op.get("tool").and_then(|t| t.as_str()).unwrap_or("");
        let op_args = op.get("arguments").cloned().unwrap_or(json!({}));

        // Only allow read-only tools in bundles
        let result = match tool {
            "gps_status" | "gps_locate" | "gps_neighborhood" | "gps_summary" | "gps_tree"
            | "gps_read" | "gps_hot" | "gps_search" | "gps_word" | "gps_changes" => {
                dispatch_tool(tool, &op_args, store, indexed, workspace)
            }
            _ => Err(format!("Tool '{}' not allowed in bundle", tool)),
        };

        results.push(json!({
            "tool": tool,
            "result": match result {
                Ok(content) => json!({"content": content, "isError": false}),
                Err(e) => json!({"content": e, "isError": true}),
            }
        }));
    }

    Ok(serde_json::to_string_pretty(&results).unwrap())
}

async fn write_response(stdout: &mut tokio::io::Stdout, response: &Value) -> anyhow::Result<()> {
    let mut output = serde_json::to_string(response)?;
    output.push('\n');
    stdout.write_all(output.as_bytes()).await?;
    stdout.flush().await?;
    Ok(())
}
