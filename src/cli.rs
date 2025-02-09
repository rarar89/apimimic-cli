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
    /// Save the authentication token.
    SetToken {
        /// The token to be saved.
        token: String,
    },
    /// Save the default project ID.
    SetProject {
        /// The project ID to be saved.
        project: String,
    },
    /// Run the CLI utility (starts the HTTP server).
    Run {
        /// Project ID
        #[arg(short, long)]
        project: String,

        /// Local address to listen on (default: 127.0.0.1:8080)
        #[arg(short, long, default_value = "127.0.0.1:8080")]
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