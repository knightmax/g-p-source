use super::auth;
use super::methods::{GpsApiImpl, GpsApiServer};
use crate::discovery;
use crate::index::SledStore;
use jsonrpsee::server::Server;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub async fn start_rpc_server(
    store: Arc<SledStore>,
    port: u16,
    workspace: &Path,
    indexed: Arc<AtomicBool>,
) -> anyhow::Result<SocketAddr> {
    let token = auth::generate_and_store_token()?;
    let _token = token; // Will be used for middleware in a later task

    let server = Server::builder()
        .build(format!("127.0.0.1:{}", port).parse::<SocketAddr>()?)
        .await?;

    let addr = server.local_addr()?;
    let actual_port = addr.port();

    let api = GpsApiImpl {
        store,
        indexed,
        workspace: workspace.to_string_lossy().to_string(),
        port: actual_port,
    };

    let handle = server.start(api.into_rpc());
    tracing::info!(%addr, "JSON-RPC server started");

    // Write discovery file with actual bound port
    discovery::write_instance(workspace, actual_port, discovery::InstanceStatus::Starting)?;

    tokio::spawn(handle.stopped());

    Ok(addr)
}
