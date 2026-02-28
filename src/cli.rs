//! Command-line interface definition.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// The main CLI argument structure.
#[derive(Parser, Debug)]
#[command(name = "mcp-guard", version, about = "Layer 7 MCP Firewall Proxy")]
pub struct Cli {
    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: Commands,
}

/// Available CLI subcommands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Launch proxy, enforce policy, and wrap target command
    Run {
        /// Path to the policy configuration file (toml or yaml format)
        #[arg(long, short)]
        policy: PathBuf,

        /// The target server executable to wrap
        #[arg(required = true)]
        target_executable: String,

        /// Arguments for the target server executable
        #[arg(last = true)]
        target_args: Vec<String>,
    },

    /// Tail or query the local audit log
    Audit {
        /// Path to the audit log file to read (JSONL format)
        #[arg(long, short)]
        log_file: Option<PathBuf>,
    },
}
