use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Configuration stored on disk.
#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub auth_token: String,
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
pub fn load_config() -> Config {
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
pub fn save_config(config: &Config) -> std::io::Result<()> {
    if let Some(config_path) = get_config_path() {
        let data = serde_json::to_string_pretty(config).unwrap();
        fs::write(config_path, data)?;
    }
    Ok(())
} 