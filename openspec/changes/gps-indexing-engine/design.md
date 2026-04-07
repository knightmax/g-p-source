## Context

g-p-source is a greenfield Rust project. There is no existing codebase — this is the initial architecture for a high-performance, incremental code indexing engine designed to feed structural codebase awareness to LLM-powered coding assistants (specifically GitHub Copilot). The engine must maintain a live symbol database across large workspaces with minimal memory overhead and sub-10ms query latency.

Key constraints:
- Must handle workspaces up to 100k files while staying under 100 MB RSS.
- Must support five language ecosystems: Java, TypeScript, Python, Rust, C#/.NET.
- Must expose a JSON-RPC API on localhost for LLM consumption.
- Must be packaged as a VS Code Copilot agent plugin with skills.

## Goals / Non-Goals

**Goals:**
- Design an asynchronous crawling engine that watches the filesystem and incrementally re-indexes only changed files.
- Define the symbol indexation schema covering definitions, references, type hierarchies, imports, and semantic proximity.
- Achieve sub-10ms p99 latency for `locate`, `get_neighborhood`, and `workspace_summary` queries.
- Keep memory consumption under 100 MB RSS for a 100k-file workspace.
- Provide a Copilot-consumable plugin that wraps the GPS API as agent skills.

**Non-Goals:**
- Cross-repository indexing or remote workspace support.
- Full semantic analysis (type inference, flow analysis) — we rely on structural/syntactic extraction only.
- Language Server Protocol (LSP) compliance — this is a complementary system, not a replacement for LSP servers.
- Real-time collaborative editing support.
- Support for languages beyond the initial five.

## Decisions

### D1: Async Runtime — Tokio

**Choice**: Use `tokio` as the async runtime.
**Rationale**: Tokio is the de facto standard for async Rust. It provides the multi-threaded work-stealing scheduler needed for concurrent file parsing and the I/O primitives for the JSON-RPC server. Alternatives like `async-std` lack the ecosystem depth (tower, tonic, jsonrpsee all target tokio).

### D2: Filesystem Watching — `notify` crate

**Choice**: Use `notify` v6+ with debounced events.
**Rationale**: `notify` is the most mature cross-platform FS watcher in Rust. It supports inotify (Linux), FSEvents (macOS), and ReadDirectoryChangesW (Windows). We'll use debounced mode (100ms window) to batch rapid edits into a single re-index pass. Alternative: polling — rejected for latency and CPU cost on large trees.

**Event flow**:
```
FS event → debounce (100ms) → classify (create/modify/delete) → enqueue(PathBuf)
→ async worker pool → parse → diff symbols → update index
```

### D3: Structural Parsing — Tree-sitter

**Choice**: Use `tree-sitter` with pre-compiled grammar crates for each language.
**Rationale**: Tree-sitter provides incremental, error-tolerant parsing that produces concrete syntax trees. It is far lighter than full compiler frontends and supports all five target languages. The incremental parsing mode allows re-parsing only the edited region of a file when the old tree is cached.

**Grammar crates**: `tree-sitter-java`, `tree-sitter-typescript`, `tree-sitter-python`, `tree-sitter-rust`, `tree-sitter-c-sharp`.

**Symbol extraction strategy**:
- Walk the CST using tree-sitter queries (S-expressions) per language.
- Extract: function/method definitions, class/struct/enum/interface declarations, module/namespace declarations, import/use statements, trait/interface implementations.
- Output: `Symbol { name, kind, file, range, parent_symbol, visibility }`.

### D4: Symbol Storage — sled embedded database

**Choice**: Use `sled` as the embedded key-value store.
**Rationale**: sled is a pure-Rust, zero-config embedded database with excellent read performance (lock-free concurrent reads) and a small memory footprint. It supports prefix scans, which are critical for symbol name resolution. RocksDB was considered but rejected for its C++ dependency, larger binary size, and more complex build chain. If sled proves insufficient at scale, migration to RocksDB is straightforward since we'll abstract behind a `SymbolStore` trait.

**Index schema** (key → value design):

| Tree | Key | Value |
|------|-----|-------|
| `sym:def` | `{qualified_name}` | `SymbolRecord { kind, file, range, visibility, parent }` |
| `sym:file` | `{file_path}\x00{symbol_name}` | `SymbolRef { qualified_name }` |
| `sym:kind` | `{kind}\x00{qualified_name}` | `()` (presence key) |
| `dep:import` | `{file_path}\x00{imported_path}` | `()` |
| `dep:reverse` | `{imported_path}\x00{file_path}` | `()` |
| `meta:file` | `{file_path}` | `FileMetadata { mtime, hash, symbol_count }` |

All values are serialized with `bincode` for compact binary representation.

### D5: Query Resolution Strategy

- **`locate(symbol_name)`**: Prefix scan on `sym:def` tree. If ambiguous, return ranked results by visibility (public > private) and proximity to caller file.
- **`get_neighborhood(file_path)`**: Read `dep:import` for direct dependencies, `dep:reverse` for reverse dependents, then gather symbols from each related file. Cap at configurable depth (default: 1 hop).
- **`workspace_summary()`**: Maintained as a pre-computed artifact updated on each index cycle. Stores: file count by language, top-level module/package structure, public API surface (exported symbols). Serialized as a compressed JSON blob in sled under `meta:summary`.

### D6: JSON-RPC Server — jsonrpsee

**Choice**: Use `jsonrpsee` for the JSON-RPC 2.0 server.
**Rationale**: `jsonrpsee` is a high-performance, tokio-native JSON-RPC framework supporting HTTP and WebSocket transports. It integrates cleanly with tower middleware for logging and rate limiting. Alternatives: `tarpc` (not JSON-RPC standard), hand-rolled (unnecessary complexity).

**Transport**: HTTP on `127.0.0.1:<port>` (port configurable, default 9741). WebSocket upgrade supported for streaming use cases.

### D7: Crawling Architecture — Actor-style pipeline

**Architecture**:
```
                    ┌─────────────┐
                    │  FS Watcher │
                    │   (notify)  │
                    └──────┬──────┘
                           │ PathBuf events
                    ┌──────▼──────┐
                    │  Dispatcher │
                    │  (channel)  │
                    └──────┬──────┘
                           │ fan-out
               ┌───────────┼───────────┐
               ▼           ▼           ▼
         ┌──────────┐┌──────────┐┌──────────┐
         │ Parser 1 ││ Parser 2 ││ Parser N │
         │(tokio    ││(tokio    ││(tokio    │
         │ spawn_   ││ spawn_   ││ spawn_   │
         │ blocking)││ blocking)││ blocking)│
         └─────┬────┘└─────┬────┘└─────┬────┘
               │           │           │
               └───────────┼───────────┘
                           ▼
                    ┌──────────────┐
                    │  Index Writer│
                    │  (sled batch)│
                    └──────┬───────┘
                           │
                    ┌──────▼──────┐
                    │  Symbol DB  │
                    │   (sled)    │
                    └─────────────┘
```

- **FS Watcher**: Single long-lived task. Sends `(EventKind, PathBuf)` over an `mpsc` channel.
- **Dispatcher**: Receives events, deduplicates by path, classifies by language (extension mapping), drops ignored paths (`.git`, `node_modules`, `target/`, `bin/`).
- **Parser Pool**: `spawn_blocking` tasks (tree-sitter is CPU-bound). Pool size = `num_cpus::get()`. Each parser loads the appropriate grammar, parses, extracts symbols, computes a diff against the previous symbols for that file.
- **Index Writer**: Single task consuming symbol diffs. Applies batch writes to sled. Updates `meta:file` and triggers `workspace_summary` recomputation (debounced).

### D8: Memory Budget Strategy

| Component | Budget | Strategy |
|-----------|--------|----------|
| sled mmap | ~40 MB | sled's default cache; tunable via `cache_capacity` |
| Tree-sitter parsers | ~5 MB | One parser instance per language, reused across files |
| AST nodes (transient) | ~10 MB peak | Parsed trees are dropped after symbol extraction |
| File metadata cache | ~15 MB | In-memory HashMap for 100k entries (~150 bytes each) |
| Channel buffers | ~5 MB | Bounded channels (4096 entries) |
| Headroom | ~25 MB | For spikes, OS overhead |
| **Total** | **< 100 MB** | |

### D9: Copilot Plugin Architecture

**Choice**: VS Code extension (TypeScript) acting as a Copilot agent plugin.
**Design**:
- The extension starts the `g-p-source` binary as a child process (or connects to an already-running instance).
- Exposes three Copilot skills:
  - `locate`: Calls `locate` JSON-RPC method, formats result as file:line:col.
  - `get_neighborhood`: Calls `get_neighborhood`, returns structured dependency map.
  - `workspace_summary`: Calls `workspace_summary`, returns condensed codebase map for system prompt injection.
- Skills are declared in the extension's `package.json` under the Copilot agent contribution point.
- Communication: HTTP to `127.0.0.1:9741` with a shared secret token for authentication.

## Risks / Trade-offs

- **[sled maturity]** sled is pre-1.0 and its maintainer has reduced activity. → **Mitigation**: Abstract storage behind a `SymbolStore` trait. If sled stalls, swap for RocksDB or redb with minimal refactoring.
- **[Tree-sitter grammar drift]** Language grammars may lag behind latest syntax (e.g., new Java features). → **Mitigation**: Pin grammar versions; update on a quarterly cadence. Error-tolerant parsing means partial parses still yield symbols.
- **[Large monorepo performance]** 100k+ files with deep dependency graphs may spike memory during initial indexing. → **Mitigation**: Initial crawl uses a bounded semaphore (max 32 concurrent parses). Stream results to sled instead of buffering.
- **[Cross-platform FS events]** notify behavior varies across OS (event ordering, rename semantics). → **Mitigation**: Normalize events in the dispatcher; use file hash comparison for ambiguous cases.
- **[Copilot API stability]** The Copilot agent/skill API is evolving and may introduce breaking changes. → **Mitigation**: Keep the plugin thin — it's a transport layer over JSON-RPC. Core logic remains in the Rust binary.
