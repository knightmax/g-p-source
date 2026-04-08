---
name: gps-word
description: >
  O(1) exact word lookup in the inverted index. Find all files and line
  numbers where a specific identifier is defined. Faster than search for
  exact symbol names. Use this skill for precise identifier lookups.
---

# GPS Word Lookup

The gpsource `word_lookup` method performs an O(1) lookup in the inverted
word index. It returns all files and line numbers where a given identifier
(function name, class name, variable) is defined as a symbol.

## How to call

### MCP (preferred)
Use the `gps_word` MCP tool:
```json
{"word": "UserService"}
```

### HTTP
```bash
curl -s http://127.0.0.1:<port> \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $(cat ~/.gps/auth-token)" \
  -d '{"jsonrpc":"2.0","method":"word_lookup","params":{"word":"UserService"},"id":1}'
```

## Response format

```json
[
  {"file": "src/service.java", "line": 5},
  {"file": "tests/service_test.java", "line": 12}
]
```

## When to use

- Quick identifier lookup when you know the exact name
- Finding all files where a symbol is defined (not used — for usages, combine with `get_neighborhood`)
- Faster than `search` for exact matches
