## ADDED Requirements

### Requirement: VS Code extension packaging
The system SHALL be packaged as a VS Code extension (TypeScript) that registers as a GitHub Copilot agent plugin, declaring skills in `package.json` under the Copilot agent contribution point.

#### Scenario: Extension activation
- **WHEN** the VS Code extension is activated in a workspace
- **THEN** it SHALL start the `g-p-source` binary as a managed child process (or connect to an already-running instance) and register all GPS skills with the Copilot agent runtime

#### Scenario: Extension deactivation
- **WHEN** the VS Code extension is deactivated
- **THEN** it SHALL gracefully shut down the `g-p-source` child process if it was started by the extension

### Requirement: Locate skill
The extension SHALL expose a `locate` Copilot skill that accepts a symbol name, calls the `locate` JSON-RPC method, and returns a formatted response with file path, line number, and column for each match.

#### Scenario: LLM invokes locate skill
- **WHEN** the Copilot agent invokes the `locate` skill with argument `"UserService"`
- **THEN** the skill SHALL return a structured result like `UserService — src/service.java:15:14 (class, public)`

#### Scenario: Locate skill with no results
- **WHEN** the Copilot agent invokes `locate` with a symbol that does not exist in the index
- **THEN** the skill SHALL return a message indicating no matches were found

### Requirement: Get neighborhood skill
The extension SHALL expose a `get_neighborhood` Copilot skill that accepts a file path and returns the dependency neighborhood (imports, importers, symbols) by calling the `get_neighborhood` JSON-RPC method.

#### Scenario: LLM invokes get_neighborhood skill
- **WHEN** the Copilot agent invokes `get_neighborhood` with `"src/service.py"`
- **THEN** the skill SHALL return a structured list of imported files, importing files, and key symbols in each related file

### Requirement: Workspace summary skill
The extension SHALL expose a `workspace_summary` Copilot skill that calls the `workspace_summary` JSON-RPC method and returns a condensed workspace map suitable for injection into the LLM system prompt.

#### Scenario: LLM invokes workspace_summary skill
- **WHEN** the Copilot agent invokes `workspace_summary`
- **THEN** the skill SHALL return a text block containing language distribution, module structure, and top public API symbols, formatted for LLM consumption (compact, token-efficient)

### Requirement: Connection management
The extension SHALL handle connection failures to the JSON-RPC server gracefully, with automatic reconnection (exponential backoff, max 5 retries) and user-visible status bar indicator.

#### Scenario: Server not running
- **WHEN** the extension attempts to connect and the `g-p-source` server is not running
- **THEN** the extension SHALL attempt to start the server binary, and if that fails, show a warning in the VS Code status bar

#### Scenario: Server connection lost
- **WHEN** the JSON-RPC connection is lost during operation
- **THEN** the extension SHALL retry connection with exponential backoff (1s, 2s, 4s, 8s, 16s) and update the status bar to indicate "GPS: Reconnecting..."

### Requirement: Auth token handling
The extension SHALL read the authentication token from `~/.gps/auth-token` and include it as a `Bearer` token in all JSON-RPC HTTP requests to the server.

#### Scenario: Auth token available
- **WHEN** the extension starts and `~/.gps/auth-token` exists
- **THEN** all subsequent JSON-RPC requests SHALL include the `Authorization: Bearer <token>` header

#### Scenario: Auth token missing
- **WHEN** the extension starts and `~/.gps/auth-token` does not exist
- **THEN** the extension SHALL wait for the server to start and create the token file, retrying for up to 10 seconds before showing an error
