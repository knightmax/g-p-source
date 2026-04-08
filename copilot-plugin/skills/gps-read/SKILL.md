---
name: gps-read
description: >
  Read the content of a source file, optionally restricted to a specific line
  range. Sensitive files (.env, credentials, keys) are automatically blocked.
  Use this skill to inspect code after locating symbols, or to read specific
  sections of a file without opening it in an editor.
---

# GPS Read

The gpsource `read_file` method reads file content with optional line ranges
and automatic sensitive-file blocking.

## How to call

### MCP (preferred)
Use the `gps_read` MCP tool:
```json
{"path": "src/service.java", "start_line": 10, "end_line": 30}
```

### HTTP
```bash
curl -s http://127.0.0.1:<port> \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $(cat ~/.gps/auth-token)" \
  -d '{"jsonrpc":"2.0","method":"read_file","params":{"path":"src/service.java","start_line":10,"end_line":30},"id":1}'
```

## Response format

```json
{
  "path": "src/service.java",
  "content": "public class UserService {\n    ...\n}",
  "start_line": 10,
  "end_line": 30,
  "total_lines": 150
}
```

## Parameters

- **path** (required): File path to read
- **start_line** (optional): First line to return (1-based, default: 1)
- **end_line** (optional): Last line to return (1-based, default: last line)

## Security

Sensitive files are automatically blocked:
- `.env*` files
- `credentials.json`, `secrets.yaml`
- `.pem`, `.key`, `.p12` certificate/key files
- SSH keys (`id_rsa`, `id_ed25519`, etc.)

## When to use

- After `locate` finds a symbol, read the surrounding code
- Inspect specific functions or classes by line range
- Review code changes at specific locations
