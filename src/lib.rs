//! # Common Repository Library
//!
//! This library provides the core functionality for managing and processing
//! shared repository configurations. It is designed to be used by the `common-repo`
//! command-line tool but can also be integrated into other applications that
//! require multi-repository configuration management.
//!
//! ## Core Concepts
//!
//! The library is built around a few key concepts:
//!
//! - **Configuration (`config`)**: Defines the schema for `.common-repo.yaml` files,
//!   including the various operations that can be performed.
//! - **In-Memory Filesystem (`filesystem`)**: A virtual filesystem used to stage
//!   changes before writing them to disk, enabling complex manipulations and dry runs.
//! - **Operators (`operators`)**: Individual actions that modify the in-memory
//!   filesystem, such as including, excluding, renaming, or templating files.
//! - **Phases (`phases`)**: A multi-stage pipeline that orchestrates the entire
//!   process, from repository discovery and cloning to applying operators and
//!   writing the final result.
//! - **Repository Management (`repository`, `git`, `cache`)**: Handles cloning,
//!   caching, and loading Git repositories.
//!
//! ## Execution Flow
//!
//! The main entry point is the `phases::orchestrator`, which executes the
//! following high-level steps:
//!
//! 1.  **Discovery**: Recursively find all inherited repositories.
//! 2.  **Cloning**: Clone all discovered repositories in parallel (with caching).
//! 3.  **Processing**: Apply operations to each repository to create an intermediate
//!     filesystem.
//! 4.  **Ordering**: Determine the correct, deterministic order to merge filesystems.
//! 5.  **Composition**: Merge all intermediate filesystems into a single composite
//!     filesystem.
//! 6.  **Local Merge**: Merge the composite filesystem with local files.
//! 7.  **Disk Output**: Write the final filesystem to the specified output directory.
//!
//! By separating the logic into these distinct modules and phases, the library
//! provides a flexible and extensible framework for managing shared configurations.

pub mod cache;
pub mod config;
pub mod error;
pub mod filesystem;
pub mod git;
pub mod merge;
pub mod operators;
pub mod path;
pub mod phases;
pub mod repository;
pub mod version;

#[cfg(test)]
mod path_proptest;
