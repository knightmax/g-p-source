## ADDED Requirements

### Requirement: Multi-language grammar support
The system SHALL support structural parsing of Java, TypeScript (including TSX), Python, Rust, and C#/.NET source files using tree-sitter grammars.

#### Scenario: Parse a Java file
- **WHEN** a `.java` file is submitted for parsing
- **THEN** the system SHALL produce a concrete syntax tree using the `tree-sitter-java` grammar

#### Scenario: Parse a TypeScript file
- **WHEN** a `.ts` or `.tsx` file is submitted for parsing
- **THEN** the system SHALL produce a concrete syntax tree using the `tree-sitter-typescript` grammar

#### Scenario: Parse a Python file
- **WHEN** a `.py` file is submitted for parsing
- **THEN** the system SHALL produce a concrete syntax tree using the `tree-sitter-python` grammar

#### Scenario: Parse a Rust file
- **WHEN** a `.rs` file is submitted for parsing
- **THEN** the system SHALL produce a concrete syntax tree using the `tree-sitter-rust` grammar

#### Scenario: Parse a C# file
- **WHEN** a `.cs` file is submitted for parsing
- **THEN** the system SHALL produce a concrete syntax tree using the `tree-sitter-c-sharp` grammar

#### Scenario: Unsupported file type
- **WHEN** a file with an unrecognized extension (e.g., `.txt`, `.md`) is submitted for parsing
- **THEN** the system SHALL skip parsing and log a debug-level message

### Requirement: Symbol extraction from AST
The system SHALL extract structural symbols from the CST/AST using tree-sitter queries, producing `Symbol` records with: name, kind, file path, byte range (start line/col, end line/col), parent symbol (if nested), and visibility.

#### Scenario: Extract class definition
- **WHEN** a Java file contains `public class UserService { ... }`
- **THEN** the system SHALL extract a symbol with `name=UserService`, `kind=class`, `visibility=public`, and the correct file path and range

#### Scenario: Extract nested method
- **WHEN** a class `UserService` contains method `findById`
- **THEN** the system SHALL extract a symbol with `name=findById`, `kind=method`, `parent=UserService`

#### Scenario: Extract import statement
- **WHEN** a Python file contains `from os.path import join`
- **THEN** the system SHALL extract an import record linking the current file to `os.path`

### Requirement: Supported symbol kinds
The system SHALL extract the following symbol kinds: `function`, `method`, `class`, `struct`, `enum`, `interface`, `trait`, `module`, `namespace`, `import`, `type_alias`, `constant`.

#### Scenario: Rust trait extraction
- **WHEN** a Rust file contains `pub trait Serializable { ... }`
- **THEN** the system SHALL extract a symbol with `kind=trait`, `name=Serializable`, `visibility=public`

#### Scenario: TypeScript interface extraction
- **WHEN** a TypeScript file contains `export interface UserDTO { ... }`
- **THEN** the system SHALL extract a symbol with `kind=interface`, `name=UserDTO`, `visibility=public`

### Requirement: Error-tolerant parsing
The system SHALL produce partial symbol extraction results even when source files contain syntax errors, leveraging tree-sitter's error recovery.

#### Scenario: File with syntax error
- **WHEN** a Java file has an unclosed brace but contains 3 valid method definitions above the error
- **THEN** the system SHALL extract all 3 valid method symbols and log a warning about the parse error

### Requirement: Incremental re-parsing
The system SHALL cache the previous tree-sitter parse tree per file and use tree-sitter's incremental parsing mode when a file is modified, re-parsing only the edited region.

#### Scenario: Small edit to large file
- **WHEN** a 10,000-line file has a 2-line method body changed
- **THEN** the system SHALL perform an incremental re-parse that is at least 2x faster than a full parse of the same file
