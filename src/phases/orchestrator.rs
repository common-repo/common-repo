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

/// Execute phases 1-4 and apply consumer-level filtering operations
///
/// Returns the composite filesystem after applying consumer-declared
/// exclude, include, and rename operations — but without merging local
/// working directory files. This represents the set of files that
/// common-repo manages through inheritance.
///
/// Used by `ls` to show only managed files, avoiding the local file
/// merge in Phase 5 that would include unmanaged local files.
pub fn execute_pull_composite(
    config: &Schema,
    repo_manager: &RepositoryManager,
    cache: &RepoCache,
) -> Result<MemoryFS> {
    use crate::config::Operation;
    use crate::operators;

    // Phase 1: Discovery and Cloning
    let repo_tree = phase1::execute(config, repo_manager, cache)?;

    // Phase 2: Processing Individual Repos
    let intermediate_fss = phase2::execute(&repo_tree, repo_manager, cache)?;

    // Phase 3: Determining Operation Order
    let operation_order = phase3::execute(&repo_tree)?;

    // Phase 4: Composite Filesystem Construction
    let mut composite_fs = phase4::execute(&operation_order, &intermediate_fss)?;

    // Apply consumer-level filtering operations (exclude/include/rename)
    // These are top-level operations in the consumer's config that should
    // filter the inherited file set.
    for operation in config {
        match operation {
            Operation::Exclude { exclude } => {
                operators::exclude::apply(exclude, &mut composite_fs)?;
            }
            Operation::Include { include } => {
                let mut filtered_fs = MemoryFS::new();
                operators::include::apply(include, &composite_fs, &mut filtered_fs)?;
                composite_fs = filtered_fs;
            }
            Operation::Rename { rename } => {
                operators::rename::apply(rename, &mut composite_fs)?;
            }
            _ => {}
        }
    }

    Ok(composite_fs)
}
