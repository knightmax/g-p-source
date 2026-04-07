use notify::{
    event::ModifyKind, Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsEventKind {
    Create,
    Modify,
    Delete,
    Rename,
}

#[derive(Debug, Clone)]
pub struct FsEvent {
    pub kind: FsEventKind,
    pub path: PathBuf,
}

pub struct FsWatcher {
    _watcher: RecommendedWatcher,
}

impl FsWatcher {
    pub fn new(
        root: &Path,
        debounce_ms: u64,
        exclude_patterns: Vec<String>,
        tx: mpsc::Sender<FsEvent>,
    ) -> anyhow::Result<Self> {
        let excludes = exclude_patterns.clone();
        let tx_clone = tx.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    for path in &event.paths {
                        if should_exclude(path, &excludes) {
                            return;
                        }
                    }
                    if let Some(fs_event) = classify_event(&event) {
                        let _ = tx_clone.blocking_send(fs_event);
                    }
                }
            },
            Config::default()
                .with_poll_interval(Duration::from_millis(debounce_ms)),
        )?;

        watcher.watch(root, RecursiveMode::Recursive)?;
        tracing::info!(path = %root.display(), "watching filesystem");

        Ok(Self { _watcher: watcher })
    }
}

fn should_exclude(path: &Path, excludes: &[String]) -> bool {
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

fn classify_event(event: &Event) -> Option<FsEvent> {
    let path = event.paths.first()?.clone();
    let kind = match &event.kind {
        EventKind::Create(_) => FsEventKind::Create,
        EventKind::Modify(ModifyKind::Name(_)) => FsEventKind::Rename,
        EventKind::Modify(_) => FsEventKind::Modify,
        EventKind::Remove(_) => FsEventKind::Delete,
        _ => return None,
    };
    Some(FsEvent { kind, path })
}
