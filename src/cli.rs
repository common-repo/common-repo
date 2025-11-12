//! CLI argument parsing and command dispatch

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
    /// Execute the CLI command
    pub fn execute(self) -> Result<()> {
        // TODO: Set up logging based on log_level
        // TODO: Set up color output based on color flag

        match self.command {
            Commands::Apply(args) => commands::apply::execute(args),
        }
    }
}
