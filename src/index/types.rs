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
}
