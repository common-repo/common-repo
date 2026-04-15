//! Orchestrator for the complete pull operation.
//!
//! Coordinates all phases to provide a clean API for the complete pull
//! operation. Both source and `self:` blocks use the same sequential
//! pipeline (`execute_sequential_pipeline`). Operations run in YAML
//! declaration order. Each nested `repo:` is resolved lazily via
//! `resolve_repo_inline`, and the sub-composite merges immediately
//! through `phase4::integrate_sub_composite` instead of a single batch
//! Phase 4 pass over all intermediates.
//!
//! The two block types differ only in starting state and finalization:
//!
//! - **`self:` blocks** start from the local working directory and write
//!   directly after the sequential pass.
//! - **Source blocks** start from an empty FS and run Phase 5 (local file
//!   merge) after the sequential pass to combine the composite with local
//!   files.
//!
//! [`execute_pull`] partitions the config into source and `self:` operations,
//! runs the sequential pipeline for sources, then runs the sequential
//! pipeline for each `self:` block.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use log::warn;

use super::{phase1, phase2, phase4, phase5, phase6, ClonedRepo, IntermediateFS};
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
    // Track auto-merge targets across nested repos so cross-upstream
    // auto-merges accumulate correctly (e.g., when a chained upstream
    // also declares auto-merge for the same file).
    let mut accumulated_auto_merge_targets = HashMap::new();

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

                    // Use auto-merge-aware integration so that chained
                    // repos with auto-merge declarations accumulate
                    // content instead of overwriting via last-write-wins.
                    let nested_intermediate = IntermediateFS::new_with_vars_and_merges(
                        nested_result.fs.clone(),
                        nested_result.upstream_url.clone(),
                        nested_result.upstream_ref.clone(),
                        HashMap::new(), // template_vars handled below
                        nested_result.merge_operations.clone(),
                    );
                    let residual = phase4::integrate_sub_composite_with_targets(
                        &mut fs,
                        &nested_intermediate,
                        &mut accumulated_auto_merge_targets,
                    )?;

                    merge_operations.extend(nested_result.merge_operations);
                    merge_operations.extend(residual);

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

            // Same auto-merge-aware integration for tree children
            let child_intermediate = IntermediateFS::new_with_vars_and_merges(
                child_result.fs.clone(),
                child_result.upstream_url.clone(),
                child_result.upstream_ref.clone(),
                HashMap::new(),
                child_result.merge_operations.clone(),
            );
            let residual = phase4::integrate_sub_composite_with_targets(
                &mut fs,
                &child_intermediate,
                &mut accumulated_auto_merge_targets,
            )?;

            merge_operations.extend(child_result.merge_operations);
            merge_operations.extend(residual);
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

/// Whether the sequential pipeline is running for a `self:` block or a
/// source block.
///
/// Both use [`execute_sequential_pipeline`], but differ in two ways:
///
/// 1. **Starting FS** — `SelfBlock` starts from local files (local files
///    form the base of the composite). `SourceBlock` starts from an empty
///    FS so that consumer-level operations (include, exclude, rename) only
///    affect upstream content. Local files are combined in Phase 5 after
///    the sequential pass.
///
/// 2. **Phase 5** — `SourceBlock` runs a Phase 5 merge to combine the
///    composite with local files after the sequential pass. `SelfBlock`
///    skips this because local files were already the starting base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PipelineMode {
    SelfBlock,
    SourceBlock,
}

/// Execute a sequential pipeline where operations fire in declaration order.
///
/// This function walks the config's operations sequentially. When a `repo:`
/// operation is encountered, it resolves inline at that position using
/// [`resolve_repo_inline`] and [`phase4::integrate_sub_composite`].
///
/// Both `self:` and source blocks use this pipeline. See [`PipelineMode`]
/// for the differences.
fn execute_sequential_pipeline(
    config: &Schema,
    repo_manager: &RepositoryManager,
    cache: &RepoCache,
    working_dir: &Path,
    output_path: Option<&Path>,
    mode: PipelineMode,
) -> Result<MemoryFS> {
    // Phase 1: Discover and clone repos eagerly
    let repo_tree = phase1::execute(config, repo_manager, cache)?;

    // Build cloned_repos map for on-demand resolution
    let cloned_repos = phase2::clone_tree_repos(&repo_tree, repo_manager)?;

    // Starting FS depends on pipeline mode:
    // - SelfBlock: local files form the base. Upstream content overlays.
    // - SourceBlock: empty FS. Consumer operations (include, exclude, rename)
    //   affect only upstream content. Local files are combined in Phase 5.
    let mut fs = match mode {
        PipelineMode::SelfBlock => phase5::load_local_fs(working_dir)?,
        PipelineMode::SourceBlock => MemoryFS::new(),
    };

    let mut all_template_vars = HashMap::new();
    let mut residual_deferred_ops: Vec<Operation> = Vec::new();
    // Accumulate auto-merge targets across all repo integrations so that a
    // later repo can trigger format-aware merge for a file declared by an
    // earlier repo (or vice versa, when the later repo has defer: true).
    let mut accumulated_auto_merge_targets = HashMap::new();
    // In source mode, snapshot upstream file content for auto-merge targets
    // at the time each repo: fires. These snapshots survive subsequent
    // consumer operations (e.g., exclude) and are used during Phase 5 to
    // merge upstream content into local files that were not in the composite
    // during the sequential pass.
    let mut auto_merge_snapshots: HashMap<String, crate::filesystem::File> = HashMap::new();

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

                    // In source mode, snapshot auto-merge source files BEFORE
                    // integration. These snapshots survive subsequent consumer
                    // ops (exclude, rename) and are used during Phase 5 to
                    // merge upstream content into local files.
                    if mode == PipelineMode::SourceBlock {
                        for op in &sub_composite.merge_operations {
                            if let Some(path) = phase4::get_auto_merge_path(op) {
                                if let Some(file) = sub_composite.fs.get_file(path) {
                                    auto_merge_snapshots.insert(path.to_string(), file.clone());
                                }
                            }
                        }
                    }

                    let residual = phase4::integrate_sub_composite_with_targets(
                        &mut fs,
                        &sub_composite,
                        &mut accumulated_auto_merge_targets,
                    )?;
                    residual_deferred_ops.extend(residual);

                    // Upstream template vars fill in defaults but do not
                    // overwrite consumer-level vars already set by a preceding
                    // template-vars operation. This matches the old batch
                    // pipeline where Phase 4 processed repos in post-order
                    // (children before parents, local root last) and the local
                    // root's consumer vars were the final write.
                    for (key, value) in sub_composite.template_vars {
                        all_template_vars.entry(key).or_insert(value);
                    }
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
                // In source mode, the merge source and/or dest may be local
                // files not yet in the composite. Load them from disk so the
                // merge can reference them. This mirrors the old batch
                // pipeline where Phase 5 loaded local files before running
                // consumer merge operations.
                if mode == PipelineMode::SourceBlock {
                    for path in [
                        operation.merge_effective_source(),
                        operation.merge_effective_dest(),
                    ]
                    .into_iter()
                    .flatten()
                    {
                        if !fs.exists(path) {
                            let disk_path = working_dir.join(path);
                            if disk_path.exists() {
                                let content = std::fs::read(&disk_path)?;
                                fs.add_file(path, crate::filesystem::File::new(content))?;
                            }
                        }
                    }
                }
                phase4::execute_merge_operation(&mut fs, operation)?;
            }
            Operation::Tools { tools } => {
                crate::operators::tools::apply(tools)?;
            }
            Operation::Self_ { .. } => {}
        }
    }

    // Source blocks: Phase 5 — combine composite with local files.
    // Local files that are not in the composite are preserved. Composite
    // files win for shared paths (CompositePrecedence invariant). Auto-merge
    // targets use format-aware merging instead of overwriting.
    if mode == PipelineMode::SourceBlock {
        let local_fs = phase5::load_local_fs(working_dir)?;
        let mut combined = local_fs;
        // Overlay composite on top of local files, with auto-merge awareness
        phase4::merge_composite_with_auto_merge(
            &mut combined,
            &fs,
            &accumulated_auto_merge_targets,
        )?;

        // Apply auto-merge snapshots for upstream files that were removed
        // from the composite by consumer operations (e.g., exclude) after
        // the repo: fired. The snapshots preserve the upstream's original
        // file content so it can merge into local files even though the
        // composite no longer contains it.
        for (path, auto_merge_op) in &accumulated_auto_merge_targets {
            // Skip paths still in the composite — handled by the overlay.
            if fs.exists(path) {
                continue;
            }
            // Merge snapshot into local file if both exist
            if combined.exists(path) {
                if let Some(snapshot_file) = auto_merge_snapshots.get(path) {
                    let temp_path = format!(".__common_repo_auto_merge_snapshot__{}", path);
                    combined.add_file(&temp_path, snapshot_file.clone())?;
                    let explicit_op =
                        phase4::make_explicit_merge_op(auto_merge_op, &temp_path, path);
                    phase4::execute_merge_operation(&mut combined, &explicit_op)?;
                    combined.remove_file(&temp_path)?;
                }
            }
        }

        fs = combined;
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

/// Execute the complete pull operation.
///
/// Partitions the config into source and `self:` operations, then runs
/// each through [`execute_sequential_pipeline`]. Both use the same
/// sequential execution model where operations fire in YAML declaration
/// order. The only difference is starting state:
///
/// - Source blocks start from an empty FS and run Phase 5 (local merge)
///   after the sequential pass.
/// - `self:` blocks start from the local working directory and write
///   directly.
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

    // Run the source pipeline using the sequential model so operations
    // execute in YAML declaration order (same code path as self: blocks).
    let final_fs = execute_sequential_pipeline(
        &source_config,
        repo_manager,
        cache,
        working_dir,
        output_path,
        PipelineMode::SourceBlock,
    )?;

    // Run self: pipelines using the same sequential execution model.
    for self_op in &self_ops {
        execute_sequential_pipeline(
            &self_op.operations,
            repo_manager,
            cache,
            working_dir,
            output_path,
            PipelineMode::SelfBlock,
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
