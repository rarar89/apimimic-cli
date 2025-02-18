mod cli;
mod config;
mod server;
mod ping;
mod utils;

use clap::Parser;
use cli::{Cli, Commands};
use env_logger::Env;
use utils::{parse_listen_address, parse_server_url};

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let cli = Cli::parse();

    // Load any saved configuration.
    let mut config = config::load_config();

    match &cli.command {
        Some(Commands::SetProject { project }) => {
            config.project = project.clone();
            if let Err(e) = config::save_config(&config) {
                eprintln!("Failed to save project: {}", e);
                std::process::exit(1);
            }
            println!("Project saved successfully.");
        }
        Some(Commands::Run { project, listen, remote, server, remote_ping }) => {
            let project = match project {
                Some(p) if !p.is_empty() => p,
                _ if !config.project.is_empty() => &config.project.clone(),
                _ => {
                    eprintln!("No project provided. Use the -p/--project flag or `set-project` command.");
                    std::process::exit(1);
                }
            };

            // Parse listen address
            let listen = match parse_listen_address(listen) {
                Ok(addr) => addr,
                Err(e) => {
                    eprintln!("Invalid listen address: {}", e);
                    std::process::exit(1);
                }
            };

            // Parse server URL
            let server = parse_server_url(server);

            server::run_server(
                &listen,
                remote.clone(),
                project.clone(),
                server.is_some(),
                server,
                remote_ping.clone(),
            ).await;
        }
        None => {
            // Default to showing help
            let _ = Cli::parse_from(&["--help"]);
        }
    }
}
