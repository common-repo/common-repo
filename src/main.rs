//! Common Repository CLI
//!
//! Binary entry point for the common-repo command-line tool.

mod cli;
mod commands;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    cli.execute()
}
