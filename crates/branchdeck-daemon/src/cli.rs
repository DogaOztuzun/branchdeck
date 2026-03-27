use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "branchdeck-daemon", about = "Branchdeck daemon server")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the daemon HTTP server
    Serve {
        /// Port to listen on
        #[arg(long, default_value_t = 13371, env = "BRANCHDECK_PORT")]
        port: u16,

        /// Workspace root directory (defaults to current directory)
        #[arg(long, env = "BRANCHDECK_WORKSPACE")]
        workspace: Option<std::path::PathBuf>,
    },
}
