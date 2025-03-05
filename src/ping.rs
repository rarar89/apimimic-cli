use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use serde::{Deserialize, Serialize};
use log::{info, error, debug};

#[derive(Debug, Serialize)]
struct PingRequest {
    version: &'static str,
    listen: String,
    project: String,
    server: Option<String>,
    remote: String,
    remote_ping: String,
}

#[derive(Debug, Deserialize)]
struct EndpointConfig {
    time: u64,
    proxied: bool,
}

#[derive(Debug, Deserialize)]
struct PingResponse {
    message: String,
    endpoints: HashMap<String, EndpointConfig>,
}

pub struct EndpointInfo {
    pub timeout: u64,
    pub proxied: bool,
    pub timestamp: std::time::Instant,
}

pub struct EndpointManager {
    endpoints: Arc<RwLock<HashMap<String, EndpointInfo>>>,
}

impl EndpointManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            endpoints: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn get_endpoint_info(&self, path: &str) -> Option<(u64, bool)> {
        let endpoints = self.endpoints.read().await;
        endpoints.get(path).and_then(|info| {
            if info.timestamp.elapsed().as_secs() > 20 {
                None
            } else {
                Some((info.timeout, info.proxied))
            }
        })
    }

    pub async fn start_ping_service(
        &self,
        listen: String,
        remote: String,
        remote_ping: String,
        project: String,
        server: Option<String>,
    ) {
        let endpoints = self.endpoints.clone();
        
        tokio::spawn(async move {
            let client = reqwest::Client::new();
            let mut interval = interval(Duration::from_secs(10));

            loop {
                interval.tick().await;

                let ping_request = PingRequest {
                    version: env!("CARGO_PKG_VERSION"),
                    listen: listen.clone(),
                    project: project.clone(),
                    server: server.clone(),
                    remote: remote.clone(),
                    remote_ping: remote_ping.clone(),
                };

                match client.post(&remote_ping)
                    .json(&ping_request)
                    .send()
                    .await {
                    Ok(response) => {
                        if response.status().is_success() {

                            let status = response.status();

                            match response.json::<PingResponse>().await {
                                Ok(ping_response) => {
                                    info!("Apimimic ping successful {}", status);
                                    debug!("Received ping response: {:?}", ping_response.message);
                                    let mut endpoints_write = endpoints.write().await;
                                    endpoints_write.clear();
                                    
                                    let now = std::time::Instant::now();
                                    for (path, config) in ping_response.endpoints {
                                        endpoints_write.insert(path, EndpointInfo {
                                            timeout: config.time,
                                            proxied: config.proxied,
                                            timestamp: now,
                                        });
                                    }
                                }
                                Err(e) => error!("Failed to parse ping response: {}", e),
                            }
                        } else {
                            error!("Ping request failed with status: {} {}", response.status(), remote_ping);
                        }
                    }
                    Err(e) => error!("Failed to send ping request: {}", e),
                }
            }
        });
    }
} 