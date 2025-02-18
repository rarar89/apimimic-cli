use clap::{Parser, Subcommand};

/// Command-line interface definition.
#[derive(Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Save the default project Key.
    SetProject {
        /// The project Key to be saved.
        project: String,
    },
    /// Run the CLI utility (starts the HTTP server).
    Run {
        /// Project Key
        #[arg(short, long)]
        project: Option<String>,

        /// Local address to listen on
        #[arg(short, long, default_value = "127.0.0.1:8080")]
        listen: String,

        /// Remote API Mimic URL
        #[arg(short, long, default_value = "https://cli.apimimic.com")]
        remote: String,

        /// Target server URL (if provided, unmocked requests will be forwarded here)
        #[arg(long)]
        server: Option<String>,

        /// Remote ping URL
        #[arg(long, default_value = "https://cli-checkin.apimimic.com")]
        remote_ping: String,
    },
} 