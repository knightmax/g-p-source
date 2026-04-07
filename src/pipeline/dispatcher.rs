use crate::index::SledStore;
use crate::index::SymbolStore;
use crate::parser::SourceParser;
use crate::watcher::{FsEvent, FsEventKind};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{Semaphore, mpsc};

/// Run the full indexing pipeline: dispatcher → parser pool → index writer.
pub async fn run_pipeline(
    mut rx: mpsc::Receiver<FsEvent>,
    store: Arc<SledStore>,
    max_concurrent: usize,
) -> anyhow::Result<()> {
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let (diff_tx, mut diff_rx) = mpsc::channel::<IndexUpdate>(4096);

    // Index writer task
    let store_writer = store.clone();
    let writer_handle = tokio::spawn(async move {
        while let Some(update) = diff_rx.recv().await {
            match update {
                IndexUpdate::Upsert { file_path, symbols } => {
                    if let Err(e) = store_writer.upsert_file_symbols(&file_path, &symbols) {
                        tracing::error!(file = %file_path, error = %e, "failed to upsert symbols");
                    }
                    // Update file metadata
                    if let Ok(content) = std::fs::read(&file_path) {
                        let hash = blake3::hash(&content);
                        let meta = crate::index::FileMetadata {
                            mtime: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                            hash: hash.as_bytes().to_vec(),
                            symbol_count: symbols.len() as u32,
                        };
                        let _ = store_writer.set_file_meta(&file_path, &meta);
                    }
                }
                IndexUpdate::Remove { file_path } => {
                    if let Err(e) = store_writer.remove_file(&file_path) {
                        tracing::error!(file = %file_path, error = %e, "failed to remove file");
                    }
                }
            }
        }
    });

    // Dispatcher + parser pool
    let mut seen = HashSet::new();

    while let Some(event) = rx.recv().await {
        let path_str = event.path.to_string_lossy().to_string();

        // Deduplicate within a batch window
        if !seen.insert(path_str.clone()) {
            continue;
        }

        match event.kind {
            FsEventKind::Delete => {
                let _ = diff_tx
                    .send(IndexUpdate::Remove {
                        file_path: path_str,
                    })
                    .await;
            }
            FsEventKind::Create | FsEventKind::Modify | FsEventKind::Rename => {
                // Check content hash to skip unchanged files
                let store_check = store.clone();
                if let Ok(content) = std::fs::read(&event.path) {
                    let new_hash = blake3::hash(&content);
                    if let Ok(Some(meta)) = store_check.get_file_meta(&path_str)
                        && meta.hash == new_hash.as_bytes().to_vec()
                    {
                        tracing::debug!(file = %path_str, "content unchanged, skipping");
                        continue;
                    }
                }

                let ext = event
                    .path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_string();

                let permit = semaphore.clone().acquire_owned().await?;
                let diff_tx = diff_tx.clone();
                let path = event.path.clone();

                tokio::task::spawn_blocking(move || {
                    let _permit = permit; // held until parsing is done

                    let mut parser = match SourceParser::new() {
                        Ok(p) => p,
                        Err(e) => {
                            tracing::error!(error = %e, "failed to create parser");
                            return;
                        }
                    };

                    let lang = match parser.language_for_extension(&ext) {
                        Some(l) => l,
                        None => return,
                    };

                    let source = match std::fs::read(&path) {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::warn!(file = %path.display(), error = %e, "failed to read file");
                            return;
                        }
                    };

                    let tree = match parser.parse(&source, lang, None) {
                        Some(t) => t,
                        None => {
                            tracing::warn!(file = %path.display(), "failed to parse file");
                            return;
                        }
                    };

                    let file_path = path.to_string_lossy().to_string();
                    let symbols = parser.extract_symbols(&tree, &source, lang, &file_path);

                    tracing::debug!(
                        file = %file_path,
                        symbols = symbols.len(),
                        "parsed file"
                    );

                    let _ = diff_tx.blocking_send(IndexUpdate::Upsert { file_path, symbols });
                });
            }
        }

        // Reset dedup set periodically (after processing a batch)
        if seen.len() > 10_000 {
            seen.clear();
        }
    }

    // Drop sender to signal writer to stop
    drop(diff_tx);
    writer_handle.await?;

    Ok(())
}

enum IndexUpdate {
    Upsert {
        file_path: String,
        symbols: Vec<crate::parser::Symbol>,
    },
    Remove {
        file_path: String,
    },
}
