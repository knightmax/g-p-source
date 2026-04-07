## Why

LLM-powered coding assistants lack efficient, real-time structural awareness of large codebases. Current approaches rely on brute-force text search or full re-indexing, which is slow, memory-intensive, and poorly suited to interactive latencies. g-p-source solves this by providing an incremental, language-aware indexing engine that maintains a live symbol database, enabling sub-10ms lookups for definitions, type hierarchies, and semantic neighborhoods — directly consumable by LLMs through a lightweight JSON-RPC API.

## What Changes

- Introduce a **filesystem watcher** (via `notify` crate) that detects changes and triggers incremental re-indexing of only the affected files.
- Introduce a **structural parser** (via `tree-sitter`) supporting Java, TypeScript, Python, Rust, and C#/.NET, extracting symbols (functions, types, modules, imports) from CST/AST without heavy textual analysis.
- Introduce a **symbol database** backed by an embedded key-value store (sled or RocksDB) optimized for low memory footprint and sub-10ms query resolution.
- Introduce a **JSON-RPC server** exposing a GPS API with three core operations: `locate(symbol_name)`, `get_neighborhood(file_path)`, and `workspace_summary()`.
- Introduce a **Copilot agent plugin** that packages the GPS API as skills, allowing GitHub Copilot to leverage structural codebase awareness in its prompts and responses.

## Capabilities

### New Capabilities
- `fs-watcher`: Filesystem monitoring with incremental diff detection — only re-indexes changed files.
- `structural-parser`: Tree-sitter-based AST/CST extraction for Java, TypeScript, Python, Rust, and C#/.NET.
- `symbol-index`: Embedded symbol database with low-footprint storage and sub-10ms query resolution for definitions and type hierarchies.
- `gps-api`: Local JSON-RPC server exposing `locate`, `get_neighborhood`, and `workspace_summary` endpoints.
- `copilot-plugin`: VS Code Copilot agent plugin packaging GPS API skills for LLM consumption.

### Modified Capabilities
<!-- No existing capabilities to modify — this is a greenfield project. -->

## Impact

- **New binary crate**: `g-p-source` Rust binary with async runtime (tokio).
- **Dependencies**: `notify`, `tree-sitter` (+ language grammars for 5 languages), `sled` or `rocksdb`, `tower`/`jsonrpsee` for JSON-RPC, `tokio` for async.
- **VS Code extension**: A Copilot agent plugin (TypeScript) that communicates with the local JSON-RPC server and exposes skills (`locate`, `get_neighborhood`, `workspace_summary`).
- **System requirements**: The engine targets < 100 MB RSS for a 100k-file workspace; all queries must resolve in < 10ms p99.
- **APIs**: New JSON-RPC 2.0 API surface (local, not networked beyond localhost).
