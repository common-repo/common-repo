//! Prose linter to detect AI writing patterns in documentation and code comments.
//!
//! This module scans markdown files and Rust doc comments for common AI-generated
//! writing patterns and reports them with suggestions for improvement.

use anyhow::Result;
use std::path::PathBuf;

/// Output format for the prose check results.
#[derive(Debug, Clone, Copy, Default)]
pub enum OutputFormat {
    /// Human-readable text output
    #[default]
    Text,
    /// JSON output for machine processing
    Json,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            _ => Err(format!("Unknown format '{}'. Use: text or json", s)),
        }
    }
}

/// Configuration for the prose check command.
#[derive(Debug)]
pub struct CheckProseConfig {
    /// Paths to check (files or directories)
    pub paths: Vec<PathBuf>,
    /// Output format
    pub format: OutputFormat,
    /// Verbose output
    pub verbose: bool,
}

impl Default for CheckProseConfig {
    fn default() -> Self {
        Self {
            paths: vec![PathBuf::from(".")],
            format: OutputFormat::Text,
            verbose: false,
        }
    }
}

/// Run the prose linter with the given configuration.
pub fn run(config: CheckProseConfig) -> Result<()> {
    if config.verbose {
        println!("Checking prose in {} path(s)...", config.paths.len());
        for path in &config.paths {
            println!("  - {}", path.display());
        }
    }

    // TODO: Implement pattern matching in subsequent tasks
    println!("Prose check command structure ready.");
    println!("Pattern matching will be implemented in the next task.");

    Ok(())
}
