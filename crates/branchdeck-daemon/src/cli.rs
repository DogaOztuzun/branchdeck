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

    /// Show daemon health, active runs, and workflow count
    Status {
        /// Daemon port
        #[arg(long, default_value_t = 13371, env = "BRANCHDECK_PORT")]
        port: u16,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Trigger a workflow
    Trigger {
        /// Workflow name to trigger
        workflow: String,

        /// Daemon port
        #[arg(long, default_value_t = 13371, env = "BRANCHDECK_PORT")]
        port: u16,

        /// Task path for the run
        #[arg(long)]
        task_path: Option<String>,

        /// Worktree path
        #[arg(long)]
        worktree_path: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List or manage runs
    Runs {
        #[command(subcommand)]
        action: Option<RunsAction>,

        /// Daemon port
        #[arg(long, default_value_t = 13371, env = "BRANCHDECK_PORT")]
        port: u16,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Check for and apply updates
    Update {
        /// Daemon port
        #[arg(long, default_value_t = 13371, env = "BRANCHDECK_PORT")]
        port: u16,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub enum RunsAction {
    /// Cancel a running run
    Cancel {
        /// Run session ID to cancel
        id: String,
    },
}
