---
name: gps-changes
description: >
  Get files that changed since a given sequence number. Use this for
  incremental polling — track changes between interactions to know which
  files have been modified, added, or deleted since you last checked.
---

# GPS Changes

The gpsource `changes_since` method returns all file changes since a given
sequence number. Each mutation in the index is assigned a monotonically
increasing sequence number.

## How to call

### MCP (preferred)
Use the `gps_changes` MCP tool:
```json
{"since": 0}
```

### HTTP
```bash
curl -s http://127.0.0.1:<port> \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $(cat ~/.gps/auth-token)" \
  -d '{"jsonrpc":"2.0","method":"changes_since","params":{"seq":0},"id":1}'
```

## Response format

```json
{
  "current_seq": 42,
  "changes": [
    {"seq": 40, "file": "src/main.rs", "operation": "Upsert", "timestamp": 1712505600},
    {"seq": 41, "file": "src/old.rs", "operation": "Remove", "timestamp": 1712505601},
    {"seq": 42, "file": "src/new.rs", "operation": "Upsert", "timestamp": 1712505602}
  ]
}
```

## Workflow

1. At session start, call `gps_status` to get `current_seq`
2. Do your work
3. Call `gps_changes` with the saved seq to see what changed
4. Update your seq cursor for next check

## When to use

- Incremental polling for file changes
- Understanding what changed since your last interaction
- Building change-aware workflows
