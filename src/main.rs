mod cli;
mod config;
mod server;

use clap::Parser;
use cli::{Cli, Commands};
use env_logger::Env;

fn main() {
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
        Some(Commands::Run { project, listen, remote, server }) => {
            // Use the project provided on the command line to override the saved project.
            let project = if project.is_empty() {
                if config.project.is_empty() {
                    eprintln!("No project provided. Use the -p/--project flag or `set-project` command.");
                    std::process::exit(1);
                }
                config.project.clone()
            } else {
                project.clone()
            };

            server::run_server(
                listen,
                remote.clone(),
                project,
                server.is_some(),
                server.clone(),
            );
        }
        None => {
            // Default to showing help
            let _ = Cli::parse_from(&["--help"]);
        }
    }
}
