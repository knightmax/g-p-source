---
name: gps-search
description: >
  Full-text search across the indexed workspace using trigram-accelerated
  matching. Returns matching files with line numbers and content. Use this
  skill to find text patterns, string literals, or code snippets across
  the codebase — much faster than grep for pre-indexed codebases.
---

# GPS Search

The gpsource `search` method performs trigram-accelerated full-text search
across all indexed files. For queries of 3+ characters, it first narrows
candidates using a trigram index, then verifies matches line-by-line.

## How to call

### MCP (preferred)
Use the `gps_search` MCP tool:
```json
{"query": "handleAuth", "max_results": 10}
```

### HTTP
```bash
curl -s http://127.0.0.1:<port> \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $(cat ~/.gps/auth-token)" \
  -d '{"jsonrpc":"2.0","method":"search","params":{"query":"handleAuth","max_results":10},"id":1}'
```

## Response format

```json
[
  {
    "file": "src/auth.rs",
    "matches": [
      {"line": 42, "content": "pub fn handleAuth(request: &Request) -> Result<Token> {"},
      {"line": 88, "content": "    // handleAuth fallback logic"}
    ]
  }
]
```

## Parameters

- **query** (required): The text to search for (case-insensitive)
- **max_results** (optional): Maximum number of files to return (default: 20)

## When to use

- Searching for string literals, comments, or text patterns
- Finding all occurrences of a pattern across the codebase
- Complementary to `locate` — use `locate` for structural definitions,
  `search` for text patterns in function bodies and comments
