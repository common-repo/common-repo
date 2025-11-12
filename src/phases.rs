//! Implementation of the 6 phases of the common-repo pull operation.
//!
//! ## Overview
//!
//! The pull operation follows 6 phases:
//! 1. Discovery and Cloning - Fetch all inherited repos in parallel (with automatic caching)
//! 2. Processing Individual Repos - Apply operations to each repo
//! 3. Determining Operation Order - Calculate deterministic merge order
//! 4. Composite Filesystem Construction - Merge all intermediate filesystems
//! 5. Local File Merging - Merge with local files
//! 6. Writing to Disk - Write final result to host filesystem
//!
//! Note: Caching happens automatically during Phase 1 via RepositoryManager, so there is no
//! separate cache update phase.
//!
//! Each phase depends only on the previous phases and the foundation layers (0-2).

use std::collections::{HashMap, HashSet};

use crate::cache::RepoCache;
use crate::config::{Operation, Schema};
use crate::error::{Error, Result};
use crate::filesystem::MemoryFS;
use crate::repository::RepositoryManager;

/// Repository tree node representing inheritance hierarchy
#[derive(Debug, Clone)]
pub struct RepoNode {
    /// Repository URL
    pub url: String,
    /// Git reference (tag, branch, commit)
    pub ref_: String,
    /// Child repositories that inherit from this one
    pub children: Vec<RepoNode>,
    /// Operations to apply to this repository
    pub operations: Vec<Operation>,
}

impl RepoNode {
    pub fn new(url: String, ref_: String, operations: Vec<Operation>) -> Self {
        Self {
            url,
            ref_,
            children: Vec::new(),
            operations,
        }
    }

    pub fn add_child(&mut self, child: RepoNode) {
        self.children.push(child);
    }
}

/// Repository dependency tree for inheritance tracking
#[derive(Debug)]
pub struct RepoTree {
    /// Root repository (the one being pulled)
    pub root: RepoNode,
    /// All repositories in the tree (for cycle detection)
    pub all_repos: HashSet<(String, String)>,
}

impl RepoTree {
    pub fn new(root: RepoNode) -> Self {
        let mut all_repos = HashSet::new();
        Self::collect_repos(&root, &mut all_repos);
        Self { root, all_repos }
    }

    fn collect_repos(node: &RepoNode, repos: &mut HashSet<(String, String)>) {
        repos.insert((node.url.clone(), node.ref_.clone()));
        for child in &node.children {
            Self::collect_repos(child, repos);
        }
    }

    /// Check if a repo exists anywhere in the tree
    ///
    /// Note: This is a simple existence check. For proper cycle detection during
    /// tree construction, use `detect_cycles()` which checks for cycles in dependency paths.
    /// Multiple branches can reference the same repo without creating a cycle.
    #[allow(dead_code)]
    pub fn would_create_cycle(&self, url: &str, ref_: &str) -> bool {
        self.all_repos
            .contains(&(url.to_string(), ref_.to_string()))
    }
}

/// Phase 1: Discovery and Cloning
///
/// Recursively discovers all inherited repositories using breadth-first traversal,
/// then clones them in parallel. Handles cycle detection and network failures.
pub mod phase1 {
    use super::*;

    /// Execute Phase 1: Discover all repos and clone in parallel
    pub fn execute(
        config: &Schema,
        repo_manager: &RepositoryManager,
        cache: &RepoCache,
    ) -> Result<RepoTree> {
        let tree = discover_repos(config)?;
        clone_parallel(&tree, repo_manager, cache)?;
        Ok(tree)
    }

    /// Recursively discover all inherited repositories
    ///
    /// Uses breadth-first traversal to discover all repositories that need to be fetched.
    /// This ensures we find all dependencies before starting any cloning operations.
    pub fn discover_repos(config: &Schema) -> Result<RepoTree> {
        // Extract repo operations and build the repository tree
        let root_node = process_config_to_node(config)?;

        // Check for cycles in the discovered tree
        let tree = RepoTree::new(root_node);
        detect_cycles(&tree.root, &mut Vec::new())?;

        // TODO: Implement full recursive discovery - parse each repo's .common-repo.yaml
        // For now, we only discover top-level repo operations

        Ok(tree)
    }

    /// Detect cycles in the repository dependency tree
    ///
    /// A cycle occurs when a repository appears multiple times in a single dependency path
    /// (from root to leaf). Multiple branches can reference the same repo - that's allowed.
    fn detect_cycles(node: &RepoNode, path: &mut Vec<(String, String)>) -> Result<()> {
        // Skip the synthetic "local" root node for cycle detection
        if node.url != "local" {
            let repo_key = (node.url.clone(), node.ref_.clone());

            // Check if this repo already appears in the current path (cycle detected)
            if path.contains(&repo_key) {
                // Build cycle description showing the circular path
                let mut cycle_path = path
                    .iter()
                    .map(|(url, ref_)| format!("{}@{}", url, ref_))
                    .collect::<Vec<_>>();
                cycle_path.push(format!("{}@{}", node.url, node.ref_));

                return Err(Error::CycleDetected {
                    cycle: cycle_path.join(" -> "),
                });
            }

            // Add this repo to the current path
            path.push(repo_key.clone());
        }

        // Recursively check all children
        for child in &node.children {
            detect_cycles(child, path)?;
        }

        // Remove this repo from path when backtracking (allows same repo in different branches)
        if node.url != "local" {
            path.pop();
        }

        Ok(())
    }

    /// Convert a configuration into a repository node
    ///
    /// Extracts repo operations as child nodes and keeps other operations in the root node.
    fn process_config_to_node(config: &Schema) -> Result<RepoNode> {
        // For the root config, we don't have a URL/ref, so we create a synthetic root
        // The root represents the local operations that will be applied

        let mut repo_operations = Vec::new();
        let mut other_operations = Vec::new();

        // Separate repo operations from other operations
        for operation in config {
            match operation {
                Operation::Repo { repo } => {
                    repo_operations.push(repo.clone());
                }
                _ => {
                    other_operations.push(operation.clone());
                }
            }
        }

        // Create root node with non-repo operations
        let mut root_node = RepoNode::new(
            "local".to_string(), // Special marker for local config
            "HEAD".to_string(),  // Not used for local
            other_operations,
        );

        // Create child nodes for each repo operation
        for repo_op in repo_operations {
            // Check for cycles before adding (same url+ref as root would be a cycle)
            if repo_op.url == "local" {
                return Err(Error::CycleDetected {
                    cycle: format!("{}@{}", repo_op.url, repo_op.r#ref),
                });
            }

            // Extract operations from the repo's `with:` clause
            let child_operations = repo_op.with;

            let child_node = RepoNode::new(repo_op.url, repo_op.r#ref, child_operations);

            root_node.add_child(child_node);
        }

        Ok(root_node)
    }

    /// Clone all repositories in the tree in parallel
    ///
    /// Uses breadth-first ordering to maximize parallelism - all repos at depth N
    /// are cloned before moving to depth N+1.
    ///
    /// Note: Currently implements sequential cloning per level. To enable true parallel cloning,
    /// RepositoryManager would need to be wrapped in Arc or made Clone, or we could use
    /// rayon/tokio for parallelization. The structure is ready for parallelization.
    pub fn clone_parallel(
        tree: &RepoTree,
        repo_manager: &RepositoryManager,
        _cache: &RepoCache, // TODO: Use cache for network failure fallback
    ) -> Result<()> {
        let mut current_level = vec![&tree.root];
        let mut next_level = Vec::new();

        while !current_level.is_empty() {
            // Collect all repos at current depth level that need cloning
            let repos_to_clone: Vec<(&str, &str)> = current_level
                .iter()
                .filter_map(|node| {
                    if node.url != "local" {
                        Some((node.url.as_str(), node.ref_.as_str()))
                    } else {
                        None
                    }
                })
                .collect();

            // Clone all repos at current depth level
            // TODO: Parallelize this loop when RepositoryManager supports Arc/Clone or when
            // using rayon/tokio. For now, sequential cloning ensures correctness.
            for (url, ref_) in repos_to_clone {
                repo_manager.fetch_repository(url, ref_)?;
            }

            // Collect next level
            for node in &current_level {
                for child in &node.children {
                    next_level.push(child);
                }
            }

            current_level = next_level.to_vec();
            next_level.clear();
        }

        Ok(())
    }
}

// Placeholder modules for remaining phases
/// Intermediate filesystem wrapper with metadata
#[derive(Debug, Clone)]
pub struct IntermediateFS {
    /// The processed filesystem
    pub fs: MemoryFS,
    /// Repository URL this FS came from (for debugging/tracking)
    pub source_url: String,
    /// Git reference used
    pub source_ref: String,
}

impl IntermediateFS {
    pub fn new(fs: MemoryFS, source_url: String, source_ref: String) -> Self {
        Self {
            fs,
            source_url,
            source_ref,
        }
    }
}

pub mod phase2 {
    use super::*;
    use crate::operators;

    /// Execute Phase 2: Process each repository into intermediate filesystem
    ///
    /// Takes the repository tree from Phase 1 and applies each repository's operations
    /// to produce intermediate filesystems. Uses the repo cache to avoid duplicate processing.
    pub fn execute(
        tree: &RepoTree,
        repo_manager: &RepositoryManager,
        cache: &RepoCache,
    ) -> Result<HashMap<String, IntermediateFS>> {
        let mut intermediate_fss = HashMap::new();

        // Process each repository in the tree
        process_repo_recursive(tree, &tree.root, repo_manager, cache, &mut intermediate_fss)?;

        Ok(intermediate_fss)
    }

    /// Recursively process repositories in the tree
    fn process_repo_recursive(
        _tree: &RepoTree,
        node: &RepoNode,
        repo_manager: &RepositoryManager,
        cache: &RepoCache,
        intermediate_fss: &mut HashMap<String, IntermediateFS>,
    ) -> Result<()> {
        // Process children first (dependencies)
        for child in &node.children {
            process_repo_recursive(_tree, child, repo_manager, cache, intermediate_fss)?;
        }

        // Process this repository
        let key = format!("{}@{}", node.url, node.ref_);
        if let std::collections::hash_map::Entry::Vacant(e) = intermediate_fss.entry(key) {
            let intermediate_fs = process_single_repo(node, repo_manager, cache)?;
            e.insert(intermediate_fs);
        }

        Ok(())
    }

    /// Process a single repository node into an intermediate filesystem
    fn process_single_repo(
        node: &RepoNode,
        repo_manager: &RepositoryManager,
        _cache: &RepoCache,
    ) -> Result<IntermediateFS> {
        let mut fs = if node.url == "local" {
            // For local config, start with empty filesystem
            MemoryFS::new()
        } else {
            // For real repositories, load from cache
            repo_manager.fetch_repository(&node.url, &node.ref_)?
        };

        // Apply all operations to the filesystem
        for operation in &node.operations {
            apply_operation(&mut fs, operation)?;
        }

        Ok(IntermediateFS::new(fs, node.url.clone(), node.ref_.clone()))
    }

    /// Apply a single operation to a filesystem
    fn apply_operation(fs: &mut MemoryFS, operation: &Operation) -> Result<()> {
        match operation {
            Operation::Include { include } => {
                // For include operations, create a new filtered filesystem
                let mut filtered_fs = MemoryFS::new();
                operators::include::apply(include, fs, &mut filtered_fs)?;
                // Replace the current filesystem with the filtered one
                *fs = filtered_fs;
                Ok(())
            }
            Operation::Exclude { exclude } => operators::exclude::apply(exclude, fs),
            Operation::Rename { rename } => operators::rename::apply(rename, fs),
            Operation::Repo { repo: _ } => {
                // Repo operations should have been processed in Phase 1
                // They create new repositories, not modify existing ones
                Ok(())
            }
            // TODO: Implement other operators when they're available
            Operation::Template { template: _ } => Err(Error::Generic(
                "Template operations not yet implemented".to_string(),
            )),
            Operation::TemplateVars { template_vars: _ } => Err(Error::Generic(
                "TemplateVars operations not yet implemented".to_string(),
            )),
            Operation::Tools { tools: _ } => Err(Error::Generic(
                "Tools operations not yet implemented".to_string(),
            )),
            Operation::Yaml { yaml: _ } => Err(Error::Generic(
                "YAML merge operations not yet implemented".to_string(),
            )),
            Operation::Json { json: _ } => Err(Error::Generic(
                "JSON merge operations not yet implemented".to_string(),
            )),
            Operation::Toml { toml: _ } => Err(Error::Generic(
                "TOML merge operations not yet implemented".to_string(),
            )),
            Operation::Ini { ini: _ } => Err(Error::Generic(
                "INI merge operations not yet implemented".to_string(),
            )),
            Operation::Markdown { markdown: _ } => Err(Error::Generic(
                "Markdown merge operations not yet implemented".to_string(),
            )),
        }
    }
}

/// Operation order for deterministic merging
#[derive(Debug, Clone)]
pub struct OperationOrder {
    /// Ordered list of repository keys in the correct merge order
    /// Format: "url@ref" (e.g., "https://github.com/user/repo@main")
    pub order: Vec<String>,
}

impl OperationOrder {
    pub fn new(order: Vec<String>) -> Self {
        Self { order }
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }
}

pub mod phase3 {
    use super::*;

    /// Execute Phase 3: Determine operation order for deterministic merging
    ///
    /// Uses depth-first traversal to build the correct order for applying operations.
    /// This ensures ancestors are processed before their dependents, guaranteeing
    /// that base configurations are applied before derived ones.
    ///
    /// Returns an OperationOrder containing repository keys in merge order.
    pub fn execute(tree: &RepoTree) -> Result<OperationOrder> {
        let mut order = Vec::new();
        let mut visited = HashSet::new();

        // Start with the root and build order depth-first
        build_order_recursive(&tree.root, &mut order, &mut visited);

        Ok(OperationOrder::new(order))
    }

    /// Recursively build operation order using depth-first traversal
    ///
    /// This ensures that dependencies (children) are processed before their parents.
    /// The resulting order guarantees that base repositories are applied before
    /// repositories that depend on them.
    fn build_order_recursive(
        node: &RepoNode,
        order: &mut Vec<String>,
        visited: &mut HashSet<String>,
    ) {
        let node_key = format!("{}@{}", node.url, node.ref_);

        // Skip if already processed (shouldn't happen in a tree, but safety check)
        if visited.contains(&node_key) {
            return;
        }

        // First, process all children (dependencies) recursively
        // This ensures dependencies come before their dependents in the final order
        for child in &node.children {
            build_order_recursive(child, order, visited);
        }

        // Then add this node to the order
        order.push(node_key.clone());
        visited.insert(node_key);
    }
}

pub mod phase4 {
    use super::*;

    /// Execute Phase 4: Build composite filesystem by merging intermediate filesystems
    ///
    /// Takes the operation order from Phase 3 and the intermediate filesystems from Phase 2,
    /// then merges them in the correct order to create the final composite filesystem.
    /// Uses last-write-wins strategy for conflicts.
    ///
    /// Returns the final composite MemoryFS ready for local merging and writing to disk.
    pub fn execute(
        order: &OperationOrder,
        intermediate_fss: &HashMap<String, IntermediateFS>,
    ) -> Result<MemoryFS> {
        let mut composite_fs = MemoryFS::new();

        // Merge filesystems in the operation order
        // Later filesystems in the order take precedence (last-write-wins)
        for repo_key in &order.order {
            if let Some(intermediate_fs) = intermediate_fss.get(repo_key) {
                merge_filesystem(&mut composite_fs, &intermediate_fs.fs)?;
            } else {
                // This shouldn't happen if Phase 2 and Phase 3 are implemented correctly
                return Err(Error::Generic(format!(
                    "Missing intermediate filesystem for repository: {}",
                    repo_key
                )));
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
}

/// Orchestrator for the complete pull operation
///
/// This module coordinates all phases to provide a clean API for the complete
/// pull operation. Currently implements Phases 1-5 for end-to-end inheritance.
pub mod orchestrator {
    use super::*;
    use crate::cache::RepoCache;
    use crate::repository::RepositoryManager;
    use std::path::Path;

    /// Execute the complete pull operation (Phases 1-5 implemented)
    ///
    /// This orchestrates the complete inheritance pipeline:
    /// 1. Discover and clone repositories (with automatic caching)
    /// 2. Process each repository with its operations
    /// 3. Determine correct merge order
    /// 4. Merge into composite filesystem
    /// 5. Merge with local files and apply local operations
    ///
    /// Returns the final MemoryFS ready for disk writing.
    pub fn execute_pull(
        config: &Schema,
        repo_manager: &RepositoryManager,
        cache: &RepoCache,
        working_dir: &Path,
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

        Ok(final_fs)
    }
}

pub mod phase5 {
    use super::*;
    use crate::config::{IniMergeOp, JsonMergeOp, MarkdownMergeOp, TomlMergeOp, YamlMergeOp};
    use crate::filesystem::File;
    use std::path::Path;

    /// Execute Phase 5: Merge composite filesystem with local files
    ///
    /// Takes the composite filesystem from Phase 4 and merges it with local files
    /// from the current directory. Local files take precedence (last-write-wins).
    /// Applies any local operations from the configuration.
    ///
    /// Returns the final filesystem ready for writing to disk.
    pub fn execute(
        composite_fs: &MemoryFS,
        local_config: &Schema,
        working_dir: &Path,
    ) -> Result<MemoryFS> {
        // Start with a copy of the composite filesystem
        let mut final_fs = composite_fs.clone();

        // Load local files and merge them in
        let local_fs = load_local_fs(working_dir)?;
        merge_local_files(&mut final_fs, &local_fs)?;

        // Apply local operations (typically merge operations)
        apply_local_operations(&mut final_fs, local_config)?;

        Ok(final_fs)
    }

    /// Load local files from the working directory into a MemoryFS
    ///
    /// Recursively walks the directory and loads all files, preserving relative paths.
    fn load_local_fs(working_dir: &Path) -> Result<MemoryFS> {
        let mut local_fs = MemoryFS::new();

        // Use walkdir to recursively find all files
        for entry in walkdir::WalkDir::new(working_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let file_path = entry.path();

            // Get relative path from working directory
            let relative_path = file_path.strip_prefix(working_dir).map_err(|_| {
                Error::Generic(format!(
                    "Failed to make path relative: {}",
                    file_path.display()
                ))
            })?;

            // Skip common-repo config file and hidden files/directories
            let path_str = relative_path.to_string_lossy();
            if path_str == ".common-repo.yaml"
                || path_str.starts_with(".git/")
                || path_str.starts_with(".")
            {
                continue;
            }

            // Read file content
            let content = std::fs::read(file_path).map_err(|e| {
                Error::Generic(format!(
                    "Failed to read local file {}: {}",
                    file_path.display(),
                    e
                ))
            })?;

            // Add to filesystem with relative path
            local_fs.add_file(relative_path, File::new(content))?;
        }

        Ok(local_fs)
    }

    /// Merge local files into the final filesystem
    ///
    /// Local files take precedence over inherited files (last-write-wins).
    /// This allows local customization and overrides.
    fn merge_local_files(final_fs: &mut MemoryFS, local_fs: &MemoryFS) -> Result<()> {
        for (path, file) in local_fs.files() {
            // Local files override inherited files
            final_fs.add_file(path, file.clone())?;
        }
        Ok(())
    }

    /// Apply local operations from the configuration
    ///
    /// These are operations that apply to the final merged filesystem,
    /// typically merge operations that combine local and inherited content.
    fn apply_local_operations(final_fs: &mut MemoryFS, local_config: &Schema) -> Result<()> {
        for operation in local_config {
            match operation {
                // For now, we only support merge operations in the local phase
                // Other operations (repo, include, exclude, rename) should be handled in earlier phases
                Operation::Yaml { yaml } => {
                    apply_yaml_merge_operation(final_fs, yaml)?;
                }
                Operation::Json { json } => {
                    apply_json_merge_operation(final_fs, json)?;
                }
                Operation::Toml { toml } => {
                    apply_toml_merge_operation(final_fs, toml)?;
                }
                Operation::Ini { ini } => {
                    apply_ini_merge_operation(final_fs, ini)?;
                }
                Operation::Markdown { markdown } => {
                    apply_markdown_merge_operation(final_fs, markdown)?;
                }
                // Template operations would go here when implemented
                Operation::Template { .. } | Operation::TemplateVars { .. } => {
                    return Err(Error::Generic(
                        "Template operations not yet implemented".to_string(),
                    ));
                }
                // These operations should not appear in local config (handled in earlier phases)
                Operation::Repo { .. }
                | Operation::Include { .. }
                | Operation::Exclude { .. }
                | Operation::Rename { .. }
                | Operation::Tools { .. } => {
                    return Err(Error::Generic(format!(
                        "Operation {:?} should not appear in local merge phase",
                        operation
                    )));
                }
            }
        }
        Ok(())
    }

    /// Apply a YAML merge operation to the filesystem
    fn apply_yaml_merge_operation(_fs: &mut MemoryFS, _yaml_op: &YamlMergeOp) -> Result<()> {
        // TODO: Implement YAML merge operations
        Err(Error::Generic(
            "YAML merge operations not yet implemented".to_string(),
        ))
    }

    /// Apply a JSON merge operation to the filesystem
    fn apply_json_merge_operation(_fs: &mut MemoryFS, _json_op: &JsonMergeOp) -> Result<()> {
        // TODO: Implement JSON merge operations
        Err(Error::Generic(
            "JSON merge operations not yet implemented".to_string(),
        ))
    }

    /// Apply a TOML merge operation to the filesystem
    fn apply_toml_merge_operation(_fs: &mut MemoryFS, _toml_op: &TomlMergeOp) -> Result<()> {
        // TODO: Implement TOML merge operations
        Err(Error::Generic(
            "TOML merge operations not yet implemented".to_string(),
        ))
    }

    /// Apply an INI merge operation to the filesystem
    fn apply_ini_merge_operation(_fs: &mut MemoryFS, _ini_op: &IniMergeOp) -> Result<()> {
        // TODO: Implement INI merge operations
        Err(Error::Generic(
            "INI merge operations not yet implemented".to_string(),
        ))
    }

    /// Apply a Markdown merge operation to the filesystem
    fn apply_markdown_merge_operation(
        _fs: &mut MemoryFS,
        _markdown_op: &MarkdownMergeOp,
    ) -> Result<()> {
        // TODO: Implement Markdown merge operations
        Err(Error::Generic(
            "Markdown merge operations not yet implemented".to_string(),
        ))
    }
}

pub mod phase6 {
    use super::*;

    pub fn execute(_final_fs: &MemoryFS, _output_path: &std::path::Path) -> Result<()> {
        // TODO: Implement Phase 6 - Writing to Disk
        Err(Error::Generic("Phase 6 not yet implemented".to_string()))
    }
}
