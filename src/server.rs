use log::{info, error};
use std::net::SocketAddr;
use std::sync::Arc;
use hyper::Server;
use hyper::service::{make_service_fn, service_fn};
use tokio::sync::oneshot;
use std::convert::Infallible;
use crate::ping::EndpointManager;

mod request;

/// Starts the HTTP server and handles incoming requests
pub async fn run_server(
    listen: &str,
    remote_base: String,
    project_id: String,
    proxy_enabled: bool,
    target_server: Option<String>,
    remote_ping: String,
) {
    info!("Starting server on {} with project_id: {}", listen, project_id);
    if proxy_enabled {
        info!("Proxy mode enabled. Target server: {:?}", target_server);
    }

    let addr: SocketAddr = listen.parse().expect("Invalid address format");

    let remote_base = remote_base.clone();
    let project_id = project_id.clone();
    let target_server = target_server.clone();

    let endpoint_manager = EndpointManager::new();
    
    // Start ping service
    Arc::clone(&endpoint_manager).start_ping_service(
        listen.to_string(),
        remote_base.clone(),
        remote_ping,
        project_id.clone(),
        target_server.clone(),
    ).await;

    let endpoint_manager = Arc::clone(&endpoint_manager);

    let make_svc = make_service_fn(move |_conn| {
        let remote_base = remote_base.clone();
        let project_id = project_id.clone();
        let target_server = target_server.clone();
        let endpoint_manager = Arc::clone(&endpoint_manager);

        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                request::handle(
                    req,
                    remote_base.clone(),
                    project_id.clone(),
                    proxy_enabled,
                    target_server.clone(),
                    Arc::clone(&endpoint_manager),
                )
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);
    
    // Create a channel for shutdown signal
    let (tx, rx) = oneshot::channel::<()>();
    
    // Handle shutdown gracefully
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
        let _ = tx.send(());
    });

    info!("Server running on http://{}", addr);
    
    if let Err(e) = server.with_graceful_shutdown(async {
        rx.await.ok();
    }).await {
        error!("Server error: {}", e);
    }
}
