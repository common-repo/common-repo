//! # Common Repository CLI
//!
//! This is the binary entry point for the `common-repo` command-line tool.
//!
//! Its primary responsibilities are:
//! - Parsing command-line arguments using `clap`.
//! - Executing the appropriate command based on the parsed arguments.
//! - Handling top-level application errors and translating them into user-friendly
//!   output.
//!
//! The core application logic is defined in the `lib.rs` library crate, ensuring
//! that the binary is a thin wrapper around the reusable library functionality.

mod cli;
mod commands;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    cli.execute()
}
