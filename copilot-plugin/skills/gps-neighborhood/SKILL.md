---
name: gps-neighborhood
description: >
  Get the dependency graph around a file — its imports, reverse importers, and
  symbols in related files. Use this skill before editing a file to understand
  blast radius, when debugging to trace dependency chains, when assessing impact
  of a refactor, or whenever the user asks "what depends on this?", "what does
  this import?", "show me the dependency graph", or "who uses this file?". Also
  use it after a locate call when you need to understand the context around a
  symbol's file. This is essential for safe refactoring — always check the
  neighborhood before renaming, moving, or deleting code.
---

# GPS Neighborhood

The `get_neighborhood` method returns the import graph around a file: what it
depends on, who depends on it, and what symbols live in each related file. This
gives you the blast radius of any change without reading dozens of files.

## How to call

```bash
curl -s http://127.0.0.1:<port> \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $(cat ~/.gps/auth-token)" \
  -d '{"jsonrpc":"2.0","method":"get_neighborhood","params":{"file_path":"<FILE>","depth":1},"id":1}'
```

Replace `<FILE>` with the workspace-relative path (e.g., `src/service/UserService.java`).
The `depth` parameter controls how many levels of dependencies to traverse (default: 1).

## Response format

```json
{
  "file": "src/service/UserService.java",
  "imports": ["src/model/User.java", "src/repository/UserRepo.java"],
  "imported_by": ["src/controller/UserController.java", "src/test/UserServiceTest.java"],
  "symbols": {
    "src/model/User.java": [
      {"file": "src/model/User.java", "line": 5, "col": 14, "kind": "class", "qualified_name": ""}
    ]
  }
}
```

- **imports** — files this file directly depends on (its upstream dependencies)
- **imported_by** — files that depend on this file (downstream consumers, a.k.a. blast radius)
- **symbols** — symbol definitions in each related file, so you don't need extra lookups

**Example 1:**
Input: "What will break if I change UserService?"
Action: `get_neighborhood` with `file_path: "src/service/UserService.java"` → check `imported_by` for affected files

**Example 2:**
Input: "Show me what this module depends on"
Action: `get_neighborhood` with the module's main file → read `imports` list

## Usage pattern

1. Call `get_neighborhood` to see what a file imports and who imports it.
2. If you're about to change a public API, check `imported_by` to see all callers.
3. Use `depth: 2` for a wider dependency radius (e.g., when refactoring a shared utility).
