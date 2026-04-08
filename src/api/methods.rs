use crate::index::SymbolStore;
use jsonrpsee::core::RpcResult;
use jsonrpsee::core::async_trait;
use jsonrpsee::proc_macros::rpc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolLocation {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub kind: String,
    pub qualified_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neighborhood {
    pub file: String,
    pub imports: Vec<String>,
    pub imported_by: Vec<String>,
    pub symbols: std::collections::HashMap<String, Vec<SymbolLocation>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSummary {
    pub total_files: u64,
    pub files_by_language: std::collections::HashMap<String, u64>,
    pub modules: Vec<String>,
    pub public_symbols: Vec<SymbolLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub status: String,
    pub indexed: bool,
    pub workspace: String,
    pub port: u16,
    pub pid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTreeEntry {
    pub path: String,
    pub language: String,
    pub line_count: u32,
    pub symbol_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotFileEntry {
    pub path: String,
    pub language: String,
    pub mtime: u64,
    pub symbol_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadFileResponse {
    pub path: String,
    pub content: String,
    pub start_line: u32,
    pub end_line: u32,
    pub total_lines: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordHit {
    pub file: String,
    pub line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub file: String,
    pub matches: Vec<SearchMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
    pub line: u32,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeEntryResponse {
    pub seq: u64,
    pub file_path: String,
    pub operation: String,
    pub timestamp: u64,
}

#[rpc(server)]
pub trait GpsApi {
    #[method(name = "locate")]
    async fn locate(&self, symbol_name: String) -> RpcResult<Vec<SymbolLocation>>;

    #[method(name = "get_neighborhood")]
    async fn get_neighborhood(
        &self,
        file_path: String,
        depth: Option<u32>,
    ) -> RpcResult<Neighborhood>;

    #[method(name = "workspace_summary")]
    async fn workspace_summary(&self) -> RpcResult<WorkspaceSummary>;

    #[method(name = "status")]
    async fn status(&self) -> RpcResult<StatusResponse>;

    #[method(name = "file_tree")]
    async fn file_tree(&self) -> RpcResult<Vec<FileTreeEntry>>;

    #[method(name = "hot_files")]
    async fn hot_files(&self, limit: Option<u32>) -> RpcResult<Vec<HotFileEntry>>;

    #[method(name = "read_file")]
    async fn read_file(
        &self,
        path: String,
        start_line: Option<u32>,
        end_line: Option<u32>,
    ) -> RpcResult<ReadFileResponse>;

    #[method(name = "search")]
    async fn search(&self, query: String, max_results: Option<u32>) -> RpcResult<Vec<SearchResult>>;

    #[method(name = "word_lookup")]
    async fn word_lookup(&self, word: String) -> RpcResult<Vec<WordHit>>;

    #[method(name = "changes_since")]
    async fn changes_since(&self, seq: u64) -> RpcResult<Vec<ChangeEntryResponse>>;
}

pub struct GpsApiImpl<S: SymbolStore> {
    pub store: Arc<S>,
    pub indexed: Arc<AtomicBool>,
    pub workspace: String,
    pub port: u16,
}

fn rpc_err(msg: String) -> jsonrpsee::types::ErrorObjectOwned {
    jsonrpsee::types::ErrorObjectOwned::owned(-32603, msg, None::<()>)
}

#[async_trait]
impl<S: SymbolStore + 'static> GpsApiServer for GpsApiImpl<S> {
    async fn locate(&self, symbol_name: String) -> RpcResult<Vec<SymbolLocation>> {
        let records = self.store.locate(&symbol_name).map_err(|e| rpc_err(e.to_string()))?;

        Ok(records
            .into_iter()
            .map(|r| SymbolLocation {
                file: r.file,
                line: r.start_line,
                col: r.start_col,
                kind: r.kind,
                qualified_name: String::new(),
            })
            .collect())
    }

    async fn get_neighborhood(
        &self,
        file_path: String,
        _depth: Option<u32>,
    ) -> RpcResult<Neighborhood> {
        let imports = self.store.get_imports(&file_path).map_err(|e| rpc_err(e.to_string()))?;
        let imported_by = self
            .store
            .get_importers(&file_path)
            .map_err(|e| rpc_err(e.to_string()))?;

        let mut symbols = std::collections::HashMap::new();
        for related in imports.iter().chain(imported_by.iter()) {
            let file_symbols = self
                .store
                .symbols_in_file(related)
                .map_err(|e| rpc_err(e.to_string()))?;
            symbols.insert(
                related.clone(),
                file_symbols
                    .into_iter()
                    .map(|r| SymbolLocation {
                        file: r.file,
                        line: r.start_line,
                        col: r.start_col,
                        kind: r.kind,
                        qualified_name: String::new(),
                    })
                    .collect(),
            );
        }

        Ok(Neighborhood {
            file: file_path,
            imports,
            imported_by,
            symbols,
        })
    }

    async fn workspace_summary(&self) -> RpcResult<WorkspaceSummary> {
        let all_files = self.store.list_all_files().map_err(|e| rpc_err(e.to_string()))?;
        let total_files = all_files.len() as u64;
        let mut files_by_language = std::collections::HashMap::new();
        for (_, meta) in &all_files {
            if !meta.language.is_empty() {
                *files_by_language.entry(meta.language.clone()).or_insert(0u64) += 1;
            }
        }

        Ok(WorkspaceSummary {
            total_files,
            files_by_language,
            modules: Vec::new(),
            public_symbols: Vec::new(),
        })
    }

    async fn status(&self) -> RpcResult<StatusResponse> {
        Ok(StatusResponse {
            status: if self.indexed.load(Ordering::Relaxed) {
                "ready".to_string()
            } else {
                "indexing".to_string()
            },
            indexed: self.indexed.load(Ordering::Relaxed),
            workspace: self.workspace.clone(),
            port: self.port,
            pid: std::process::id(),
        })
    }

    async fn file_tree(&self) -> RpcResult<Vec<FileTreeEntry>> {
        let all_files = self.store.list_all_files().map_err(|e| rpc_err(e.to_string()))?;
        let mut entries: Vec<FileTreeEntry> = all_files
            .into_iter()
            .map(|(path, meta)| FileTreeEntry {
                path,
                language: meta.language,
                line_count: meta.line_count,
                symbol_count: meta.symbol_count,
            })
            .collect();
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    async fn hot_files(&self, limit: Option<u32>) -> RpcResult<Vec<HotFileEntry>> {
        let limit = limit.unwrap_or(20) as usize;
        let files = self.store.hot_files(limit).map_err(|e| rpc_err(e.to_string()))?;
        Ok(files
            .into_iter()
            .map(|(path, meta)| HotFileEntry {
                path,
                language: meta.language,
                mtime: meta.mtime,
                symbol_count: meta.symbol_count,
            })
            .collect())
    }

    async fn read_file(
        &self,
        path: String,
        start_line: Option<u32>,
        end_line: Option<u32>,
    ) -> RpcResult<ReadFileResponse> {
        // Block sensitive files
        if crate::sensitive::is_sensitive_file(std::path::Path::new(&path)) {
            return Err(rpc_err("Access denied: sensitive file".to_string()));
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| rpc_err(format!("Failed to read file: {}", e)))?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len() as u32;
        let start = start_line.unwrap_or(1).max(1) as usize;
        let end = end_line.unwrap_or(total_lines).min(total_lines) as usize;

        let selected: String = lines
            .get(start.saturating_sub(1)..end.min(lines.len()))
            .unwrap_or(&[])
            .join("\n");

        Ok(ReadFileResponse {
            path,
            content: selected,
            start_line: start as u32,
            end_line: end as u32,
            total_lines,
        })
    }

    async fn search(
        &self,
        query: String,
        max_results: Option<u32>,
    ) -> RpcResult<Vec<SearchResult>> {
        let max = max_results.unwrap_or(20) as usize;

        // Extract trigrams from query for index lookup
        let query_lower = query.to_lowercase();
        let chars: Vec<char> = query_lower.chars().collect();
        let trigrams: Vec<String> = if chars.len() >= 3 {
            chars
                .windows(3)
                .map(|w| w.iter().collect::<String>())
                .collect()
        } else {
            Vec::new()
        };

        let candidate_files = if trigrams.is_empty() {
            // Short query: fall back to listing all files
            self.store
                .list_all_files()
                .map_err(|e| rpc_err(e.to_string()))?
                .into_iter()
                .map(|(path, _)| path)
                .collect()
        } else {
            self.store
                .search_trigrams(&trigrams)
                .map_err(|e| rpc_err(e.to_string()))?
        };

        let mut results = Vec::new();
        for file_path in candidate_files {
            if results.len() >= max {
                break;
            }
            // Read the file and find actual matching lines
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                let mut matches = Vec::new();
                for (i, line) in content.lines().enumerate() {
                    if line.to_lowercase().contains(&query_lower) {
                        matches.push(SearchMatch {
                            line: (i + 1) as u32,
                            content: line.to_string(),
                        });
                        if matches.len() >= 5 {
                            break; // Limit matches per file
                        }
                    }
                }
                if !matches.is_empty() {
                    results.push(SearchResult {
                        file: file_path,
                        matches,
                    });
                }
            }
        }

        Ok(results)
    }

    async fn word_lookup(&self, word: String) -> RpcResult<Vec<WordHit>> {
        let locations = self.store.lookup_word(&word).map_err(|e| rpc_err(e.to_string()))?;
        Ok(locations
            .into_iter()
            .map(|wl| WordHit {
                file: wl.file,
                line: wl.line,
            })
            .collect())
    }

    async fn changes_since(&self, seq: u64) -> RpcResult<Vec<ChangeEntryResponse>> {
        let entries = self.store.changes_since(seq).map_err(|e| rpc_err(e.to_string()))?;
        Ok(entries
            .into_iter()
            .map(|e| ChangeEntryResponse {
                seq: e.seq,
                file_path: e.file_path,
                operation: format!("{:?}", e.operation),
                timestamp: e.timestamp,
            })
            .collect())
    }
}
