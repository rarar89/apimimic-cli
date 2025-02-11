use log::{info, error, debug};
use serde_json::json;
use std::net::SocketAddr;
use hyper::{Body, Request, Response, Server};
use hyper::service::{make_service_fn, service_fn};
use tokio::sync::oneshot;
use url;
use std::convert::Infallible;
use hyper::body::HttpBody;
use bytes::Buf;

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
                handle_request(
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

/// Handles an individual incoming HTTP request.
async fn handle_request(
    req: Request<Body>,
    remote_base: String,
    project_id: String,
    proxy_enabled: bool,
    target_server: Option<String>,
) -> Result<Response<Body>, Infallible> {
    
    // First get copies/clones of everything we need
    let method_str = req.method().to_string();
    let uri_string = req.uri().to_string();
    let request_url = req.uri().path_and_query()
        .map(|p| p.as_str())
        .unwrap_or("")
        .to_string();
    let headers = req.headers().clone();
    
    // Convert headers to Vec after cloning
    let headers: Vec<(String, String)> = headers
        .iter()
        .filter(|(name, _)| name.as_str().to_lowercase() != "host")
        .map(|(name, value)| {
            (name.as_str().to_string(), 
             value.to_str().unwrap_or_default().to_string())
        })
        .collect();

    // Now we can safely consume the request
    let whole_body = match req.collect().await {
        Ok(body) => body.aggregate(),
        Err(e) => {
            error!("Failed to collect body: {}", e);
            return Ok(Response::builder()
                .status(500)
                .body(Body::from(format!("Failed to collect body: {}", e)))
                .unwrap());
        }
    };

    let remaining = whole_body.remaining();
    let data: serde_json::Value = match serde_json::from_reader(whole_body.reader()) {
        Ok(data) => data,
        Err(e) => {
            if remaining == 0 {
                // If body is empty, use empty JSON object
                serde_json::Value::Object(serde_json::Map::new())
            } else {
                error!("Failed to parse JSON: {}", e);
                return Ok(Response::builder()
                    .status(400)
                    .body(Body::from(format!("Invalid JSON: {}", e)))
                    .unwrap());
            }
        }
    };

    // And respond with the new JSON.
    let json = serde_json::to_string(&data).unwrap();

    // Create the JSON payload
    let payload = json!({
        "method": method_str,
        "headers": headers.clone().into_iter().collect::<std::collections::HashMap<_, _>>(),
        "body": data,
        "path": request_url.trim_start_matches('/').to_string()
    });

    let payload_to_send = &serde_json::json!({
        "method": method_str,
        "headers": headers.clone().into_iter().collect::<std::collections::HashMap<_, _>>(),
        "body": data,
        "path": request_url.trim_start_matches('/').to_string()
    });

    debug!("Payload: {}", payload_to_send);

    // Create reqwest client
    let client = reqwest::Client::new();
    
    // Build request to API Mimic
    let mut mimic_req = client.post(&remote_base)
        .header("apimimic-project-id", &project_id)
        .header("Content-Type", "Application/json")
        .header("Content-Length", payload_to_send.to_string().len().to_string());

    if proxy_enabled && target_server.is_some() {
        if let Some(target) = &target_server {
            mimic_req = mimic_req.header("apimimic-cli-proxy", target);
        }
    }

    // Add original headers
    for (name, value) in &headers {
        mimic_req = mimic_req.header(name, value);
    }

    // Send request to API Mimic
    let mimic_resp = match mimic_req.json(&payload_to_send).send().await {
        Ok(resp) => resp,
        Err(e) => {
            error!("Failed to contact remote server: {}", e);
            return Ok(Response::builder()
                .status(502)
                .body(Body::from(format!("Failed to contact remote server: {}", e)))
                .unwrap());
        }
    };

    let status = mimic_resp.status();
    let mimic_headers = mimic_resp.headers().clone();
    let mimic_body = match mimic_resp.bytes().await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Error reading remote response body: {}", e);
            return Ok(Response::builder()
                .status(500)
                .body(Body::from("Failed to read remote response body"))
                .unwrap());
        }
    };

    // Check if we need to proxy
    let should_proxy = proxy_enabled 
        && target_server.is_some() 
        && mimic_headers.get("apimimic-proxy-request").is_some();

    if should_proxy {
        if let Some(server_url) = target_server {
            let server_url = format!("{}{}", server_url.trim_end_matches('/'), uri_string);
            info!("Proxying request to target server: {}", server_url);

            let mut proxy_req = client.request(
                reqwest::Method::from_bytes(method_str.as_bytes()).unwrap(),
                &server_url
            );

            // Add original headers to proxy request
            for (name, value) in headers {
                if name.to_lowercase() != "host" {
                    proxy_req = proxy_req.header(&name, &value);
                }
            }

            // Set host header from target server URL
            if let Ok(parsed_url) = url::Url::parse(&server_url) {
                if let Some(host) = parsed_url.host_str() {
                    let host_value = if let Some(port) = parsed_url.port() {
                        format!("{}:{}", host, port)
                    } else {
                        host.to_string()
                    };
                    proxy_req = proxy_req.header("Host", &host_value);
                }
            }

            // Send request to target server
            match proxy_req.body(Body::from(json)).send().await {
                Ok(proxy_resp) => {
                    let proxy_status = proxy_resp.status();
                    let proxy_headers = proxy_resp.headers().clone();
                    let proxy_body = match proxy_resp.bytes().await {
                        Ok(bytes) => bytes,
                        Err(e) => {
                            error!("Error reading target server response body: {}", e);
                            return Ok(Response::builder()
                                .status(500)
                                .body(Body::from("Failed to read proxy response body"))
                                .unwrap());
                        }
                    };

                    let mut response = Response::builder()
                        .status(proxy_status);

                    // Add proxy response headers
                    if let Some(headers) = response.headers_mut() {
                        for (name, value) in proxy_headers {
                            if let Some(name) = name {
                                headers.insert(name, value);
                            }
                        }
                    }

                    return Ok(response.body(Body::from(proxy_body)).unwrap());
                }
                Err(e) => {
                    error!("Failed to contact target server: {}", e);
                    return Ok(Response::builder()
                        .status(502)
                        .body(Body::from(format!("Failed to contact target server: {}", e)))
                        .unwrap());
                }
            }
        }
    }

    // Return API Mimic response if not proxying
    let mut response = Response::builder()
        .status(status);

    // Add API Mimic response headers
    if let Some(headers) = response.headers_mut() {
        for (name, value) in mimic_headers {
            if let Some(name) = name {
                headers.insert(name, value);
            }
        }
    }

    Ok(response.body(Body::from(mimic_body)).unwrap())
} 