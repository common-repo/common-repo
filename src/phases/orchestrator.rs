//! Orchestrator for the complete pull operation.
//!
//! Coordinates all phases to provide a clean API for the complete pull
//! operation. Two pipeline modes are supported:
//!
//! - **Batch pipeline** (`execute_source_pipeline`): for top-level source
//!   operations (everything outside `self:`). Phases 1-6 run in batch; Phase 4
//!   calls `phase4::execute` once over the full intermediate map from Phase 2.
//! - **Sequential pipeline** (`execute_sequential_pipeline`): for `self:`
//!   blocks. Operations run in YAML declaration order. Each nested `repo:` is
//!   resolved lazily via `resolve_repo_inline`, and the sub-composite merges
//!   immediately through `phase4::integrate_sub_composite` instead of a single
//!   batch Phase 4 pass over all intermediates.
//!
//! [`execute_pull`] partitions the config into source and `self:` operations,
//! runs the batch pipeline for sources, then runs the sequential pipeline
//! for each `self:` block.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use log::warn;

use super::{phase1, phase2, phase3, phase4, phase5, phase6, ClonedRepo, IntermediateFS};
use crate::cache::RepoCache;
use crate::config::{Operation, Schema, SelfOp};
use crate::error::Result;
use crate::filesystem::MemoryFS;
use crate::repository::RepositoryManager;

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

/// Resolve a single repo inline at its declaration position.
///
/// Given a [`ClonedRepo`] (already cloned in Phase 1), processes the repo's
/// operations sequentially and returns an [`IntermediateFS`] containing the
/// sub-composite filesystem, collected deferred merge operations, and
/// template variables.
///
/// The sub-repo's pipeline runs in isolation — it operates on its own
/// [`crate::filesystem::MemoryFS`], never seeing or modifying a parent
/// composite.
///
/// When the repo's operations contain nested `repo:` references, those are
/// resolved recursively by looking them up in `cloned_repos` and calling
/// this function again. Circular references and diamond dependencies (shared
/// sub-repos) are detected via a visited set; already-resolved repos are
/// skipped to prevent duplicate integration.
///
/// # Arguments
///
/// * `cloned` — The pre-cloned repository to process (raw FS + operations).
/// * `cloned_repos` — All repos cloned by Phase 1. Nested `repo:` references
///   are looked up by `(url, ref)` to handle Phase 1 operation enrichment.
/// * `cache` — In-process cache for deduplicating repeated processing of
///   the same repo+operations combination.
///
/// # Returns
///
/// An [`IntermediateFS`] whose `fs` field is the sub-composite, whose
/// `merge_operations` includes both this repo's deferred merges and any
/// propagated from nested `repo:` resolutions, and whose `template_vars`
/// includes vars from nested repos (last-write-wins).
pub(crate) fn resolve_repo_inline(
    cloned: &ClonedRepo,
    cloned_repos: &HashMap<String, ClonedRepo>,
    cache: &RepoCache,
) -> Result<IntermediateFS> {
    let mut visited = HashSet::new();
    visited.insert(format!("{}@{}", cloned.url, cloned.ref_));
    resolve_repo_inline_inner(cloned, cloned_repos, cache, &mut visited)
}

/// Recursive inner implementation with cycle detection via `visited` set.
fn resolve_repo_inline_inner(
    cloned: &ClonedRepo,
    cloned_repos: &HashMap<String, ClonedRepo>,
    cache: &RepoCache,
    visited: &mut HashSet<String>,
) -> Result<IntermediateFS> {
    let has_repo_ops = cloned
        .operations
        .iter()
        .any(|op| matches!(op, Operation::Repo { .. }));

    // Fast path: no nested repo: ops and no tree children → delegate to
    // process_cloned_repo which uses the in-process cache for deduplication.
    if !has_repo_ops && cloned.children_keys.is_empty() {
        return phase2::process_cloned_repo(cloned, cache);
    }

    // Slow path: operations contain repo: references or this node has tree
    // children (repos discovered from the upstream .common-repo.yaml).
    // Process sequentially so repo: ops fire at their declaration position.
    let mut template_vars = phase2::collect_template_vars(&cloned.operations)?;
    let mut merge_operations = phase2::collect_merge_operations(&cloned.operations);

    let mut fs = cloned.fs.clone();

    for operation in &cloned.operations {
        match operation {
            Operation::Repo { repo } => {
                let visit_key = format!("{}@{}", repo.url, repo.r#ref);

                if !visited.insert(visit_key) {
                    log::debug!(
                        "Skipping already-resolved repo: {}@{} (cycle or shared dependency)",
                        repo.url,
                        repo.r#ref
                    );
                    continue;
                }

                // Phase 1 enriches each repo node with upstream filtering +
                // deferred ops, so the cloned_repos key may differ from the
                // raw (url, ref, with) on the Operation::Repo. Look up by
                // (url, ref) to match regardless of enrichment.
                let candidates: Vec<_> = cloned_repos
                    .values()
                    .filter(|c| c.url == repo.url && c.ref_ == repo.r#ref)
                    .collect();

                if candidates.len() > 1 {
                    warn!(
                        "Multiple cloned repos match nested {}@{} ({} candidates); using first",
                        repo.url,
                        repo.r#ref,
                        candidates.len()
                    );
                }

                if let Some(nested_cloned) = candidates.into_iter().next() {
                    let nested_result =
                        resolve_repo_inline_inner(nested_cloned, cloned_repos, cache, visited)?;

                    fs.merge(&nested_result.fs);

                    merge_operations.extend(nested_result.merge_operations);

                    template_vars.extend(nested_result.template_vars);
                } else {
                    warn!(
                        "Nested repo: reference not found in cloned repos, skipping: {}@{}",
                        repo.url, repo.r#ref
                    );
                }
            }
            other => {
                phase2::apply_operation(&mut fs, other)?;
            }
        }
    }

    // Integrate tree children: repos discovered from the upstream
    // .common-repo.yaml that were extracted as tree children during Phase 1
    // discovery rather than kept in the operations list. Exact-key lookup
    // is safe here because children_keys are captured from node_key() at
    // the same time as the HashMap insertion in clone_tree_repos_recursive.
    for child_key in &cloned.children_keys {
        if let Some(child_cloned) = cloned_repos.get(child_key) {
            let visit_key = format!("{}@{}", child_cloned.url, child_cloned.ref_);
            if !visited.insert(visit_key) {
                log::debug!(
                    "Skipping already-resolved tree child: {} (cycle or shared dependency)",
                    child_key
                );
                continue;
            }

            let child_result =
                resolve_repo_inline_inner(child_cloned, cloned_repos, cache, visited)?;

            fs.merge(&child_result.fs);
            merge_operations.extend(child_result.merge_operations);
            template_vars.extend(child_result.template_vars);
        } else {
            warn!(
                "Tree child not found in cloned repos, skipping: {}",
                child_key
            );
        }
    }

    Ok(IntermediateFS::new_with_vars_and_merges(
        fs,
        cloned.url.clone(),
        cloned.ref_.clone(),
        template_vars,
        merge_operations,
    ))
}

/// Execute a sequential pipeline where operations fire in declaration order.
///
/// Unlike [`execute_source_pipeline`] where Phases 2-4 process all repos in
/// batch, this function walks the config's operations sequentially. When a
/// `repo:` operation is encountered, it resolves inline at that position
/// using [`resolve_repo_inline`] and [`phase4::integrate_sub_composite`].
///
/// This pipeline is used for `self:` blocks where the operation order
/// matters: a preceding `include` or `rename` must transform the FS before
/// a subsequent `repo:` integrates its sub-composite.
fn execute_sequential_pipeline(
    config: &Schema,
    repo_manager: &RepositoryManager,
    cache: &RepoCache,
    working_dir: &Path,
    output_path: Option<&Path>,
) -> Result<MemoryFS> {
    // Phase 1: Discover and clone repos eagerly
    let repo_tree = phase1::execute(config, repo_manager, cache)?;

    // Build cloned_repos map for on-demand resolution
    let cloned_repos = phase2::clone_tree_repos(&repo_tree, repo_manager)?;

    // Load local files from the working directory
    let mut fs = phase5::load_local_fs(working_dir)?;

    let mut all_template_vars = HashMap::new();
    let mut residual_deferred_ops: Vec<Operation> = Vec::new();

    // Sequential pass: walk operations in declaration order
    for operation in config {
        match operation {
            Operation::Include { include } => {
                let mut filtered_fs = MemoryFS::new();
                crate::operators::include::apply(include, &fs, &mut filtered_fs)?;
                fs = filtered_fs;
            }
            Operation::Exclude { exclude } => {
                crate::operators::exclude::apply(exclude, &mut fs)?;
            }
            Operation::Rename { rename } => {
                crate::operators::rename::apply(rename, &mut fs)?;
            }
            Operation::Repo { repo } => {
                // Phase 1 enriches each repo node with upstream filtering +
                // deferred ops, so the cloned_repos key differs from the raw
                // (url, ref, with) on the Operation::Repo. Look up by
                // (url, ref) — if multiple candidates match (same URL+ref
                // with different operations), we take the first and warn.
                let candidates: Vec<_> = cloned_repos
                    .values()
                    .filter(|c| c.url == repo.url && c.ref_ == repo.r#ref)
                    .collect();

                if candidates.len() > 1 {
                    warn!(
                        "Multiple cloned repos match {}@{} ({} candidates); using first match",
                        repo.url,
                        repo.r#ref,
                        candidates.len()
                    );
                }

                let cloned = candidates.into_iter().next();

                if let Some(cloned) = cloned {
                    let sub_composite = resolve_repo_inline(cloned, &cloned_repos, cache)?;

                    let residual = phase4::integrate_sub_composite(&mut fs, &sub_composite)?;
                    residual_deferred_ops.extend(residual);

                    all_template_vars.extend(sub_composite.template_vars);
                } else {
                    warn!(
                        "Repo reference not found in cloned repos, skipping: {}@{}",
                        repo.url, repo.r#ref
                    );
                }
            }
            Operation::Template { template } => {
                crate::operators::template::mark(template, &mut fs)?;
            }
            Operation::TemplateVars { template_vars } => {
                crate::operators::template_vars::collect(template_vars, &mut all_template_vars)?;
            }
            Operation::Yaml { .. }
            | Operation::Json { .. }
            | Operation::Toml { .. }
            | Operation::Ini { .. }
            | Operation::Markdown { .. }
            | Operation::Xml { .. } => {
                phase4::execute_merge_operation(&mut fs, operation)?;
            }
            Operation::Tools { tools } => {
                crate::operators::tools::apply(tools)?;
            }
            Operation::Self_ { .. } => {}
        }
    }

    // Execute residual deferred merges (dest wasn't in the FS during integration)
    for op in &residual_deferred_ops {
        phase4::execute_merge_operation(&mut fs, op)?;
    }

    // Process templates with all collected variables
    crate::operators::template::process(&mut fs, &all_template_vars)?;

    // Phase 6: Write to disk (if output path provided)
    if let Some(output) = output_path {
        phase6::execute(&fs, output)?;
    }

    Ok(fs)
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

    // Run self: pipelines using sequential execution so repo: ops
    // resolve inline at their declaration position.
    for self_op in &self_ops {
        execute_sequential_pipeline(
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
    use crate::phases::ClonedRepo;
    use std::collections::HashMap;

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

    #[test]
    fn test_resolve_repo_inline_recursive_repo_ops() {
        use crate::cache::RepoCache;
        use crate::config::RepoOp;
        use crate::filesystem::MemoryFS;

        let cache = RepoCache::new();

        // Child repo: has a single file
        let mut child_fs = MemoryFS::new();
        child_fs.add_file_string("child.txt", "from child").unwrap();
        let child_cloned = ClonedRepo::new(
            child_fs,
            "https://github.com/test/child.git".to_string(),
            "main".to_string(),
            vec![], // no operations on child
        );

        // Parent repo: has its own file + a repo: op referencing child
        let mut parent_fs = MemoryFS::new();
        parent_fs
            .add_file_string("parent.txt", "from parent")
            .unwrap();
        let parent_cloned = ClonedRepo::new(
            parent_fs,
            "https://github.com/test/parent.git".to_string(),
            "main".to_string(),
            vec![Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/test/child.git".to_string(),
                    r#ref: "main".to_string(),
                    path: None,
                    with: vec![], // matches child's operations
                },
            }],
        );

        // Put child in the cloned_repos map so recursive lookup works
        let mut cloned_repos = HashMap::new();
        cloned_repos.insert(child_cloned.node_key(), child_cloned);

        let result = resolve_repo_inline(&parent_cloned, &cloned_repos, &cache).unwrap();

        // Parent's own file is present
        assert!(result.fs.exists("parent.txt"));
        // Child's file was integrated via recursive resolution
        assert!(result.fs.exists("child.txt"));
        assert_eq!(
            String::from_utf8(result.fs.get_file("child.txt").unwrap().content.clone()).unwrap(),
            "from child"
        );
    }

    #[test]
    fn test_resolve_repo_inline_nested_two_levels() {
        use crate::cache::RepoCache;
        use crate::config::RepoOp;
        use crate::filesystem::MemoryFS;

        let cache = RepoCache::new();

        // Grandchild: single file
        let mut grandchild_fs = MemoryFS::new();
        grandchild_fs
            .add_file_string("deep.txt", "from grandchild")
            .unwrap();
        let grandchild = ClonedRepo::new(
            grandchild_fs,
            "https://github.com/test/grandchild.git".to_string(),
            "v1".to_string(),
            vec![],
        );

        // Child: has its own file + repo: op referencing grandchild
        let mut child_fs = MemoryFS::new();
        child_fs.add_file_string("mid.txt", "from child").unwrap();
        let child = ClonedRepo::new(
            child_fs,
            "https://github.com/test/child.git".to_string(),
            "main".to_string(),
            vec![Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/test/grandchild.git".to_string(),
                    r#ref: "v1".to_string(),
                    path: None,
                    with: vec![],
                },
            }],
        );

        // Parent: has its own file + repo: op referencing child
        let mut parent_fs = MemoryFS::new();
        parent_fs.add_file_string("top.txt", "from parent").unwrap();
        let parent = ClonedRepo::new(
            parent_fs,
            "https://github.com/test/parent.git".to_string(),
            "main".to_string(),
            vec![Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/test/child.git".to_string(),
                    r#ref: "main".to_string(),
                    path: None,
                    with: vec![Operation::Repo {
                        repo: RepoOp {
                            url: "https://github.com/test/grandchild.git".to_string(),
                            r#ref: "v1".to_string(),
                            path: None,
                            with: vec![],
                        },
                    }],
                },
            }],
        );

        let mut cloned_repos = HashMap::new();
        cloned_repos.insert(grandchild.node_key(), grandchild);
        cloned_repos.insert(child.node_key(), child);

        let result = resolve_repo_inline(&parent, &cloned_repos, &cache).unwrap();

        // All three levels' files should be present
        assert!(result.fs.exists("top.txt"));
        assert!(result.fs.exists("mid.txt"));
        assert!(result.fs.exists("deep.txt"));
    }

    #[test]
    fn test_resolve_repo_inline_isolation_from_parent() {
        use crate::cache::RepoCache;
        use crate::filesystem::MemoryFS;

        let cache = RepoCache::new();

        // Sub-repo has only its own files
        let mut fs = MemoryFS::new();
        fs.add_file_string("sub.txt", "sub content").unwrap();

        let cloned = ClonedRepo::new(
            fs,
            "https://github.com/test/sub.git".to_string(),
            "main".to_string(),
            vec![],
        );

        let cloned_repos = HashMap::new();

        let result = resolve_repo_inline(&cloned, &cloned_repos, &cache).unwrap();

        // Sub-repo only contains its own file
        assert!(result.fs.exists("sub.txt"));
        // No parent files leak into the sub-repo's result
        let file_count = result.fs.files().count();
        assert_eq!(file_count, 1);
    }

    #[test]
    fn test_resolve_repo_inline_collects_merge_operations() {
        use crate::cache::RepoCache;
        use crate::config::YamlMergeOp;
        use crate::filesystem::MemoryFS;

        let cache = RepoCache::new();

        let mut fs = MemoryFS::new();
        fs.add_file_string("base.yaml", "key: value").unwrap();
        fs.add_file_string("fragment.yaml", "extra: data").unwrap();

        let cloned = ClonedRepo::new(
            fs,
            "https://github.com/test/repo.git".to_string(),
            "main".to_string(),
            vec![Operation::Yaml {
                yaml: YamlMergeOp {
                    source: Some("fragment.yaml".to_string()),
                    dest: Some("base.yaml".to_string()),
                    auto_merge: Some(".pre-commit-config.yaml".to_string()),
                    ..Default::default()
                },
            }],
        );

        let cloned_repos = HashMap::new();

        let result = resolve_repo_inline(&cloned, &cloned_repos, &cache).unwrap();

        // Deferred merge operation should be collected
        assert_eq!(result.merge_operations.len(), 1);
        assert!(matches!(
            &result.merge_operations[0],
            Operation::Yaml { .. }
        ));
    }

    #[test]
    fn test_resolve_repo_inline_collects_template_vars() {
        use crate::cache::RepoCache;
        use crate::config::TemplateVars;
        use crate::filesystem::MemoryFS;

        let cache = RepoCache::new();

        let mut fs = MemoryFS::new();
        fs.add_file_string("file.txt", "hello __COMMON_REPO__NAME__")
            .unwrap();

        let mut vars = HashMap::new();
        vars.insert("NAME".to_string(), "world".to_string());

        let cloned = ClonedRepo::new(
            fs,
            "https://github.com/test/repo.git".to_string(),
            "main".to_string(),
            vec![Operation::TemplateVars {
                template_vars: TemplateVars { vars },
            }],
        );

        let cloned_repos = HashMap::new();

        let result = resolve_repo_inline(&cloned, &cloned_repos, &cache).unwrap();

        assert_eq!(result.template_vars.get("NAME").unwrap(), "world");
    }

    #[test]
    fn test_resolve_repo_inline_applies_include_filter() {
        use crate::cache::RepoCache;
        use crate::filesystem::MemoryFS;

        let cache = RepoCache::new();

        // Build a ClonedRepo with 3 files, only src/** should survive
        let mut fs = MemoryFS::new();
        fs.add_file_string("README.md", "top-level readme").unwrap();
        fs.add_file_string("src/lib.rs", "fn main() {}").unwrap();
        fs.add_file_string("src/util.rs", "pub fn helper() {}")
            .unwrap();

        let cloned = ClonedRepo::new(
            fs,
            "https://github.com/test/repo.git".to_string(),
            "main".to_string(),
            vec![Operation::Include {
                include: IncludeOp {
                    patterns: vec!["src/**".to_string()],
                },
            }],
        );

        let cloned_repos = HashMap::new();

        let result = resolve_repo_inline(&cloned, &cloned_repos, &cache).unwrap();

        // Only src/ files survive the include filter
        assert!(result.fs.exists("src/lib.rs"));
        assert!(result.fs.exists("src/util.rs"));
        assert!(!result.fs.exists("README.md"));

        // Metadata preserved
        assert_eq!(result.upstream_url, "https://github.com/test/repo.git");
        assert_eq!(result.upstream_ref, "main");
    }

    #[test]
    fn test_resolve_repo_inline_missing_nested_repo_skips() {
        use crate::cache::RepoCache;
        use crate::config::RepoOp;
        use crate::filesystem::MemoryFS;

        let cache = RepoCache::new();

        let mut fs = MemoryFS::new();
        fs.add_file_string("parent.txt", "from parent").unwrap();

        // Parent references a repo that does NOT exist in cloned_repos
        let cloned = ClonedRepo::new(
            fs,
            "https://github.com/test/parent.git".to_string(),
            "main".to_string(),
            vec![Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/test/nonexistent.git".to_string(),
                    r#ref: "main".to_string(),
                    path: None,
                    with: vec![],
                },
            }],
        );

        let cloned_repos = HashMap::new();

        // Should succeed without error, just skip the missing repo
        let result = resolve_repo_inline(&cloned, &cloned_repos, &cache).unwrap();
        assert!(result.fs.exists("parent.txt"));
        assert_eq!(result.fs.files().count(), 1);
    }

    #[test]
    fn test_resolve_repo_inline_interleaved_filter_and_repo_ops() {
        use crate::cache::RepoCache;
        use crate::config::{ExcludeOp, RepoOp};
        use crate::filesystem::MemoryFS;

        let cache = RepoCache::new();

        // Child repo provides two files
        let mut child_fs = MemoryFS::new();
        child_fs.add_file_string("keep.txt", "keep me").unwrap();
        child_fs.add_file_string("remove.txt", "remove me").unwrap();
        let child = ClonedRepo::new(
            child_fs,
            "https://github.com/test/child.git".to_string(),
            "main".to_string(),
            vec![],
        );

        // Parent: include src/** → repo: child → exclude remove.txt
        // The exclude fires AFTER the repo: integrates child's files
        let mut parent_fs = MemoryFS::new();
        parent_fs
            .add_file_string("src/code.rs", "fn main() {}")
            .unwrap();
        parent_fs
            .add_file_string("other.txt", "should be filtered")
            .unwrap();
        let parent = ClonedRepo::new(
            parent_fs,
            "https://github.com/test/parent.git".to_string(),
            "main".to_string(),
            vec![
                // First: include limits parent to src/**
                Operation::Include {
                    include: IncludeOp {
                        patterns: vec!["src/**".to_string()],
                    },
                },
                // Second: repo: integrates child files into the FS
                Operation::Repo {
                    repo: RepoOp {
                        url: "https://github.com/test/child.git".to_string(),
                        r#ref: "main".to_string(),
                        path: None,
                        with: vec![],
                    },
                },
                // Third: exclude removes remove.txt (which came from child)
                Operation::Exclude {
                    exclude: ExcludeOp {
                        patterns: vec!["remove.txt".to_string()],
                    },
                },
            ],
        );

        let mut cloned_repos = HashMap::new();
        cloned_repos.insert(child.node_key(), child);

        let result = resolve_repo_inline(&parent, &cloned_repos, &cache).unwrap();

        // src/code.rs: survived include
        assert!(result.fs.exists("src/code.rs"));
        // keep.txt: came from child, survived exclude
        assert!(result.fs.exists("keep.txt"));
        // other.txt: filtered out by include
        assert!(!result.fs.exists("other.txt"));
        // remove.txt: came from child but removed by exclude
        assert!(!result.fs.exists("remove.txt"));
    }

    #[test]
    fn test_resolve_repo_inline_circular_reference_detected() {
        use crate::cache::RepoCache;
        use crate::config::RepoOp;
        use crate::filesystem::MemoryFS;

        let cache = RepoCache::new();

        // Repo A references repo B, and repo B references repo A
        let mut fs_a = MemoryFS::new();
        fs_a.add_file_string("a.txt", "from A").unwrap();
        let repo_a = ClonedRepo::new(
            fs_a,
            "https://github.com/test/a.git".to_string(),
            "main".to_string(),
            vec![Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/test/b.git".to_string(),
                    r#ref: "main".to_string(),
                    path: None,
                    with: vec![Operation::Repo {
                        repo: RepoOp {
                            url: "https://github.com/test/a.git".to_string(),
                            r#ref: "main".to_string(),
                            path: None,
                            with: vec![],
                        },
                    }],
                },
            }],
        );

        let mut fs_b = MemoryFS::new();
        fs_b.add_file_string("b.txt", "from B").unwrap();
        let repo_b = ClonedRepo::new(
            fs_b,
            "https://github.com/test/b.git".to_string(),
            "main".to_string(),
            // B's operations reference A (cycle)
            vec![Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/test/a.git".to_string(),
                    r#ref: "main".to_string(),
                    path: None,
                    with: vec![],
                },
            }],
        );

        let mut cloned_repos = HashMap::new();
        cloned_repos.insert(repo_b.node_key(), repo_b);

        // Should not stack overflow — cycle is detected and skipped
        let result = resolve_repo_inline(&repo_a, &cloned_repos, &cache).unwrap();
        assert!(result.fs.exists("a.txt"));
        assert!(result.fs.exists("b.txt"));
    }

    #[test]
    fn test_resolve_repo_inline_propagates_nested_merge_ops() {
        use crate::cache::RepoCache;
        use crate::config::{RepoOp, YamlMergeOp};
        use crate::filesystem::MemoryFS;

        let cache = RepoCache::new();

        // The merge op that will be applied to the child repo.
        // In the real pipeline, this comes from inheritance discovery or
        // the parent's with: clause — the ClonedRepo's operations and the
        // parent's with: list are identical so the lookup key matches.
        let merge_op = Operation::Yaml {
            yaml: YamlMergeOp {
                source: Some("fragment.yaml".to_string()),
                dest: Some("config.yaml".to_string()),
                auto_merge: Some("config.yaml".to_string()),
                ..Default::default()
            },
        };

        // Child stored in cloned_repos with the merge op as its operation
        let mut child_fs = MemoryFS::new();
        child_fs
            .add_file_string("fragment.yaml", "extra: data")
            .unwrap();
        let child = ClonedRepo::new(
            child_fs,
            "https://github.com/test/child.git".to_string(),
            "main".to_string(),
            vec![merge_op.clone()],
        );

        // Parent references child with the same with: ops so keys match
        let mut parent_fs = MemoryFS::new();
        parent_fs
            .add_file_string("parent.txt", "from parent")
            .unwrap();
        let parent = ClonedRepo::new(
            parent_fs,
            "https://github.com/test/parent.git".to_string(),
            "main".to_string(),
            vec![Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/test/child.git".to_string(),
                    r#ref: "main".to_string(),
                    path: None,
                    with: vec![merge_op],
                },
            }],
        );

        let mut cloned_repos = HashMap::new();
        cloned_repos.insert(child.node_key(), child);

        let result = resolve_repo_inline(&parent, &cloned_repos, &cache).unwrap();

        // Child's deferred merge op should propagate to parent's result
        assert_eq!(result.merge_operations.len(), 1);
        assert!(matches!(
            &result.merge_operations[0],
            Operation::Yaml { yaml } if yaml.source == Some("fragment.yaml".to_string())
        ));
    }

    #[test]
    fn test_resolve_repo_inline_propagates_nested_template_vars() {
        use crate::cache::RepoCache;
        use crate::config::{RepoOp, TemplateVars};
        use crate::filesystem::MemoryFS;

        let cache = RepoCache::new();

        // The child's template-vars operation (same in with: and in ClonedRepo
        // so the lookup key matches)
        let mut child_vars = HashMap::new();
        child_vars.insert("CHILD_VAR".to_string(), "child_value".to_string());
        let child_tv_op = Operation::TemplateVars {
            template_vars: TemplateVars {
                vars: child_vars.clone(),
            },
        };

        let mut child_fs = MemoryFS::new();
        child_fs.add_file_string("child.txt", "data").unwrap();
        let child = ClonedRepo::new(
            child_fs,
            "https://github.com/test/child.git".to_string(),
            "main".to_string(),
            vec![child_tv_op.clone()],
        );

        // Parent defines its own template vars + references child with
        // matching with: ops so the key lookup succeeds
        let mut parent_vars = HashMap::new();
        parent_vars.insert("PARENT_VAR".to_string(), "parent_value".to_string());
        let parent = ClonedRepo::new(
            MemoryFS::new(),
            "https://github.com/test/parent.git".to_string(),
            "main".to_string(),
            vec![
                Operation::TemplateVars {
                    template_vars: TemplateVars { vars: parent_vars },
                },
                Operation::Repo {
                    repo: RepoOp {
                        url: "https://github.com/test/child.git".to_string(),
                        r#ref: "main".to_string(),
                        path: None,
                        with: vec![child_tv_op],
                    },
                },
            ],
        );

        let mut cloned_repos = HashMap::new();
        cloned_repos.insert(child.node_key(), child);

        let result = resolve_repo_inline(&parent, &cloned_repos, &cache).unwrap();

        // Both parent and child template vars present
        assert_eq!(
            result.template_vars.get("PARENT_VAR").unwrap(),
            "parent_value"
        );
        assert_eq!(
            result.template_vars.get("CHILD_VAR").unwrap(),
            "child_value"
        );
    }
}
