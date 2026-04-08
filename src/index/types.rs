use serde::{Deserialize, Serialize};

/// A stored symbol record in the index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolRecord {
    pub kind: String,
    pub file: String,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub visibility: String,
    pub parent: Option<String>,
}

/// A lightweight reference from file→symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolRef {
    pub qualified_name: String,
}

/// Metadata stored per indexed file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub mtime: u64,
    pub hash: Vec<u8>,
    pub symbol_count: u32,
    /// Language identifier (e.g. "java", "typescript", "python", "rust", "csharp")
    #[serde(default)]
    pub language: String,
    /// Total number of lines in the file
    #[serde(default)]
    pub line_count: u32,
}

/// A tracked change entry for the changes log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEntry {
    pub seq: u64,
    pub file_path: String,
    pub operation: ChangeOp,
    pub timestamp: u64,
}

/// The type of change operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeOp {
    Upsert,
    Remove,
}

/// A word index entry: file path + line number.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordLocation {
    pub file: String,
    pub line: u32,
}
