---
name: gps-hot
description: >
  Get the most recently modified files in the workspace. Use this skill to
  understand what's actively being worked on, or to focus attention on files
  that have changed recently.
---

# GPS Hot Files

The gpsource `hot_files` method returns the most recently modified files
in the indexed workspace, sorted by modification time (newest first).

## How to call

### MCP (preferred)
Use the `gps_hot` MCP tool:
```json
{"limit": 10}
```

### HTTP
```bash
curl -s http://127.0.0.1:<port> \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $(cat ~/.gps/auth-token)" \
  -d '{"jsonrpc":"2.0","method":"hot_files","params":{"limit":10},"id":1}'
```

## Response format

```json
[
  {"path": "src/api/methods.rs", "language": "rust", "mtime": 1712505600, "symbols": 12},
  {"path": "src/pipeline/dispatcher.rs", "language": "rust", "mtime": 1712505500, "symbols": 5}
]
```

## When to use

- Understanding what's actively being developed
- Orienting in a codebase by seeing recent activity
- Finding related files that were changed together
