use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub port: u16,
    pub pid: u32,
    pub workspace: String,
    pub status: InstanceStatus,
    pub started_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstanceStatus {
    Starting,
    Indexing,
    Ready,
    Error,
}

fn instances_dir() -> PathBuf {
    dirs::home_dir()
        .expect("no home directory")
        .join(".gps")
        .join("instances")
}

pub fn workspace_hash(workspace: &Path) -> String {
    let canonical = workspace.to_string_lossy();
    let hash = blake3::hash(canonical.as_bytes());
    hash.to_hex()[..16].to_string()
}

pub fn instance_file(workspace: &Path) -> PathBuf {
    instances_dir().join(format!("{}.json", workspace_hash(workspace)))
}

pub fn write_instance(workspace: &Path, port: u16, status: InstanceStatus) -> anyhow::Result<()> {
    let dir = instances_dir();
    fs::create_dir_all(&dir)?;

    let info = InstanceInfo {
        port,
        pid: std::process::id(),
        workspace: workspace.to_string_lossy().to_string(),
        status,
        started_at: chrono_now(),
    };

    let path = instance_file(workspace);
    let json = serde_json::to_string_pretty(&info)?;
    fs::write(&path, json)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }

    tracing::debug!(path = %path.display(), "wrote discovery file");
    Ok(())
}

#[allow(dead_code)]
pub fn read_instance(workspace: &Path) -> anyhow::Result<Option<InstanceInfo>> {
    let path = instance_file(workspace);
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(&path)?;
    let info: InstanceInfo = serde_json::from_str(&contents)?;
    Ok(Some(info))
}

pub fn remove_instance(workspace: &Path) {
    let path = instance_file(workspace);
    let _ = fs::remove_file(&path);
}

/// Check if a PID is still alive
#[allow(dead_code)]
pub fn is_pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        // On non-unix, assume alive
        true
    }
}

fn chrono_now() -> String {
    // Simple ISO timestamp without pulling in chrono
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}
