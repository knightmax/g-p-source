use crate::parser::LanguageRegistry;
use crate::watcher::FsEvent;
use tokio::sync::mpsc;
use walkdir::WalkDir;

/// Walk the workspace directory and enqueue all supported source files for initial indexing.
pub async fn crawl_workspace(
    root: &std::path::Path,
    exclude: &[String],
    tx: mpsc::Sender<FsEvent>,
) -> anyhow::Result<u64> {
    let registry = LanguageRegistry::new();
    let mut count = 0u64;

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_excluded(e.path(), exclude))
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        if registry.language_for_extension(ext).is_some() {
            tx.send(FsEvent {
                kind: crate::watcher::FsEventKind::Create,
                path: path.to_path_buf(),
            })
            .await?;
            count += 1;
        }
    }

    tracing::info!(files = count, "initial workspace crawl complete");
    Ok(count)
}

fn is_excluded(path: &std::path::Path, excludes: &[String]) -> bool {
    for component in path.components() {
        let s = component.as_os_str().to_string_lossy();
        for exclude in excludes {
            if s == *exclude {
                return true;
            }
        }
    }
    false
}
