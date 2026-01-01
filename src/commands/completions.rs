//! # Completions Command Implementation
//!
//! This module implements the `completions` subcommand, which generates shell
//! completion scripts for various shells. The completions are generated using
//! `clap_complete` and can be installed to enable tab-completion for all
//! `common-repo` commands and options.
//!
//! ## Supported Shells
//!
//! - **Bash**: Add to `.bashrc` or source directly
//! - **Zsh**: Add to `fpath` or source directly
//! - **Fish**: Save to `~/.config/fish/completions/`
//! - **PowerShell**: Add to PowerShell profile
//!
//! ## Example
//!
//! ```bash
//! # Generate and install bash completions
//! common-repo completions bash > ~/.local/share/bash-completion/completions/common-repo
//!
//! # Generate zsh completions
//! common-repo completions zsh > ~/.zfunc/_common-repo
//! ```

use anyhow::Result;
use clap::{Args, CommandFactory, ValueEnum};
use clap_complete::{generate, Shell};
use std::io;

use crate::cli::Cli;

/// Shell types for completion generation
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CompletionShell {
    /// Bourne Again Shell
    Bash,
    /// Z Shell
    Zsh,
    /// Fish Shell
    Fish,
    /// PowerShell
    #[value(name = "powershell")]
    PowerShell,
    /// Elvish Shell
    Elvish,
}

impl From<CompletionShell> for Shell {
    fn from(shell: CompletionShell) -> Self {
        match shell {
            CompletionShell::Bash => Shell::Bash,
            CompletionShell::Zsh => Shell::Zsh,
            CompletionShell::Fish => Shell::Fish,
            CompletionShell::PowerShell => Shell::PowerShell,
            CompletionShell::Elvish => Shell::Elvish,
        }
    }
}

/// Generate shell completion scripts
#[derive(Args, Debug)]
pub struct CompletionsArgs {
    /// The shell to generate completions for
    #[arg(value_enum)]
    pub shell: CompletionShell,
}

/// Execute the `completions` command.
///
/// Generates shell completion scripts for the specified shell and writes them
/// to stdout. Users can redirect the output to an appropriate file for their
/// shell configuration.
pub fn execute(args: CompletionsArgs) -> Result<()> {
    let mut cmd = Cli::command();
    let shell: Shell = args.shell.into();
    generate(shell, &mut cmd, "common-repo", &mut io::stdout());
    Ok(())
}
