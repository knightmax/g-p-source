/// Patterns for sensitive files that should never be indexed or served via read_file.
/// Inspired by codedb's security model.

const SENSITIVE_PATTERNS: &[&str] = &[
    ".env",
    ".env.local",
    ".env.production",
    ".env.development",
    ".env.staging",
    ".env.test",
    "credentials.json",
    "credentials.yaml",
    "credentials.yml",
    "secrets.json",
    "secrets.yaml",
    "secrets.yml",
    "service-account.json",
    ".aws/credentials",
    ".aws/config",
];

const SENSITIVE_EXTENSIONS: &[&str] = &[
    "pem", "key", "p12", "pfx", "jks", "keystore", "ppk",
];

const SENSITIVE_PREFIXES: &[&str] = &["id_rsa", "id_ed25519", "id_ecdsa", "id_dsa"];

/// Check if a file path refers to a sensitive file that should not be indexed or read.
pub fn is_sensitive_file(path: &std::path::Path) -> bool {
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return false,
    };

    // Check exact file name matches
    let lower_name = file_name.to_lowercase();
    for pattern in SENSITIVE_PATTERNS {
        if lower_name == *pattern {
            return true;
        }
    }

    // Check .env* prefix
    if lower_name.starts_with(".env") {
        return true;
    }

    // Check sensitive extensions
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let lower_ext = ext.to_lowercase();
        for sensitive_ext in SENSITIVE_EXTENSIONS {
            if lower_ext == *sensitive_ext {
                return true;
            }
        }
    }

    // Check sensitive prefixes (SSH keys)
    for prefix in SENSITIVE_PREFIXES {
        if lower_name.starts_with(prefix) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detects_env_files() {
        assert!(is_sensitive_file(Path::new(".env")));
        assert!(is_sensitive_file(Path::new(".env.local")));
        assert!(is_sensitive_file(Path::new(".env.production")));
        assert!(is_sensitive_file(Path::new(".env.custom")));
    }

    #[test]
    fn detects_credential_files() {
        assert!(is_sensitive_file(Path::new("credentials.json")));
        assert!(is_sensitive_file(Path::new("secrets.yaml")));
        assert!(is_sensitive_file(Path::new("service-account.json")));
    }

    #[test]
    fn detects_key_files() {
        assert!(is_sensitive_file(Path::new("server.pem")));
        assert!(is_sensitive_file(Path::new("private.key")));
        assert!(is_sensitive_file(Path::new("cert.p12")));
    }

    #[test]
    fn detects_ssh_keys() {
        assert!(is_sensitive_file(Path::new("id_rsa")));
        assert!(is_sensitive_file(Path::new("id_ed25519")));
        assert!(is_sensitive_file(Path::new("id_rsa.pub")));
    }

    #[test]
    fn allows_normal_files() {
        assert!(!is_sensitive_file(Path::new("src/main.rs")));
        assert!(!is_sensitive_file(Path::new("package.json")));
        assert!(!is_sensitive_file(Path::new("README.md")));
        assert!(!is_sensitive_file(Path::new("config.ts")));
    }
}
