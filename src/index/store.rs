use super::types::{ChangeEntry, FileMetadata, SymbolRecord, WordLocation};
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

    /// List all indexed files with their metadata (for file_tree).
    fn list_all_files(&self) -> anyhow::Result<Vec<(String, FileMetadata)>>;

    /// Get files ordered by most recent mtime (for hot_files).
    fn hot_files(&self, limit: usize) -> anyhow::Result<Vec<(String, FileMetadata)>>;

    /// Insert word index entries for a file.
    fn upsert_word_index(&self, file_path: &str, words: &[WordLocation]) -> anyhow::Result<()>;

    /// Remove word index entries for a file.
    fn remove_word_index(&self, file_path: &str) -> anyhow::Result<()>;

    /// Lookup a word in the inverted index.
    fn lookup_word(&self, word: &str) -> anyhow::Result<Vec<WordLocation>>;

    /// Insert trigram index entries for a file.
    fn upsert_trigram_index(&self, file_path: &str, trigrams: &[String]) -> anyhow::Result<()>;

    /// Remove trigram index entries for a file.
    fn remove_trigram_index(&self, file_path: &str) -> anyhow::Result<()>;

    /// Search for files matching all given trigrams (intersection).
    fn search_trigrams(&self, trigrams: &[String]) -> anyhow::Result<Vec<String>>;

    /// Record a change to the changes log and return the new sequence number.
    fn record_change(&self, file_path: &str, op: super::types::ChangeOp) -> anyhow::Result<u64>;

    /// Get all changes since a given sequence number.
    fn changes_since(&self, seq: u64) -> anyhow::Result<Vec<ChangeEntry>>;

    /// Get the current (latest) sequence number.
    fn current_seq(&self) -> anyhow::Result<u64>;
}
