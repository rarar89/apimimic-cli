use serde::Deserialize;
use std::io::Read;
use tiny_http::{Header, Request, Response, Server};
use std::thread;

/// Structure of the JSON response from the remote API Mimic service.
#[derive(Deserialize)]
struct RemoteResponse {
   
}

/// Starts the HTTP server and handles incoming requests
pub fn run_server(
    listen: &str,
    remote_base: String,
    project_id: String,
    proxy_enabled: bool,
    target_server: Option<String>,
) {
    println!("Starting server on {}", listen);
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
    // Log the incoming request.
    println!("Received request: {} {}", request.method(), request.url());

    // Read the request body.
    let mut body = Vec::new();
    if let Err(e) = request.as_reader().read_to_end(&mut body) {
        let _ = request.respond(Response::from_string(format!("Failed to read request: {}", e))
            .with_status_code(500));
        return;
    }

    // Construct the remote URL by concatenating the remote base with the request URL.
    let remote_url = format!("{}{}", remote_base.trim_end_matches('/'), request.url());

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
    let mut original_headers = Vec::new();
    for header in request.headers() {
        if header.field.as_str().to_string().to_lowercase() == "host" {
            continue;
        }
        original_headers.push((
            header.field.as_str().to_string(),
            header.value.as_str().to_string()
        ));
    }

    // Send the request to API Mimic service
    let mut request_builder = ureq::request(method_str, &remote_url)
        .set("apimimic-project-id", &project_id);

    // Add proxy header if proxy mode is enabled and target server is set
    if proxy_enabled && target_server.is_some() {
        if let Some(target) = &target_server {
            request_builder = request_builder.set("apimimic-cli-proxy", target);
        }
    }

    // Add original headers to the API Mimic request
    for (name, value) in &original_headers {
        request_builder = request_builder.set(name, value);
    }

    let remote_resp = request_builder.send_bytes(&body);

    match remote_resp {
        Ok(resp) => {
            let status = resp.status();
            let mut headers = Vec::new();
            for h in resp.headers_names() {
                if let Some(value) = resp.header(&h) {
                    headers.push((h.to_string(), value.to_string()));
                }
            }
            
            let mut buf = Vec::new();
            if let Err(e) = resp.into_reader().read_to_end(&mut buf) {
                eprintln!("Error reading remote response body: {}", e);
                let _ = request.respond(
                    Response::from_string(format!("Failed to read remote response: {}", e))
                        .with_status_code(500),
                );
                return;
            }

            // Check if we need to proxy based on the apimimic-proxy-request header
            let should_proxy = proxy_enabled 
                && target_server.is_some() 
                && headers.iter().any(|(h, _)| h.to_lowercase() == "apimimic-proxy-request");

            if should_proxy {
                if let Some(server_url) = target_server {
                    let server_url = format!("{}{}", server_url.trim_end_matches('/'), request.url());
                    println!("Forwarding to target server: {}", server_url);
                    
                    let mut proxy_request = ureq::request(method_str, &server_url);
                    
                    // Add original request headers to proxy request
                    for (name, value) in original_headers {
                        proxy_request = proxy_request.set(&name, &value);
                    }

                    match proxy_request.send_bytes(&body) {
                        Ok(proxy_resp) => {
                            let proxy_status = proxy_resp.status();
                            let mut proxy_headers = Vec::new();
                            for h in proxy_resp.headers_names() {
                                if let Some(val) = proxy_resp.header(&h) {
                                    proxy_headers.push(Header::from_bytes(h.as_bytes(), val.as_bytes()).unwrap());
                                }
                            }
                            let mut proxy_body = Vec::new();
                            if let Err(e) = proxy_resp.into_reader().read_to_end(&mut proxy_body) {
                                eprintln!("Error reading target server response body: {}", e);
                                let _ = request.respond(
                                    Response::from_string("Failed to read proxy response body")
                                        .with_status_code(500),
                                );
                                return;
                            }

                            let response = Response::from_data(proxy_body).with_status_code(proxy_status);
                            let response = proxy_headers.into_iter().fold(response, |resp, h| resp.with_header(h));
                            let _ = request.respond(response);
                        }
                        Err(e) => {
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
            let _ = request.respond(
                Response::from_string(format!("Failed to contact remote server: {}", e))
                    .with_status_code(502),
            );
        }
    }
} 