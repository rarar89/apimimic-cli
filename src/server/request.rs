use log::{info, error, debug};
use hyper::{Body, Request, Response};
use url;
use std::convert::Infallible;
use hyper::body::HttpBody;
use bytes::Buf;
use crate::ping::EndpointManager;
use std::sync::Arc;
use tokio::time::sleep;
use std::time::Duration;

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

    // Check endpoint configuration
    if let Some((timeout, should_proxy)) = endpoint_info {

        // If endpoint should be proxied and we have a target server
        if should_proxy && target_server.is_some() {
            debug!("Proxying request to {} with {}ms timeout", path, timeout);
            
            // Convert headers to Vec after cloning
            let headers: Vec<(String, String)> = headers
                .iter()
                .filter(|(name, _)| name.as_str().to_lowercase() != "host")
                .map(|(name, value)| {
                    (name.as_str().to_string(), 
                     value.to_str().unwrap_or_default().to_string())
                })
                .collect();

            // Collect the body
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

            let whole_body_remaining = whole_body.remaining();

            // Parse body as JSON
            let data: serde_json::Value = match serde_json::from_reader(whole_body.reader()) {
                Ok(data) => data,
                Err(e) => {
                    if whole_body_remaining == 0 {
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

            // Create reqwest client
            let client = reqwest::Client::new();
            let server_url = format!("{}{}", target_server.unwrap(), uri_string);
            
            // Build proxy request
            let mut proxy_req = client.request(
                reqwest::Method::from_bytes(method_str.as_bytes()).unwrap(),
                &server_url
            );

            // Add headers
            for (name, value) in &headers {
                if name.to_lowercase() != "host" {
                    proxy_req = proxy_req.header(name, value);
                }
            }

            // Set host header
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

            // Send the request
            match proxy_req.json(&data).send().await {
                Ok(proxy_resp) => {
                    let proxy_status = proxy_resp.status();
                    let proxy_headers = proxy_resp.headers().clone();
                    let proxy_body = match proxy_resp.bytes().await {
                        Ok(bytes) => bytes,
                        Err(e) => {
                            error!("Error reading proxy response body: {}", e);
                            return Ok(Response::builder()
                                .status(500)
                                .body(Body::from("Failed to read proxy response body"))
                                .unwrap());
                        }
                    };

                    let mut response = Response::builder().status(proxy_status);
                    
                    // Add proxy response headers
                    if let Some(headers) = response.headers_mut() {
                        for (name, value) in proxy_headers {
                            if let Some(name) = name {
                                headers.insert(name, value);
                            }
                        }
                    }

                    if timeout > 0 {
                        info!("Sleeping for {}ms", timeout);
                        sleep(Duration::from_millis(timeout)).await;
                    }

                    info!("Returning proxied response with status: {}", proxy_status);
                    return Ok(response.body(Body::from(proxy_body)).unwrap());
                }
                Err(e) => {
                    error!("Failed to contact proxy target: {}", e);
                    return Ok(Response::builder()
                        .status(502)
                        .body(Body::from(format!("Failed to contact proxy target: {}", e)))
                        .unwrap());
                }
            }
        }
    }

    // If we get here, handle normally through API Mimic
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

                        // Timout
                    if let Some((timeout, _should_proxy)) = endpoint_manager.get_endpoint_info(&path).await {
                        // Apply timeout regardless of proxy status
                        if timeout > 0 {
                            info!("Sleeping for {}ms", timeout);
                            sleep(Duration::from_millis(timeout)).await;
                        }
                    }

                    info!("Returning response from target server: {}", proxy_status);
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