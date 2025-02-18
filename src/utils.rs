/// Formats the listen address by removing http:// prefix and rejecting https://
pub fn parse_listen_address(listen: &str) -> Result<String, String> {
    if listen.to_lowercase().starts_with("https://") {
        return Err("HTTPS is not supported for local server".to_string());
    }
    
    Ok(listen.to_lowercase()
        .trim_start_matches("http://")
        .to_string())
}

/// Ensures server URL has http:// prefix
pub fn parse_server_url(server: &Option<String>) -> Option<String> {
    server.as_ref().map(|s| {
        if !s.to_lowercase().starts_with("http://") && 
           !s.to_lowercase().starts_with("https://") {
            format!("http://{}", s)
        } else {
            s.to_string()
        }
    })
} 