## ADDED Requirements

### Requirement: JSON-RPC 2.0 server
The system SHALL expose a JSON-RPC 2.0 server over HTTP on `127.0.0.1` with a configurable port (default: 9741).

#### Scenario: Server startup
- **WHEN** the g-p-source binary starts
- **THEN** a JSON-RPC server SHALL begin listening on `127.0.0.1:9741` (or the configured port) and log the bound address

#### Scenario: Invalid JSON-RPC request
- **WHEN** a client sends a malformed JSON-RPC request
- **THEN** the server SHALL respond with a standard JSON-RPC error (code -32700 for parse error, -32600 for invalid request)

### Requirement: locate method
The system SHALL expose a `locate` JSON-RPC method that accepts a `symbol_name` (string) parameter and returns an array of matching symbol locations, each containing `file` (string), `line` (integer, 1-based), `col` (integer, 1-based), `kind` (string), and `qualified_name` (string).

#### Scenario: Locate an existing symbol
- **WHEN** a client calls `locate("UserService")` and `UserService` is defined in `src/service.java` at line 15, column 14
- **THEN** the response SHALL include `{"file": "src/service.java", "line": 15, "col": 14, "kind": "class", "qualified_name": "com.example.UserService"}`

#### Scenario: Locate with no matches
- **WHEN** a client calls `locate("NonExistentSymbol")`
- **THEN** the response SHALL be an empty array `[]`

#### Scenario: Locate with multiple matches
- **WHEN** multiple symbols match the name `Config` across different files
- **THEN** the response SHALL return all matches, ordered by visibility (public first) then alphabetically by qualified name

### Requirement: get_neighborhood method
The system SHALL expose a `get_neighborhood` JSON-RPC method that accepts a `file_path` (string) and an optional `depth` (integer, default: 1) and returns the direct dependencies and reverse dependents of the file, along with the symbols defined in each related file.

#### Scenario: Neighborhood of a file with imports
- **WHEN** a client calls `get_neighborhood("src/service.py")` and `service.py` imports `model.py` and is imported by `controller.py`
- **THEN** the response SHALL include `imports: ["src/model.py"]`, `imported_by: ["src/controller.py"]`, and a `symbols` map keyed by file path containing the symbol list for each related file

#### Scenario: Neighborhood of an isolated file
- **WHEN** a client calls `get_neighborhood("src/standalone.py")` and the file has no imports or importers
- **THEN** the response SHALL return empty `imports` and `imported_by` arrays, with only the file's own symbols

### Requirement: workspace_summary method
The system SHALL expose a `workspace_summary` JSON-RPC method that takes no parameters and returns a condensed workspace map containing: total file count, file count by language, top-level module/package structure, and a list of public API symbols (exported classes, functions, traits).

#### Scenario: Summary of a multi-language workspace
- **WHEN** a client calls `workspace_summary()` on a workspace with 200 Java files, 150 TypeScript files, and 50 Python files
- **THEN** the response SHALL include `total_files: 400`, language counts, module tree, and a list of public symbols capped at a configurable limit (default: 500)

#### Scenario: Summary after file deletion
- **WHEN** a file is deleted and the index is updated, then `workspace_summary()` is called
- **THEN** the response SHALL reflect the updated counts and no longer include symbols from the deleted file

### Requirement: Authentication
The system SHALL require a shared secret token (passed as a `Bearer` token in the `Authorization` HTTP header) for all JSON-RPC requests. The token SHALL be generated at startup and written to a well-known file (`~/.gps/auth-token`).

#### Scenario: Valid token
- **WHEN** a client sends a request with a valid `Authorization: Bearer <token>` header
- **THEN** the server SHALL process the request normally

#### Scenario: Missing or invalid token
- **WHEN** a client sends a request without an `Authorization` header or with an invalid token
- **THEN** the server SHALL respond with HTTP 401 Unauthorized

### Requirement: Localhost-only binding
The system SHALL bind exclusively to `127.0.0.1` and SHALL NOT accept connections from external network interfaces.

#### Scenario: External connection attempt
- **WHEN** an external client attempts to connect to the server via the machine's public IP
- **THEN** the connection SHALL be refused at the TCP level
