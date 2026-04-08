---
name: gps-bundle
description: >
  Execute multiple GPS queries in a single call. Reduces round-trip overhead
  when you need several pieces of information at once. Supports up to 20
  read-only operations per bundle.
---

# GPS Bundle

The gpsource `gps_bundle` MCP tool executes multiple read-only queries in
a single call, reducing round-trip overhead for complex information gathering.

## How to call

### MCP (preferred)
Use the `gps_bundle` MCP tool:
```json
{
  "operations": [
    {"tool": "gps_status"},
    {"tool": "gps_locate", "arguments": {"symbol_name": "UserService"}},
    {"tool": "gps_hot", "arguments": {"limit": 5}},
    {"tool": "gps_tree"}
  ]
}
```

## Response format

```json
[
  {"tool": "gps_status", "result": {"content": "...", "isError": false}},
  {"tool": "gps_locate", "result": {"content": "...", "isError": false}},
  {"tool": "gps_hot", "result": {"content": "...", "isError": false}},
  {"tool": "gps_tree", "result": {"content": "...", "isError": false}}
]
```

## Allowed tools

Only read-only tools are allowed in bundles:
- `gps_status`, `gps_locate`, `gps_neighborhood`, `gps_summary`
- `gps_tree`, `gps_read`, `gps_hot`
- `gps_search`, `gps_word`, `gps_changes`

## Limits

- Maximum 20 operations per bundle
- Only read-only tools (no mutations)

## When to use

- Session start: bundle `gps_status` + `gps_tree` + `gps_hot` for full context
- Before editing: bundle `gps_locate` + `gps_neighborhood` + `gps_read` for comprehensive info
- Any time you need multiple queries — bundle them to reduce latency
