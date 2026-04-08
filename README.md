# g-p-source

A high-performance, incremental code indexing engine written in Rust. It watches your workspace filesystem, parses source files with [tree-sitter](https://tree-sitter.github.io/tree-sitter/), and maintains a live symbol database queryable through a JSON-RPC API and MCP protocol — designed to feed structural codebase awareness to LLM-powered coding assistants (GitHub Copilot, Claude Code, and any MCP-compatible CLI).

## Features

- **Incremental indexing** — Only re-parses files that actually changed (content-hashed via BLAKE3).
- **Five languages** — Java, TypeScript/TSX, Python, Rust, C#/.NET, with per-language tree-sitter queries.
- **Sub-10ms queries** — Symbol lookups, dependency neighborhoods, and workspace summaries served from an embedded [sled](https://github.com/spacejam/sled) database.
- **Real-time filesystem watching** — Powered by [notify](https://github.com/notify-rs/notify) with debounced events (configurable window).
- **JSON-RPC 2.0 API** — Ten methods (`locate`, `get_neighborhood`, `workspace_summary`, `status`, `file_tree`, `hot_files`, `read_file`, `search`, `word_lookup`, `changes_since`) served over HTTP on localhost.
- **MCP support** — Run with `--mcp` for native Model Context Protocol over stdio, compatible with any MCP client. 11 tools available including batch queries.
- **Full-text search** — Trigram-indexed full-text search across all indexed files for fast code search.
- **Inverted word index** — O(1) exact identifier lookup by symbol name.
- **File reading with ranges** — Read source files with optional line ranges; sensitive files are automatically blocked.
- **Hot files tracking** — Recently modified files sorted by modification time for change-aware navigation.
- **Change tracking** — Monotonic sequence numbers for incremental polling via `changes_since(seq)`.
- **Sensitive file blocking** — Automatic exclusion of `.env*`, `credentials.json`, `secrets.*`, `.pem`, `.key`, SSH keys, and more from indexing and reading.
- **Batch queries** — Bundle up to 20 read-only queries in a single MCP call to reduce round-trip overhead.
- **Dynamic port** — Binds to port 0 by default (OS-assigned). The actual port is written to a discovery file.
- **Per-workspace instances** — Each workspace gets its own sled database and server instance, discoverable via `~/.gps/instances/<hash>.json`.
- **Auth by default** — Each server instance generates a random bearer token written to `~/.gps/auth-token` (mode 0600).
- **Copilot Agent Plugin** — Ships as an IDE-agnostic agent plugin with skills, MCP server, and an `@gps` agent.

## Architecture

```
┌──────────────┐     ┌────────────┐     ┌──────────────┐     ┌─────────────┐
│  FS Watcher  │────▶│ Dispatcher │────▶│  Parser Pool │────▶│  sled Index │
│   (notify)   │     │ (channel)  │     │ (tree-sitter)│     │  (9 trees)  │
└──────────────┘     └────────────┘     └──────────────┘     └──────┬──────┘
                                                                    │
                                              ┌─────────────────────┼─────────────────────┐
                                              │                     │                     │
                                       ┌──────▼──────┐      ┌──────▼──────┐      ┌───────▼──────┐
                                       │  JSON-RPC   │      │  MCP stdio  │      │  Discovery   │
                                       │  (HTTP)     │      │  (--mcp)    │      │  (~/.gps/)   │
                                       └──────┬──────┘      └──────┬──────┘      └──────────────┘
                                              │                     │
                                       ┌──────▼──────┐      ┌──────▼──────┐
                                       │ curl / CLIs │      │ MCP Clients │
                                       └─────────────┘      └─────────────┘
```

**Pipeline flow:** FS event → debounce (100ms) → content hash check → tree-sitter parse (spawn_blocking) → symbol diff → sled batch write.

## Getting Started

### Prerequisites

- Rust 1.85+ (edition 2024)

### Build & Install

```bash
cargo build --release
# Or install globally:
cargo install --path .
```

### Run

```bash
# Index the current directory (dynamic port)
gpsource

# Index a specific workspace with a fixed port
gpsource --workspace-root /path/to/project --port 8080

# Run as MCP server over stdio
gpsource --mcp --workspace-root /path/to/project

# List all running instances
gpsource --discovery

# Kill all running instances
gpsource --kill
```

### CLI Options

| Flag | Default | Description |
|------|---------|-------------|
| `-w, --workspace-root` | `.` | Root directory to index |
| `-p, --port` | `0` (dynamic) | JSON-RPC server port (0 = OS picks a free port) |
| `--mcp` | `false` | Run as MCP server over stdio instead of HTTP |
| `--discovery` | `false` | List all running gpsource instances (JSON) and exit |
| `--kill` | `false` | Kill all running gpsource instances and exit |
| `--cache-capacity` | `41943040` (40 MB) | sled cache size in bytes |
| `--exclude` | `.git,node_modules,target,bin,obj,build,dist,__pycache__` | Comma-separated exclusion patterns |
| `--debounce-ms` | `100` | FS event debounce window in milliseconds |

### Environment Variables

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | Tracing filter (e.g. `info`, `g_p_source=debug`) |

## Instance Discovery

Each workspace gets its own gpsource process. On startup, the server writes a discovery file:

```
~/.gps/instances/<hash>.json
```

Where `<hash>` is the first 16 characters of the BLAKE3 hash of the canonical workspace path.

```json
{
  "port": 54321,
  "pid": 12345,
  "workspace": "/home/user/my-project",
  "status": "ready",
  "started_at": "1712505600"
}
```

Or use the built-in CLI commands:

```bash
# List all running instances as JSON (cleans up stale PIDs automatically)
gpsource --discovery

# Kill all running instances
gpsource --kill
```

**Status lifecycle:** `starting` → `indexing` → `ready`

Use the bundled discovery script to find the right instance:

```bash
copilot-plugin/scripts/gps-discover.sh /path/to/workspace
```

## API

The server listens on `http://127.0.0.1:<port>` and expects a `Bearer <token>` authorization header. The token is written to `~/.gps/auth-token` on startup.

### `status`

Check if the server is up and initial indexation is complete. **Call this first at session start.**

```json
{"jsonrpc":"2.0","method":"status","params":{},"id":1}
```

**Response:**

```json
{
  "result": {
    "status": "ready",
    "indexed": true,
    "workspace": "/home/user/my-project",
    "port": 54321,
    "pid": 12345
  }
}
```

### `locate`

Find symbols by name (prefix match).

```json
{"jsonrpc":"2.0","method":"locate","params":{"symbol_name":"UserService"},"id":1}
```

**Response:**

```json
{
  "result": [
    {
      "file": "src/service.java",
      "line": 5,
      "col": 14,
      "kind": "class",
      "qualified_name": "src/service.java.UserService"
    }
  ]
}
```

### `get_neighborhood`

Get the dependency neighborhood of a file — imports, reverse importers, and symbols in related files.

```json
{"jsonrpc":"2.0","method":"get_neighborhood","params":{"file_path":"src/service.java","depth":1},"id":2}
```

### `workspace_summary`

Get a condensed workspace overview: total files, language breakdown, total lines, and total symbols.

```json
{"jsonrpc":"2.0","method":"workspace_summary","params":{},"id":3}
```

**Response:**

```json
{
  "result": {
    "total_files": 142,
    "total_lines": 28500,
    "total_symbols": 1830,
    "files_by_language": {"rust": 45, "typescript": 67, "python": 30}
  }
}
```

### `file_tree`

Get the full file tree of the indexed workspace with language, line count, and symbol count per file.

```json
{"jsonrpc":"2.0","method":"file_tree","params":{},"id":4}
```

**Response:**

```json
{
  "result": [
    {"path": "src/main.rs", "language": "rust", "line_count": 170, "symbol_count": 8},
    {"path": "src/config.rs", "language": "rust", "line_count": 57, "symbol_count": 3}
  ]
}
```

### `hot_files`

Get the most recently modified files, sorted by modification time (newest first).

```json
{"jsonrpc":"2.0","method":"hot_files","params":{"limit":10},"id":5}
```

**Response:**

```json
{
  "result": [
    {"path": "src/api/methods.rs", "language": "rust", "mtime": 1712505600, "symbol_count": 12}
  ]
}
```

### `read_file`

Read the content of a source file, optionally restricted to a line range. Sensitive files (`.env*`, credentials, keys) are automatically blocked.

```json
{"jsonrpc":"2.0","method":"read_file","params":{"path":"src/main.rs","start_line":10,"end_line":30},"id":6}
```

**Response:**

```json
{
  "result": {
    "path": "src/main.rs",
    "content": "fn main() {\n    ...\n}",
    "start_line": 10,
    "end_line": 30,
    "total_lines": 170
  }
}
```

### `search`

Trigram-accelerated full-text search across all indexed files. Returns matching files with line numbers and content.

```json
{"jsonrpc":"2.0","method":"search","params":{"query":"handleAuth","max_results":10},"id":7}
```

**Response:**

```json
{
  "result": [
    {
      "file": "src/auth.rs",
      "matches": [
        {"line": 42, "content": "pub fn handleAuth(request: &Request) -> Result<Token> {"}
      ]
    }
  ]
}
```

### `word_lookup`

O(1) exact identifier lookup in the inverted word index. Returns all files and line numbers where a symbol name is defined.

```json
{"jsonrpc":"2.0","method":"word_lookup","params":{"word":"UserService"},"id":8}
```

**Response:**

```json
{
  "result": [
    {"file": "src/service.java", "line": 5}
  ]
}
```

### `changes_since`

Get files that changed since a given sequence number. Use for incremental polling.

```json
{"jsonrpc":"2.0","method":"changes_since","params":{"seq":42},"id":9}
```

**Response:**

```json
{
  "result": [
    {"seq": 43, "file_path": "src/main.rs", "operation": "Upsert", "timestamp": 1712505600}
  ]
}
```

## MCP Mode

When started with `--mcp`, gpsource communicates over stdio using the Model Context Protocol. This mode is used by the agent plugin's `.mcp.json` and is compatible with any MCP client.

**Exposed tools:**

| Tool | Description |
|------|-------------|
| `gps_status` | Health check — is the engine running and indexation complete? Includes current change sequence number. |
| `gps_locate` | Find symbol definitions by name (prefix match) |
| `gps_neighborhood` | Get imports, importers, and related symbols for a file |
| `gps_summary` | Get workspace overview: languages, file count, line count, symbol count |
| `gps_tree` | Full file tree with language, line count, and symbol count per file |
| `gps_read` | Read file content with optional line range; sensitive files blocked |
| `gps_hot` | Recently modified files sorted by mtime |
| `gps_search` | Trigram-accelerated full-text search across all files |
| `gps_word` | O(1) exact identifier lookup in the inverted word index |
| `gps_changes` | Files changed since a given sequence number (incremental polling) |
| `gps_bundle` | Batch up to 20 read-only queries in a single call |

## Copilot Agent Plugin

The `copilot-plugin/` directory is an IDE-agnostic [agent plugin](https://code.visualstudio.com/docs/copilot/customization/agent-plugins) that teaches LLMs to use the GPS index instead of naive code traversal.

### Plugin Structure

```
copilot-plugin/
├── plugin.json                          # Plugin metadata
├── .mcp.json                            # MCP server definition (gpsource --mcp)
├── agents/
│   └── gps.agent.md                     # @gps agent with API instructions
├── skills/
│   ├── gps-session-init/SKILL.md        # Session startup: check/start/wait
│   ├── gps-locate/SKILL.md              # Symbol location skill
│   ├── gps-neighborhood/SKILL.md        # Dependency graph skill
│   ├── gps-summary/SKILL.md             # Workspace overview skill
│   ├── gps-tree/SKILL.md               # File tree with metadata
│   ├── gps-read/SKILL.md               # File reading with line ranges
│   ├── gps-hot/SKILL.md                # Hot (recently modified) files
│   ├── gps-search/SKILL.md             # Full-text trigram search
│   ├── gps-word/SKILL.md               # Exact word lookup
│   ├── gps-changes/SKILL.md            # Change tracking / polling
│   └── gps-bundle/SKILL.md             # Batch queries
└── scripts/
    ├── gps-discover.sh / .ps1           # Find running instance for a workspace
    ├── gps-start.sh / .ps1              # Start gpsource if not running
    └── gps-status.sh / .ps1             # Health check via JSON-RPC API
```

### Install as Agent Plugin

```bash
# From source (VS Code / Copilot)
# Command Palette → "Chat: Install Plugin From Source"
# Enter: https://github.com/knightmax/g-p-source

# Or register locally:
# settings.json
{
  "chat.pluginLocations": {
    "/path/to/g-p-source/copilot-plugin": true
  }
}
```

### How the LLM Uses It

1. **Session start** — The `gps-session-init` skill auto-checks if gpsource is running, starts it if needed, and waits for indexation to complete.
2. **Symbol lookup** — Instead of grepping, the LLM calls `locate` or `word_lookup` for sub-10ms structural definition search.
3. **Code search** — The LLM calls `search` for trigram-accelerated full-text search across the codebase, or `word` for exact identifier matches.
4. **Code reading** — The LLM calls `read_file` to inspect specific code sections with line ranges, without needing file system access.
5. **Impact analysis** — Before editing, the LLM calls `get_neighborhood` to understand dependencies and blast radius.
6. **Orientation** — In unfamiliar codebases, the LLM calls `workspace_summary` and `file_tree` for instant architecture overview.
7. **Change awareness** — The LLM calls `hot_files` to see recent activity, and `changes_since` for incremental polling.
8. **Efficiency** — The LLM uses `gps_bundle` to batch multiple queries in a single call, reducing round-trip overhead.

### Works Without MCP

When MCP is not available (e.g., enterprise restrictions), the skills instruct the LLM to use `curl` against the HTTP JSON-RPC API with the discovery mechanism. No MCP dependency required.

## Index Schema

The sled database stores data across nine trees:

| Tree | Key | Value | Purpose |
|------|-----|-------|---------|
| `sym:def` | `{qualified_name}` | `SymbolRecord` | Symbol definitions |
| `sym:file` | `{file}\x00{name}` | `SymbolRef` | File → symbol mapping |
| `sym:kind` | `{kind}\x00{name}` | `()` | Kind-based lookup |
| `dep:import` | `{file}\x00{imported}` | `()` | Import graph |
| `dep:reverse` | `{imported}\x00{file}` | `()` | Reverse import graph |
| `meta:file` | `{file}` | `FileMetadata` | mtime, hash, symbol count, language, line count |
| `word:idx` | `{word}\x00{file}:{line}` | `()` | Inverted word index for O(1) identifier lookup |
| `tri:idx` | `{trigram}\x00{file}` | `()` | Trigram index for full-text search |
| `changes:log` | `{seq}` (u64 big-endian) | `ChangeEntry` | Monotonic change log for incremental polling |

All values are serialized with [bincode](https://github.com/bincode-org/bincode).

## Extracted Symbol Kinds

| Kind | Languages |
|------|-----------|
| `class` | Java, TypeScript, Python, C# |
| `struct` | Rust, C# |
| `enum` | Java, Rust, C#, TypeScript |
| `interface` | Java, TypeScript, C# |
| `trait` | Rust |
| `function` | TypeScript, Python, Rust |
| `method` | Java, TypeScript, Python, Rust, C# |
| `module` | Rust, Python |
| `namespace` | C#, TypeScript |
| `import` | All |

## Testing

```bash
# Run all tests (unit + integration)
cargo test

# Run only parser query tests
cargo test --test parser_tests

# Run only store CRUD tests
cargo test --test store_tests

# Run new features tests (word index, trigram search, changes, hot files, sensitive files)
cargo test --test new_features_tests

# Run the pipeline integration test
cargo test --test pipeline_integration

# Run instance discovery tests
cargo test --test discovery_tests

# Run benchmarks (500k symbol locate latency)
cargo test --test benchmarks -- --nocapture
```

## Project Structure

```
src/
├── main.rs              # Entry point: HTTP mode or MCP mode
├── lib.rs               # Library crate root
├── config.rs            # CLI args (clap) — port, --mcp, excludes
├── discovery.rs         # Instance discovery: write/read ~/.gps/instances/
├── watcher/
│   ├── mod.rs
│   └── fs_watcher.rs    # notify-based FS watcher
├── parser/
│   ├── mod.rs
│   ├── language_registry.rs  # Extension → language mapping
│   ├── queries.rs            # Tree-sitter S-expression queries
│   ├── source_parser.rs      # Incremental parsing + symbol extraction
│   ├── symbol.rs             # Symbol, SymbolKind, Visibility types
│   └── symbol_diff.rs        # Diff computation between parses
├── index/
│   ├── mod.rs
│   ├── store.rs         # SymbolStore trait
│   ├── sled_store.rs    # sled implementation
│   └── types.rs         # SymbolRecord, FileMetadata
├── pipeline/
│   ├── mod.rs
│   ├── dispatcher.rs    # Async event loop with dedup + semaphore
│   └── initial_crawl.rs # Recursive directory walk on startup
├── api/
│   ├── mod.rs
│   ├── auth.rs          # Token generation (~/.gps/auth-token)
│   ├── methods.rs       # JSON-RPC method definitions (10 methods)
│   └── server.rs        # jsonrpsee HTTP server + discovery file write
├── sensitive.rs         # Sensitive file pattern matching (.env, keys, etc.)
└── mcp/
    ├── mod.rs
    └── stdio_server.rs  # MCP protocol over stdin/stdout (11 tools)

copilot-plugin/              # IDE-agnostic Copilot agent plugin (11 skills)
├── plugin.json
├── .mcp.json
├── agents/
│   └── gps.agent.md
├── skills/
│   ├── gps-session-init/SKILL.md
│   ├── gps-locate/SKILL.md
│   ├── gps-neighborhood/SKILL.md
│   ├── gps-summary/SKILL.md
│   ├── gps-tree/SKILL.md
│   ├── gps-read/SKILL.md
│   ├── gps-hot/SKILL.md
│   ├── gps-search/SKILL.md
│   ├── gps-word/SKILL.md
│   ├── gps-changes/SKILL.md
│   └── gps-bundle/SKILL.md
└── scripts/
    ├── gps-discover.sh
    ├── gps-start.sh
    └── gps-status.sh

tests/
├── parser_tests.rs          # Per-language symbol extraction
├── store_tests.rs           # SymbolStore CRUD operations
├── new_features_tests.rs    # Word index, trigram, changes, hot files, sensitive files
├── pipeline_integration.rs  # Add → modify → delete cycle
├── discovery_tests.rs       # Instance discovery
└── benchmarks.rs            # Locate latency on 500k symbols
```

## Performance Targets

| Metric | Target |
|--------|--------|
| `locate` p99 latency | < 10 ms |
| `get_neighborhood` p99 latency | < 10 ms |
| `workspace_summary` p99 latency | < 10 ms |
| Memory (100k files) | < 100 MB RSS |
| Initial crawl (10k files) | < 30 s |
| Incremental re-index (single file) | < 50 ms |

## Tech Stack

| Component | Crate / Tool |
|-----------|-------------|
| Async runtime | `tokio` |
| FS watcher | `notify` v6 |
| Parser | `tree-sitter` v0.24 |
| Database | `sled` v0.34 |
| RPC server | `jsonrpsee` v0.24 |
| MCP server | Custom stdio (built-in) |
| Content hashing | `blake3` |
| Serialization | `bincode` + `serde` |
| CLI | `clap` v4 |
| Tracing | `tracing` + `tracing-subscriber` |

## License

See [LICENSE](LICENSE).
