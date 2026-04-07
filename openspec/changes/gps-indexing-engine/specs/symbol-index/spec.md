## ADDED Requirements

### Requirement: Symbol definition storage
The system SHALL store symbol definitions in a `sym:def` tree keyed by qualified name, with values containing kind, file path, byte range, visibility, and parent symbol reference.

#### Scenario: Store and retrieve a symbol
- **WHEN** a symbol `com.example.UserService.findById` is indexed
- **THEN** a lookup on `sym:def` with key `com.example.UserService.findById` SHALL return a `SymbolRecord` with the correct file, range, kind, and visibility

#### Scenario: Overwrite on re-index
- **WHEN** a file is re-indexed and a symbol's line range has changed
- **THEN** the `sym:def` entry SHALL be updated with the new range

### Requirement: File-to-symbol index
The system SHALL maintain a `sym:file` tree keyed by `{file_path}\x00{symbol_name}` to enable efficient listing of all symbols in a given file.

#### Scenario: List symbols in a file
- **WHEN** a query requests all symbols in `src/main.rs`
- **THEN** a prefix scan on `sym:file` with prefix `src/main.rs\x00` SHALL return all symbols defined in that file

### Requirement: Kind-based index
The system SHALL maintain a `sym:kind` tree keyed by `{kind}\x00{qualified_name}` to support queries like "all classes" or "all interfaces".

#### Scenario: List all classes
- **WHEN** a query requests all symbols of kind `class`
- **THEN** a prefix scan on `sym:kind` with prefix `class\x00` SHALL return all class symbols in the workspace

### Requirement: Import dependency tracking
The system SHALL maintain `dep:import` (file → imported files) and `dep:reverse` (imported file → importing files) trees to track inter-file dependencies.

#### Scenario: Direct dependency lookup
- **WHEN** `src/service.py` imports `src/model.py` and `src/utils.py`
- **THEN** a prefix scan on `dep:import` with key prefix `src/service.py\x00` SHALL return both `src/model.py` and `src/utils.py`

#### Scenario: Reverse dependency lookup
- **WHEN** `src/model.py` is imported by `src/service.py` and `src/controller.py`
- **THEN** a prefix scan on `dep:reverse` with key prefix `src/model.py\x00` SHALL return both importing files

### Requirement: File metadata tracking
The system SHALL store file metadata (mtime, content hash, symbol count) in a `meta:file` tree to support incremental indexing decisions.

#### Scenario: Check file freshness
- **WHEN** the watcher reports a modify event for `src/lib.rs`
- **THEN** the system SHALL compare the current content hash against the stored hash in `meta:file` before deciding to re-parse

### Requirement: Sub-10ms query latency
All read queries (`locate`, symbol listing, dependency lookups) SHALL resolve in under 10ms at p99 for a workspace with up to 100k indexed files and 1M symbols.

#### Scenario: Locate query under load
- **WHEN** the index contains 500k symbols across 50k files
- **THEN** a `locate("UserService")` query SHALL return results in under 10ms

### Requirement: Atomic batch writes
The system SHALL apply all symbol updates for a single file as an atomic batch write to ensure index consistency (no partial updates visible to readers).

#### Scenario: Crash during index write
- **WHEN** the process crashes mid-write while updating symbols for `src/main.rs`
- **THEN** the index SHALL either contain the complete old set of symbols or the complete new set — never a partial mix

### Requirement: Low memory footprint
The sled database SHALL be configured with a cache capacity that keeps total process RSS under 100 MB for a workspace of 100k files.

#### Scenario: Large workspace memory usage
- **WHEN** the system indexes a workspace with 100k source files and 1M symbols
- **THEN** the process RSS SHALL remain under 100 MB
