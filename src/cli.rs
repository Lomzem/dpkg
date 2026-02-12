use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "dpkg", version, about = "Declarative package manager for Arch Linux")]
pub struct Cli {
    /// Path to config file
    #[arg(short, long, env = "DPKG_CONFIG", global = true)]
    pub config: Option<PathBuf>,

    /// Show what would be done without executing
    #[arg(short = 'n', long, global = true)]
    pub dry_run: bool,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress non-error output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Synchronize system state with configuration file (default)
    Sync {
        /// Skip confirmation for removals
        #[arg(long)]
        no_confirm: bool,

        /// Only install missing packages, don't remove orphans
        #[arg(long)]
        only_install: bool,

        /// Only remove orphans, don't install packages
        #[arg(long)]
        only_remove: bool,
    },

    /// Display current synchronization status
    Status,

    /// Validate configuration file syntax
    Validate,

    /// Show differences between config and system state
    Diff,
}

impl Cli {
    pub fn config_path(&self) -> PathBuf {
        if let Some(ref path) = self.config {
            path.clone()
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
            PathBuf::from(home).join(".config/dpkg/pkg.conf")
        }
    }
}
