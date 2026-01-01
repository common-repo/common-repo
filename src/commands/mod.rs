//! # CLI Command Implementations
//!
//! This module contains the implementation for each subcommand of the `common-repo`
//! command-line tool. Each subcommand is defined in its own file to keep the
//! logic separated and maintainable.
//!
//! ## Structure
//!
//! Each command module typically contains:
//! - An `Args` struct that defines the command-specific arguments and options,
//!   derived using `clap`.
//! - An `execute` function that takes the parsed `Args` and performs the
//!   command's logic.
//!
//! The `execute` function is the main entry point for the command and is
//! responsible for orchestrating the necessary operations, calling into the
//! `common_repo` library to perform the core logic.

pub mod add;
pub mod apply;
pub mod cache;
pub mod check;
pub mod completions;
pub mod diff;
pub mod info;
pub mod init;
pub mod ls;
pub mod tree;
pub mod update;
pub mod validate;
