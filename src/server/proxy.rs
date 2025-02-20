use hyper::{Body, Response};
use log::{debug, error, info};
use std::convert::Infallible;
use tokio::time::sleep;
use std::time::Duration;
use itertools::Itertools;

/// Handles proxying a request to a target server
pub async fn proxy_request(
    client: &reqwest::Client,
    method_str: String,
    uri_string: String,
    server_url: String,
    headers: Vec<(String, String)>,
    body: impl Into<reqwest::Body>,
    timeout: Option<u64>,
) -> Result<Response<Body>, Infallible> {
    let full_url = format!("{}{}", server_url.trim_end_matches('/'), uri_string);
    info!("Proxying request to target server: {}", full_url);

    let mut proxy_req = client.request(
        reqwest::Method::from_bytes(method_str.as_bytes()).unwrap(),
        &full_url
    );

    debug!("headers sent to target server: {:?}", headers);

    // Add original headers to proxy request
    for (name, value) in headers {
        if name.to_lowercase() != "host" {
          proxy_req = proxy_req.header(&name, &value);
        }
    }

    // Set host header from target server URL
    if let Ok(parsed_url) = url::Url::parse(&full_url) {
        if let Some(host) = parsed_url.host_str() {
            let host_value = if let Some(port) = parsed_url.port() {
                format!("{}:{}", host, port)
            } else {
                host.to_string()
            };
            debug!("host_value: {}", host_value);
            proxy_req = proxy_req.header("Host", &host_value);
        }
    }

    // Send request to target server
    match proxy_req.body(body).send().await {
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
            
            let headers_clone = proxy_headers.clone();
            let cookies: Vec<_> = headers_clone
                .get_all("set-cookie")
                .iter()
                .filter_map(|v| v.to_str().ok())
                .collect();
            
          
            let mut response_builder = Response::builder()
                .status(proxy_status);

            // Add regular headers
            debug!("response headers: {:?}", proxy_headers);
            for (name, value) in &proxy_headers.into_iter().collect_vec() {
                if let Some(name) = name {
                    if name != "set-cookie" {  // Skip set-cookie headers as we'll handle them separately
                      response_builder = response_builder.header(name.as_str(), value);
                    }
                }
            }

            // Add cookies
            for cookie in cookies {
                debug!("adding cookie header: {:?}", cookie);
                response_builder = response_builder.header("set-cookie", cookie);
            }

            debug!("response headers after appending: {:?}", response_builder.headers_mut());

            // Apply timeout if specified
            if let Some(timeout) = timeout {
                if timeout > 0 {
                    info!("Sleeping for {}ms", timeout);
                    sleep(Duration::from_millis(timeout)).await;
                }
            }

            info!("Returning response from target server: {}", proxy_status);
            Ok(response_builder.body(Body::from(proxy_body)).unwrap())
        }
        Err(e) => {
            error!("Failed to contact target server: {}", e);
            Ok(Response::builder()
                .status(502)
                .body(Body::from(format!("Failed to contact target server: {}", e)))
                .unwrap())
        }
    }
} 