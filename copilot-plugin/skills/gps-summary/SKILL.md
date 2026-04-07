---
name: gps-summary
description: >
  Get a high-level overview of the workspace: total files, language breakdown,
  module list, and public API surface. Use this skill as the first step when
  exploring an unfamiliar codebase, when the user asks "what's in this project?",
  "show me the architecture", "what languages are used?", "give me an overview",
  or any orientation question. Also use it when you need to understand the shape
  of a codebase before diving into specifics. This returns the answer in
  milliseconds instead of recursively listing directories and reading files.
---

# GPS Summary

The `workspace_summary` method returns a condensed overview of the entire indexed
workspace: how many files, which languages, what modules exist, and what the
public API surface looks like. This is the fastest way to orient yourself in any
codebase — one call instead of dozens of `list_dir` and `read_file` operations.

## How to call

```bash
curl -s http://127.0.0.1:<port> \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $(cat ~/.gps/auth-token)" \
  -d '{"jsonrpc":"2.0","method":"workspace_summary","params":{},"id":1}'
```

## Response format

```json
{
  "total_files": 1247,
  "files_by_language": {
    "java": 890,
    "typescript": 230,
    "python": 127
  },
  "modules": ["com.example.service", "com.example.model", "com.example.api"],
  "public_symbols": [
    {"file": "src/api/UserController.java", "line": 12, "col": 1, "kind": "class", "qualified_name": ""}
  ]
}
```

## Recommended workflow

1. Call `workspace_summary` to orient yourself — understand the languages, scale, and modules.
2. Identify the main modules and entry points from the response.
3. Use `gps-locate` to find specific symbols mentioned in the summary.
4. Use `gps-neighborhood` to explore dependency relationships around key files.

This gives you structural awareness in seconds instead of spending minutes
scanning the filesystem.

**Example:**
Input: "I just cloned this repo, what's in it?"
Action: call `workspace_summary` → report languages, file count, key modules, and suggest next steps
