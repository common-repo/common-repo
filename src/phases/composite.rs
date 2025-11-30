//! Phase 4: Composite Filesystem Construction
//!
//! This is the fourth phase of the `common-repo` execution pipeline. Its
//! responsibility is to construct a single, composite filesystem from all the
//! intermediate filesystems produced by Phase 2, using the operation order
//! calculated in Phase 3.
//!
//! ## Process
//!
//! 1.  **Variable Consolidation**: The template variables from all
//!     `IntermediateFS` instances are collected into a single, unified set.
//!     If the same variable is defined in multiple repositories, the value
//!     from the repository that appears later in the `OperationOrder` takes
//!     precedence (i.e., a "last-write-wins" strategy). This is consistent
//!     with how file merging works.
//!
//! 2.  **Template Processing**: Once the variables are consolidated, each
//!     `IntermediateFS`'s underlying `MemoryFS` is processed for templates
//!     using the consolidated set of variables. This step substitutes all the
//!     `${VAR}` placeholders with their final values.
//!
//! 3.  **Filesystem Merging**: After template processing, the `MemoryFS` from
//!     each `IntermediateFS` is merged into the composite filesystem. The merge
//!     is performed in the `OperationOrder`, which again ensures a "last-write-wins"
//!     behavior, where files from more specific repositories overwrite those
//!     from their ancestors.
//!
//! This phase produces a single `MemoryFS` that represents the complete,
//! inherited configuration, with all templates processed and all files merged,
//! ready for the final local merge in the next phase.

use std::collections::HashMap;

use super::{IntermediateFS, OperationOrder};
use crate::config::Operation;
use crate::error::{Error, Result};
use crate::filesystem::MemoryFS;

/// Executes Phase 4 of the pipeline.
///
/// This function orchestrates the construction of the composite filesystem
/// by first processing all templates with a unified set of variables and
/// then merging the resulting filesystems in the correct order.
pub fn execute(
    order: &OperationOrder,
    intermediate_fss: &HashMap<String, IntermediateFS>,
) -> Result<MemoryFS> {
    // First, collect all template variables from all intermediate filesystems in operation order
    let mut all_template_vars = HashMap::new();
    for repo_key in &order.order {
        if let Some(intermediate_fs) = intermediate_fss.get(repo_key) {
            for (key, value) in &intermediate_fs.template_vars {
                // Later repositories override earlier ones (consistent with other operations)
                all_template_vars.insert(key.clone(), value.clone());
            }
        }
    }

    // Process templates in each intermediate filesystem
    let mut processed_fss = HashMap::new();
    for (repo_key, intermediate_fs) in intermediate_fss {
        let mut processed_fs = intermediate_fs.fs.clone();
        crate::operators::template::process(&mut processed_fs, &all_template_vars)?;
        processed_fss.insert(repo_key.clone(), processed_fs);
    }

    // Merge processed filesystems in the operation order
    // Later filesystems in the order take precedence (last-write-wins)
    let mut composite_fs = MemoryFS::new();
    for repo_key in &order.order {
        if let Some(processed_fs) = processed_fss.get(repo_key) {
            merge_filesystem(&mut composite_fs, processed_fs)?;

            // Execute merge operations for this repository after its filesystem is merged
            if let Some(intermediate_fs) = intermediate_fss.get(repo_key) {
                for merge_op in &intermediate_fs.merge_operations {
                    execute_merge_operation(&mut composite_fs, merge_op)?;
                }
            }
        } else {
            // This shouldn't happen if Phase 2 and Phase 3 are implemented correctly
            return Err(Error::Filesystem {
                message: format!(
                    "Missing intermediate filesystem for repository: {}",
                    repo_key
                ),
            });
        }
    }

    Ok(composite_fs)
}

/// Merge a source filesystem into a target filesystem
///
/// All files from source_fs are copied to target_fs. If a file already exists
/// in target_fs, it is overwritten (last-write-wins strategy).
/// This preserves file metadata from the source filesystem.
fn merge_filesystem(target_fs: &mut MemoryFS, source_fs: &MemoryFS) -> Result<()> {
    for (path, file) in source_fs.files() {
        target_fs.add_file(path, file.clone())?;
    }
    Ok(())
}

/// Execute a single merge operation on the composite filesystem
///
/// This function dispatches to the appropriate merge operation handler
/// based on the operation type (YAML, JSON, TOML, INI, or Markdown).
fn execute_merge_operation(fs: &mut MemoryFS, operation: &Operation) -> Result<()> {
    match operation {
        Operation::Yaml { yaml } => super::phase5::apply_yaml_merge_operation(fs, yaml),
        Operation::Json { json } => super::phase5::apply_json_merge_operation(fs, json),
        Operation::Toml { toml } => super::phase5::apply_toml_merge_operation(fs, toml),
        Operation::Ini { ini } => super::phase5::apply_ini_merge_operation(fs, ini),
        Operation::Markdown { markdown } => {
            super::phase5::apply_markdown_merge_operation(fs, markdown)
        }
        _ => {
            // Non-merge operations should not be passed to this function
            Err(Error::Filesystem {
                message: format!("Unexpected non-merge operation: {:?}", operation),
            })
        }
    }
}
