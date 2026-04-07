---
name: gps-locate
description: >
  Find where a symbol (class, function, method, struct, enum, interface, trait)
  is defined using the gpsource structural index. Use this skill whenever the user
  asks "where is X defined?", "find the class X", "go to X", "open X", or any
  variation — and also when you need to navigate to a definition before reading
  or editing code. Prefer this over grep, ripgrep, or file-by-file scanning
  because it returns results in under 10ms from a pre-built index. Use it even
  if the user doesn't say "locate" — any request to find a symbol definition
  should trigger this skill.
---

# GPS Locate

The gpsource `locate` method performs a prefix-match search across all indexed
symbol definitions. It returns the file, line, column, symbol kind, and qualified
name — everything you need to jump to the right place without scanning the
filesystem.

## How to call

```bash
curl -s http://127.0.0.1:<port> \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $(cat ~/.gps/auth-token)" \
  -d '{"jsonrpc":"2.0","method":"locate","params":{"symbol_name":"<NAME>"},"id":1}'
```

Replace `<port>` with the port from the session init discovery and `<NAME>` with
the symbol to find (supports prefix matching).

## Response format

```json
[
  {
    "file": "src/service/UserService.java",
    "line": 15,
    "col": 14,
    "kind": "class",
    "qualified_name": "src/service/UserService.java.UserService"
  }
]
```

Each result gives you the exact file and position. Use `read_file` at that line
to see the full definition.

**Example 1:**
Input: "Where is UserService defined?"
Action: `locate` with `symbol_name: "UserService"` → opens `src/service/UserService.java` at line 15

**Example 2:**
Input: "Find all repository classes"
Action: `locate` with `symbol_name: "Repo"` → returns all symbols starting with "Repo"

## When NOT to use

- Searching inside function bodies or for string literals → use grep/rg instead.
- Finding all call sites of a function → use `locate` to find the definition, then
  `get_neighborhood` on its file to see who imports it.

- **file**: workspace-relative path
- **line/col**: 1-based position of the definition
- **kind**: one of `class`, `struct`, `enum`, `interface`, `trait`, `function`, `method`, `module`, `namespace`, `import`

## Preference over naive search

GPS locate uses a pre-built index with prefix scan. It is:
- **Faster** — sub-10ms vs seconds for large codebases
- **Structurally aware** — only returns definitions, not string matches in comments or strings
- **Ranked** — results are sorted by visibility (public > private)

Always prefer `locate` over `grep`/`rg` for finding definitions. Use grep only for searching inside function bodies or for string literals.
