use std::io::Read;
use tiny_http::{Header, Request, Response, Server};
use std::thread;
use log::{info, error, debug};
use serde_json::json;

/// Starts the HTTP server and handles incoming requests
pub fn run_server(
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
    let server = Server::http(listen).unwrap();

    // Handle each incoming request in its own thread.
    for request in server.incoming_requests() {
        let remote_base = remote_base.clone();
        let target_server = target_server.clone();
        let project_id = project_id.clone();
        thread::spawn(move || {
            handle_request(request, remote_base, project_id, proxy_enabled, target_server)
        });
    }
}

/// Handles an individual incoming HTTP request.
fn handle_request(
    mut request: Request,
    remote_base: String,
    project_id: String,
    proxy_enabled: bool,
    target_server: Option<String>,
) {
    info!("Received request: {} {}", request.method(), request.url());
    debug!("Request headers: {:?}", request.headers());

    // Read the request body.
    let mut body = Vec::new();
    if let Err(e) = request.as_reader().read_to_end(&mut body) {
        error!("Failed to read request body: {}", e);
        let _ = request.respond(Response::from_string(format!("Failed to read request: {}", e))
            .with_status_code(500));
        return;
    }
    debug!("Request body length: {} bytes", body.len());

    // Construct the remote URL by concatenating the remote base with the request URL.
    let remote_url = remote_base;
    info!("Forwarding request to API Mimic: {}", remote_url);

    // Forward the request to the remote API Mimic service.
    let method_str = match request.method() {
        &tiny_http::Method::Get => "GET",
        &tiny_http::Method::Post => "POST",
        &tiny_http::Method::Put => "PUT",
        &tiny_http::Method::Delete => "DELETE",
        &tiny_http::Method::Head => "HEAD",
        &tiny_http::Method::Connect => "CONNECT",
        &tiny_http::Method::Options => "OPTIONS",
        &tiny_http::Method::Trace => "TRACE",
        &tiny_http::Method::Patch => "PATCH",
        _ => "GET", // Default to GET for any unhandled methods
    };

    // Collect original request headers
    let original_headers = process_headers(request.headers());

    // In handle_request function, before sending the request to API Mimic:
    let request_url = request.url().to_string();
    
    // Create the JSON payload and serialize it
    let payload = json!({
        "method": method_str,
        "headers": original_headers.clone().into_iter().collect::<std::collections::HashMap<_, _>>(),
        "body": body,
        "path": request_url.trim_start_matches('/')
    });
    let payload_str = serde_json::to_string(&payload).unwrap();

    // Send the request to API Mimic service
    let mut request_builder = ureq::request("POST", &remote_url)
        .set("apimimic-project-id", &project_id)
        .set("Content-Type", "application/json");

    // Add proxy header if proxy mode is enabled and target server is set
    if proxy_enabled && target_server.is_some() {
        if let Some(target) = &target_server {
            request_builder = request_builder.set("apimimic-cli-proxy", target);
        }
    }

    // Add original headers to the API Mimic request
    for (name, value) in &original_headers {
        request_builder = request_builder.set(name, value);
        debug!("Added header to API Mimic request: {} = {}", name, value);
    }

    let remote_resp = request_builder.send_string(&payload_str);

    match remote_resp {
        Ok(resp) => {
            let status = resp.status();
            info!("Received response from API Mimic with status: {}", status);
            let mut headers = Vec::new();
            for h in resp.headers_names() {
                if let Some(value) = resp.header(&h) {
                    headers.push((h.to_string(), value.to_string()));
                    debug!("Response header: {} = {}", h, value);
                }
            }
            
            let mut buf = Vec::new();
            if let Err(e) = resp.into_reader().read_to_end(&mut buf) {
                error!("Error reading remote response body: {}", e);
                let _ = request.respond(
                    Response::from_string(format!("Failed to read remote response: {}", e))
                        .with_status_code(500),
                );
                return;
            }
            debug!("Response body length: {} bytes", buf.len());

            // Check if we need to proxy based on the apimimic-proxy-request header
            let should_proxy = proxy_enabled 
                && target_server.is_some() 
                && headers.iter().any(|(h, _)| h.to_lowercase() == "apimimic-proxy-request");

            if should_proxy {
                if let Some(server_url) = target_server {
                    let server_url = format!("{}{}", server_url.trim_end_matches('/'), request.url());
                    info!("Proxying request to target server: {}", server_url);
                    
                    let mut proxy_request = ureq::request(method_str, &server_url);
                    
                    // Add original request headers to proxy request
                    for (name, value) in original_headers {
                        proxy_request = proxy_request.set(&name, &value);
                        debug!("Added header to proxy request: {} = {}", name, value);
                    }

                    match proxy_request.send_bytes(&body) {
                        Ok(proxy_resp) => {
                            let proxy_status = proxy_resp.status();
                            info!("Received response from target server with status: {}", proxy_status);
                            let mut proxy_headers = Vec::new();
                            for h in proxy_resp.headers_names() {
                                if let Some(val) = proxy_resp.header(&h) {
                                    proxy_headers.push(Header::from_bytes(h.as_bytes(), val.as_bytes()).unwrap());
                                    debug!("Proxy response header: {} = {}", h, val);
                                }
                            }
                            let mut proxy_body = Vec::new();
                            if let Err(e) = proxy_resp.into_reader().read_to_end(&mut proxy_body) {
                                error!("Error reading target server response body: {}", e);
                                let _ = request.respond(
                                    Response::from_string("Failed to read proxy response body")
                                        .with_status_code(500),
                                );
                                return;
                            }
                            debug!("Proxy response body length: {} bytes", proxy_body.len());

                            let response = Response::from_data(proxy_body).with_status_code(proxy_status);
                            let response = proxy_headers.into_iter().fold(response, |resp, h| resp.with_header(h));
                            let _ = request.respond(response);
                        }
                        Err(e) => {
                            error!("Failed to contact target server: {}", e);
                            let _ = request.respond(
                                Response::from_string(format!("Failed to contact target server: {}", e))
                                    .with_status_code(502),
                            );
                        }
                    }
                    return;
                }
            }

            // Return the API Mimic response if not proxying
            info!("Returning API Mimic response with status: {}", status);
            let response = {
                let headers = headers
                    .into_iter()
                    .map(|(k, v)| Header::from_bytes(k.as_bytes(), v.as_bytes()).unwrap())
                    .collect::<Vec<_>>();
                let mut resp = Response::from_data(buf).with_status_code(status);
                for header in headers {
                    resp = resp.with_header(header);
                }
                resp
            };

            let _ = request.respond(response);
        }
        Err(e) => {
            error!("Failed to contact remote server: {}", e);
            let _ = request.respond(
                Response::from_string(format!("Failed to contact remote server: {}", e))
                    .with_status_code(502),
            );
        }
    }
}

fn process_headers(headers: &[Header]) -> Vec<(String, String)> {
    headers
        .iter()
        .filter(|header| header.field.as_str().to_string().to_lowercase() != "host")
        .map(|header| (
            header.field.as_str().to_string(),
            header.value.as_str().to_string()
        ))
        .collect()
} 