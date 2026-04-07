## ADDED Requirements

### Requirement: Filesystem event detection
The system SHALL use the `notify` crate to watch the workspace root recursively for file create, modify, delete, and rename events across macOS (FSEvents), Linux (inotify), and Windows (ReadDirectoryChangesW).

#### Scenario: File created in workspace
- **WHEN** a new source file is created within the watched workspace
- **THEN** the system SHALL emit a create event containing the absolute file path within 200ms

#### Scenario: File modified in workspace
- **WHEN** an existing source file is saved with changes
- **THEN** the system SHALL emit a modify event containing the absolute file path within 200ms

#### Scenario: File deleted from workspace
- **WHEN** a source file is deleted from the workspace
- **THEN** the system SHALL emit a delete event containing the absolute file path within 200ms

#### Scenario: File renamed in workspace
- **WHEN** a source file is renamed within the workspace
- **THEN** the system SHALL emit a rename event (or delete+create pair) containing both the old and new file paths

### Requirement: Event debouncing
The system SHALL debounce filesystem events with a configurable window (default: 100ms) to coalesce rapid successive edits to the same file into a single re-index trigger.

#### Scenario: Rapid successive saves
- **WHEN** a file is saved 5 times within 50ms
- **THEN** the system SHALL produce exactly one re-index event for that file

### Requirement: Path filtering
The system SHALL ignore events from configurable excluded paths (default: `.git/`, `node_modules/`, `target/`, `bin/`, `obj/`, `build/`, `dist/`, `__pycache__/`).

#### Scenario: Change in excluded directory
- **WHEN** a file inside `node_modules/` is modified
- **THEN** the system SHALL NOT emit any event or trigger re-indexing

#### Scenario: Change in included directory
- **WHEN** a file inside `src/` is modified
- **THEN** the system SHALL emit a modify event normally

### Requirement: Incremental diff detection
The system SHALL compare the current file content hash against the stored hash in `meta:file` and skip re-indexing if the content is unchanged (e.g., touch without modification).

#### Scenario: File touched without content change
- **WHEN** a file's mtime changes but its content hash is identical to the stored hash
- **THEN** the system SHALL update the mtime in metadata but SHALL NOT trigger re-parsing

#### Scenario: File content actually changed
- **WHEN** a file's content hash differs from the stored hash
- **THEN** the system SHALL trigger a full re-parse and symbol diff for that file
