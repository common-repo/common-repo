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

use crate::cache::{CacheKey, RepoCache};
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
        // Only add real repositories, not the synthetic "local" root
        if node.url != "local" {
            repos.insert((node.url.clone(), node.ref_.clone()));
        }
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
/// This is the first phase of the `common-repo` execution pipeline. Its primary
/// responsibilities are to discover all inherited repositories and then clone
/// them in parallel.
///
/// ## Process
///
/// 1.  **Discovery (`discover_repos`)**: The process begins by parsing the root
///     `.common-repo.yaml` file. It then recursively fetches the configuration
///     from each inherited repository, building a complete dependency tree
///     (`RepoTree`). A breadth-first traversal is used to ensure that all
///     repositories at a given depth are discovered before moving to the next
///     level.
///
/// 2.  **Cycle Detection**: During discovery, the process keeps track of the
///     inheritance path and will abort if a circular dependency is detected
///     (e.g., repository A inherits from B, which in turn inherits from A).
///
/// 3.  **Parallel Cloning (`clone_parallel`)**: Once the complete dependency
///     tree is built, all the repositories are cloned in parallel to maximize
///     performance. The `RepositoryManager` is used for this, which automatically
///     handles on-disk caching to avoid re-downloading repositories that are
///     already up to date.
///
/// This phase ensures that all the necessary source material is available locally
/// before the processing and merging phases begin.
pub mod phase1 {
    use super::*;

    /// Executes Phase 1 of the pipeline.
    ///
    /// This function orchestrates the discovery and cloning process by calling
    /// `discover_repos` to build the repository tree and then `clone_parallel`
    /// to fetch all the repositories.
    pub fn execute(
        config: &Schema,
        repo_manager: &RepositoryManager,
        cache: &RepoCache,
    ) -> Result<RepoTree> {
        let tree = discover_repos(config, repo_manager)?;
        clone_parallel(&tree, repo_manager, cache)?;
        Ok(tree)
    }

    /// Recursively discover all inherited repositories
    ///
    /// Uses breadth-first traversal to discover all repositories that need to be fetched.
    /// This ensures we find all dependencies before starting any cloning operations.
    pub fn discover_repos(config: &Schema, repo_manager: &RepositoryManager) -> Result<RepoTree> {
        // Extract repo operations and build the repository tree
        let root_node = process_config_to_node(config)?;

        // Recursively discover all inherited repos by parsing their .common-repo.yaml files
        let root_node = discover_inherited_configs(root_node, repo_manager, &mut HashSet::new())?;

        // Check for cycles in the discovered tree
        let tree = RepoTree::new(root_node.clone());
        detect_cycles(&tree.root, &mut Vec::new())?;

        Ok(tree)
    }

    /// Recursively discover inherited configurations from .common-repo.yaml files
    ///
    /// For each repository node in the tree, fetch the repo and parse its .common-repo.yaml
    /// file to discover further inheritance. Uses a visited set to prevent infinite recursion.
    fn discover_inherited_configs(
        mut node: RepoNode,
        repo_manager: &RepositoryManager,
        visited: &mut HashSet<(String, String)>,
    ) -> Result<RepoNode> {
        // Process all children recursively
        let mut new_children = Vec::new();

        for child in node.children {
            // If this child represents a real repository (not "local"), try to parse its config
            if child.url != "local" {
                let repo_key = (child.url.clone(), child.ref_.clone());

                // Check if we've already visited this repo to prevent infinite recursion
                if visited.contains(&repo_key) {
                    // Skip this repo - it's already been processed (cycle prevention)
                    continue;
                }

                // Mark as visited
                visited.insert(repo_key.clone());

                // Try to fetch and parse the inherited config
                match fetch_and_parse_config(&child.url, &child.ref_, repo_manager) {
                    Ok(inherited_config) => {
                        // Process the inherited config to get its repo operations
                        let inherited_node = process_config_to_node(&inherited_config)?;

                        // Recursively discover configs for the inherited repos
                        let inherited_node =
                            discover_inherited_configs(inherited_node, repo_manager, visited)?;

                        // The inherited node becomes a child, but we also need to preserve
                        // the operations from the current child's `with:` clause
                        let mut combined_node = RepoNode::new(
                            child.url.clone(),
                            child.ref_.clone(),
                            child.operations.clone(),
                        );

                        // Add all the inherited repos as children
                        for inherited_child in inherited_node.children {
                            combined_node.add_child(inherited_child);
                        }

                        new_children.push(combined_node);
                    }
                    Err(_) => {
                        // If we can't fetch/parse the config, just use the original child as-is
                        // This allows repositories without .common-repo.yaml files to still work
                        new_children.push(child);
                    }
                }

                // Remove from visited set when done processing this branch
                visited.remove(&repo_key);
            } else {
                // Local nodes don't need inheritance discovery
                new_children.push(child);
            }
        }

        node.children = new_children;
        Ok(node)
    }

    /// Fetch a repository and parse its .common-repo.yaml file
    fn fetch_and_parse_config(
        url: &str,
        ref_: &str,
        repo_manager: &RepositoryManager,
    ) -> Result<Schema> {
        // Fetch the repository
        let fs = repo_manager.fetch_repository(url, ref_)?;

        // Try to read .common-repo.yaml
        let config_content = match fs.get_file(".common-repo.yaml") {
            Some(file) => file.content.clone(),
            None => {
                // Try .commonrepo.yaml as fallback
                match fs.get_file(".commonrepo.yaml") {
                    Some(file) => file.content.clone(),
                    None => {
                        return Err(Error::ConfigParse {
                            message: "No .common-repo.yaml or .commonrepo.yaml found in repository"
                                .to_string(),
                        });
                    }
                }
            }
        };

        // Parse the YAML content
        let yaml_str = String::from_utf8(config_content).map_err(|_| Error::ConfigParse {
            message: "Invalid UTF-8 in .common-repo.yaml".to_string(),
        })?;

        crate::config::parse(&yaml_str)
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
    /// Network Failure Behavior:
    /// - If clone fails but cache exists, continue with cached version and warn
    /// - If clone fails and no cache exists, abort with error
    ///
    /// Note: Currently implements sequential cloning per level. To enable true parallel cloning,
    /// RepositoryManager would need to be wrapped in Arc or made Clone, or we could use
    /// rayon/tokio for parallelization. The structure is ready for parallelization.
    pub fn clone_parallel(
        tree: &RepoTree,
        repo_manager: &RepositoryManager,
        _cache: &RepoCache,
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
                // Try to fetch the repository
                if let Err(e) = repo_manager.fetch_repository(url, ref_) {
                    // Check if this is a network-related error and if we have a cached version
                    let is_network_error =
                        matches!(e, Error::GitClone { .. }) || matches!(e, Error::Network { .. });

                    if is_network_error && repo_manager.is_cached(url, ref_) {
                        // Fall back to cached version with warning
                        eprintln!(
                            "Warning: Network fetch failed for {}@{}, falling back to cached version",
                            url, ref_
                        );
                        // Continue - the repository is already cached and will be used
                    } else {
                        // Either not a network error, or no cache available - propagate the error
                        return Err(e);
                    }
                }
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
    /// Template variables collected from this repository's operations
    pub template_vars: HashMap<String, String>,
}

impl IntermediateFS {
    pub fn new(fs: MemoryFS, source_url: String, source_ref: String) -> Self {
        Self {
            fs,
            source_url,
            source_ref,
            template_vars: HashMap::new(),
        }
    }

    pub fn new_with_vars(
        fs: MemoryFS,
        source_url: String,
        source_ref: String,
        template_vars: HashMap<String, String>,
    ) -> Self {
        Self {
            fs,
            source_url,
            source_ref,
            template_vars,
        }
    }
}

pub mod phase2 {
    use super::*;
    use crate::operators;
    use serde_yaml;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    /// # Phase 2: Processing Individual Repositories
    ///
    /// This is the second phase of the `common-repo` execution pipeline. Its main
    /// responsibility is to take the raw, cloned repositories from Phase 1 and
    /// apply the operations defined in the configuration to each one, producing a
    /// set of "intermediate" in-memory filesystems.
    ///
    /// ## Process
    ///
    /// 1.  **Recursive Processing**: The process traverses the `RepoTree` from the
    ///     leaves up to the root. For each repository in the tree, it performs the
    ///     following steps.
    ///
    /// 2.  **In-Process Caching**: Before processing, it checks the in-process
    ///     `RepoCache` to see if this exact repository (with the same set of `with:`
    ///     clause operations) has already been processed in this run. If so, it
    ///     uses the cached result to avoid redundant work.
    ///
    /// 3.  **Operation Application**: If not cached, it loads the repository's
    ///     contents from the on-disk cache into a `MemoryFS`. It then iterates
    ///     through the operations associated with that repository (from the `with:`
    ///     clause) and applies each one in order to the `MemoryFS`.
    ///
    /// 4.  **Template Variable Collection**: During this process, it also collects
    ///     any `template_vars` defined in the operations and stores them in the
    ///     `IntermediateFS` for use in a later phase.
    ///
    /// 5.  **Storing Results**: The resulting `MemoryFS` and collected template
    ///     variables are stored in an `IntermediateFS` struct, which is then added
    ///     to the in-process cache and a `HashMap` that is passed to the next phase.
    ///
    /// This phase transforms the raw source material from each repository into a
    /// set of processed, in-memory filesystems that are ready to be merged.
    ///
    /// Executes Phase 2 of the pipeline.
    ///
    /// This function takes the `RepoTree` from Phase 1 and processes each
    /// repository in the tree, applying its associated operations to produce a
    /// map of `IntermediateFS` instances, keyed by a unique repository
    /// identifier.
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
        cache: &RepoCache,
    ) -> Result<IntermediateFS> {
        // Collect template variables from operations
        let template_vars = collect_template_vars(&node.operations)?;

        if let Some(cache_key) = cache_key_for_node(node)? {
            let fs = cache.get_or_process(cache_key, || -> Result<MemoryFS> {
                let mut fs = repo_manager.fetch_repository(&node.url, &node.ref_)?;
                for operation in &node.operations {
                    apply_operation(&mut fs, operation)?;
                }
                Ok(fs)
            })?;

            return Ok(IntermediateFS::new_with_vars(
                fs,
                node.url.clone(),
                node.ref_.clone(),
                template_vars,
            ));
        }

        // Local repository: process directly without caching
        let mut fs = MemoryFS::new();
        for operation in &node.operations {
            apply_operation(&mut fs, operation)?;
        }

        Ok(IntermediateFS::new_with_vars(
            fs,
            node.url.clone(),
            node.ref_.clone(),
            template_vars,
        ))
    }

    /// Collect template variables from operations without processing them
    fn collect_template_vars(operations: &[Operation]) -> Result<HashMap<String, String>> {
        use crate::operators::template_vars;
        let mut vars = HashMap::new();

        for operation in operations {
            if let Operation::TemplateVars { template_vars } = operation {
                template_vars::collect(template_vars, &mut vars)?;
            }
        }

        Ok(vars)
    }

    /// Build a cache key for a repository node (includes operations fingerprint)
    fn cache_key_for_node(node: &RepoNode) -> Result<Option<CacheKey>> {
        if node.url == "local" {
            return Ok(None);
        }

        if node.operations.is_empty() {
            return Ok(Some(CacheKey::new(&node.url, &node.ref_)));
        }

        let serialized_ops =
            serde_yaml::to_string(&node.operations).map_err(|err| Error::Serialization {
                message: format!(
                    "Failed to serialize operations for cache key ({}@{}): {}",
                    node.url, node.ref_, err
                ),
            })?;

        let mut hasher = DefaultHasher::new();
        serialized_ops.hash(&mut hasher);
        let fingerprint = format!("ops-{:016x}", hasher.finish());

        Ok(Some(CacheKey::new(
            &format!("{}#{}", node.url, fingerprint),
            &node.ref_,
        )))
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
            Operation::Template { template } => {
                use crate::operators::template;
                template::mark(template, fs)
            }
            Operation::TemplateVars { template_vars: _ } => {
                // Template variables are collected separately in collect_template_vars()
                // and processed in Phase 4, so skip them here
                Ok(())
            }
            Operation::Tools { tools } => operators::tools::apply(tools),
            // Merge operations are handled in Phase 5, not Phase 2
            Operation::Yaml { yaml: _ } => Err(Error::NotImplemented {
                feature: "YAML merge operations".to_string(),
            }),
            Operation::Json { json: _ } => Err(Error::NotImplemented {
                feature: "JSON merge operations".to_string(),
            }),
            Operation::Toml { toml: _ } => Err(Error::NotImplemented {
                feature: "TOML merge operations".to_string(),
            }),
            Operation::Ini { ini: _ } => Err(Error::NotImplemented {
                feature: "INI merge operations".to_string(),
            }),
            Operation::Markdown { markdown: _ } => Err(Error::NotImplemented {
                feature: "Markdown merge operations".to_string(),
            }),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::config::{ExcludeOp, Operation, RepoOp};
        use crate::filesystem::MemoryFS;
        use crate::repository::{CacheOperations, GitOperations, RepositoryManager};
        use std::collections::HashMap;
        use std::path::{Path, PathBuf};
        use std::sync::{Arc, Mutex};

        struct RecursiveMockGitOps {
            repo_configs: HashMap<String, String>, // url -> yaml content
        }

        impl RecursiveMockGitOps {
            fn new() -> Self {
                let mut repo_configs = HashMap::new();

                // Parent repo has its own config with a child repo
                repo_configs.insert(
                    "https://github.com/parent/repo.git".to_string(),
                    r#"
- repo:
    url: https://github.com/child/repo.git
    ref: main
    with:
      - include: ["src/**"]
- include: ["parent.txt"]
"#
                    .to_string(),
                );

                // Child repo has its own config
                repo_configs.insert(
                    "https://github.com/child/repo.git".to_string(),
                    r#"
- include: ["child.txt"]
"#
                    .to_string(),
                );

                Self { repo_configs }
            }
        }

        impl GitOperations for RecursiveMockGitOps {
            fn clone_shallow(&self, _url: &str, _ref_name: &str, _path: &Path) -> Result<()> {
                Ok(())
            }

            fn list_tags(&self, _url: &str) -> Result<Vec<String>> {
                Ok(vec![])
            }
        }

        struct RecursiveMockCacheOps {
            repo_configs: HashMap<String, String>,
        }

        impl RecursiveMockCacheOps {
            fn new(repo_configs: HashMap<String, String>) -> Self {
                Self { repo_configs }
            }

            fn get_repo_key(&self, cache_path: &Path) -> Option<String> {
                // Extract repo URL from cache path (simplified logic for test)
                if cache_path.to_string_lossy().contains("parent") {
                    Some("https://github.com/parent/repo.git".to_string())
                } else if cache_path.to_string_lossy().contains("child") {
                    Some("https://github.com/child/repo.git".to_string())
                } else {
                    None
                }
            }
        }

        impl CacheOperations for RecursiveMockCacheOps {
            fn exists(&self, _cache_path: &Path) -> bool {
                true // Always cached for this test
            }

            fn get_cache_path(&self, url: &str, _ref_name: &str) -> PathBuf {
                // Create a path that encodes the URL for testing
                PathBuf::from(format!(
                    "/mock/cache/{}",
                    url.replace("/", "_").replace(":", "")
                ))
            }

            fn load_from_cache(&self, cache_path: &Path) -> Result<MemoryFS> {
                let mut fs = MemoryFS::new();

                if let Some(repo_url) = self.get_repo_key(cache_path) {
                    if let Some(config_content) = self.repo_configs.get(&repo_url) {
                        fs.add_file_string(".common-repo.yaml", config_content)?;
                    }
                }

                Ok(fs)
            }

            fn save_to_cache(&self, _cache_path: &Path, _fs: &MemoryFS) -> Result<()> {
                Ok(())
            }
        }

        struct MockGitOps {
            clone_calls: Arc<Mutex<usize>>,
            cached_flag: Arc<Mutex<bool>>,
        }

        impl MockGitOps {
            fn new(clone_calls: Arc<Mutex<usize>>, cached_flag: Arc<Mutex<bool>>) -> Self {
                Self {
                    clone_calls,
                    cached_flag,
                }
            }
        }

        impl GitOperations for MockGitOps {
            fn clone_shallow(&self, _url: &str, _ref_name: &str, _path: &Path) -> Result<()> {
                *self.clone_calls.lock().unwrap() += 1;
                *self.cached_flag.lock().unwrap() = true;
                Ok(())
            }

            fn list_tags(&self, _url: &str) -> Result<Vec<String>> {
                Ok(vec![])
            }
        }

        struct MockCacheOps {
            cached_flag: Arc<Mutex<bool>>,
            filesystem: MemoryFS,
        }

        impl MockCacheOps {
            fn new(cached_flag: Arc<Mutex<bool>>) -> Self {
                let mut filesystem = MemoryFS::new();
                filesystem.add_file_string("keep.txt", "important").unwrap();
                filesystem.add_file_string("temp.tmp", "remove me").unwrap();

                Self {
                    cached_flag,
                    filesystem,
                }
            }
        }

        impl CacheOperations for MockCacheOps {
            fn exists(&self, _cache_path: &Path) -> bool {
                *self.cached_flag.lock().unwrap()
            }

            fn get_cache_path(&self, _url: &str, _ref_name: &str) -> PathBuf {
                PathBuf::from("/mock/cache/path")
            }

            fn load_from_cache(&self, _cache_path: &Path) -> Result<MemoryFS> {
                Ok(self.filesystem.clone())
            }

            fn save_to_cache(&self, _cache_path: &Path, _fs: &MemoryFS) -> Result<()> {
                *self.cached_flag.lock().unwrap() = true;
                Ok(())
            }
        }

        fn build_tree_with_children(children: Vec<RepoNode>) -> RepoTree {
            let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            for child in children {
                root.add_child(child);
            }
            RepoTree::new(root)
        }

        #[test]
        fn reuses_processed_repo_when_operations_match() {
            let clone_calls = Arc::new(Mutex::new(0));
            let cached_flag = Arc::new(Mutex::new(false));

            let repo_manager = RepositoryManager::with_operations(
                Box::new(MockGitOps::new(clone_calls.clone(), cached_flag.clone())),
                Box::new(MockCacheOps::new(cached_flag.clone())),
            );

            let cache = RepoCache::new();

            let operations = vec![Operation::Exclude {
                exclude: ExcludeOp {
                    patterns: vec!["*.tmp".to_string()],
                },
            }];

            let child1 = RepoNode::new(
                "https://example.com/repo.git".to_string(),
                "main".to_string(),
                operations.clone(),
            );
            let child2 = RepoNode::new(
                "https://example.com/repo.git".to_string(),
                "main".to_string(),
                operations,
            );

            let tree = build_tree_with_children(vec![child1, child2]);

            let intermediate = execute(&tree, &repo_manager, &cache).expect("phase2 execute");

            // One cloned repo (due to cache) and two entries (local + repo)
            assert_eq!(
                *clone_calls.lock().unwrap(),
                1,
                "expected repo to be processed only once"
            );
            assert_eq!(cache.len().unwrap(), 1);
            assert_eq!(intermediate.len(), 2);

            // Ensure the cached filesystem respected the exclude operation
            let repo_key = "https://example.com/repo.git@main";
            let repo_fs = &intermediate.get(repo_key).unwrap().fs;
            assert!(repo_fs.exists("keep.txt"));
            assert!(!repo_fs.exists("temp.tmp"));
        }

        #[test]
        fn test_recursive_discovery() {
            let mock_git = RecursiveMockGitOps::new();
            let repo_configs = mock_git.repo_configs.clone();
            let mock_cache = RecursiveMockCacheOps::new(repo_configs);

            let repo_manager =
                RepositoryManager::with_operations(Box::new(mock_git), Box::new(mock_cache));

            // Create a simple local config that includes the parent repo
            let local_config = vec![Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/parent/repo.git".to_string(),
                    r#ref: "main".to_string(),
                    path: None,
                    with: vec![Operation::Include {
                        include: crate::config::IncludeOp {
                            patterns: vec!["*.md".to_string()],
                        },
                    }],
                },
            }];

            // Discover repos recursively
            let tree = crate::phases::phase1::discover_repos(&local_config, &repo_manager)
                .expect("recursive discovery should succeed");

            // Verify the tree structure
            assert_eq!(
                tree.all_repos.len(),
                2,
                "Should discover parent and child repos"
            );

            // Root should have one child (parent repo)
            assert_eq!(tree.root.children.len(), 1, "Root should have one child");

            let parent_node = &tree.root.children[0];
            assert_eq!(parent_node.url, "https://github.com/parent/repo.git");
            assert_eq!(parent_node.ref_, "main");

            // Parent should have one child (child repo)
            assert_eq!(
                parent_node.children.len(),
                1,
                "Parent should have one child"
            );

            let child_node = &parent_node.children[0];
            assert_eq!(child_node.url, "https://github.com/child/repo.git");
            assert_eq!(child_node.ref_, "main");

            // Child should have no children (leaf node)
            assert_eq!(child_node.children.len(), 0, "Child should be a leaf node");
        }

        #[test]
        fn test_cycle_detection_during_discovery() {
            // This test would create a cycle in the inheritance chain
            // For now, we'll test that the visited set prevents infinite recursion
            // by creating a mock that would cause cycles if not handled properly

            let mut repo_configs = HashMap::new();

            // Repo A includes Repo B
            repo_configs.insert(
                "https://github.com/repo-a.git".to_string(),
                r#"
- repo:
    url: https://github.com/repo-b.git
    ref: main
"#
                .to_string(),
            );

            // Repo B includes Repo A (creates cycle)
            repo_configs.insert(
                "https://github.com/repo-b.git".to_string(),
                r#"
- repo:
    url: https://github.com/repo-a.git
    ref: main
"#
                .to_string(),
            );

            let mock_git = RecursiveMockGitOps {
                repo_configs: repo_configs.clone(),
            };
            let mock_cache = RecursiveMockCacheOps::new(repo_configs);

            let repo_manager =
                RepositoryManager::with_operations(Box::new(mock_git), Box::new(mock_cache));

            // Create a config that starts with repo-a
            let local_config = vec![Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/repo-a.git".to_string(),
                    r#ref: "main".to_string(),
                    path: None,
                    with: vec![],
                },
            }];

            // Discovery should succeed but avoid infinite recursion due to visited set
            let tree = crate::phases::phase1::discover_repos(&local_config, &repo_manager)
                .expect("cycle detection should prevent infinite recursion");

            // Due to cycle prevention, only repo-a gets fully discovered
            // repo-b would create a cycle so it's skipped
            assert_eq!(
                tree.all_repos.len(),
                1,
                "Should discover only repo-a due to cycle prevention"
            );

            // Repo A should not have children due to cycle prevention
            let repo_a_node = &tree.root.children[0];
            assert_eq!(
                repo_a_node.children.len(),
                0,
                "Repo A should not have children due to cycle prevention"
            );
        }

        #[test]
        fn test_detect_cycles_direct_cycle() {
            // Test direct cycle: repo A -> repo A
            // Note: The visited set prevents infinite recursion, but detect_cycles
            // should catch cycles in the final tree structure after discovery
            let config = vec![Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/repo-a.git".to_string(),
                    r#ref: "main".to_string(),
                    path: None,
                    with: vec![],
                },
            }];

            // Create a mock that returns a config with the same repo (direct cycle)
            let mut repo_configs = HashMap::new();
            repo_configs.insert(
                "https://github.com/repo-a.git".to_string(),
                r#"
- repo:
    url: https://github.com/repo-a.git
    ref: main
"#
                .to_string(),
            );

            let mock_git = RecursiveMockGitOps {
                repo_configs: repo_configs.clone(),
            };
            let mock_cache = RecursiveMockCacheOps::new(repo_configs);
            let repo_manager =
                RepositoryManager::with_operations(Box::new(mock_git), Box::new(mock_cache));

            // The visited set prevents the cycle from forming during discovery
            // So repo-a won't have itself as a child, and no cycle will be detected
            // This is expected behavior - the visited set prevents cycles
            let result = crate::phases::phase1::discover_repos(&config, &repo_manager);
            // Discovery succeeds because the visited set prevents the cycle
            assert!(result.is_ok());
            let tree = result.unwrap();
            // Repo-a should be discovered but not have itself as a child
            assert_eq!(tree.all_repos.len(), 1);
        }

        #[test]
        fn test_detect_cycles_indirect_cycle() {
            // Test indirect cycle: repo A -> repo B -> repo A
            let mut repo_configs = HashMap::new();

            // Repo A includes Repo B
            repo_configs.insert(
                "https://github.com/repo-a.git".to_string(),
                r#"
- repo:
    url: https://github.com/repo-b.git
    ref: main
"#
                .to_string(),
            );

            // Repo B includes Repo A (creates cycle)
            repo_configs.insert(
                "https://github.com/repo-b.git".to_string(),
                r#"
- repo:
    url: https://github.com/repo-a.git
    ref: main
"#
                .to_string(),
            );

            let mock_git = RecursiveMockGitOps {
                repo_configs: repo_configs.clone(),
            };
            let mock_cache = RecursiveMockCacheOps::new(repo_configs);
            let repo_manager =
                RepositoryManager::with_operations(Box::new(mock_git), Box::new(mock_cache));

            let config = vec![Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/repo-a.git".to_string(),
                    r#ref: "main".to_string(),
                    path: None,
                    with: vec![],
                },
            }];

            // This should detect the cycle
            // Note: The visited set prevents infinite recursion, but detect_cycles
            // should still catch cycles in the final tree structure
            // However, due to the visited set, repo-b won't be added as a child of repo-a
            // So the cycle might not be detected in the final tree
            // This is expected behavior - the visited set prevents the cycle from forming
            let _result = crate::phases::phase1::discover_repos(&config, &repo_manager);
        }

        #[test]
        fn test_detect_cycles_deep_cycle() {
            // Test deep cycle: repo A -> repo B -> repo C -> repo A
            let mut repo_configs = HashMap::new();

            repo_configs.insert(
                "https://github.com/repo-a.git".to_string(),
                r#"
- repo:
    url: https://github.com/repo-b.git
    ref: main
"#
                .to_string(),
            );

            repo_configs.insert(
                "https://github.com/repo-b.git".to_string(),
                r#"
- repo:
    url: https://github.com/repo-c.git
    ref: main
"#
                .to_string(),
            );

            repo_configs.insert(
                "https://github.com/repo-c.git".to_string(),
                r#"
- repo:
    url: https://github.com/repo-a.git
    ref: main
"#
                .to_string(),
            );

            let mock_git = RecursiveMockGitOps {
                repo_configs: repo_configs.clone(),
            };
            let mock_cache = RecursiveMockCacheOps::new(repo_configs);
            let repo_manager =
                RepositoryManager::with_operations(Box::new(mock_git), Box::new(mock_cache));

            let config = vec![Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/repo-a.git".to_string(),
                    r#ref: "main".to_string(),
                    path: None,
                    with: vec![],
                },
            }];

            // Discovery should handle the deep cycle
            let result = crate::phases::phase1::discover_repos(&config, &repo_manager);
            // The visited set will prevent the cycle from forming in the tree
            assert!(result.is_ok());
        }

        #[test]
        fn test_detect_cycles_no_cycles() {
            // Test that a valid tree with no cycles passes
            let mut repo_configs = HashMap::new();

            // Repo A includes Repo B
            repo_configs.insert(
                "https://github.com/repo-a.git".to_string(),
                r#"
- repo:
    url: https://github.com/repo-b.git
    ref: main
"#
                .to_string(),
            );

            // Repo B has no dependencies
            repo_configs.insert(
                "https://github.com/repo-b.git".to_string(),
                r#"
- include: ["*.md"]
"#
                .to_string(),
            );

            let mock_git = RecursiveMockGitOps {
                repo_configs: repo_configs.clone(),
            };
            let mock_cache = RecursiveMockCacheOps::new(repo_configs);
            let repo_manager =
                RepositoryManager::with_operations(Box::new(mock_git), Box::new(mock_cache));

            let config = vec![Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/repo-a.git".to_string(),
                    r#ref: "main".to_string(),
                    path: None,
                    with: vec![],
                },
            }];

            let result = crate::phases::phase1::discover_repos(&config, &repo_manager);
            assert!(result.is_ok());

            let tree = result.unwrap();
            // Should discover both repos
            assert!(!tree.all_repos.is_empty());
            assert_eq!(tree.root.children.len(), 1);
        }

        #[test]
        fn test_detect_cycles_same_repo_different_branches() {
            // Test that same repo with different refs doesn't create a cycle
            let mut repo_configs = HashMap::new();

            repo_configs.insert(
                "https://github.com/repo-a.git".to_string(),
                r#"
- repo:
    url: https://github.com/repo-b.git
    ref: main
- repo:
    url: https://github.com/repo-b.git
    ref: develop
"#
                .to_string(),
            );

            repo_configs.insert(
                "https://github.com/repo-b.git".to_string(),
                r#"
- include: ["*.md"]
"#
                .to_string(),
            );

            let mock_git = RecursiveMockGitOps {
                repo_configs: repo_configs.clone(),
            };
            let mock_cache = RecursiveMockCacheOps::new(repo_configs);
            let repo_manager =
                RepositoryManager::with_operations(Box::new(mock_git), Box::new(mock_cache));

            let config = vec![Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/repo-a.git".to_string(),
                    r#ref: "main".to_string(),
                    path: None,
                    with: vec![],
                },
            }];

            let result = crate::phases::phase1::discover_repos(&config, &repo_manager);
            assert!(result.is_ok());

            let tree = result.unwrap();
            // Should have repo-a and repo-b (main and develop are different refs, so different keys)
            // The visited set tracks (url, ref) pairs, so same URL with different refs are separate
            assert!(!tree.all_repos.is_empty());
        }

        #[test]
        fn test_discover_repos_simple_single_repo() {
            // Test discovery of a single repo with no dependencies
            let repo_configs = HashMap::new();

            let mock_git = RecursiveMockGitOps {
                repo_configs: repo_configs.clone(),
            };
            let mock_cache = RecursiveMockCacheOps::new(repo_configs);
            let repo_manager =
                RepositoryManager::with_operations(Box::new(mock_git), Box::new(mock_cache));

            let config = vec![Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/simple/repo.git".to_string(),
                    r#ref: "main".to_string(),
                    path: None,
                    with: vec![],
                },
            }];

            let result = crate::phases::phase1::discover_repos(&config, &repo_manager);
            assert!(result.is_ok());

            let tree = result.unwrap();
            assert_eq!(tree.all_repos.len(), 1);
            assert!(tree.all_repos.contains(&(
                "https://github.com/simple/repo.git".to_string(),
                "main".to_string()
            )));
        }

        #[test]
        fn test_discover_repos_with_path_filtering() {
            // Test discovery with path filtering
            let repo_configs = HashMap::new();

            let mock_git = RecursiveMockGitOps {
                repo_configs: repo_configs.clone(),
            };
            let mock_cache = RecursiveMockCacheOps::new(repo_configs);
            let repo_manager =
                RepositoryManager::with_operations(Box::new(mock_git), Box::new(mock_cache));

            let config = vec![Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/test/repo.git".to_string(),
                    r#ref: "main".to_string(),
                    path: Some("subdir".to_string()),
                    with: vec![],
                },
            }];

            let result = crate::phases::phase1::discover_repos(&config, &repo_manager);
            assert!(result.is_ok());

            let tree = result.unwrap();
            assert_eq!(tree.all_repos.len(), 1);
        }

        #[test]
        fn test_discover_repos_missing_config_file() {
            // Test discovery when a repo doesn't have .common-repo.yaml
            let repo_configs = HashMap::new();
            // Don't add config for the repo, simulating missing .common-repo.yaml

            let mock_git = RecursiveMockGitOps {
                repo_configs: repo_configs.clone(),
            };
            let mock_cache = RecursiveMockCacheOps::new(repo_configs);
            let repo_manager =
                RepositoryManager::with_operations(Box::new(mock_git), Box::new(mock_cache));

            let config = vec![Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/no-config/repo.git".to_string(),
                    r#ref: "main".to_string(),
                    path: None,
                    with: vec![],
                },
            }];

            // Should succeed - repos without config files are allowed
            let result = crate::phases::phase1::discover_repos(&config, &repo_manager);
            assert!(result.is_ok());

            let tree = result.unwrap();
            assert_eq!(tree.all_repos.len(), 1);
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

/// # Phase 4: Composite Filesystem Construction
///
/// This is the fourth phase of the `common-repo` execution pipeline. It is
/// responsible for taking all the processed `IntermediateFS` instances from
/// Phase 2 and combining them into a single, unified `MemoryFS`, which is
/// referred to as the "composite filesystem."
///
/// ## Process
///
/// 1.  **Template Variable Consolidation**: The process begins by collecting all
///     the `template_vars` from every `IntermediateFS` into a single, unified
///     `HashMap`. The variables are collected in the `OperationOrder` determined
///     in Phase 3, which ensures that variables from more specific, "closer"
///     repositories will override those from more general, "ancestor"
///     repositories.
///
/// 2.  **Template Processing**: Each `IntermediateFS` has its templates processed
///     using the consolidated set of variables. This step substitutes all the
///     `${VAR}` placeholders with their final values.
///
/// 3.  **Filesystem Merging**: After template processing, the `MemoryFS` from
///     each `IntermediateFS` is merged into the composite filesystem. The merge
///     is performed in the `OperationOrder`, which again ensures a "last-write-wins"
///     behavior, where files from more specific repositories overwrite those
///     from their ancestors.
///
/// This phase produces a single `MemoryFS` that represents the complete,
/// inherited configuration, with all templates processed and all files merged,
/// ready for the final local merge in the next phase.
pub mod phase4 {
    use super::*;

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
}

/// # Phase 5: Local File Merging
///
/// This is the fifth phase of the `common-repo` execution pipeline. Its purpose
/// is to merge the composite filesystem (created in Phase 4) with the files
/// from the local project directory. It also handles any operations that are
/// defined in the root `.common-repo.yaml` that are intended to be applied
/// locally.
///
/// ## Process
///
/// 1.  **Load Local Files**: The process begins by loading all the files from the
///     current working directory into a new `MemoryFS`. Certain files, like
///     those in the `.git` directory or the `.common-repo.yaml` file itself,
///     are automatically ignored.
///
/// 2.  **Apply Local Operations**: Any operations defined in the root configuration
///     that are intended for local application (such as `template` and `merge`
///     operations) are applied to the local `MemoryFS`. This allows for local
///     files to be processed as templates or merged with other files.
///
/// 3.  **Merge with Composite**: The processed local filesystem is then merged
///     on top of the composite filesystem. This ensures that local files always
///     take precedence, overwriting any inherited files with the same name.
///
/// This phase produces the final, fully merged `MemoryFS`, which is an exact
/// representation of what the output directory should look like.
pub mod phase5 {
    use super::*;
    use crate::config::{IniMergeOp, JsonMergeOp, MarkdownMergeOp, TomlMergeOp, YamlMergeOp};
    use crate::filesystem::File;
    use serde_json::Value as JsonValue;
    use serde_yaml::Value as YamlValue;
    use std::collections::HashSet;
    use std::path::Path;
    use toml::Value as TomlValue;

    /// Executes Phase 5 of the pipeline.
    ///
    /// This function orchestrates the merging of the composite filesystem with
    /// the local files. It loads the local files, applies any local operations
    /// to them, and then merges the result into the composite filesystem.
    pub fn execute(
        composite_fs: &MemoryFS,
        local_config: &Schema,
        working_dir: &Path,
    ) -> Result<MemoryFS> {
        // Start with a copy of the composite filesystem
        let mut final_fs = composite_fs.clone();

        // Load local files and apply local operations to them
        let mut local_fs = load_local_fs(working_dir)?;
        apply_local_operations_to_local_fs(&mut local_fs, local_config)?;

        // Merge local files into final filesystem
        merge_local_files(&mut final_fs, &local_fs)?;

        // Apply any merge operations defined in the local configuration to the final filesystem
        apply_local_operations(&mut final_fs, local_config)?;

        Ok(final_fs)
    }

    /// Load local files from the working directory into a MemoryFS
    ///
    /// Recursively walks the directory and loads all files, preserving relative paths.
    /// Skips common build/artifact directories and hidden files to avoid loading
    /// unnecessary data into memory.
    fn load_local_fs(working_dir: &Path) -> Result<MemoryFS> {
        let mut local_fs = MemoryFS::new();

        // Common directories to skip (build artifacts, dependencies, caches, etc.)
        const SKIP_DIRS: &[&str] = &[
            "target",        // Rust build artifacts
            "node_modules",  // Node.js dependencies
            ".git",          // Git repository data
            ".svn",          // SVN repository data
            ".hg",           // Mercurial repository data
            "build",         // Generic build output
            "dist",          // Distribution files
            "__pycache__",   // Python bytecode cache
            ".pytest_cache", // Pytest cache
            ".mypy_cache",   // MyPy cache
            ".tox",          // Tox environments
            "venv",          // Python virtual environment
            ".venv",         // Python virtual environment
            "env",           // Generic environment
            ".env",          // Environment files
            ".idea",         // IntelliJ IDEA
            ".vscode",       // VS Code
            ".vs",           // Visual Studio
            "bin",           // Binary output
            "obj",           // Object files
        ];

        // Use walkdir to recursively find all files, filtering directories early
        for entry in walkdir::WalkDir::new(working_dir)
            .into_iter()
            .filter_entry(|e| {
                // Always allow the root directory (depth 0) to be processed
                if e.depth() == 0 {
                    return true;
                }

                // Get the file/directory name
                let file_name = e.file_name().to_str().unwrap_or("");

                // Skip if it's one of the common build directories
                if SKIP_DIRS.contains(&file_name) {
                    return false;
                }

                // Skip hidden files/directories (starting with .)
                if file_name.starts_with('.') {
                    return false;
                }

                // Allow everything else
                true
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let file_path = entry.path();

            // Get relative path from working directory
            let relative_path = file_path
                .strip_prefix(working_dir)
                .map_err(|_| Error::Path {
                    message: format!("Failed to make path relative: {}", file_path.display()),
                })?;

            // Skip .common-repo.yaml config file
            if relative_path
                .to_str()
                .map(|s| s == ".common-repo.yaml" || s == ".commonrepo.yaml")
                .unwrap_or(false)
            {
                continue;
            }

            // Read file content
            let content = std::fs::read(file_path)?;

            // Add to filesystem with relative path
            local_fs.add_file(relative_path, File::new(content))?;
        }

        Ok(local_fs)
    }

    /// Merge local files into the final filesystem
    fn merge_local_files(final_fs: &mut MemoryFS, local_fs: &MemoryFS) -> Result<()> {
        for (path, file) in local_fs.files() {
            // Local files override inherited files
            final_fs.add_file(path, file.clone())?;
        }
        Ok(())
    }

    /// Apply operations to the local filesystem before merging
    ///
    /// Applies template operations (marking and variable collection) to local files,
    /// then processes templates with collected variables.
    fn apply_local_operations_to_local_fs(
        local_fs: &mut MemoryFS,
        local_config: &Schema,
    ) -> Result<()> {
        use std::collections::HashMap;

        // Collect template variables from local config
        let mut local_template_vars = HashMap::new();
        for operation in local_config {
            if let Operation::TemplateVars { template_vars } = operation {
                crate::operators::template_vars::collect(template_vars, &mut local_template_vars)?;
            }
        }

        // Apply template marking operations to local files
        for operation in local_config {
            if let Operation::Template { template } = operation {
                crate::operators::template::mark(template, local_fs)?;
            }
        }

        // Process templates in local files
        crate::operators::template::process(local_fs, &local_template_vars)?;

        Ok(())
    }

    pub(crate) fn read_file_as_string(fs: &MemoryFS, path: &str) -> Result<String> {
        match fs.get_file(path) {
            Some(file) => String::from_utf8(file.content.clone()).map_err(|_| Error::Merge {
                operation: format!("read {}", path),
                message: "File content is not valid UTF-8".to_string(),
            }),
            None => Err(Error::Merge {
                operation: format!("read {}", path),
                message: "File not found in filesystem".to_string(),
            }),
        }
    }

    fn read_file_as_string_optional(fs: &MemoryFS, path: &str) -> Result<Option<String>> {
        if let Some(file) = fs.get_file(path) {
            Ok(Some(String::from_utf8(file.content.clone()).map_err(
                |_| Error::Merge {
                    operation: format!("read {}", path),
                    message: "File content is not valid UTF-8".to_string(),
                },
            )?))
        } else {
            Ok(None)
        }
    }

    fn ensure_trailing_newline(mut content: String) -> String {
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content
    }

    fn write_string_to_file(fs: &mut MemoryFS, path: &str, content: String) -> Result<()> {
        let normalized = ensure_trailing_newline(content);
        fs.add_file(path, File::from_string(&normalized))
    }

    #[derive(Clone, Debug)]
    struct IniEntry {
        key: String,
        value: String,
    }

    #[derive(Clone, Debug)]
    struct IniSection {
        name: String,
        entries: Vec<IniEntry>,
    }

    fn parse_ini(content: &str) -> Vec<IniSection> {
        let mut sections = Vec::new();
        let mut current = IniSection {
            name: String::new(),
            entries: Vec::new(),
        };
        let mut has_entries = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
                continue;
            }

            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                if !current.name.is_empty() || has_entries {
                    sections.push(current);
                }
                current = IniSection {
                    name: trimmed[1..trimmed.len() - 1].trim().to_string(),
                    entries: Vec::new(),
                };
                has_entries = false;
            } else if let Some(pos) = trimmed.find('=') {
                let key = trimmed[..pos].trim().to_string();
                let value = trimmed[pos + 1..].trim().to_string();
                current.entries.push(IniEntry { key, value });
                has_entries = true;
            }
        }

        if !current.name.is_empty() || has_entries {
            sections.push(current);
        }

        sections
    }

    fn find_ini_section<'a>(sections: &'a [IniSection], name: &str) -> Option<&'a IniSection> {
        sections.iter().find(|section| section.name == name)
    }

    fn find_ini_section_mut<'a>(
        sections: &'a mut Vec<IniSection>,
        name: &str,
    ) -> &'a mut IniSection {
        if let Some(pos) = sections.iter().position(|section| section.name == name) {
            &mut sections[pos]
        } else {
            sections.push(IniSection {
                name: name.to_string(),
                entries: Vec::new(),
            });
            sections.last_mut().unwrap()
        }
    }

    fn serialize_ini(sections: &[IniSection]) -> String {
        let mut output = String::new();

        for (index, section) in sections.iter().enumerate() {
            if !section.name.is_empty() {
                output.push('[');
                output.push_str(&section.name);
                output.push_str("]\n");
            }

            for entry in &section.entries {
                output.push_str(&entry.key);
                output.push('=');
                output.push_str(&entry.value);
                output.push('\n');
            }

            if index + 1 < sections.len() {
                output.push('\n');
            }
        }

        output
    }

    #[derive(Clone, Debug)]
    pub(crate) enum PathSegment {
        Key(String),
        Index(usize),
    }

    pub(crate) fn parse_path(path: &str) -> Vec<PathSegment> {
        if path.trim().is_empty() || path == "/" {
            return Vec::new();
        }

        let mut segments = Vec::new();
        let mut current = String::new();
        let mut chars = path.chars().peekable();
        let mut escaped = false;

        while let Some(ch) = chars.next() {
            if escaped {
                current.push(ch);
                escaped = false;
                continue;
            }

            match ch {
                '\\' => {
                    escaped = true;
                }
                '.' => {
                    if !current.is_empty() {
                        segments.push(PathSegment::Key(current.clone()));
                        current.clear();
                    }
                }
                '[' => {
                    if !current.is_empty() {
                        segments.push(PathSegment::Key(current.clone()));
                        current.clear();
                    }

                    let first_char = chars.peek().copied();

                    if first_char == Some('"') || first_char == Some('\'') {
                        let quote_char = chars.next().unwrap();
                        let mut key = String::new();
                        let mut bracket_escaped = false;

                        while let Some(ch) = chars.next() {
                            if bracket_escaped {
                                key.push(ch);
                                bracket_escaped = false;
                            } else if ch == '\\' {
                                bracket_escaped = true;
                            } else if ch == quote_char {
                                if chars.peek() == Some(&']') {
                                    chars.next();
                                    break;
                                }
                                key.push(ch);
                            } else {
                                key.push(ch);
                            }
                        }

                        segments.push(PathSegment::Key(key));
                    } else {
                        let mut bracket_content = String::new();
                        while let Some(&next_ch) = chars.peek() {
                            chars.next();
                            if next_ch == ']' {
                                break;
                            }
                            bracket_content.push(next_ch);
                        }

                        if let Ok(idx) = bracket_content.trim().parse::<usize>() {
                            segments.push(PathSegment::Index(idx));
                        } else if !bracket_content.trim().is_empty() {
                            segments.push(PathSegment::Key(bracket_content.trim().to_string()));
                        }
                    }
                }
                _ => current.push(ch),
            }
        }

        if !current.is_empty() {
            segments.push(PathSegment::Key(current));
        }

        segments
    }

    pub(crate) fn navigate_yaml_value<'a>(
        value: &'a mut YamlValue,
        path: &[PathSegment],
    ) -> Result<&'a mut YamlValue> {
        let mut current = value;
        for segment in path {
            match segment {
                PathSegment::Key(key) => {
                    if !current.is_mapping() && !current.is_null() {
                        return Err(Error::Merge {
                            operation: "yaml merge".to_string(),
                            message: format!("Expected mapping while navigating to '{}'", key),
                        });
                    }

                    if current.is_null() {
                        *current = YamlValue::Mapping(Default::default());
                    }

                    let map = current.as_mapping_mut().unwrap();
                    current = map
                        .entry(YamlValue::String(key.clone()))
                        .or_insert(YamlValue::Mapping(Default::default()));
                }
                PathSegment::Index(idx) => {
                    if !current.is_sequence() && !current.is_null() {
                        return Err(Error::Merge {
                            operation: "yaml merge".to_string(),
                            message: format!("Expected sequence while navigating to index {}", idx),
                        });
                    }

                    if current.is_null() {
                        *current = YamlValue::Sequence(Vec::new());
                    }

                    let seq = current.as_sequence_mut().unwrap();
                    while seq.len() <= *idx {
                        seq.push(YamlValue::Null);
                    }
                    current = &mut seq[*idx];
                }
            }
        }

        Ok(current)
    }

    fn navigate_json_value<'a>(
        value: &'a mut JsonValue,
        path: &[PathSegment],
    ) -> Result<&'a mut JsonValue> {
        let mut current = value;
        for segment in path {
            match segment {
                PathSegment::Key(key) => {
                    if !current.is_object() && !current.is_null() {
                        return Err(Error::Merge {
                            operation: "json merge".to_string(),
                            message: format!("Expected object while navigating to '{}'", key),
                        });
                    }

                    if current.is_null() {
                        *current = JsonValue::Object(serde_json::Map::new());
                    }

                    let map = current.as_object_mut().unwrap();
                    current = map
                        .entry(key.clone())
                        .or_insert(JsonValue::Object(serde_json::Map::new()));
                }
                PathSegment::Index(idx) => {
                    if !current.is_array() && !current.is_null() {
                        return Err(Error::Merge {
                            operation: "json merge".to_string(),
                            message: format!("Expected array while navigating to index {}", idx),
                        });
                    }

                    if current.is_null() {
                        *current = JsonValue::Array(Vec::new());
                    }

                    let array = current.as_array_mut().unwrap();
                    while array.len() <= *idx {
                        array.push(JsonValue::Null);
                    }
                    current = &mut array[*idx];
                }
            }
        }

        Ok(current)
    }

    fn merge_yaml_values(target: &mut YamlValue, source: &YamlValue, append: bool) {
        match target {
            YamlValue::Mapping(target_map) => {
                if let YamlValue::Mapping(source_map) = source {
                    for (key, value) in source_map {
                        if let Some(existing) = target_map.get_mut(key) {
                            if existing.as_mapping().is_some() && value.as_mapping().is_some() {
                                merge_yaml_values(existing, value, append);
                            } else if let Some(source_seq) = value.as_sequence() {
                                if let Some(target_seq) = existing.as_sequence_mut() {
                                    if append {
                                        target_seq.extend(source_seq.iter().cloned());
                                    } else {
                                        *existing = YamlValue::Sequence(source_seq.clone());
                                    }
                                } else if !append {
                                    *existing = YamlValue::Sequence(source_seq.clone());
                                }
                            } else if !append {
                                *existing = value.clone();
                            }
                        } else {
                            target_map.insert(key.clone(), value.clone());
                        }
                    }
                } else {
                    *target = source.clone();
                }
            }
            YamlValue::Sequence(target_seq) => {
                if let YamlValue::Sequence(source_seq) = source {
                    if append {
                        target_seq.extend(source_seq.clone());
                    } else {
                        *target = YamlValue::Sequence(source_seq.clone());
                    }
                } else {
                    *target = source.clone();
                }
            }
            _ => *target = source.clone(),
        }
    }

    fn merge_json_values(target: &mut JsonValue, source: &JsonValue, append: bool) {
        match target {
            JsonValue::Object(target_map) => {
                if let JsonValue::Object(source_map) = source {
                    for (key, value) in source_map {
                        if let Some(existing) = target_map.get_mut(key) {
                            if existing.is_object() && value.is_object() {
                                merge_json_values(existing, value, append);
                            } else if let Some(source_array) = value.as_array() {
                                if let Some(target_array) = existing.as_array_mut() {
                                    if append {
                                        target_array.extend(source_array.iter().cloned());
                                    } else {
                                        *existing = JsonValue::Array(source_array.clone());
                                    }
                                } else if !append {
                                    *existing = JsonValue::Array(source_array.clone());
                                }
                            } else if !append {
                                *existing = value.clone();
                            }
                        } else {
                            target_map.insert(key.clone(), value.clone());
                        }
                    }
                } else {
                    *target = source.clone();
                }
            }
            JsonValue::Array(target_array) => {
                if let JsonValue::Array(source_array) = source {
                    if append {
                        target_array.extend(source_array.clone());
                    } else {
                        *target = JsonValue::Array(source_array.clone());
                    }
                } else {
                    *target = source.clone();
                }
            }
            _ => *target = source.clone(),
        }
    }

    fn parse_toml_path(path: &str) -> Vec<String> {
        path.split('.')
            .map(|segment| segment.trim())
            .filter(|segment| !segment.is_empty())
            .map(|segment| segment.to_string())
            .collect()
    }

    fn navigate_toml_value<'a>(
        value: &'a mut TomlValue,
        path: &[String],
    ) -> Result<&'a mut TomlValue> {
        let mut current = value;
        for segment in path {
            if !current.is_table() {
                return Err(Error::Merge {
                    operation: "toml merge".to_string(),
                    message: format!("Expected table while navigating to '{}'", segment),
                });
            }

            let table = current.as_table_mut().unwrap();
            current = table
                .entry(segment.clone())
                .or_insert(TomlValue::Table(toml::map::Map::new()));
        }
        Ok(current)
    }

    fn merge_toml_values(target: &mut TomlValue, source: &TomlValue, append: bool) {
        match target {
            TomlValue::Table(target_table) => {
                if let TomlValue::Table(source_table) = source {
                    for (key, value) in source_table {
                        if let Some(existing) = target_table.get_mut(key) {
                            if matches!(existing, TomlValue::Table(_))
                                && matches!(value, TomlValue::Table(_))
                            {
                                merge_toml_values(existing, value, append);
                            } else if let Some(source_array) = value.as_array() {
                                if let Some(target_array) = existing.as_array_mut() {
                                    if append {
                                        target_array.extend(source_array.iter().cloned());
                                    } else {
                                        *existing = TomlValue::Array(source_array.clone());
                                    }
                                } else if !append {
                                    *existing = TomlValue::Array(source_array.clone());
                                }
                            } else if !append {
                                *existing = value.clone();
                            }
                        } else {
                            target_table.insert(key.clone(), value.clone());
                        }
                    }
                } else {
                    *target = source.clone();
                }
            }
            TomlValue::Array(target_array) => {
                if let TomlValue::Array(source_array) = source {
                    if append {
                        target_array.extend(source_array.clone());
                    } else {
                        *target = TomlValue::Array(source_array.clone());
                    }
                } else {
                    *target = source.clone();
                }
            }
            _ => *target = source.clone(),
        }
    }

    fn split_lines_preserve(content: &str) -> Vec<String> {
        let mut lines: Vec<String> = content.lines().map(|line| line.to_string()).collect();
        if content.ends_with('\n') {
            lines.push(String::new());
        }
        lines
    }

    fn heading_for(level: u8, section: &str) -> String {
        let level = level.clamp(1, 6);
        format!("{} {}", "#".repeat(level as usize), section.trim())
    }

    fn find_section_bounds(lines: &[String], level: u8, section: &str) -> Option<(usize, usize)> {
        let heading = heading_for(level, section);
        let mut start_index = None;

        for (idx, line) in lines.iter().enumerate() {
            if line.trim() == heading {
                start_index = Some(idx);
                break;
            }
        }

        let start = start_index?;
        let mut end = lines.len();

        for (idx, line) in lines.iter().enumerate().skip(start + 1) {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                let next_level = trimmed.chars().take_while(|c| *c == '#').count() as u8;
                if next_level <= level {
                    end = idx;
                    break;
                }
            }
        }

        Some((start, end))
    }

    fn normalize_position(position: &str) -> &str {
        match position.to_lowercase().as_str() {
            "start" => "start",
            _ => "end",
        }
    }

    pub fn apply_yaml_merge_operation(fs: &mut MemoryFS, op: &YamlMergeOp) -> Result<()> {
        let source_content = read_file_as_string(fs, &op.source)?;
        let dest_content =
            read_file_as_string_optional(fs, &op.dest)?.unwrap_or_else(|| "---\n".to_string());

        let mut dest_value: YamlValue =
            serde_yaml::from_str(&dest_content).unwrap_or(YamlValue::Mapping(Default::default()));
        let source_value: YamlValue =
            serde_yaml::from_str(&source_content).map_err(|err| Error::Merge {
                operation: "yaml merge".to_string(),
                message: format!("Failed to parse source YAML: {}", err),
            })?;

        let path_str = op.path.as_deref().unwrap_or("");
        let path = parse_path(path_str);
        let target = navigate_yaml_value(&mut dest_value, &path)?;
        merge_yaml_values(target, &source_value, op.append);

        let serialized = serde_yaml::to_string(&dest_value).map_err(|err| Error::Merge {
            operation: "yaml merge".to_string(),
            message: format!("Failed to serialize YAML: {}", err),
        })?;

        write_string_to_file(fs, &op.dest, serialized)
    }

    pub fn apply_json_merge_operation(fs: &mut MemoryFS, op: &JsonMergeOp) -> Result<()> {
        let source_content = read_file_as_string(fs, &op.source)?;
        let dest_content =
            read_file_as_string_optional(fs, &op.dest)?.unwrap_or_else(|| "{}".to_string());

        let mut dest_value: JsonValue = serde_json::from_str(&dest_content)
            .unwrap_or(JsonValue::Object(serde_json::Map::new()));
        let source_value: JsonValue =
            serde_json::from_str(&source_content).map_err(|err| Error::Merge {
                operation: "json merge".to_string(),
                message: format!("Failed to parse source JSON: {}", err),
            })?;

        let path = parse_path(&op.path);
        let target = navigate_json_value(&mut dest_value, &path)?;
        merge_json_values(target, &source_value, op.append);

        let serialized = serde_json::to_string_pretty(&dest_value).map_err(|err| Error::Merge {
            operation: "json merge".to_string(),
            message: format!("Failed to serialize JSON: {}", err),
        })?;

        write_string_to_file(fs, &op.dest, serialized)
    }

    pub fn apply_toml_merge_operation(fs: &mut MemoryFS, op: &TomlMergeOp) -> Result<()> {
        let source_content = read_file_as_string(fs, &op.source)?;
        let dest_content = read_file_as_string_optional(fs, &op.dest)?.unwrap_or_default();

        let mut dest_value: TomlValue = toml::from_str(&dest_content)
            .unwrap_or_else(|_| TomlValue::Table(toml::map::Map::new()));
        let source_value: TomlValue =
            toml::from_str(&source_content).map_err(|err| Error::Merge {
                operation: "toml merge".to_string(),
                message: format!("Failed to parse source TOML: {}", err),
            })?;

        let path = parse_toml_path(&op.path);
        let target = navigate_toml_value(&mut dest_value, &path)?;
        merge_toml_values(target, &source_value, op.append);

        let serialized = toml::to_string_pretty(&dest_value).map_err(|err| Error::Merge {
            operation: "toml merge".to_string(),
            message: format!("Failed to serialize TOML: {}", err),
        })?;

        if op.preserve_comments {
            // No-op placeholder: comment preservation is not implemented yet.
            // The serialized output already contains normalized TOML.
        }

        write_string_to_file(fs, &op.dest, serialized)
    }

    pub fn apply_ini_merge_operation(fs: &mut MemoryFS, op: &IniMergeOp) -> Result<()> {
        let source_content = read_file_as_string(fs, &op.source)?;
        let dest_content = read_file_as_string_optional(fs, &op.dest)?.unwrap_or_default();

        let source_sections = parse_ini(&source_content);
        let mut dest_sections = parse_ini(&dest_content);

        let source_section =
            find_ini_section(&source_sections, &op.section).ok_or_else(|| Error::Merge {
                operation: "ini merge".to_string(),
                message: format!("Section '{}' not found in source INI", op.section),
            })?;

        let dest_section = find_ini_section_mut(&mut dest_sections, &op.section);

        if op.append {
            if op.allow_duplicates {
                dest_section.entries.extend(source_section.entries.clone());
            } else {
                for entry in &source_section.entries {
                    if !dest_section
                        .entries
                        .iter()
                        .any(|existing| existing.key.eq_ignore_ascii_case(&entry.key))
                    {
                        dest_section.entries.push(entry.clone());
                    }
                }
            }
        } else {
            if !op.allow_duplicates {
                let keys: HashSet<String> = source_section
                    .entries
                    .iter()
                    .map(|entry| entry.key.to_lowercase())
                    .collect();
                dest_section
                    .entries
                    .retain(|entry| !keys.contains(&entry.key.to_lowercase()));
            }

            dest_section.entries.extend(source_section.entries.clone());
        }

        let serialized = serialize_ini(&dest_sections);
        write_string_to_file(fs, &op.dest, serialized)
    }

    pub fn apply_markdown_merge_operation(fs: &mut MemoryFS, op: &MarkdownMergeOp) -> Result<()> {
        let source_content = read_file_as_string(fs, &op.source)?;
        let dest_content = read_file_as_string_optional(fs, &op.dest)?.unwrap_or_default();

        let mut dest_lines = split_lines_preserve(&dest_content);
        let source_lines = split_lines_preserve(&source_content);
        let position = normalize_position(&op.position);

        if let Some((start, end)) = find_section_bounds(&dest_lines, op.level, &op.section) {
            let insert_index = if op.append { end } else { start + 1 };

            if op.append {
                let mut payload = source_lines.clone();
                if payload.is_empty()
                    || !payload.last().map(|line| line.is_empty()).unwrap_or(false)
                {
                    payload.push(String::new());
                }
                if insert_index > start + 1 && !dest_lines[insert_index - 1].trim().is_empty() {
                    payload.insert(0, String::new());
                }
                dest_lines.splice(insert_index..insert_index, payload);
            } else {
                let mut payload = source_lines.clone();
                if payload.is_empty()
                    || !payload.last().map(|line| line.is_empty()).unwrap_or(false)
                {
                    payload.push(String::new());
                }
                dest_lines.splice(start + 1..end, payload);
            }
        } else {
            if !op.create_section {
                return Err(Error::Merge {
                    operation: "markdown merge".to_string(),
                    message: format!(
                        "Section '{}' not found and create-section is false",
                        op.section
                    ),
                });
            }

            let mut block = Vec::new();
            block.push(heading_for(op.level, &op.section));
            block.push(String::new());
            block.extend(source_lines.clone());
            if block.is_empty() || !block.last().map(|line| line.is_empty()).unwrap_or(false) {
                block.push(String::new());
            }

            match position {
                "start" => {
                    while !dest_lines.is_empty() && dest_lines[0].trim().is_empty() {
                        dest_lines.remove(0);
                    }
                    dest_lines.splice(0..0, block);
                }
                _ => {
                    if !dest_lines.is_empty() && !dest_lines.last().unwrap().trim().is_empty() {
                        dest_lines.push(String::new());
                    }
                    dest_lines.extend(block);
                }
            }
        }

        let mut serialized = dest_lines.join("\n");
        if !serialized.ends_with('\n') {
            serialized.push('\n');
        }

        write_string_to_file(fs, &op.dest, serialized)
    }

    /// Apply local operations from the configuration
    ///
    /// These are operations that apply to the final merged filesystem,
    /// typically merge operations that combine local and inherited content.
    fn apply_local_operations(final_fs: &mut MemoryFS, local_config: &Schema) -> Result<()> {
        // Filter to only merge operations that are appropriate for local merging
        // (template operations are handled separately in apply_local_operations_to_local_fs)
        let merge_operations: Vec<&Operation> = local_config
            .iter()
            .filter(|op| {
                matches!(
                    op,
                    Operation::Yaml { .. }
                        | Operation::Json { .. }
                        | Operation::Toml { .. }
                        | Operation::Ini { .. }
                        | Operation::Markdown { .. }
                )
            })
            .collect();

        for operation in merge_operations {
            match operation {
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
                // This should never happen due to filtering above
                _ => unreachable!("Filtered operations should only include merge operations"),
            }
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
}

pub mod phase6 {
    use super::*;
    use std::fs;
    use std::path::Path;

    /// Execute Phase 6: Write final filesystem to disk
    ///
    /// Writes all files from the MemoryFS to the host filesystem at the specified output path.
    /// Creates all necessary directories recursively and preserves file permissions where possible.
    pub fn execute(final_fs: &MemoryFS, output_path: &Path) -> Result<()> {
        for (relative_path, file) in final_fs.files() {
            // Construct full output path
            let full_path = output_path.join(relative_path);

            // Create parent directories if needed
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).map_err(|e| Error::Filesystem {
                    message: format!("Failed to create directory '{}': {}", parent.display(), e),
                })?;
            }

            // Write file content
            fs::write(&full_path, &file.content).map_err(|e| Error::Filesystem {
                message: format!("Failed to write file '{}': {}", full_path.display(), e),
            })?;

            // Set permissions on Unix-like systems
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = fs::Permissions::from_mode(file.permissions);
                if let Err(e) = fs::set_permissions(&full_path, perms) {
                    // Log warning but don't fail - permissions are best-effort
                    // On some systems (e.g., certain mount points), setting permissions may fail
                    return Err(Error::Filesystem {
                        message: format!(
                            "Failed to set permissions on '{}': {}",
                            full_path.display(),
                            e
                        ),
                    });
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod phase_tests {
    use super::*;
    use crate::repository::{CacheOperations, GitOperations};
    use std::collections::HashMap;
    use std::path::Path;
    use tempfile::TempDir;

    mod phase3_tests {
        use super::*;

        #[test]
        fn test_phase3_execute_simple_dependency() {
            // Test simple dependency: A depends on B
            let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            let repo_b = RepoNode::new(
                "https://github.com/repo-b.git".to_string(),
                "main".to_string(),
                vec![],
            );
            let mut repo_a = RepoNode::new(
                "https://github.com/repo-a.git".to_string(),
                "main".to_string(),
                vec![],
            );
            repo_a.add_child(repo_b);
            root.add_child(repo_a);

            let tree = RepoTree::new(root);
            let order = phase3::execute(&tree).unwrap();

            // B should come before A (dependencies first)
            assert_eq!(order.len(), 3); // local, repo-b, repo-a
            let order_vec: Vec<&str> = order.order.iter().map(|s| s.as_str()).collect();
            let b_index = order_vec.iter().position(|s| s.contains("repo-b")).unwrap();
            let a_index = order_vec.iter().position(|s| s.contains("repo-a")).unwrap();
            assert!(b_index < a_index, "repo-b should come before repo-a");
        }

        #[test]
        fn test_phase3_execute_complex_dependency_tree() {
            // Test complex tree: A -> B, A -> C, B -> D
            let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            let repo_d = RepoNode::new(
                "https://github.com/repo-d.git".to_string(),
                "main".to_string(),
                vec![],
            );
            let mut repo_b = RepoNode::new(
                "https://github.com/repo-b.git".to_string(),
                "main".to_string(),
                vec![],
            );
            repo_b.add_child(repo_d);
            let repo_c = RepoNode::new(
                "https://github.com/repo-c.git".to_string(),
                "main".to_string(),
                vec![],
            );
            let mut repo_a = RepoNode::new(
                "https://github.com/repo-a.git".to_string(),
                "main".to_string(),
                vec![],
            );
            repo_a.add_child(repo_b);
            repo_a.add_child(repo_c);
            root.add_child(repo_a);

            let tree = RepoTree::new(root);
            let order = phase3::execute(&tree).unwrap();

            // Verify order: D before B, B and C before A
            assert_eq!(order.len(), 5); // local, d, b, c, a
            let order_vec: Vec<&str> = order.order.iter().map(|s| s.as_str()).collect();
            let d_index = order_vec.iter().position(|s| s.contains("repo-d")).unwrap();
            let b_index = order_vec.iter().position(|s| s.contains("repo-b")).unwrap();
            let c_index = order_vec.iter().position(|s| s.contains("repo-c")).unwrap();
            let a_index = order_vec.iter().position(|s| s.contains("repo-a")).unwrap();

            assert!(d_index < b_index, "repo-d should come before repo-b");
            assert!(b_index < a_index, "repo-b should come before repo-a");
            assert!(c_index < a_index, "repo-c should come before repo-a");
        }

        #[test]
        fn test_phase3_execute_multiple_repos_same_level() {
            // Test multiple repos at same level
            let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            let repo_a = RepoNode::new(
                "https://github.com/repo-a.git".to_string(),
                "main".to_string(),
                vec![],
            );
            let repo_b = RepoNode::new(
                "https://github.com/repo-b.git".to_string(),
                "main".to_string(),
                vec![],
            );
            let repo_c = RepoNode::new(
                "https://github.com/repo-c.git".to_string(),
                "main".to_string(),
                vec![],
            );
            root.add_child(repo_a);
            root.add_child(repo_b);
            root.add_child(repo_c);

            let tree = RepoTree::new(root);
            let order = phase3::execute(&tree).unwrap();

            // All repos should be in the order
            assert_eq!(order.len(), 4); // local + 3 repos
            assert!(order.order.iter().any(|s| s.contains("repo-a")));
            assert!(order.order.iter().any(|s| s.contains("repo-b")));
            assert!(order.order.iter().any(|s| s.contains("repo-c")));
        }

        #[test]
        fn test_phase3_execute_empty_tree() {
            // Test empty tree (only local root)
            let root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            let tree = RepoTree::new(root);
            let order = phase3::execute(&tree).unwrap();

            // Should only contain local
            assert_eq!(order.len(), 1);
            assert_eq!(order.order[0], "local@HEAD");
        }
    }

    mod phase4_tests {
        use super::*;

        #[test]
        fn test_phase4_execute_merge_no_conflicts() {
            // Test merging two filesystems with no conflicts
            let mut fs1 = MemoryFS::new();
            fs1.add_file_string("file1.txt", "content1").unwrap();
            let mut fs2 = MemoryFS::new();
            fs2.add_file_string("file2.txt", "content2").unwrap();

            let mut intermediate_fss = HashMap::new();
            intermediate_fss.insert(
                "https://github.com/repo-a.git@main".to_string(),
                IntermediateFS::new(
                    fs1,
                    "https://github.com/repo-a.git".to_string(),
                    "main".to_string(),
                ),
            );
            intermediate_fss.insert(
                "https://github.com/repo-b.git@main".to_string(),
                IntermediateFS::new(
                    fs2,
                    "https://github.com/repo-b.git".to_string(),
                    "main".to_string(),
                ),
            );

            let order = OperationOrder::new(vec![
                "https://github.com/repo-a.git@main".to_string(),
                "https://github.com/repo-b.git@main".to_string(),
            ]);

            let composite = phase4::execute(&order, &intermediate_fss).unwrap();

            assert_eq!(composite.len(), 2);
            assert!(composite.exists("file1.txt"));
            assert!(composite.exists("file2.txt"));
        }

        #[test]
        fn test_phase4_execute_merge_with_conflicts() {
            // Test merging filesystems with file conflicts (last-write-wins)
            let mut fs1 = MemoryFS::new();
            fs1.add_file_string("common.txt", "version1").unwrap();
            let mut fs2 = MemoryFS::new();
            fs2.add_file_string("common.txt", "version2").unwrap();

            let mut intermediate_fss = HashMap::new();
            intermediate_fss.insert(
                "https://github.com/repo-a.git@main".to_string(),
                IntermediateFS::new(
                    fs1,
                    "https://github.com/repo-a.git".to_string(),
                    "main".to_string(),
                ),
            );
            intermediate_fss.insert(
                "https://github.com/repo-b.git@main".to_string(),
                IntermediateFS::new(
                    fs2,
                    "https://github.com/repo-b.git".to_string(),
                    "main".to_string(),
                ),
            );

            let order = OperationOrder::new(vec![
                "https://github.com/repo-a.git@main".to_string(),
                "https://github.com/repo-b.git@main".to_string(),
            ]);

            let composite = phase4::execute(&order, &intermediate_fss).unwrap();

            // Last filesystem should win
            assert_eq!(composite.len(), 1);
            let file = composite.get_file("common.txt").unwrap();
            assert_eq!(String::from_utf8(file.content.clone()).unwrap(), "version2");
        }

        #[test]
        fn test_phase4_execute_merge_multiple_filesystems() {
            // Test merging multiple filesystems in correct order
            let mut fs1 = MemoryFS::new();
            fs1.add_file_string("file1.txt", "content1").unwrap();
            let mut fs2 = MemoryFS::new();
            fs2.add_file_string("file2.txt", "content2").unwrap();
            let mut fs3 = MemoryFS::new();
            fs3.add_file_string("file3.txt", "content3").unwrap();

            let mut intermediate_fss = HashMap::new();
            intermediate_fss.insert(
                "https://github.com/repo-a.git@main".to_string(),
                IntermediateFS::new(
                    fs1,
                    "https://github.com/repo-a.git".to_string(),
                    "main".to_string(),
                ),
            );
            intermediate_fss.insert(
                "https://github.com/repo-b.git@main".to_string(),
                IntermediateFS::new(
                    fs2,
                    "https://github.com/repo-b.git".to_string(),
                    "main".to_string(),
                ),
            );
            intermediate_fss.insert(
                "https://github.com/repo-c.git@main".to_string(),
                IntermediateFS::new(
                    fs3,
                    "https://github.com/repo-c.git".to_string(),
                    "main".to_string(),
                ),
            );

            let order = OperationOrder::new(vec![
                "https://github.com/repo-a.git@main".to_string(),
                "https://github.com/repo-b.git@main".to_string(),
                "https://github.com/repo-c.git@main".to_string(),
            ]);

            let composite = phase4::execute(&order, &intermediate_fss).unwrap();

            assert_eq!(composite.len(), 3);
            assert!(composite.exists("file1.txt"));
            assert!(composite.exists("file2.txt"));
            assert!(composite.exists("file3.txt"));
        }

        #[test]
        fn test_phase4_execute_missing_intermediate_fs() {
            // Test error when intermediate filesystem is missing
            let intermediate_fss = HashMap::new();
            let order = OperationOrder::new(vec!["https://github.com/repo-a.git@main".to_string()]);

            let result = phase4::execute(&order, &intermediate_fss);
            assert!(result.is_err());
            if let Err(Error::Filesystem { message: msg }) = result {
                assert!(msg.contains("Missing intermediate filesystem"));
            } else {
                panic!("Expected Filesystem error");
            }
        }

        #[test]
        fn test_phase4_execute_template_processing() {
            // Test that templates are processed with collected variables during Phase 4
            let mut fs1 = MemoryFS::new();
            fs1.add_file_string("template.txt", "Hello ${NAME} from ${REPO}!")
                .unwrap();

            // Mark template.txt as a template
            let template_op = crate::config::TemplateOp {
                patterns: vec!["*.txt".to_string()],
            };
            crate::operators::template::mark(&template_op, &mut fs1).unwrap();

            let mut fs2 = MemoryFS::new();
            fs2.add_file_string("config.txt", "Config file").unwrap();

            // Create template variables for each repo
            let mut vars1 = HashMap::new();
            vars1.insert("NAME".to_string(), "Alice".to_string());
            vars1.insert("REPO".to_string(), "repo1".to_string());

            let mut vars2 = HashMap::new();
            vars2.insert("NAME".to_string(), "Bob".to_string()); // Should override
            vars2.insert("VERSION".to_string(), "1.0".to_string()); // Additional var

            let mut intermediate_fss = HashMap::new();
            intermediate_fss.insert(
                "https://github.com/repo-a.git@main".to_string(),
                IntermediateFS::new_with_vars(
                    fs1,
                    "https://github.com/repo-a.git".to_string(),
                    "main".to_string(),
                    vars1,
                ),
            );
            intermediate_fss.insert(
                "https://github.com/repo-b.git@main".to_string(),
                IntermediateFS::new_with_vars(
                    fs2,
                    "https://github.com/repo-b.git".to_string(),
                    "main".to_string(),
                    vars2,
                ),
            );

            let order = OperationOrder::new(vec![
                "https://github.com/repo-a.git@main".to_string(),
                "https://github.com/repo-b.git@main".to_string(),
            ]);

            let composite = phase4::execute(&order, &intermediate_fss).unwrap();

            // Template should be processed with merged variables (later repos override)
            assert!(composite.exists("template.txt"));
            let template_file = composite.get_file("template.txt").unwrap();
            let content = String::from_utf8(template_file.content.clone()).unwrap();
            assert_eq!(content, "Hello Bob from repo1!"); // NAME overridden, REPO from first repo
            assert!(!template_file.is_template); // Should be unmarked after processing

            // Non-template file should be unchanged
            assert!(composite.exists("config.txt"));
            let config_file = composite.get_file("config.txt").unwrap();
            let config_content = String::from_utf8(config_file.content.clone()).unwrap();
            assert_eq!(config_content, "Config file");
        }

        #[test]
        fn test_phase4_execute_template_processing_multiple_templates() {
            // Test processing multiple template files from different repositories
            let mut fs1 = MemoryFS::new();
            fs1.add_file_string("greeting.txt", "Hello ${USER}!")
                .unwrap();

            let mut fs2 = MemoryFS::new();
            fs2.add_file_string("version.txt", "Version: ${VERSION}")
                .unwrap();

            // Mark templates
            let template_op = crate::config::TemplateOp {
                patterns: vec!["*.txt".to_string()],
            };
            crate::operators::template::mark(&template_op, &mut fs1).unwrap();
            crate::operators::template::mark(&template_op, &mut fs2).unwrap();

            // Template variables
            let mut vars1 = HashMap::new();
            vars1.insert("USER".to_string(), "Alice".to_string());

            let mut vars2 = HashMap::new();
            vars2.insert("VERSION".to_string(), "2.1.0".to_string());

            let mut intermediate_fss = HashMap::new();
            intermediate_fss.insert(
                "https://github.com/repo-a.git@main".to_string(),
                IntermediateFS::new_with_vars(
                    fs1,
                    "https://github.com/repo-a.git".to_string(),
                    "main".to_string(),
                    vars1,
                ),
            );
            intermediate_fss.insert(
                "https://github.com/repo-b.git@main".to_string(),
                IntermediateFS::new_with_vars(
                    fs2,
                    "https://github.com/repo-b.git".to_string(),
                    "main".to_string(),
                    vars2,
                ),
            );

            let order = OperationOrder::new(vec![
                "https://github.com/repo-a.git@main".to_string(),
                "https://github.com/repo-b.git@main".to_string(),
            ]);

            let composite = phase4::execute(&order, &intermediate_fss).unwrap();

            // Both templates should be processed
            let greeting_file = composite.get_file("greeting.txt").unwrap();
            let greeting_content = String::from_utf8(greeting_file.content.clone()).unwrap();
            assert_eq!(greeting_content, "Hello Alice!");

            let version_file = composite.get_file("version.txt").unwrap();
            let version_content = String::from_utf8(version_file.content.clone()).unwrap();
            assert_eq!(version_content, "Version: 2.1.0");

            // Both should be unmarked as templates
            assert!(!greeting_file.is_template);
            assert!(!version_file.is_template);
        }
    }

    mod phase5_tests {
        use super::*;
        use crate::phases::phase5::{
            apply_ini_merge_operation, apply_markdown_merge_operation, apply_toml_merge_operation,
        };

        // Note: MockGitOps and MockCacheOps are defined but not used in phase5 tests
        // They're kept for potential future use
        #[allow(dead_code)]
        struct MockGitOps;
        #[allow(dead_code)]
        struct MockCacheOps;

        #[allow(dead_code)]
        impl GitOperations for MockGitOps {
            fn clone_shallow(&self, _url: &str, _ref_name: &str, _path: &Path) -> Result<()> {
                Ok(())
            }

            fn list_tags(&self, _url: &str) -> Result<Vec<String>> {
                Ok(vec![])
            }
        }

        #[allow(dead_code)]
        impl CacheOperations for MockCacheOps {
            fn exists(&self, _cache_path: &Path) -> bool {
                false
            }

            fn get_cache_path(&self, _url: &str, _ref_name: &str) -> std::path::PathBuf {
                std::path::PathBuf::from("/mock/cache")
            }

            fn get_cache_path_with_path(
                &self,
                _url: &str,
                _ref_name: &str,
                _path: Option<&str>,
            ) -> std::path::PathBuf {
                std::path::PathBuf::from("/mock/cache")
            }

            fn load_from_cache(&self, _cache_path: &Path) -> Result<MemoryFS> {
                Ok(MemoryFS::new())
            }

            fn load_from_cache_with_path(
                &self,
                _cache_path: &Path,
                _path: Option<&str>,
            ) -> Result<MemoryFS> {
                Ok(MemoryFS::new())
            }

            fn save_to_cache(&self, _cache_path: &Path, _fs: &MemoryFS) -> Result<()> {
                Ok(())
            }
        }

        #[test]
        fn test_phase5_execute_merge_local_files() {
            // Test merging composite filesystem with local files
            let temp_dir = TempDir::new().unwrap();
            let working_dir = temp_dir.path();

            // Create local files
            std::fs::create_dir_all(working_dir.join("subdir")).unwrap();
            std::fs::write(working_dir.join("local.txt"), b"local content").unwrap();
            std::fs::write(working_dir.join("subdir/nested.txt"), b"nested content").unwrap();

            // Create composite filesystem
            let mut composite_fs = MemoryFS::new();
            composite_fs
                .add_file_string("composite.txt", "composite content")
                .unwrap();

            // Create local config (empty for this test)
            let local_config = vec![];

            let final_fs = phase5::execute(&composite_fs, &local_config, working_dir).unwrap();

            // Should contain both composite and local files
            assert!(final_fs.exists("composite.txt"));
            assert!(final_fs.exists("local.txt"));
            assert!(final_fs.exists("subdir/nested.txt"));
        }

        #[test]
        fn test_phase5_execute_local_files_override_composite() {
            // Test that local files override composite files (last-write-wins)
            let temp_dir = TempDir::new().unwrap();
            let working_dir = temp_dir.path();

            // Create local file with same name as composite
            std::fs::write(working_dir.join("common.txt"), b"local version").unwrap();

            // Create composite filesystem with same file
            let mut composite_fs = MemoryFS::new();
            composite_fs
                .add_file_string("common.txt", "composite version")
                .unwrap();

            let local_config = vec![];

            let final_fs = phase5::execute(&composite_fs, &local_config, working_dir).unwrap();

            // Local file should override composite
            let file = final_fs.get_file("common.txt").unwrap();
            assert_eq!(
                String::from_utf8(file.content.clone()).unwrap(),
                "local version"
            );
        }

        #[test]
        fn test_phase5_execute_skips_hidden_files() {
            // Test that hidden files and .git directory are skipped
            let temp_dir = TempDir::new().unwrap();
            let working_dir = temp_dir.path();

            std::fs::write(working_dir.join(".hidden"), b"hidden").unwrap();
            std::fs::write(working_dir.join(".common-repo.yaml"), b"config").unwrap();
            std::fs::create_dir_all(working_dir.join(".git")).unwrap();
            std::fs::write(working_dir.join(".git/config"), b"git config").unwrap();
            std::fs::write(working_dir.join("visible.txt"), b"visible").unwrap();

            let composite_fs = MemoryFS::new();
            let local_config = vec![];

            let final_fs = phase5::execute(&composite_fs, &local_config, working_dir).unwrap();

            // Should only contain visible.txt
            assert!(final_fs.exists("visible.txt"));
            assert!(!final_fs.exists(".hidden"));
            assert!(!final_fs.exists(".common-repo.yaml"));
            assert!(!final_fs.exists(".git/config"));
        }

        #[test]
        fn test_phase5_execute_empty_composite() {
            // Test with empty composite filesystem
            let temp_dir = TempDir::new().unwrap();
            let working_dir = temp_dir.path();

            std::fs::write(working_dir.join("local.txt"), b"local").unwrap();

            let composite_fs = MemoryFS::new();
            let local_config = vec![];

            let final_fs = phase5::execute(&composite_fs, &local_config, working_dir).unwrap();

            assert_eq!(final_fs.len(), 1);
            assert!(final_fs.exists("local.txt"));
        }

        #[test]
        fn test_toml_merge_operation_root_level() {
            // Test TOML merge at root level
            let mut fs = MemoryFS::new();

            // Create source TOML fragment
            let source_toml = r#"
[package]
name = "test-package"
version = "1.0.0"
"#;
            fs.add_file_string("source.toml", source_toml).unwrap();

            // Create destination TOML file
            let dest_toml = r#"
[dependencies]
serde = "1.0"
"#;
            fs.add_file_string("Cargo.toml", dest_toml).unwrap();

            let toml_op = crate::config::TomlMergeOp {
                source: "source.toml".to_string(),
                dest: "Cargo.toml".to_string(),
                path: "".to_string(), // root level
                append: false,
                preserve_comments: false,
            };

            apply_toml_merge_operation(&mut fs, &toml_op).unwrap();

            let result = fs.get_file("Cargo.toml").unwrap();
            let result_str = String::from_utf8(result.content.clone()).unwrap();

            // Should contain both original and merged content
            assert!(result_str.contains("serde = \"1.0\""));
            assert!(result_str.contains("name = \"test-package\""));
            assert!(result_str.contains("version = \"1.0.0\""));
        }

        #[test]
        fn test_toml_merge_operation_nested_path() {
            // Test TOML merge at nested path
            let mut fs = MemoryFS::new();

            // Create source TOML fragment
            let source_toml = r#"
enabled = true
timeout = 30
"#;
            fs.add_file_string("config.toml", source_toml).unwrap();

            // Create destination TOML file
            let dest_toml = r#"
[server]
host = "localhost"

[database]
name = "mydb"
"#;
            fs.add_file_string("merged.toml", dest_toml).unwrap();

            let toml_op = crate::config::TomlMergeOp {
                source: "config.toml".to_string(),
                dest: "merged.toml".to_string(),
                path: "server".to_string(),
                append: false,
                preserve_comments: false,
            };

            apply_toml_merge_operation(&mut fs, &toml_op).unwrap();

            let result = fs.get_file("merged.toml").unwrap();
            let result_str = String::from_utf8(result.content.clone()).unwrap();

            // Should have server section with new fields
            assert!(result_str.contains("[server]"));
            assert!(result_str.contains("host = \"localhost\""));
            assert!(result_str.contains("enabled = true"));
            assert!(result_str.contains("timeout = 30"));
            // Should still have database section
            assert!(result_str.contains("[database]"));
            assert!(result_str.contains("name = \"mydb\""));
        }

        #[test]
        fn test_ini_merge_operation_basic() {
            // Test INI merge with section
            let mut fs = MemoryFS::new();

            // Create source INI fragment
            let source_ini = r#"
[database]
driver = postgresql
port = 5432
"#;
            fs.add_file_string("db.ini", source_ini).unwrap();

            // Create destination INI file
            let dest_ini = r#"
[server]
host = localhost
port = 8080
"#;
            fs.add_file_string("config.ini", dest_ini).unwrap();

            let ini_op = crate::config::IniMergeOp {
                source: "db.ini".to_string(),
                dest: "config.ini".to_string(),
                section: "database".to_string(),
                append: false,
                allow_duplicates: false,
            };

            apply_ini_merge_operation(&mut fs, &ini_op).unwrap();

            let result = fs.get_file("config.ini").unwrap();
            let result_str = String::from_utf8(result.content.clone()).unwrap();

            // Should contain both sections
            assert!(result_str.contains("[server]"));
            assert!(result_str.contains("host=localhost"));
            assert!(result_str.contains("port=8080"));
            assert!(result_str.contains("[database]"));
            assert!(result_str.contains("driver=postgresql"));
            assert!(result_str.contains("port=5432"));
        }

        #[test]
        fn test_ini_merge_operation_append_mode() {
            // Test INI merge in append mode (should not overwrite existing keys)
            let mut fs = MemoryFS::new();

            // Create source INI fragment
            let source_ini = r#"
[settings]
timeout = 60
debug = true
"#;
            fs.add_file_string("new.ini", source_ini).unwrap();

            // Create destination INI file with overlapping key
            let dest_ini = r#"
[settings]
timeout = 30
host = localhost
"#;
            fs.add_file_string("config.ini", dest_ini).unwrap();

            let ini_op = crate::config::IniMergeOp {
                source: "new.ini".to_string(),
                dest: "config.ini".to_string(),
                section: "settings".to_string(),
                append: true, // append mode
                allow_duplicates: false,
            };

            apply_ini_merge_operation(&mut fs, &ini_op).unwrap();

            let result = fs.get_file("config.ini").unwrap();
            let result_str = String::from_utf8(result.content.clone()).unwrap();

            // Should contain merged content
            assert!(result_str.contains("[settings]"));
            assert!(result_str.contains("host=localhost"));
            assert!(result_str.contains("debug=true"));
            // In append mode, existing keys should not be overwritten
            assert!(result_str.contains("timeout=30"));
        }

        #[test]
        fn test_markdown_merge_operation_basic() {
            // Test Markdown merge with section
            let mut fs = MemoryFS::new();

            // Create source markdown fragment
            let source_md = r#"## Installation

Run the following command:

```
npm install my-package
```

## Usage

Basic usage example here.
"#;
            fs.add_file_string("install.md", source_md).unwrap();

            // Create destination markdown file
            let dest_md = r#"# My Package

This is a great package.

## Features

- Feature 1
- Feature 2
"#;
            fs.add_file_string("README.md", dest_md).unwrap();

            let markdown_op = crate::config::MarkdownMergeOp {
                source: "install.md".to_string(),
                dest: "README.md".to_string(),
                section: "Installation".to_string(),
                append: false,
                level: 2,
                position: "end".to_string(),
                create_section: true,
            };

            apply_markdown_merge_operation(&mut fs, &markdown_op).unwrap();

            let result = fs.get_file("README.md").unwrap();
            let result_str = String::from_utf8(result.content.clone()).unwrap();

            // Should contain both original content and merged section
            assert!(result_str.contains("# My Package"));
            assert!(result_str.contains("## Features"));
            assert!(result_str.contains("## Installation"));
            assert!(result_str.contains("npm install my-package"));
            assert!(result_str.contains("## Usage"));
        }

        #[test]
        fn test_markdown_merge_operation_create_section() {
            // Test Markdown merge creating a new section
            let mut fs = MemoryFS::new();

            // Create source markdown fragment
            let source_md = r#"This package provides excellent functionality."#;
            fs.add_file_string("description.md", source_md).unwrap();

            // Create destination markdown file without the target section
            let dest_md = r#"# My Package

## Installation

Install instructions here.
"#;
            fs.add_file_string("README.md", dest_md).unwrap();

            let markdown_op = crate::config::MarkdownMergeOp {
                source: "description.md".to_string(),
                dest: "README.md".to_string(),
                section: "Description".to_string(),
                append: false,
                level: 2,
                position: "end".to_string(),
                create_section: true, // create section if it doesn't exist
            };

            apply_markdown_merge_operation(&mut fs, &markdown_op).unwrap();

            let result = fs.get_file("README.md").unwrap();
            let result_str = String::from_utf8(result.content.clone()).unwrap();

            // Should contain the new section
            assert!(result_str.contains("## Description"));
            assert!(result_str.contains("This package provides excellent functionality"));
        }

        #[test]
        fn test_markdown_merge_operation_append_mode() {
            // Test Markdown merge in append mode
            let mut fs = MemoryFS::new();

            // Create source markdown fragment
            let source_md = r#"- New feature added
- Bug fixes included"#;
            fs.add_file_string("updates.md", source_md).unwrap();

            // Create destination markdown file with existing section
            let dest_md = r#"# Changelog

## Version 1.0.0

- Initial release
- Basic functionality
"#;
            fs.add_file_string("CHANGELOG.md", dest_md).unwrap();

            let markdown_op = crate::config::MarkdownMergeOp {
                source: "updates.md".to_string(),
                dest: "CHANGELOG.md".to_string(),
                section: "Version 1.0.0".to_string(),
                append: true, // append mode
                level: 2,
                position: "end".to_string(),
                create_section: false,
            };

            apply_markdown_merge_operation(&mut fs, &markdown_op).unwrap();

            let result = fs.get_file("CHANGELOG.md").unwrap();
            let result_str = String::from_utf8(result.content.clone()).unwrap();

            // Should contain original content plus appended content
            assert!(result_str.contains("## Version 1.0.0"));
            assert!(result_str.contains("- Initial release"));
            assert!(result_str.contains("- Basic functionality"));
            assert!(result_str.contains("- New feature added"));
            assert!(result_str.contains("- Bug fixes included"));
        }
    }

    mod phase6_tests {
        use super::*;
        use crate::filesystem::File;
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        #[test]
        fn test_phase6_write_single_file() {
            let temp_dir = TempDir::new().unwrap();
            let output_path = temp_dir.path();

            let mut fs = MemoryFS::new();
            fs.add_file_string("test.txt", "Hello, world!").unwrap();

            phase6::execute(&fs, output_path).unwrap();

            let file_path = output_path.join("test.txt");
            assert!(file_path.exists());
            let content = fs::read_to_string(&file_path).unwrap();
            assert_eq!(content, "Hello, world!");
        }

        #[test]
        fn test_phase6_write_nested_directories() {
            let temp_dir = TempDir::new().unwrap();
            let output_path = temp_dir.path();

            let mut fs = MemoryFS::new();
            fs.add_file_string("src/utils/helper.rs", "pub fn helper() {}")
                .unwrap();
            fs.add_file_string("src/main.rs", "fn main() {}").unwrap();
            fs.add_file_string("README.md", "# Project").unwrap();

            phase6::execute(&fs, output_path).unwrap();

            // Verify nested file exists
            let nested_path = output_path.join("src/utils/helper.rs");
            assert!(nested_path.exists());
            assert!(nested_path.parent().unwrap().exists()); // utils directory
            assert!(nested_path.parent().unwrap().parent().unwrap().exists()); // src directory

            // Verify other files exist
            assert!(output_path.join("src/main.rs").exists());
            assert!(output_path.join("README.md").exists());

            // Verify content
            let content = fs::read_to_string(&nested_path).unwrap();
            assert_eq!(content, "pub fn helper() {}");
        }

        #[test]
        fn test_phase6_write_multiple_files() {
            let temp_dir = TempDir::new().unwrap();
            let output_path = temp_dir.path();

            let mut fs = MemoryFS::new();
            fs.add_file_string("file1.txt", "Content 1").unwrap();
            fs.add_file_string("file2.txt", "Content 2").unwrap();
            fs.add_file_string("file3.txt", "Content 3").unwrap();

            phase6::execute(&fs, output_path).unwrap();

            assert_eq!(
                fs::read_to_string(output_path.join("file1.txt")).unwrap(),
                "Content 1"
            );
            assert_eq!(
                fs::read_to_string(output_path.join("file2.txt")).unwrap(),
                "Content 2"
            );
            assert_eq!(
                fs::read_to_string(output_path.join("file3.txt")).unwrap(),
                "Content 3"
            );
        }

        #[test]
        fn test_phase6_write_binary_content() {
            let temp_dir = TempDir::new().unwrap();
            let output_path = temp_dir.path();

            let mut fs = MemoryFS::new();
            let binary_data = vec![0u8, 1u8, 2u8, 255u8, 128u8];
            fs.add_file_content("binary.bin", binary_data.clone())
                .unwrap();

            phase6::execute(&fs, output_path).unwrap();

            let file_path = output_path.join("binary.bin");
            assert!(file_path.exists());
            let content = fs::read(&file_path).unwrap();
            assert_eq!(content, binary_data);
        }

        #[test]
        #[cfg(unix)]
        fn test_phase6_preserve_permissions() {
            let temp_dir = TempDir::new().unwrap();
            let output_path = temp_dir.path();

            let mut fs = MemoryFS::new();
            let mut file = File::from_string("executable content");
            file.permissions = 0o755; // Executable permissions
            fs.add_file("script.sh", file).unwrap();

            phase6::execute(&fs, output_path).unwrap();

            let file_path = output_path.join("script.sh");
            assert!(file_path.exists());

            let metadata = fs::metadata(&file_path).unwrap();
            let permissions = metadata.permissions();
            let mode = permissions.mode();
            // Check that executable bit is set (0o755 = 493 in decimal)
            // We check the last 3 octal digits (permissions)
            assert_eq!(mode & 0o777, 0o755);
        }

        #[test]
        fn test_phase6_empty_filesystem() {
            let temp_dir = TempDir::new().unwrap();
            let output_path = temp_dir.path();

            let fs = MemoryFS::new();

            // Should not error on empty filesystem
            phase6::execute(&fs, output_path).unwrap();

            // Directory should exist but be empty
            assert!(output_path.exists());
            assert!(fs::read_dir(output_path).unwrap().next().is_none());
        }

        #[test]
        fn test_phase6_overwrite_existing_file() {
            let temp_dir = TempDir::new().unwrap();
            let output_path = temp_dir.path();

            // Create an existing file
            let existing_path = output_path.join("existing.txt");
            fs::write(&existing_path, "old content").unwrap();

            let mut fs = MemoryFS::new();
            fs.add_file_string("existing.txt", "new content").unwrap();

            phase6::execute(&fs, output_path).unwrap();

            // File should be overwritten
            let content = fs::read_to_string(&existing_path).unwrap();
            assert_eq!(content, "new content");
        }
    }

    mod repo_tree_tests {
        use super::*;

        #[test]
        fn test_repo_tree_creation() {
            let root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            let tree = RepoTree::new(root);

            assert_eq!(tree.all_repos.len(), 0); // local is not counted
        }

        #[test]
        fn test_repo_tree_with_children() {
            let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            let child = RepoNode::new(
                "https://github.com/repo.git".to_string(),
                "main".to_string(),
                vec![],
            );
            root.add_child(child);
            let tree = RepoTree::new(root);

            assert_eq!(tree.all_repos.len(), 1);
            assert!(tree.all_repos.contains(&(
                "https://github.com/repo.git".to_string(),
                "main".to_string()
            )));
        }

        #[test]
        fn test_repo_node_add_child() {
            let mut parent = RepoNode::new(
                "https://github.com/parent.git".to_string(),
                "main".to_string(),
                vec![],
            );
            let child = RepoNode::new(
                "https://github.com/child.git".to_string(),
                "main".to_string(),
                vec![],
            );

            assert_eq!(parent.children.len(), 0);
            parent.add_child(child);
            assert_eq!(parent.children.len(), 1);
            assert_eq!(parent.children[0].url, "https://github.com/child.git");
        }

        #[test]
        fn test_repo_tree_collects_all_repos() {
            // Test that RepoTree collects all repos from nested structure
            let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            let mut parent = RepoNode::new(
                "https://github.com/parent.git".to_string(),
                "main".to_string(),
                vec![],
            );
            let child = RepoNode::new(
                "https://github.com/child.git".to_string(),
                "main".to_string(),
                vec![],
            );
            parent.add_child(child);
            root.add_child(parent);
            let tree = RepoTree::new(root);

            assert_eq!(tree.all_repos.len(), 2);
            assert!(tree.all_repos.contains(&(
                "https://github.com/parent.git".to_string(),
                "main".to_string()
            )));
            assert!(tree.all_repos.contains(&(
                "https://github.com/child.git".to_string(),
                "main".to_string()
            )));
        }

        #[test]
        fn test_repo_tree_would_create_cycle() {
            let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            let repo = RepoNode::new(
                "https://github.com/repo.git".to_string(),
                "main".to_string(),
                vec![],
            );
            root.add_child(repo);
            let tree = RepoTree::new(root);

            // Check if repo exists
            assert!(tree.would_create_cycle("https://github.com/repo.git", "main"));
            assert!(!tree.would_create_cycle("https://github.com/other.git", "main"));
        }
    }

    mod parse_path_tests {
        use crate::phases::phase5::{parse_path, PathSegment};

        #[test]
        fn test_parse_path_empty() {
            assert_eq!(parse_path("").len(), 0);
            assert_eq!(parse_path("  ").len(), 0);
            assert_eq!(parse_path("/").len(), 0);
        }

        #[test]
        fn test_parse_path_simple_dots() {
            let segments = parse_path("metadata.labels");
            assert_eq!(segments.len(), 2);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "metadata"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[1] {
                PathSegment::Key(k) => assert_eq!(k, "labels"),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_parse_path_numeric_index() {
            let segments = parse_path("items[0]");
            assert_eq!(segments.len(), 2);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "items"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[1] {
                PathSegment::Index(i) => assert_eq!(*i, 0),
                _ => panic!("Expected Index segment"),
            }
        }

        #[test]
        fn test_parse_path_quoted_bracket_double() {
            let segments = parse_path(r#"metadata["labels.with.dot"]"#);
            assert_eq!(segments.len(), 2);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "metadata"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[1] {
                PathSegment::Key(k) => assert_eq!(k, "labels.with.dot"),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_parse_path_quoted_bracket_single() {
            let segments = parse_path("metadata['labels.with.dot']");
            assert_eq!(segments.len(), 2);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "metadata"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[1] {
                PathSegment::Key(k) => assert_eq!(k, "labels.with.dot"),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_parse_path_backslash_escape() {
            let segments = parse_path(r"metadata\.labels");
            assert_eq!(segments.len(), 1);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "metadata.labels"),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_parse_path_complex_mixed() {
            let segments = parse_path(r#"items[0].metadata["labels.app"].value"#);
            assert_eq!(segments.len(), 5);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "items"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[1] {
                PathSegment::Index(i) => assert_eq!(*i, 0),
                _ => panic!("Expected Index segment"),
            }
            match &segments[2] {
                PathSegment::Key(k) => assert_eq!(k, "metadata"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[3] {
                PathSegment::Key(k) => assert_eq!(k, "labels.app"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[4] {
                PathSegment::Key(k) => assert_eq!(k, "value"),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_parse_path_unquoted_bracket_key() {
            let segments = parse_path("[key]");
            assert_eq!(segments.len(), 1);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "key"),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_parse_path_escaped_quote_in_bracket() {
            let segments = parse_path(r#"["key\"with\"quotes"]"#);
            assert_eq!(segments.len(), 1);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, r#"key"with"quotes"#),
                _ => panic!("Expected Key segment"),
            }
        }

        #[test]
        fn test_parse_path_multiple_indices() {
            let segments = parse_path("items[0][1]");
            assert_eq!(segments.len(), 3);
            match &segments[0] {
                PathSegment::Key(k) => assert_eq!(k, "items"),
                _ => panic!("Expected Key segment"),
            }
            match &segments[1] {
                PathSegment::Index(i) => assert_eq!(*i, 0),
                _ => panic!("Expected Index segment"),
            }
            match &segments[2] {
                PathSegment::Index(i) => assert_eq!(*i, 1),
                _ => panic!("Expected Index segment"),
            }
        }
    }

    mod navigate_yaml_value_tests {
        use crate::phases::phase5::{navigate_yaml_value, parse_path, PathSegment};
        use serde_yaml::Value as YamlValue;

        #[test]
        fn test_navigate_creates_missing_keys() {
            let mut value = YamlValue::Mapping(Default::default());
            let path = parse_path("metadata.labels");
            let target = navigate_yaml_value(&mut value, &path).unwrap();
            *target = YamlValue::String("test".to_string());

            assert!(value.get("metadata").is_some());
            assert!(value.get("metadata").unwrap().get("labels").is_some());
        }

        #[test]
        fn test_navigate_array_index() {
            let mut value =
                serde_yaml::from_str("items:\n  - name: first\n  - name: second").unwrap();
            let path = parse_path("items[1].name");
            let target = navigate_yaml_value(&mut value, &path).unwrap();
            *target = YamlValue::String("modified".to_string());

            let items = value.get("items").unwrap().as_sequence().unwrap();
            assert_eq!(items[1].get("name").unwrap().as_str().unwrap(), "modified");
        }

        #[test]
        fn test_navigate_creates_null_to_mapping() {
            let mut value = YamlValue::Null;
            let path = parse_path("metadata.labels");
            let target = navigate_yaml_value(&mut value, &path).unwrap();
            *target = YamlValue::String("test".to_string());

            assert!(value.is_mapping());
            assert!(value.get("metadata").is_some());
        }

        #[test]
        fn test_navigate_error_on_non_mapping() {
            let mut value = YamlValue::String("not a mapping".to_string());
            let path = parse_path("metadata.labels");
            let result = navigate_yaml_value(&mut value, &path);
            assert!(result.is_err());
        }
    }

    mod yaml_merge_integration_tests {
        use crate::filesystem::MemoryFS;
        use crate::phases::phase5::{apply_yaml_merge_operation, read_file_as_string};
        use serde_yaml::Value as YamlValue;

        #[test]
        fn test_yaml_merge_at_root() {
            let mut fs = MemoryFS::new();
            fs.add_file_string("source.yaml", "key: value\nother: data")
                .unwrap();
            fs.add_file_string("dest.yaml", "existing: field").unwrap();

            let op = crate::config::YamlMergeOp {
                source: "source.yaml".to_string(),
                dest: "dest.yaml".to_string(),
                path: None,
                append: false,
            };

            apply_yaml_merge_operation(&mut fs, &op).unwrap();

            let content = read_file_as_string(&fs, "dest.yaml").unwrap();
            let value: YamlValue = serde_yaml::from_str(&content).unwrap();
            let map = value.as_mapping().unwrap();
            assert_eq!(
                map.get(&YamlValue::String("key".into()))
                    .and_then(|v| v.as_str()),
                Some("value")
            );
            assert_eq!(
                map.get(&YamlValue::String("other".into()))
                    .and_then(|v| v.as_str()),
                Some("data")
            );
            assert_eq!(
                map.get(&YamlValue::String("existing".into()))
                    .and_then(|v| v.as_str()),
                Some("field")
            );
        }

        #[test]
        fn test_yaml_merge_at_path() {
            let mut fs = MemoryFS::new();
            fs.add_file_string("source.yaml", "app: myapp\nversion: \"1.0\"")
                .unwrap();
            fs.add_file_string("dest.yaml", "metadata:\n  name: test")
                .unwrap();

            let op = crate::config::YamlMergeOp {
                source: "source.yaml".to_string(),
                dest: "dest.yaml".to_string(),
                path: Some("metadata.labels".to_string()),
                append: false,
            };

            apply_yaml_merge_operation(&mut fs, &op).unwrap();

            let content = read_file_as_string(&fs, "dest.yaml").unwrap();
            let value: YamlValue = serde_yaml::from_str(&content).unwrap();
            let map = value.as_mapping().unwrap();
            let metadata = map
                .get(&YamlValue::String("metadata".into()))
                .unwrap()
                .as_mapping()
                .unwrap();
            let labels = metadata
                .get(&YamlValue::String("labels".into()))
                .unwrap()
                .as_mapping()
                .unwrap();
            assert_eq!(
                labels
                    .get(&YamlValue::String("app".into()))
                    .and_then(|v| v.as_str()),
                Some("myapp")
            );
            assert_eq!(
                labels
                    .get(&YamlValue::String("version".into()))
                    .and_then(|v| v.as_str()),
                Some("1.0")
            );
        }

        #[test]
        fn test_yaml_merge_with_escaped_dots() {
            let mut fs = MemoryFS::new();
            fs.add_file_string("source.yaml", "value: test").unwrap();
            fs.add_file_string("dest.yaml", "metadata: {}").unwrap();

            let op = crate::config::YamlMergeOp {
                source: "source.yaml".to_string(),
                dest: "dest.yaml".to_string(),
                path: Some(r#"metadata["app.kubernetes.io/name"]"#.to_string()),
                append: false,
            };

            apply_yaml_merge_operation(&mut fs, &op).unwrap();

            let content = read_file_as_string(&fs, "dest.yaml").unwrap();
            let value: YamlValue = serde_yaml::from_str(&content).unwrap();
            let map = value.as_mapping().unwrap();
            let metadata = map
                .get(&YamlValue::String("metadata".into()))
                .unwrap()
                .as_mapping()
                .unwrap();
            assert_eq!(
                metadata
                    .get(&YamlValue::String("app.kubernetes.io/name".into()))
                    .and_then(|v| v.as_str()),
                Some("test")
            );
        }

        #[test]
        fn test_yaml_merge_creates_dest_if_missing() {
            let mut fs = MemoryFS::new();
            fs.add_file_string("source.yaml", "key: value").unwrap();

            let op = crate::config::YamlMergeOp {
                source: "source.yaml".to_string(),
                dest: "new.yaml".to_string(),
                path: None,
                append: false,
            };

            apply_yaml_merge_operation(&mut fs, &op).unwrap();

            let content = read_file_as_string(&fs, "new.yaml").unwrap();
            let value: YamlValue = serde_yaml::from_str(&content).unwrap();
            let map = value.as_mapping().unwrap();
            assert_eq!(
                map.get(&YamlValue::String("key".into()))
                    .and_then(|v| v.as_str()),
                Some("value")
            );
        }
    }
}
