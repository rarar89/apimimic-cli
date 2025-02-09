use serde::Deserialize;
use std::io::Read;
use tiny_http::{Header, Request, Response, Server};
use std::thread;

/// Structure of the JSON response from the remote API Mimic service.
#[derive(Deserialize)]
struct RemoteResponse {
    proxy: bool,
}

/// Starts the HTTP server and handles incoming requests
pub fn run_server(
    listen: &str,
    remote_base: String,
    project_id: String,
    auth_token: String,
    proxy_enabled: bool,
    backend_base: Option<String>,
) {
    println!("Starting server on {}", listen);
    let server = Server::http(listen).unwrap();

    // Handle each incoming request in its own thread.
    for request in server.incoming_requests() {
        let remote_base = remote_base.clone();
        let auth_token = auth_token.clone();
        let backend_base = backend_base.clone();
        let project_id = project_id.clone();
        thread::spawn(move || {
            handle_request(request, remote_base, project_id, auth_token, proxy_enabled, backend_base)
        });
    }
}

/// Handles an individual incoming HTTP request.
fn handle_request(
    mut request: Request,
    remote_base: String,
    project_id: String,
    auth_token: String,
    proxy_enabled: bool,
    backend_base: Option<String>,
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

    // Create a new request and collect headers
    let mut headers = Vec::new();
    for header in request.headers() {
        if header.field.as_str().to_string().to_lowercase() == "host" {
            continue;
        }
        headers.push((
            header.field.as_str().to_string(),
            header.value.as_str().to_string()
        ));
    }

    // Send the request (using send_bytes to support any method and binary body).
    let remote_resp = ureq::request(method_str, &remote_url)
        .set("Apimimic-authorization", &format!("Bearer {}", auth_token))
        .set("Apimimic-project-id", &project_id)
        .send_bytes(&body);

    let (status, resp_headers, resp_body) = match remote_resp {
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
            }
            (status, headers, buf)
        }
        Err(e) => {
            let _ = request.respond(
                Response::from_string(format!("Failed to contact remote server: {}", e))
                    .with_status_code(502),
            );
            return;
        }
    };

    // Determine if we need to proxy the request to the backend.
    let should_proxy = if proxy_enabled {
        if let Ok(remote_json) = serde_json::from_slice::<RemoteResponse>(&resp_body) {
            remote_json.proxy
        } else {
            // If the response isn't valid JSON, assume it's a mocked response.
            false
        }
    } else {
        false
    };

    if should_proxy {
        // If in proxy mode and the remote response indicates proxying,
        // forward the request to the local backend.
        if let Some(backend) = backend_base {
            let backend_url = format!("{}{}", backend.trim_end_matches('/'), request.url());
            println!("Forwarding to backend: {}", backend_url);
            
            let backend_resp = ureq::request(method_str, &backend_url)
                .send_bytes(&body);

            match backend_resp {
                Ok(resp) => {
                    let status = resp.status();
                    let mut headers = Vec::new();
                    for h in resp.headers_names() {
                        if let Some(val) = resp.header(&h) {
                            headers.push(Header::from_bytes(h.as_bytes(), val.as_bytes()).unwrap());
                        }
                    }
                    let mut buf = Vec::new();
                    if let Err(e) = resp.into_reader().read_to_end(&mut buf) {
                        eprintln!("Error reading backend response body: {}", e);
                    }
                    let response = Response::from_data(buf).with_status_code(status);
                    let response = headers.into_iter().fold(response, |resp, h| resp.with_header(h));
                    let _ = request.respond(response);
                }
                Err(e) => {
                    let _ = request.respond(
                        Response::from_string(format!("Failed to contact backend: {}", e))
                            .with_status_code(502),
                    );
                }
            }
            return;
        } else {
            let _ = request.respond(
                Response::from_string("Proxy mode enabled but no backend URL provided")
                    .with_status_code(500),
            );
            return;
        }
    }

    // Otherwise, return the remote response.
    let response = {
        // Build headers for tiny-http.
        let headers = resp_headers
            .into_iter()
            .map(|(k, v)| Header::from_bytes(k.as_bytes(), v.as_bytes()).unwrap())
            .collect::<Vec<_>>();
        let mut resp = Response::from_data(resp_body).with_status_code(status);
        for header in headers {
            resp = resp.with_header(header);
        }
        resp
    };

    let _ = request.respond(response);
} 