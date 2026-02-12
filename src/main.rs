mod cli;
mod commands;
mod config;
mod error;
mod output;
mod system;

use std::process;

use clap::Parser;

use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();
    let config_path = cli.config_path();

    let result = match cli.command {
        Some(Command::Sync {
            no_confirm,
            only_install,
            only_remove,
        }) => commands::sync::run(
            &config_path,
            &commands::sync::SyncOptions {
                dry_run: cli.dry_run,
                verbose: cli.verbose,
                quiet: cli.quiet,
                no_confirm,
                only_install,
                only_remove,
            },
        ),
        Some(Command::Status) => commands::status::run(&config_path, cli.quiet),
        Some(Command::Validate) => commands::validate::run(&config_path, cli.quiet),
        Some(Command::Diff) => commands::diff::run(&config_path, cli.quiet),
        // Default: sync with no extra options
        None => commands::sync::run(
            &config_path,
            &commands::sync::SyncOptions {
                dry_run: cli.dry_run,
                verbose: cli.verbose,
                quiet: cli.quiet,
                no_confirm: false,
                only_install: false,
                only_remove: false,
            },
        ),
    };

    if let Err(e) = result {
        output::error(&format!("Error: {e}"));
        process::exit(e.exit_code());
    }
}
