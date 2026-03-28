//! Orchestrator for the complete pull operation
//!
//! This module coordinates all phases to provide a clean API for the complete
//! pull operation. Currently implements Phases 1-6 for end-to-end inheritance.

use super::{phase1, phase2, phase3, phase4, phase5, phase6};
use crate::cache::RepoCache;
use crate::config::{Operation, Schema, SelfOp};
use crate::error::Result;
use crate::filesystem::MemoryFS;
use crate::repository::RepositoryManager;
use std::path::Path;

/// Partition a config into self operations and source operations.
///
/// Self operations run in an isolated pipeline. Source operations
/// run in the main pipeline that produces the composite filesystem.
pub fn partition_self_operations(config: &Schema) -> (Vec<SelfOp>, Schema) {
    let mut self_ops = Vec::new();
    let mut source_ops = Vec::new();

    for op in config {
        match op {
            Operation::Self_ { self_ } => self_ops.push(self_.clone()),
            other => source_ops.push(other.clone()),
        }
    }

    (self_ops, source_ops)
}

/// Execute the source pipeline (Phases 1-6) for a given config.
///
/// This is the core pipeline logic, extracted so it can be called
/// once for the main source config and again for each self: block.
fn execute_source_pipeline(
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
    let (composite_fs, deferred_ops) = phase4::execute(&operation_order, &intermediate_fss)?;

    // Phase 5: Local File Merging (receives deferred ops)
    let final_fs = phase5::execute(&composite_fs, config, working_dir, &deferred_ops)?;

    // Phase 6: Write to Disk (if output path provided)
    if let Some(output) = output_path {
        phase6::execute(&final_fs, output)?;
    }

    Ok(final_fs)
}

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
/// If the config contains `self:` blocks, they are partitioned out and
/// run as independent pipeline invocations after the source pipeline
/// completes. Self pipeline output is written to the working directory
/// but never enters the composite filesystem that downstream consumers
/// see.
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
    // Partition self: operations from source operations
    let (self_ops, source_config) = partition_self_operations(config);

    // Run the source pipeline (main composite — what consumers see)
    let final_fs = execute_source_pipeline(
        &source_config,
        repo_manager,
        cache,
        working_dir,
        output_path,
    )?;

    // Run self: pipelines (isolated, local-only)
    for self_op in &self_ops {
        execute_source_pipeline(
            &self_op.operations,
            repo_manager,
            cache,
            working_dir,
            output_path,
        )?;
    }

    Ok(final_fs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{IncludeOp, Operation, SelfOp};

    #[test]
    fn test_partition_self_operations() {
        let config = vec![
            Operation::Include {
                include: IncludeOp {
                    patterns: vec!["src/**".to_string()],
                },
            },
            Operation::Self_ {
                self_: SelfOp {
                    operations: vec![Operation::Include {
                        include: IncludeOp {
                            patterns: vec!["**/*".to_string()],
                        },
                    }],
                },
            },
        ];

        let (self_ops, source_ops) = partition_self_operations(&config);
        assert_eq!(self_ops.len(), 1);
        assert_eq!(source_ops.len(), 1);
        assert!(matches!(source_ops[0], Operation::Include { .. }));
    }

    #[test]
    fn test_partition_no_self_operations() {
        let config = vec![Operation::Include {
            include: IncludeOp {
                patterns: vec!["**/*".to_string()],
            },
        }];

        let (self_ops, source_ops) = partition_self_operations(&config);
        assert_eq!(self_ops.len(), 0);
        assert_eq!(source_ops.len(), 1);
    }

    #[test]
    fn test_partition_empty_config() {
        let config: Schema = vec![];
        let (self_ops, source_ops) = partition_self_operations(&config);
        assert_eq!(self_ops.len(), 0);
        assert_eq!(source_ops.len(), 0);
    }

    #[test]
    fn test_composite_precedence_with_deferred_merge_full_flow() {
        use crate::config::{JsonMergeOp, Operation};
        use crate::filesystem::MemoryFS;
        use crate::phases::{IntermediateFS, OperationOrder};
        use std::collections::HashMap;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let working_dir = temp_dir.path();

        // Consumer has these local files:
        // - package.json (full file, will be merge destination)
        // - local_only.txt (not in any upstream)
        // - config.yaml (also provided by upstream — upstream should win)
        std::fs::write(
            working_dir.join("package.json"),
            r#"{"name": "my-app", "version": "1.0.0"}"#,
        )
        .unwrap();
        std::fs::write(working_dir.join("local_only.txt"), b"my local file").unwrap();
        std::fs::write(working_dir.join("config.yaml"), b"old: value").unwrap();

        // Upstream provides:
        // - config.yaml (whole file, should win over local)
        // - fragment.json (merge fragment for package.json)
        // - deferred merge: fragment.json -> package.json
        let mut fs1 = MemoryFS::new();
        fs1.add_file_string("config.yaml", "new: value").unwrap();
        fs1.add_file_string("fragment.json", r#"{"scripts": {"test": "jest"}}"#)
            .unwrap();

        let merge_op = Operation::Json {
            json: JsonMergeOp {
                source: Some("fragment.json".to_string()),
                dest: Some("package.json".to_string()),
                ..Default::default()
            },
        };

        let mut intermediate_fss = HashMap::new();
        intermediate_fss.insert(
            "https://github.com/upstream.git@main".to_string(),
            IntermediateFS::new_with_vars_and_merges(
                fs1,
                "https://github.com/upstream.git".to_string(),
                "main".to_string(),
                HashMap::new(),
                vec![merge_op],
            ),
        );

        let order = OperationOrder::new(vec!["https://github.com/upstream.git@main".to_string()]);

        // Phase 4: build composite, collect deferred ops
        let (composite_fs, deferred_ops) =
            crate::phases::phase4::execute(&order, &intermediate_fss).unwrap();

        assert_eq!(deferred_ops.len(), 1);

        // Phase 5: combine with local, execute deferred merges
        let local_config = vec![];
        let final_fs = crate::phases::phase5::execute(
            &composite_fs,
            &local_config,
            working_dir,
            &deferred_ops,
        )
        .unwrap();

        // config.yaml: composite wins (upstream updated version)
        let config = final_fs.get_file("config.yaml").unwrap();
        assert_eq!(
            String::from_utf8(config.content.clone()).unwrap(),
            "new: value"
        );

        // local_only.txt: preserved (not in composite)
        assert!(final_fs.exists("local_only.txt"));

        // package.json: local base with fragment merged in
        let pkg = final_fs.get_file("package.json").unwrap();
        let json: serde_json::Value =
            serde_json::from_str(&String::from_utf8(pkg.content.clone()).unwrap()).unwrap();
        assert_eq!(json["name"], "my-app");
        assert_eq!(json["version"], "1.0.0");
        assert_eq!(json["scripts"]["test"], "jest");
    }

    #[test]
    fn test_partition_multiple_self_operations() {
        let config = vec![
            Operation::Self_ {
                self_: SelfOp {
                    operations: vec![Operation::Include {
                        include: IncludeOp {
                            patterns: vec!["a/**".to_string()],
                        },
                    }],
                },
            },
            Operation::Include {
                include: IncludeOp {
                    patterns: vec!["src/**".to_string()],
                },
            },
            Operation::Self_ {
                self_: SelfOp {
                    operations: vec![Operation::Include {
                        include: IncludeOp {
                            patterns: vec!["b/**".to_string()],
                        },
                    }],
                },
            },
        ];

        let (self_ops, source_ops) = partition_self_operations(&config);
        assert_eq!(self_ops.len(), 2);
        assert_eq!(source_ops.len(), 1);
    }
}
