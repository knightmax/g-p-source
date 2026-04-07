use rand::Rng;
use std::path::PathBuf;

const AUTH_TOKEN_DIR: &str = ".gps";
const AUTH_TOKEN_FILE: &str = "auth-token";

pub fn generate_and_store_token() -> anyhow::Result<String> {
    let token: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();

    let dir = dirs_home().join(AUTH_TOKEN_DIR);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(AUTH_TOKEN_FILE);
    std::fs::write(&path, &token)?;

    // Restrict permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    tracing::info!(path = %path.display(), "auth token written");
    Ok(token)
}

#[allow(dead_code)]
pub fn read_token() -> anyhow::Result<String> {
    let path = dirs_home().join(AUTH_TOKEN_DIR).join(AUTH_TOKEN_FILE);
    Ok(std::fs::read_to_string(path)?.trim().to_string())
}

fn dirs_home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}
