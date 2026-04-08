---
name: gps-tree
description: >
  Get the full file tree of the indexed workspace with language detection,
  line counts, and symbol counts per file. Use this skill to understand
  the project structure and find relevant files before diving in.
---

# GPS Tree

The gpsource `file_tree` method returns a structured overview of all indexed
files in the workspace: their path, detected language, line count, and number
of symbols extracted by tree-sitter.

## How to call

### MCP (preferred)
Use the `gps_tree` MCP tool with no arguments.

### HTTP
```bash
curl -s http://127.0.0.1:<port> \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $(cat ~/.gps/auth-token)" \
  -d '{"jsonrpc":"2.0","method":"file_tree","params":{},"id":1}'
```

## Response format

```json
[
  {"path": "src/main.rs", "language": "rust", "lines": 170, "symbols": 8},
  {"path": "src/config.rs", "language": "rust", "lines": 57, "symbols": 3},
  {"path": "src/lib.ts", "language": "typescript", "lines": 45, "symbols": 5}
]
```

## When to use

- First step when exploring an unfamiliar codebase
- Understanding the project layout before making changes
- Finding which files might contain relevant code (by language or symbol count)
