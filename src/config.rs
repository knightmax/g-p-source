use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "gpsource", about = "Incremental code indexing engine")]
pub struct Config {
    /// Workspace root directory to index
    #[arg(short, long, default_value = ".")]
    pub workspace_root: PathBuf,

    /// JSON-RPC server port (0 = dynamic)
    #[arg(short, long, default_value_t = 0)]
    pub port: u16,

    /// Run as MCP server over stdio instead of HTTP
    #[arg(long)]
    pub mcp: bool,

    /// sled cache capacity in bytes (default: 40 MB)
    #[arg(long, default_value_t = 40 * 1024 * 1024)]
    pub cache_capacity: u64,

    /// Paths to exclude from watching (comma-separated)
    #[arg(long, value_delimiter = ',', default_values_t = default_excludes())]
    pub exclude: Vec<String>,

    /// FS event debounce window in milliseconds
    #[arg(long, default_value_t = 100)]
    pub debounce_ms: u64,
}

fn default_excludes() -> Vec<String> {
    vec![
        ".git".into(),
        "node_modules".into(),
        "target".into(),
        "bin".into(),
        "obj".into(),
        "build".into(),
        "dist".into(),
        "__pycache__".into(),
    ]
}

impl Config {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
