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
use log::LevelFilter;

use crate::commands;

/// Common Repository - Manage repository configuration inheritance
#[derive(Parser, Debug)]
#[command(name = "common-repo")]
#[command(
    version,
    about,
    long_about = "Common Repository - Manage repository configuration inheritance"
)]
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
    /// Add a repository to the configuration file
    Add(commands::add::AddArgs),

    /// Apply the .common-repo.yaml configuration to the current repository
    Apply(commands::apply::ApplyArgs),

    /// Check configuration validity and check for repository updates
    Check(commands::check::CheckArgs),

    /// Generate shell completion scripts
    Completions(commands::completions::CompletionsArgs),

    /// Show differences between current files and configuration result
    Diff(commands::diff::DiffArgs),

    /// Initialize a new .common-repo.yaml configuration file
    Init(commands::init::InitArgs),

    /// Update repository refs to newer versions
    Update(commands::update::UpdateArgs),

    /// Show information about a repository or the current configuration
    Info(commands::info::InfoArgs),

    /// List files that would be created/modified by the configuration
    Ls(commands::ls::LsArgs),

    /// Validate a .common-repo.yaml configuration file
    Validate(commands::validate::ValidateArgs),

    /// Manage repository cache
    Cache(commands::cache::CacheArgs),

    /// Display the repository inheritance tree
    Tree(commands::tree::TreeArgs),
}

impl Cli {
    /// Execute the parsed CLI command
    pub fn execute(self) -> Result<()> {
        // Initialize logger based on log level
        self.init_logger()?;

        match self.command {
            Commands::Add(args) => commands::add::execute(args),
            Commands::Apply(args) => commands::apply::execute(args),
            Commands::Check(args) => commands::check::execute(args),
            Commands::Completions(args) => commands::completions::execute(args),
            Commands::Diff(args) => {
                // Diff command uses exit code 1 to indicate changes exist
                // (following the convention of diff(1) and git diff)
                match commands::diff::execute(args) {
                    Ok(()) => Ok(()),
                    Err(e) if e.to_string() == "CHANGES_DETECTED" => {
                        // Exit with code 1 for changes detected
                        std::process::exit(common_repo::exit_codes::ERROR);
                    }
                    Err(e) => Err(e),
                }
            }
            Commands::Info(args) => commands::info::execute(args),
            Commands::Init(args) => commands::init::execute(args),
            Commands::Ls(args) => commands::ls::execute(args),
            Commands::Update(args) => commands::update::execute(args),
            Commands::Validate(args) => commands::validate::execute(args, &self.color),
            Commands::Cache(args) => commands::cache::execute(args),
            Commands::Tree(args) => commands::tree::execute(args, &self.color),
        }
    }

    /// Initialize the logger with the specified log level and color settings
    fn init_logger(&self) -> Result<()> {
        let log_level = self.parse_log_level()?;
        let use_color = self.should_use_color();

        env_logger::Builder::from_default_env()
            .filter_level(log_level)
            .write_style(if use_color {
                env_logger::WriteStyle::Auto
            } else {
                env_logger::WriteStyle::Never
            })
            .format_timestamp(None)
            .format_module_path(false)
            .format_target(false)
            .try_init()
            .map_err(|e| anyhow::anyhow!("Failed to initialize logger: {}", e))?;

        Ok(())
    }

    /// Parse the log level string into a LevelFilter
    fn parse_log_level(&self) -> Result<LevelFilter> {
        match self.log_level.to_lowercase().as_str() {
            "error" => Ok(LevelFilter::Error),
            "warn" => Ok(LevelFilter::Warn),
            "info" => Ok(LevelFilter::Info),
            "debug" => Ok(LevelFilter::Debug),
            "trace" => Ok(LevelFilter::Trace),
            "off" => Ok(LevelFilter::Off),
            _ => Err(anyhow::anyhow!(
                "Invalid log level: '{}'. Valid options are: error, warn, info, debug, trace, off",
                self.log_level
            )),
        }
    }

    /// Determine whether to use color output based on the color setting
    fn should_use_color(&self) -> bool {
        match self.color.to_lowercase().as_str() {
            "always" => true,
            "never" => false,
            "auto" => console::Term::stdout().features().colors_supported(),
            _ => {
                // Default to auto if invalid value provided
                eprintln!(
                    "Warning: Invalid color option '{}', using 'auto'. Valid options are: always, never, auto",
                    self.color
                );
                console::Term::stdout().features().colors_supported()
            }
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
