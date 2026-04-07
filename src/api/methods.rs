use crate::index::SymbolStore;
use jsonrpsee::core::async_trait;
use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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
}

pub struct GpsApiImpl<S: SymbolStore> {
    pub store: Arc<S>,
    pub indexed: Arc<AtomicBool>,
    pub workspace: String,
    pub port: u16,
}

#[async_trait]
impl<S: SymbolStore + 'static> GpsApiServer for GpsApiImpl<S> {
    async fn locate(&self, symbol_name: String) -> RpcResult<Vec<SymbolLocation>> {
        let records = self
            .store
            .locate(&symbol_name)
            .map_err(|e| jsonrpsee::types::ErrorObjectOwned::owned(-32603, e.to_string(), None::<()>))?;

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
        let imports = self
            .store
            .get_imports(&file_path)
            .map_err(|e| jsonrpsee::types::ErrorObjectOwned::owned(-32603, e.to_string(), None::<()>))?;
        let imported_by = self
            .store
            .get_importers(&file_path)
            .map_err(|e| jsonrpsee::types::ErrorObjectOwned::owned(-32603, e.to_string(), None::<()>))?;

        let mut symbols = std::collections::HashMap::new();
        for related in imports.iter().chain(imported_by.iter()) {
            let file_symbols = self
                .store
                .symbols_in_file(related)
                .map_err(|e| jsonrpsee::types::ErrorObjectOwned::owned(-32603, e.to_string(), None::<()>))?;
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
        // Return a basic summary - the full pre-computed version comes in pipeline task
        Ok(WorkspaceSummary {
            total_files: 0,
            files_by_language: std::collections::HashMap::new(),
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
}
