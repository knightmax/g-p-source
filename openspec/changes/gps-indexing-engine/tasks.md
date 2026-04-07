## 1. Project Scaffold & Dependencies

- [x] 1.1 Initialize Rust workspace with `cargo init` and configure `Cargo.toml` with edition 2021
- [x] 1.2 Add core dependencies: `tokio`, `notify`, `tree-sitter`, `sled`, `jsonrpsee`, `bincode`, `serde`, `num_cpus`, `blake3` (hashing)
- [x] 1.3 Add tree-sitter grammar crates: `tree-sitter-java`, `tree-sitter-typescript`, `tree-sitter-python`, `tree-sitter-rust`, `tree-sitter-c-sharp`
- [x] 1.4 Create module structure: `src/{watcher, parser, index, api, config}.rs` and `src/main.rs` entry point
- [x] 1.5 Set up configuration loading (CLI args + optional config file) for port, exclude paths, cache capacity, workspace root

## 2. Filesystem Watcher

- [x] 2.1 Implement `FsWatcher` struct wrapping `notify::RecommendedWatcher` with debounced events (100ms window)
- [x] 2.2 Implement path filtering logic: exclude `.git/`, `node_modules/`, `target/`, `bin/`, `obj/`, `build/`, `dist/`, `__pycache__/` and configurable patterns
- [x] 2.3 Implement event classification: map `notify` events to internal `FsEvent { kind: Create|Modify|Delete|Rename, path }` enum
- [x] 2.4 Wire watcher output to bounded `mpsc` channel (capacity 4096) for dispatcher consumption
- [x] 2.5 Implement initial workspace crawl: walk directory tree on startup, enqueue all supported files for first-time indexing

## 3. Structural Parser

- [x] 3.1 Create `LanguageRegistry` mapping file extensions to tree-sitter `Language` objects for all 5 supported languages
- [x] 3.2 Define `Symbol` struct: `{ name, qualified_name, kind, file, range: (start_line, start_col, end_line, end_col), parent, visibility }`
- [x] 3.3 Write tree-sitter S-expression queries for Java symbol extraction (classes, methods, fields, imports, interfaces, enums)
- [x] 3.4 Write tree-sitter queries for TypeScript/TSX symbol extraction (classes, functions, interfaces, imports, type aliases, exports)
- [x] 3.5 Write tree-sitter queries for Python symbol extraction (classes, functions, imports, module-level assignments)
- [x] 3.6 Write tree-sitter queries for Rust symbol extraction (structs, enums, traits, impls, functions, modules, use statements)
- [x] 3.7 Write tree-sitter queries for C# symbol extraction (classes, interfaces, methods, namespaces, using statements)
- [x] 3.8 Implement `Parser` struct that holds a `tree_sitter::Parser` per language, supports incremental re-parsing with cached old trees
- [x] 3.9 Implement `extract_symbols(tree, source, lang) -> Vec<Symbol>` using the query system
- [x] 3.10 Implement symbol diff: compare old symbols vs new symbols for a file, produce `SymbolDiff { added, removed, modified }`

## 4. Symbol Index (sled)

- [x] 4.1 Define `SymbolStore` trait with methods: `upsert_file_symbols`, `remove_file`, `locate`, `symbols_in_file`, `symbols_by_kind`, `get_imports`, `get_importers`, `get_file_meta`, `set_file_meta`
- [x] 4.2 Implement `SledStore` opening sled DB with 6 trees: `sym:def`, `sym:file`, `sym:kind`, `dep:import`, `dep:reverse`, `meta:file`
- [x] 4.3 Implement `upsert_file_symbols` using sled atomic batch: remove old symbols for file, insert new symbols across all trees
- [x] 4.4 Implement `remove_file`: atomic removal of all entries for a deleted file across all trees
- [x] 4.5 Implement `locate(name)`: prefix scan on `sym:def`, rank by visibility and return results
- [x] 4.6 Implement `get_imports` / `get_importers`: prefix scans on `dep:import` and `dep:reverse`
- [x] 4.7 Implement `FileMetadata` storage: mtime, blake3 content hash, symbol count in `meta:file` tree
- [x] 4.8 Configure sled cache capacity to target < 40 MB for the database layer

## 5. Async Crawling Pipeline

- [x] 5.1 Implement `Dispatcher` task: consume from watcher channel, deduplicate paths, classify by language, drop unsupported files
- [x] 5.2 Implement parser worker pool using `tokio::task::spawn_blocking` with bounded semaphore (max `num_cpus` concurrent parses)
- [x] 5.3 Implement content hash check: read file, compute blake3 hash, compare with `meta:file` stored hash, skip if unchanged
- [x] 5.4 Implement `IndexWriter` task: consume `SymbolDiff` results from parser workers, apply batch writes to sled store
- [x] 5.5 Implement workspace summary pre-computation: after each index write batch, update `meta:summary` (debounced, 500ms)
- [x] 5.6 Wire the full pipeline: watcher → dispatcher → parser pool → index writer, with graceful shutdown on SIGINT/SIGTERM

## 6. GPS JSON-RPC API

- [x] 6.1 Set up `jsonrpsee` HTTP server bound to `127.0.0.1:9741` (configurable port)
- [x] 6.2 Implement auth token generation at startup: write random token to `~/.gps/auth-token`, add tower middleware to validate `Bearer` token
- [x] 6.3 Implement `locate(symbol_name) -> Vec<SymbolLocation>` RPC method delegating to `SymbolStore::locate`
- [x] 6.4 Implement `get_neighborhood(file_path, depth?) -> Neighborhood` RPC method: gather imports, importers, and their symbols up to depth
- [x] 6.5 Implement `workspace_summary() -> WorkspaceSummary` RPC method: read pre-computed summary from sled `meta:summary`
- [x] 6.6 Add proper JSON-RPC error codes: -32700 (parse error), -32600 (invalid request), -32601 (method not found), -32602 (invalid params)

## 7. Copilot Agent Plugin (VS Code Extension)

- [x] 7.1 Scaffold VS Code extension project (TypeScript): `package.json`, `tsconfig.json`, extension entry point
- [x] 7.2 Configure `package.json` with Copilot agent contribution point declaring `locate`, `get_neighborhood`, `workspace_summary` skills
- [x] 7.3 Implement `g-p-source` binary lifecycle management: start as child process, monitor health, restart on crash
- [x] 7.4 Implement auth token reader: read `~/.gps/auth-token`, retry with backoff for up to 10 seconds if missing
- [x] 7.5 Implement JSON-RPC HTTP client with auth header injection and connection retry (exponential backoff: 1s, 2s, 4s, 8s, 16s)
- [x] 7.6 Implement `locate` skill handler: call RPC, format results as `Symbol — file:line:col (kind, visibility)`
- [x] 7.7 Implement `get_neighborhood` skill handler: call RPC, format as structured dependency map
- [x] 7.8 Implement `workspace_summary` skill handler: call RPC, format as compact token-efficient text block
- [x] 7.9 Add VS Code status bar indicator: "GPS: Ready", "GPS: Indexing...", "GPS: Reconnecting...", "GPS: Error"
- [x] 7.10 Package extension as `.vsix` for distribution

## 8. Testing & Validation

- [x] 8.1 Unit tests for each tree-sitter query set: verify symbol extraction against known source files per language
- [x] 8.2 Unit tests for `SymbolStore` trait implementation: CRUD operations, prefix scans, atomic batch consistency
- [x] 8.3 Integration test for the full pipeline: create temp workspace, add/modify/delete files, verify index state
- [x] 8.4 Integration test for JSON-RPC API: start server, call all 3 methods, verify responses and auth enforcement
- [x] 8.5 Benchmark: `locate` query latency on index with 500k symbols (target < 10ms p99)
- [x] 8.6 Benchmark: memory usage with 100k-file synthetic workspace (target < 100 MB RSS)
- [x] 8.7 End-to-end test: VS Code extension starts server, invokes each skill, verifies formatted output
