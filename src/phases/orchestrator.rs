//! Orchestrator for the complete pull operation
//!
//! This module coordinates all phases to provide a clean API for the complete
//! pull operation. Currently implements Phases 1-6 for end-to-end inheritance.

use super::{phase1, phase2, phase3, phase4, phase5, phase6};
use crate::cache::RepoCache;
use crate::config::Schema;
use crate::error::Result;
use crate::filesystem::MemoryFS;
use crate::repository::RepositoryManager;
use std::path::Path;

/// Execute the complete pull operation (Phases 1-6)
///
/// This orchestrates the complete inheritance pipeline:
/// 1. Discover and clone repositories (with automatic caching)
/// 2. Process each repository with its operations
/// 3. Determine correct merge order
/// 4. Merge into composite filesystem
/// 5. Merge with local files and apply local operations
/// 6. Write final filesystem to disk (if output_path is provided)
///
/// If `output_path` is `None`, returns the final MemoryFS without writing to disk.
/// If `output_path` is `Some(path)`, writes to disk and returns the MemoryFS.
pub fn execute_pull(
    config: &Schema,
    repo_manager: &RepositoryManager,
    cache: &RepoCache,
    working_dir: &Path,
    output_path: Option<&Path>,
) -> Result<MemoryFS> {
    // Phase 1: Discovery and Cloning
    let repo_tree = phase1::execute(config, repo_manager, cache)?;

    // Phase 2: Processing Individual Repos
    let intermediate_fss = phase2::execute(&repo_tree, repo_manager, cache)?;

    // Phase 3: Determining Operation Order
    let operation_order = phase3::execute(&repo_tree)?;

    // Phase 4: Composite Filesystem Construction
    let composite_fs = phase4::execute(&operation_order, &intermediate_fss)?;

    // Phase 5: Local File Merging
    let final_fs = phase5::execute(&composite_fs, config, working_dir)?;

    // Phase 6: Write to Disk (if output path provided)
    if let Some(output) = output_path {
        phase6::execute(&final_fs, output)?;
    }

    Ok(final_fs)
}
