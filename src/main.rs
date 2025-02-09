mod cli;
mod config;
mod server;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    // Load any saved configuration.
    let mut config = config::load_config();

    match &cli.command {
        Some(Commands::SetToken { token }) => {
            config.auth_token = token.clone();
            if let Err(e) = config::save_config(&config) {
                eprintln!("Failed to save token: {}", e);
                std::process::exit(1);
            }
            println!("Token saved successfully.");
        }
        Some(Commands::Run { project, listen, remote, token, proxy, backend }) => {
            // Use the token provided on the command line (if any) to override the saved token.
            if let Some(t) = token {
                config.auth_token = t.clone();
                // Also save the new token.
                if let Err(e) = config::save_config(&config) {
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

            server::run_server(
                listen,
                remote.clone(),
                project.clone(),
                config.auth_token,
                *proxy,
                backend.clone(),
            );
        }
        None => {
            // Default to showing help
            let _ = Cli::parse_from(&["--help"]);
        }
    }
}
