//! # CLI Argument Parsing and Command Dispatch
//!
//! This module defines the command-line interface for the `common-repo` tool
//! using the `clap` library. It is responsible for:
//!
//! - Defining the top-level CLI structure, including global arguments like
//!   `--color` and `--log-level`.
//! - Defining the available subcommands (e.g., `apply`, `check`, `update`).
//! - Parsing the command-line arguments provided by the user.
//! - Dispatching to the appropriate command implementation based on the
//!   parsed arguments.
//!
//! Each subcommand is implemented in its own module under `src/commands/` to
//! keep the code organized and maintainable.

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::commands;

/// Common Repository - Manage repository configuration inheritance
#[derive(Parser, Debug)]
#[command(name = "common-repo")]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Subcommand to execute
    #[command(subcommand)]
    command: Commands,

    /// Colorize output (always, never, auto)
    #[arg(long, global = true, value_name = "WHEN", default_value = "auto")]
    color: String,

    /// Set log level (error, warn, info, debug, trace)
    #[arg(long, global = true, value_name = "LEVEL", default_value = "info")]
    log_level: String,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Apply the .common-repo.yaml configuration to the current repository
    Apply(commands::apply::ApplyArgs),

    /// Check configuration validity and check for repository updates
    Check(commands::check::CheckArgs),

    /// Update repository refs to newer versions
    Update(commands::update::UpdateArgs),
    // Future commands will be added here:
    // /// Validate a .common-repo.yaml configuration file
    // Validate(commands::validate::ValidateArgs),
    //
    // /// Initialize a new .common-repo.yaml configuration
    // Init(commands::init::InitArgs),
    //
    // /// Manage repository cache
    // Cache(commands::cache::CacheArgs),
}

impl Cli {
    /// Execute the parsed CLI command
    pub fn execute(self) -> Result<()> {
        // TODO: Initialize a logger (e.g., env_logger) based on `self.log_level`.
        // TODO: Configure color output for the terminal using a library like `termcolor`
        //       based on `self.color`.

        match self.command {
            Commands::Apply(args) => commands::apply::execute(args),
            Commands::Check(args) => commands::check::execute(args),
            Commands::Update(args) => commands::update::execute(args),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_execute_check_command() {
        // Test that check command dispatching works (covers line 69)
        let cli = Cli {
            command: Commands::Check(commands::check::CheckArgs {
                config: PathBuf::from("/nonexistent/config.yaml"),
                cache_root: None,
                updates: false,
            }),
            color: "auto".to_string(),
            log_level: "info".to_string(),
        };

        // This should fail because the config file doesn't exist, but it covers the match arm
        let result = cli.execute();
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_update_command() {
        // Test that update command dispatching works (covers line 70)
        let cli = Cli {
            command: Commands::Update(commands::update::UpdateArgs {
                config: PathBuf::from("/nonexistent/config.yaml"),
                cache_root: None,
                compatible: false,
                latest: false,
                yes: false,
                dry_run: true, // Use dry run to avoid actual changes
            }),
            color: "auto".to_string(),
            log_level: "info".to_string(),
        };

        // This should fail because the config file doesn't exist, but it covers the match arm
        let result = cli.execute();
        assert!(result.is_err());
    }
}
