use clap::{Parser, Subcommand};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::thread;
use tiny_http::{Header, Request, Response, Server};

/// Configuration stored on disk.
#[derive(Serialize, Deserialize, Default)]
struct Config {
    auth_token: String,
}

/// Structure of the JSON response from the remote API Mimic service.
#[derive(Deserialize)]
struct RemoteResponse {
    proxy: bool,
    // Optionally you can add more fields like status, headers, body, etc.
}

/// Command-line interface definition.
#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show help message.
    Help,
    /// Save the authentication token.
    SetToken {
        /// The token to be saved.
        token: String,
    },
    /// Run the CLI utility (starts the HTTP server).
    Run {
        /// Local address to listen on (default: 0.0.0.0:8080)
        #[arg(short, long, default_value = "0.0.0.0:8080")]
        listen: String,

        /// Remote API Mimic URL (default: https://cli.apimimic.com)
        #[arg(short, long, default_value = "https://cli.apimimic.com")]
        remote: String,

        /// Authorization token (overrides saved token)
        #[arg(short, long)]
        token: Option<String>,

        /// Enable proxy mode (forward requests to a local backend)
        #[arg(long)]
        proxy: bool,

        /// Local backend URL (required if proxy mode is enabled)
        #[arg(long)]
        backend: Option<String>,
    },
}

/// Get the configuration file path in a cross-platform way.
fn get_config_path() -> Option<PathBuf> {
    if let Some(proj_dirs) = ProjectDirs::from("com", "apimimic", "apimimic-cli") {
        let config_dir = proj_dirs.config_dir();
        fs::create_dir_all(config_dir).ok()?;
        let config_file = config_dir.join("config.json");
        Some(config_file)
    } else {
        None
    }
}

/// Load the configuration from disk.
fn load_config() -> Config {
    if let Some(config_path) = get_config_path() {
        if let Ok(data) = fs::read_to_string(config_path) {
            if let Ok(cfg) = serde_json::from_str(&data) {
                return cfg;
            }
        }
    }
    Config::default()
}

/// Save the configuration to disk.
fn save_config(config: &Config) -> std::io::Result<()> {
    if let Some(config_path) = get_config_path() {
        let data = serde_json::to_string_pretty(config).unwrap();
        fs::write(config_path, data)?;
    }
    Ok(())
}

/// Handles an individual incoming HTTP request.
fn handle_request(
    request: Request,
    remote_base: String,
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
    // (You might want to improve this by using the `url` crate in production.)
    let remote_url = format!("{}{}", remote_base.trim_end_matches('/'), request.url());

    // Forward the request to the remote API Mimic service.
    let mut remote_req = ureq::request(request.method(), &remote_url);
    remote_req.set("Authorization", &format!("Bearer {}", auth_token));

    // Copy headers from the incoming request.
    for header in request.headers() {
        // Skip Host header since the client library sets it automatically.
        if header.field.as_str().to_lowercase() == "host" {
            continue;
        }
        remote_req.set(header.field.as_str(), header.value.as_str());
    }

    // Send the request (using send_bytes to support any method and binary body).
    let remote_resp = remote_req.send_bytes(&body);

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
            // If the response isn’t valid JSON, assume it’s a mocked response.
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
            let mut backend_req = ureq::request(request.method(), &backend_url);
            // Copy original headers.
            for header in request.headers() {
                if header.field.as_str().to_lowercase() == "host" {
                    continue;
                }
                backend_req.set(header.field.as_str(), header.value.as_str());
            }
            let backend_resp = backend_req.send_bytes(&body);
            match backend_resp {
                Ok(resp) => {
                    let status = resp.status();
                    let mut headers = Vec::new();
                    for h in resp.headers_names() {
                        if let Some(val) = resp.header(&h) {
                            headers.push(Header::from_bytes(&h, val).unwrap());
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
            .map(|(k, v)| Header::from_bytes(&k, &v).unwrap())
            .collect::<Vec<_>>();
        let mut resp = Response::from_data(resp_body).with_status_code(status);
        for header in headers {
            resp = resp.with_header(header);
        }
        resp
    };

    let _ = request.respond(response);
}

fn main() {
    let cli = Cli::parse();

    // Load any saved configuration.
    let mut config = load_config();

    match &cli.command {
        Some(Commands::Help) => {
            // The auto-generated help from clap is usually enough.
            Cli::command().print_help().unwrap();
        }
        Some(Commands::SetToken { token }) => {
            config.auth_token = token.clone();
            if let Err(e) = save_config(&config) {
                eprintln!("Failed to save token: {}", e);
                std::process::exit(1);
            }
            println!("Token saved successfully.");
        }
        Some(Commands::Run { listen, remote, token, proxy, backend }) | None => {
            // Use the token provided on the command line (if any) to override the saved token.
            if let Some(t) = token {
                config.auth_token = t.clone();
                // Also save the new token.
                if let Err(e) = save_config(&config) {
                    eprintln!("Failed to save token: {}", e);
                }
            }

            if config.auth_token.is_empty() {
                eprintln!("No token provided. Use the --token flag or `set-token` command.");
                std::process::exit(1);
            }

            if *proxy && backend.is_none() {
                eprintln!("Proxy mode enabled but no backend URL provided.");
                std::process::exit(1);
            }

            println!("Starting server on {}", listen);
            let server = Server::http(listen).unwrap();

            // Clone values to move into the request handling closure.
            let remote_base = remote.clone();
            let auth_token = config.auth_token.clone();
            let proxy_enabled = *proxy;
            let backend_base = backend.clone();

            // Handle each incoming request in its own thread.
            for request in server.incoming_requests() {
                let remote_base = remote_base.clone();
                let auth_token = auth_token.clone();
                let backend_base = backend_base.clone();
                thread::spawn(move || {
                    handle_request(request, remote_base, auth_token, proxy_enabled, backend_base)
                });
            }
        }
    }
}
