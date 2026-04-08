mod api;
mod config;
mod discovery;
mod index;
mod mcp;
mod parser;
mod pipeline;
mod sensitive;
mod watcher;

use config::Config;
use index::SledStore;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::parse_args();

    // Discovery mode: list running instances and exit
    if config.discovery {
        let instances = discovery::list_instances();
        if instances.is_empty() {
            println!("No running gpsource instances found.");
        } else {
            println!("{}", serde_json::to_string_pretty(&instances)?);
        }
        return Ok(());
    }

    // Kill mode: terminate all running instances and exit
    if config.kill {
        let killed = discovery::kill_all_instances();
        match killed {
            0 => println!("No running gpsource instances found."),
            1 => println!("Killed 1 instance."),
            n => println!("Killed {n} instances."),
        }
        return Ok(());
    }

    let workspace = std::fs::canonicalize(&config.workspace_root)?;

    // MCP mode: run as stdio MCP server (no HTTP, no tracing to stdout)
    if config.mcp {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_writer(std::io::stderr)
            .init();

        let db_path = workspace.join(".gps-index");
        let store = Arc::new(SledStore::open(&db_path, config.cache_capacity)?);
        let indexed = Arc::new(AtomicBool::new(false));

        // Start watcher + crawl in background
        let (tx, rx) = tokio::sync::mpsc::channel(4096);
        let _watcher = watcher::FsWatcher::new(
            &workspace,
            config.debounce_ms,
            config.exclude.clone(),
            tx.clone(),
        )?;

        let crawl_excludes = config.exclude.clone();
        let crawl_tx = tx.clone();
        let crawl_root = workspace.clone();
        let indexed_flag = indexed.clone();
        tokio::spawn(async move {
            if let Err(e) =
                pipeline::initial_crawl::crawl_workspace(&crawl_root, &crawl_excludes, crawl_tx)
                    .await
            {
                tracing::error!(error = %e, "initial crawl failed");
            }
            indexed_flag.store(true, Ordering::Relaxed);
        });

        let pipeline_store = store.clone();
        let max_concurrent = num_cpus::get();
        tokio::spawn(async move {
            if let Err(e) = pipeline::run_pipeline(rx, pipeline_store, max_concurrent).await {
                tracing::error!(error = %e, "pipeline error");
            }
        });

        // Run MCP server on stdio (blocks until stdin closes)
        mcp::stdio_server::run_mcp_server(store, indexed, workspace.to_string_lossy().to_string())
            .await?;

        return Ok(());
    }

    // HTTP mode (default)
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    tracing::info!(workspace = %workspace.display(), "gpsource starting");

    // Open symbol index
    let db_path = workspace.join(".gps-index");
    let store = Arc::new(SledStore::open(&db_path, config.cache_capacity)?);
    tracing::info!(path = %db_path.display(), "symbol index opened");

    let indexed = Arc::new(AtomicBool::new(false));

    // Start JSON-RPC server (port 0 = OS picks a free port)
    let addr =
        api::start_rpc_server(store.clone(), config.port, &workspace, indexed.clone()).await?;
    tracing::info!(%addr, "GPS API ready");

    // Update discovery to indexing status
    discovery::write_instance(&workspace, addr.port(), discovery::InstanceStatus::Indexing)?;

    // Set up FS event channel
    let (tx, rx) = tokio::sync::mpsc::channel(4096);

    // Start filesystem watcher
    let _watcher = watcher::FsWatcher::new(
        &workspace,
        config.debounce_ms,
        config.exclude.clone(),
        tx.clone(),
    )?;

    // Initial workspace crawl
    let crawl_excludes = config.exclude.clone();
    let crawl_tx = tx.clone();
    let crawl_root = workspace.clone();
    let indexed_flag = indexed.clone();
    let crawl_workspace_path = workspace.clone();
    let crawl_port = addr.port();
    tokio::spawn(async move {
        if let Err(e) =
            pipeline::initial_crawl::crawl_workspace(&crawl_root, &crawl_excludes, crawl_tx).await
        {
            tracing::error!(error = %e, "initial crawl failed");
        }
        indexed_flag.store(true, Ordering::Relaxed);
        tracing::info!("initial indexation complete");
        let _ = discovery::write_instance(
            &crawl_workspace_path,
            crawl_port,
            discovery::InstanceStatus::Ready,
        );
    });

    // Run the indexing pipeline (blocks until shutdown)
    let max_concurrent = num_cpus::get();
    tracing::info!(workers = max_concurrent, "starting indexing pipeline");

    // Handle shutdown signal
    let pipeline_store = store.clone();
    let shutdown_workspace = workspace.clone();
    tokio::select! {
        result = pipeline::run_pipeline(rx, pipeline_store, max_concurrent) => {
            if let Err(e) = result {
                tracing::error!(error = %e, "pipeline error");
            }
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("shutting down");
        }
    }

    // Cleanup discovery file on exit
    discovery::remove_instance(&shutdown_workspace);

    Ok(())
}
