use log::{info, error, debug};
use hyper::{Body, Request, Response};
use std::convert::Infallible;
use hyper::body::HttpBody;
use bytes::Buf;
use crate::ping::EndpointManager;
use std::sync::Arc;
use tokio::time::sleep;
use std::time::Duration;
use crate::server::proxy::proxy_request;

/// Handles an individual incoming HTTP request.
pub async fn handle(
    req: Request<Body>,
    remote_base: String,
    project_id: String,
    proxy_enabled: bool,
    target_server: Option<String>,
    endpoint_manager: Arc<EndpointManager>,
) -> Result<Response<Body>, Infallible> {
    // First get copies/clones of everything we need
    let method_str = req.method().to_string();
    let uri_string = req.uri().to_string();
    let request_url = req.uri().path_and_query()
        .map(|p| p.as_str())
        .unwrap_or("")
        .to_string();
    let headers = req.headers().clone();
    
    // Get path without leading slash for endpoint lookup
    let path = request_url.trim_start_matches('/').to_string();

    let endpoint_info: Option<(u64, bool)> = endpoint_manager.get_endpoint_info(&path).await;

    // Now we can safely consume the request
    let whole_body = match req.collect().await {
        Ok(body) => body.aggregate(),
        Err(e) => {
            error!("Failed to collect body: {}", e);
            return Ok(Response::builder()
                .status(500)
                .body(Body::from(format!("Apimimic: Failed to collect body: {}", e)))
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
                let error_json = serde_json::json!({"message": format!("Apimimic: Invalid JSON: {}", e)}).to_string();
                return Ok(Response::builder()
                    .status(500)
                    .header("Content-Type", "application/json")
                    .body(Body::from(error_json))
                    .unwrap());
            }
        }
    };

    // Convert headers to Vec after cloning
    let headers: Vec<(String, String)> = headers
    .iter()
    .filter(|(name, _)| name.as_str().to_lowercase() != "host")
    .map(|(name, value)| {
        (name.as_str().to_string(), 
            value.to_str().unwrap_or_default().to_string())
    })
    .collect();

    // And respond with the new JSON.
    let json = serde_json::to_string(&data).unwrap();

    // Check endpoint configuration
    if let Some((timeout, should_proxy)) = endpoint_info {

        // If endpoint should be proxied and we have a target server
        if should_proxy && target_server.is_some() {
            debug!("Proxying request to {} with {}ms timeout", path, timeout);
            return proxy_request(
                &reqwest::Client::new(),
                method_str,
                uri_string,
                target_server.unwrap(),
                headers,
                json,
                Some(timeout),
            ).await;
        }
    }

    // Create the JSON payload
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

    info!("Sending request to API Mimic: {} {}", remote_base, request_url.trim_start_matches('/').to_string());

    debug!("Payload: {}", payload_to_send);

    // Send request to API Mimic
    let mimic_resp = match mimic_req.json(&payload_to_send).send().await {
        Ok(resp) => resp,
        Err(e) => {
            error!("Failed to contact remote server: {}", e);
            let error_json = serde_json::json!({"message": format!("Apimimic: Failed to contact remote server: {}", e)}).to_string();
            return Ok(Response::builder()
                .status(500)
                .header("Content-Type", "application/json")
                .body(Body::from(error_json))
                .unwrap());
        }
    };

    let status = mimic_resp.status();
    let mimic_headers = mimic_resp.headers().clone();
    let mimic_body = match mimic_resp.bytes().await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Error reading remote response body: {}", e);
            let error_json = serde_json::json!({"message": "Apimimic: Failed to read remote response body"}).to_string();
            return Ok(Response::builder()
                .status(500)
                .header("Content-Type", "application/json")
                .body(Body::from(error_json))
                .unwrap());
        }
    };

    // Check if we need to proxy
    let should_proxy = proxy_enabled 
        && target_server.is_some() 
        && mimic_headers.get("apimimic-proxy-request").is_some();

    if should_proxy {
        if let Some(server_url) = target_server {
            return proxy_request(
                &client,
                method_str,
                uri_string,
                server_url,
                headers,
                json,
                endpoint_manager.get_endpoint_info(&path).await.map(|(t, _)| t),
            ).await;
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

    // Timout
    if let Some((timeout, _should_proxy)) = endpoint_manager.get_endpoint_info(&path).await {
        // Apply timeout regardless of proxy status
        if timeout > 0 {
            info!("Sleeping for {}ms", timeout);
            sleep(Duration::from_millis(timeout)).await;
        }
    }

    info!("Returning response from API Mimic: {}", status);

    Ok(response.body(Body::from(mimic_body)).unwrap())
} 