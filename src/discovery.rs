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
pub fn is_pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        // On Windows, check via the Windows API
        #[cfg(windows)]
        {
            use std::ptr;
            let handle = unsafe {
                windows_sys::Win32::System::Threading::OpenProcess(
                    0x0001, // PROCESS_TERMINATE not needed, PROCESS_QUERY_LIMITED_INFORMATION
                    0,
                    pid,
                )
            };
            if handle.is_null() {
                return false;
            }
            unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) };
            true
        }
        #[cfg(not(windows))]
        {
            true
        }
    }
}

/// List all running instances, cleaning up stale ones along the way.
pub fn list_instances() -> Vec<InstanceInfo> {
    let dir = instances_dir();
    if !dir.exists() {
        return Vec::new();
    }

    let mut instances = Vec::new();
    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json") {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(info) = serde_json::from_str::<InstanceInfo>(&contents) {
                    if is_pid_alive(info.pid) {
                        instances.push(info);
                    } else {
                        // Stale instance — clean up
                        let _ = fs::remove_file(&path);
                    }
                }
            }
        }
    }

    instances
}

/// Kill all running instances and remove their discovery files.
/// Returns the number of instances killed.
pub fn kill_all_instances() -> usize {
    let dir = instances_dir();
    if !dir.exists() {
        return 0;
    }

    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };

    let mut killed = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json") {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(info) = serde_json::from_str::<InstanceInfo>(&contents) {
                    if info.pid != std::process::id() && is_pid_alive(info.pid) {
                        kill_pid(info.pid);
                        killed += 1;
                    }
                }
            }
            let _ = fs::remove_file(&path);
        }
    }

    killed
}

fn kill_pid(pid: u32) {
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, libc::SIGTERM); }
    }
    #[cfg(windows)]
    {
        // On Windows, use taskkill
        let _ = std::process::Command::new("taskkill")
            .args([&"/PID", &pid.to_string(), "/F"])
            .output();
    }
}

fn chrono_now() -> String {
    // Simple ISO timestamp without pulling in chrono
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}
