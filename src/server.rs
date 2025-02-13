use log::{info, error};
use std::net::SocketAddr;
use hyper::Server;
use hyper::service::{make_service_fn, service_fn};
use tokio::sync::oneshot;
use std::convert::Infallible;

mod request;

/// Starts the HTTP server and handles incoming requests
pub async fn run_server(
    listen: &str,
    remote_base: String,
    project_id: String,
    proxy_enabled: bool,
    target_server: Option<String>,
) {
    info!("Starting server on {} with project_id: {}", listen, project_id);
    if proxy_enabled {
        info!("Proxy mode enabled. Target server: {:?}", target_server);
    }

    let addr: SocketAddr = listen.parse().expect("Invalid address format");

    let remote_base = remote_base.clone();
    let project_id = project_id.clone();
    let target_server = target_server.clone();

    let make_svc = make_service_fn(move |_conn| {
        let remote_base = remote_base.clone();
        let project_id = project_id.clone();
        let target_server = target_server.clone();

        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                request::handle(
                    req,
                    remote_base.clone(),
                    project_id.clone(),
                    proxy_enabled,
                    target_server.clone(),
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
