use super::types::{FileMetadata, SymbolRecord};
use crate::parser::Symbol;

/// Trait abstracting the symbol storage backend.
pub trait SymbolStore: Send + Sync {
    fn upsert_file_symbols(&self, file_path: &str, symbols: &[Symbol]) -> anyhow::Result<()>;
    fn remove_file(&self, file_path: &str) -> anyhow::Result<()>;
    fn locate(&self, name: &str) -> anyhow::Result<Vec<SymbolRecord>>;
    fn symbols_in_file(&self, file_path: &str) -> anyhow::Result<Vec<SymbolRecord>>;
    #[allow(dead_code)]
    fn symbols_by_kind(&self, kind: &str) -> anyhow::Result<Vec<String>>;
    fn get_imports(&self, file_path: &str) -> anyhow::Result<Vec<String>>;
    fn get_importers(&self, file_path: &str) -> anyhow::Result<Vec<String>>;
    fn get_file_meta(&self, file_path: &str) -> anyhow::Result<Option<FileMetadata>>;
    fn set_file_meta(&self, file_path: &str, meta: &FileMetadata) -> anyhow::Result<()>;
}
