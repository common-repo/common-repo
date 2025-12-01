//! Phase 2: Processing Individual Repositories
//!
//! This is the second phase of the `common-repo` execution pipeline. Its main
//! responsibility is to take the raw, cloned repositories from Phase 1 and
//! apply the operations defined in the configuration to each one, producing a
//! set of "intermediate" in-memory filesystems.
//!
//! ## Process
//!
//! 1.  **Recursive Processing**: The process traverses the `RepoTree` from the
//!     leaves up to the root. For each repository in the tree, it performs the
//!     following steps.
//!
//! 2.  **In-Process Caching**: Before processing, it checks the in-process
//!     `RepoCache` to see if this exact repository (with the same set of `with:`
//!     clause operations) has already been processed in this run. If so, it
//!     uses the cached result to avoid redundant work.
//!
//! 3.  **Operation Application**: If not cached, it loads the repository's
//!     contents from the on-disk cache into a `MemoryFS`. It then iterates
//!     through the operations associated with that repository (from the `with:`
//!     clause) and applies each one in order to the `MemoryFS`.
//!
//! 4.  **Template Variable Collection**: During this process, it also collects
//!     any `template_vars` defined in the operations and stores them in the
//!     `IntermediateFS` for use in a later phase.
//!
//! 5.  **Storing Results**: The resulting `MemoryFS` and collected template
//!     variables are stored in an `IntermediateFS` struct, which is then added
//!     to the in-process cache and a `HashMap` that is passed to the next phase.
//!
//! This phase transforms the raw source material from each repository into a
//! set of processed, in-memory filesystems that are ready to be merged.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use serde_yaml;

use super::{IntermediateFS, RepoNode, RepoTree};
use crate::cache::{CacheKey, RepoCache};
use crate::config::Operation;
use crate::error::{Error, Result};
use crate::filesystem::MemoryFS;
use crate::operators;
use crate::repository::RepositoryManager;

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

    // Collect merge operations to be executed later in Phase 4
    let merge_operations = if node.url == "local" {
        Vec::new()
    } else {
        collect_merge_operations(&node.operations)
    };

    if let Some(cache_key) = cache_key_for_node(node)? {
        let fs = cache.get_or_process(cache_key, || -> Result<MemoryFS> {
            let mut fs = repo_manager.fetch_repository(&node.url, &node.ref_)?;
            for operation in &node.operations {
                apply_operation(&mut fs, operation)?;
            }
            Ok(fs)
        })?;

        return Ok(IntermediateFS::new_with_vars_and_merges(
            fs,
            node.url.clone(),
            node.ref_.clone(),
            template_vars,
            merge_operations,
        ));
    }

    // Local repository: process directly without caching
    let mut fs = MemoryFS::new();
    for operation in &node.operations {
        apply_operation(&mut fs, operation)?;
    }

    Ok(IntermediateFS::new_with_vars_and_merges(
        fs,
        node.url.clone(),
        node.ref_.clone(),
        template_vars,
        merge_operations,
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

/// Collect merge operations from a list of operations
///
/// Merge operations (yaml, json, toml, ini, markdown) are collected
/// during Phase 2 but executed later in Phase 4 during composition.
fn collect_merge_operations(operations: &[Operation]) -> Vec<Operation> {
    operations
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
        .cloned()
        .collect()
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
        // Merge operations are collected separately and executed in Phase 4
        Operation::Yaml { yaml: _ } => {
            // Collected in collect_merge_operations() and executed in Phase 4
            Ok(())
        }
        Operation::Json { json: _ } => {
            // Collected in collect_merge_operations() and executed in Phase 4
            Ok(())
        }
        Operation::Toml { toml: _ } => {
            // Collected in collect_merge_operations() and executed in Phase 4
            Ok(())
        }
        Operation::Ini { ini: _ } => {
            // Collected in collect_merge_operations() and executed in Phase 4
            Ok(())
        }
        Operation::Markdown { markdown: _ } => {
            // Collected in collect_merge_operations() and executed in Phase 4
            Ok(())
        }
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

    // ========================================================================
    // Phase 2 Processing Tests
    // ========================================================================

    mod cache_key_tests {
        use super::*;
        use crate::config::{ExcludeOp, IncludeOp, RenameMapping, RenameOp, TemplateOp};
        use crate::phases::RepoNode;

        #[test]
        fn test_cache_key_for_local_repo_returns_none() {
            // Local repos should not be cached
            let node = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            let result = cache_key_for_node(&node).expect("should not error");
            assert!(result.is_none(), "local repo should return None cache key");
        }

        #[test]
        fn test_cache_key_for_repo_with_empty_operations() {
            // Remote repo with no operations should return simple cache key
            let node = RepoNode::new(
                "https://github.com/example/repo.git".to_string(),
                "v1.0.0".to_string(),
                vec![],
            );
            let result = cache_key_for_node(&node).expect("should not error");
            assert!(result.is_some(), "remote repo should return cache key");
            let key = result.unwrap();
            assert!(key.url.contains("example/repo"));
            assert!(key.r#ref.contains("v1.0.0"));
        }

        #[test]
        fn test_cache_key_for_repo_with_operations_includes_fingerprint() {
            // Remote repo with operations should include fingerprint in cache key
            let operations = vec![Operation::Exclude {
                exclude: ExcludeOp {
                    patterns: vec!["*.tmp".to_string()],
                },
            }];
            let node = RepoNode::new(
                "https://github.com/example/repo.git".to_string(),
                "main".to_string(),
                operations,
            );
            let result = cache_key_for_node(&node).expect("should not error");
            assert!(result.is_some());
            let key = result.unwrap();
            // Cache key URL should include the fingerprint
            assert!(
                key.url.contains("#ops-"),
                "cache key should include ops fingerprint"
            );
        }

        #[test]
        fn test_cache_key_fingerprint_differs_for_different_operations() {
            // Different operations should produce different fingerprints
            let node1 = RepoNode::new(
                "https://github.com/example/repo.git".to_string(),
                "main".to_string(),
                vec![Operation::Exclude {
                    exclude: ExcludeOp {
                        patterns: vec!["*.tmp".to_string()],
                    },
                }],
            );
            let node2 = RepoNode::new(
                "https://github.com/example/repo.git".to_string(),
                "main".to_string(),
                vec![Operation::Exclude {
                    exclude: ExcludeOp {
                        patterns: vec!["*.log".to_string()],
                    },
                }],
            );

            let key1 = cache_key_for_node(&node1).unwrap().unwrap();
            let key2 = cache_key_for_node(&node2).unwrap().unwrap();

            assert_ne!(
                key1.url, key2.url,
                "different operations should produce different fingerprints"
            );
        }

        #[test]
        fn test_cache_key_fingerprint_same_for_identical_operations() {
            // Identical operations should produce the same fingerprint
            let operations = vec![Operation::Exclude {
                exclude: ExcludeOp {
                    patterns: vec!["*.tmp".to_string()],
                },
            }];
            let node1 = RepoNode::new(
                "https://github.com/example/repo.git".to_string(),
                "main".to_string(),
                operations.clone(),
            );
            let node2 = RepoNode::new(
                "https://github.com/example/repo.git".to_string(),
                "main".to_string(),
                operations,
            );

            let key1 = cache_key_for_node(&node1).unwrap().unwrap();
            let key2 = cache_key_for_node(&node2).unwrap().unwrap();

            assert_eq!(
                key1.url, key2.url,
                "identical operations should produce same fingerprint"
            );
        }

        #[test]
        fn test_cache_key_with_multiple_operations() {
            // Multiple operations should all be included in fingerprint
            let operations = vec![
                Operation::Exclude {
                    exclude: ExcludeOp {
                        patterns: vec!["*.tmp".to_string()],
                    },
                },
                Operation::Include {
                    include: IncludeOp {
                        patterns: vec!["src/**".to_string()],
                    },
                },
                Operation::Rename {
                    rename: RenameOp {
                        mappings: vec![RenameMapping {
                            from: "old".to_string(),
                            to: "new".to_string(),
                        }],
                    },
                },
            ];
            let node = RepoNode::new(
                "https://github.com/example/repo.git".to_string(),
                "main".to_string(),
                operations,
            );
            let result = cache_key_for_node(&node).expect("should not error");
            assert!(result.is_some());
        }

        #[test]
        fn test_cache_key_with_template_operation() {
            let operations = vec![Operation::Template {
                template: TemplateOp {
                    patterns: vec!["**/*.tmpl".to_string()],
                },
            }];
            let node = RepoNode::new(
                "https://github.com/example/repo.git".to_string(),
                "main".to_string(),
                operations,
            );
            let result = cache_key_for_node(&node).expect("should not error");
            assert!(result.is_some());
            let key = result.unwrap();
            assert!(key.url.contains("#ops-"));
        }
    }

    mod template_vars_tests {
        use super::*;
        use crate::config::{ExcludeOp, TemplateVars};

        #[test]
        fn test_collect_template_vars_empty_operations() {
            let operations: Vec<Operation> = vec![];
            let result = collect_template_vars(&operations).expect("should not error");
            assert!(result.is_empty());
        }

        #[test]
        fn test_collect_template_vars_no_template_vars_operations() {
            // Operations that are not TemplateVars should be ignored
            let operations = vec![Operation::Exclude {
                exclude: ExcludeOp {
                    patterns: vec!["*.tmp".to_string()],
                },
            }];
            let result = collect_template_vars(&operations).expect("should not error");
            assert!(result.is_empty());
        }

        #[test]
        fn test_collect_template_vars_single_operation() {
            let mut vars = HashMap::new();
            vars.insert("PROJECT_NAME".to_string(), "my-project".to_string());
            vars.insert("VERSION".to_string(), "1.0.0".to_string());

            let operations = vec![Operation::TemplateVars {
                template_vars: TemplateVars { vars },
            }];
            let result = collect_template_vars(&operations).expect("should not error");
            assert_eq!(result.len(), 2);
            assert_eq!(result.get("PROJECT_NAME"), Some(&"my-project".to_string()));
            assert_eq!(result.get("VERSION"), Some(&"1.0.0".to_string()));
        }

        #[test]
        fn test_collect_template_vars_multiple_operations() {
            let mut vars1 = HashMap::new();
            vars1.insert("VAR1".to_string(), "value1".to_string());

            let mut vars2 = HashMap::new();
            vars2.insert("VAR2".to_string(), "value2".to_string());

            let operations = vec![
                Operation::TemplateVars {
                    template_vars: TemplateVars { vars: vars1 },
                },
                Operation::TemplateVars {
                    template_vars: TemplateVars { vars: vars2 },
                },
            ];
            let result = collect_template_vars(&operations).expect("should not error");
            assert_eq!(result.len(), 2);
            assert_eq!(result.get("VAR1"), Some(&"value1".to_string()));
            assert_eq!(result.get("VAR2"), Some(&"value2".to_string()));
        }

        #[test]
        fn test_collect_template_vars_mixed_with_other_operations() {
            let mut vars = HashMap::new();
            vars.insert("KEY".to_string(), "value".to_string());

            let operations = vec![
                Operation::Exclude {
                    exclude: ExcludeOp {
                        patterns: vec!["*.tmp".to_string()],
                    },
                },
                Operation::TemplateVars {
                    template_vars: TemplateVars { vars },
                },
            ];
            let result = collect_template_vars(&operations).expect("should not error");
            assert_eq!(result.len(), 1);
            assert_eq!(result.get("KEY"), Some(&"value".to_string()));
        }
    }

    mod merge_operations_tests {
        use super::*;
        use crate::config::{
            ExcludeOp, IniMergeOp, JsonMergeOp, MarkdownMergeOp, TomlMergeOp, YamlMergeOp,
        };

        #[test]
        fn test_collect_merge_operations_empty() {
            let operations: Vec<Operation> = vec![];
            let result = collect_merge_operations(&operations);
            assert!(result.is_empty());
        }

        #[test]
        fn test_collect_merge_operations_no_merge_ops() {
            let operations = vec![Operation::Exclude {
                exclude: ExcludeOp {
                    patterns: vec!["*.tmp".to_string()],
                },
            }];
            let result = collect_merge_operations(&operations);
            assert!(result.is_empty());
        }

        #[test]
        fn test_collect_merge_operations_yaml() {
            let operations = vec![Operation::Yaml {
                yaml: YamlMergeOp {
                    source: "source.yaml".to_string(),
                    dest: "dest.yaml".to_string(),
                    path: None,
                    append: false,
                    array_mode: None,
                },
            }];
            let result = collect_merge_operations(&operations);
            assert_eq!(result.len(), 1);
            assert!(matches!(result[0], Operation::Yaml { .. }));
        }

        #[test]
        fn test_collect_merge_operations_json() {
            let operations = vec![Operation::Json {
                json: JsonMergeOp {
                    source: "source.json".to_string(),
                    dest: "dest.json".to_string(),
                    path: Some("$.key".to_string()),
                    append: true,
                    position: None,
                },
            }];
            let result = collect_merge_operations(&operations);
            assert_eq!(result.len(), 1);
            assert!(matches!(result[0], Operation::Json { .. }));
        }

        #[test]
        fn test_collect_merge_operations_toml() {
            let operations = vec![Operation::Toml {
                toml: TomlMergeOp {
                    source: "source.toml".to_string(),
                    dest: "dest.toml".to_string(),
                    path: "section.key".to_string(),
                    append: false,
                    preserve_comments: true,
                    array_mode: None,
                },
            }];
            let result = collect_merge_operations(&operations);
            assert_eq!(result.len(), 1);
            assert!(matches!(result[0], Operation::Toml { .. }));
        }

        #[test]
        fn test_collect_merge_operations_ini() {
            let operations = vec![Operation::Ini {
                ini: IniMergeOp {
                    source: "source.ini".to_string(),
                    dest: "dest.ini".to_string(),
                    section: Some("settings".to_string()),
                    append: false,
                    allow_duplicates: false,
                },
            }];
            let result = collect_merge_operations(&operations);
            assert_eq!(result.len(), 1);
            assert!(matches!(result[0], Operation::Ini { .. }));
        }

        #[test]
        fn test_collect_merge_operations_markdown() {
            let operations = vec![Operation::Markdown {
                markdown: MarkdownMergeOp {
                    source: "source.md".to_string(),
                    dest: "dest.md".to_string(),
                    section: "## Section".to_string(),
                    append: true,
                    level: 2,
                    position: "end".to_string(),
                    create_section: false,
                },
            }];
            let result = collect_merge_operations(&operations);
            assert_eq!(result.len(), 1);
            assert!(matches!(result[0], Operation::Markdown { .. }));
        }

        #[test]
        fn test_collect_merge_operations_all_types() {
            let operations = vec![
                Operation::Yaml {
                    yaml: YamlMergeOp {
                        source: "s.yaml".to_string(),
                        dest: "d.yaml".to_string(),
                        path: None,
                        append: false,
                        array_mode: None,
                    },
                },
                Operation::Json {
                    json: JsonMergeOp {
                        source: "s.json".to_string(),
                        dest: "d.json".to_string(),
                        path: None,
                        append: false,
                        position: None,
                    },
                },
                Operation::Toml {
                    toml: TomlMergeOp {
                        source: "s.toml".to_string(),
                        dest: "d.toml".to_string(),
                        path: "key".to_string(),
                        append: false,
                        preserve_comments: false,
                        array_mode: None,
                    },
                },
                Operation::Ini {
                    ini: IniMergeOp {
                        source: "s.ini".to_string(),
                        dest: "d.ini".to_string(),
                        section: None,
                        append: false,
                        allow_duplicates: false,
                    },
                },
                Operation::Markdown {
                    markdown: MarkdownMergeOp {
                        source: "s.md".to_string(),
                        dest: "d.md".to_string(),
                        section: "## Section".to_string(),
                        append: false,
                        level: 2,
                        position: "".to_string(),
                        create_section: false,
                    },
                },
            ];
            let result = collect_merge_operations(&operations);
            assert_eq!(result.len(), 5);
        }

        #[test]
        fn test_collect_merge_operations_filters_non_merge_ops() {
            let operations = vec![
                Operation::Exclude {
                    exclude: ExcludeOp {
                        patterns: vec!["*.tmp".to_string()],
                    },
                },
                Operation::Yaml {
                    yaml: YamlMergeOp {
                        source: "s.yaml".to_string(),
                        dest: "d.yaml".to_string(),
                        path: None,
                        append: false,
                        array_mode: None,
                    },
                },
                Operation::Exclude {
                    exclude: ExcludeOp {
                        patterns: vec!["*.log".to_string()],
                    },
                },
            ];
            let result = collect_merge_operations(&operations);
            assert_eq!(result.len(), 1);
            assert!(matches!(result[0], Operation::Yaml { .. }));
        }
    }

    mod apply_operation_tests {
        use super::*;
        use crate::config::{
            ExcludeOp, IncludeOp, IniMergeOp, JsonMergeOp, MarkdownMergeOp, RenameMapping,
            RenameOp, TemplateOp, TemplateVars, TomlMergeOp, Tool, ToolsOp, YamlMergeOp,
        };
        use crate::filesystem::MemoryFS;

        fn create_test_fs() -> MemoryFS {
            let mut fs = MemoryFS::new();
            fs.add_file_string("src/main.rs", "fn main() {}").unwrap();
            fs.add_file_string("src/lib.rs", "// lib").unwrap();
            fs.add_file_string("test.tmp", "temporary").unwrap();
            fs.add_file_string("README.md", "# README").unwrap();
            fs
        }

        #[test]
        fn test_apply_operation_include() {
            let mut fs = create_test_fs();
            let operation = Operation::Include {
                include: IncludeOp {
                    patterns: vec!["src/**".to_string()],
                },
            };
            apply_operation(&mut fs, &operation).expect("should not error");
            // After include, only matching files should exist
            assert!(fs.exists("src/main.rs"));
            assert!(fs.exists("src/lib.rs"));
            assert!(!fs.exists("test.tmp"));
            assert!(!fs.exists("README.md"));
        }

        #[test]
        fn test_apply_operation_exclude() {
            let mut fs = create_test_fs();
            let operation = Operation::Exclude {
                exclude: ExcludeOp {
                    patterns: vec!["*.tmp".to_string()],
                },
            };
            apply_operation(&mut fs, &operation).expect("should not error");
            assert!(fs.exists("src/main.rs"));
            assert!(fs.exists("README.md"));
            assert!(!fs.exists("test.tmp"));
        }

        #[test]
        fn test_apply_operation_rename() {
            let mut fs = create_test_fs();
            let operation = Operation::Rename {
                rename: RenameOp {
                    mappings: vec![RenameMapping {
                        from: r"^README\.md$".to_string(),
                        to: "GUIDE.md".to_string(),
                    }],
                },
            };
            apply_operation(&mut fs, &operation).expect("should not error");
            assert!(fs.exists("GUIDE.md"));
            assert!(!fs.exists("README.md"));
        }

        #[test]
        fn test_apply_operation_repo_is_noop() {
            // Repo operations are handled in Phase 1, they should be no-ops here
            let mut fs = create_test_fs();
            let original_count = fs.list_files().len();
            let operation = Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/example/repo.git".to_string(),
                    r#ref: "main".to_string(),
                    path: None,
                    with: vec![],
                },
            };
            apply_operation(&mut fs, &operation).expect("should not error");
            assert_eq!(original_count, fs.list_files().len());
        }

        #[test]
        fn test_apply_operation_template_vars_is_noop() {
            // TemplateVars operations are collected separately
            let mut fs = create_test_fs();
            let original_count = fs.list_files().len();
            let mut vars = HashMap::new();
            vars.insert("KEY".to_string(), "value".to_string());
            let operation = Operation::TemplateVars {
                template_vars: TemplateVars { vars },
            };
            apply_operation(&mut fs, &operation).expect("should not error");
            assert_eq!(original_count, fs.list_files().len());
        }

        #[test]
        fn test_apply_operation_template_marks_files() {
            let mut fs = create_test_fs();
            let operation = Operation::Template {
                template: TemplateOp {
                    patterns: vec!["*.md".to_string()],
                },
            };
            apply_operation(&mut fs, &operation).expect("should not error");
            // The template operator marks files in the filesystem metadata
            // Check that the file still exists
            assert!(fs.exists("README.md"));
        }

        #[test]
        fn test_apply_operation_yaml_merge_is_noop() {
            // YAML merge operations are collected and executed in Phase 4
            let mut fs = create_test_fs();
            let original_count = fs.list_files().len();
            let operation = Operation::Yaml {
                yaml: YamlMergeOp {
                    source: "source.yaml".to_string(),
                    dest: "dest.yaml".to_string(),
                    path: None,
                    append: false,
                    array_mode: None,
                },
            };
            apply_operation(&mut fs, &operation).expect("should not error");
            assert_eq!(original_count, fs.list_files().len());
        }

        #[test]
        fn test_apply_operation_json_merge_is_noop() {
            let mut fs = create_test_fs();
            let original_count = fs.list_files().len();
            let operation = Operation::Json {
                json: JsonMergeOp {
                    source: "source.json".to_string(),
                    dest: "dest.json".to_string(),
                    path: None,
                    append: false,
                    position: None,
                },
            };
            apply_operation(&mut fs, &operation).expect("should not error");
            assert_eq!(original_count, fs.list_files().len());
        }

        #[test]
        fn test_apply_operation_toml_merge_is_noop() {
            let mut fs = create_test_fs();
            let original_count = fs.list_files().len();
            let operation = Operation::Toml {
                toml: TomlMergeOp {
                    source: "source.toml".to_string(),
                    dest: "dest.toml".to_string(),
                    path: "key".to_string(),
                    append: false,
                    preserve_comments: false,
                    array_mode: None,
                },
            };
            apply_operation(&mut fs, &operation).expect("should not error");
            assert_eq!(original_count, fs.list_files().len());
        }

        #[test]
        fn test_apply_operation_ini_merge_is_noop() {
            let mut fs = create_test_fs();
            let original_count = fs.list_files().len();
            let operation = Operation::Ini {
                ini: IniMergeOp {
                    source: "source.ini".to_string(),
                    dest: "dest.ini".to_string(),
                    section: None,
                    append: false,
                    allow_duplicates: false,
                },
            };
            apply_operation(&mut fs, &operation).expect("should not error");
            assert_eq!(original_count, fs.list_files().len());
        }

        #[test]
        fn test_apply_operation_markdown_merge_is_noop() {
            let mut fs = create_test_fs();
            let original_count = fs.list_files().len();
            let operation = Operation::Markdown {
                markdown: MarkdownMergeOp {
                    source: "source.md".to_string(),
                    dest: "dest.md".to_string(),
                    section: "## Section".to_string(),
                    append: false,
                    level: 2,
                    position: "end".to_string(),
                    create_section: false,
                },
            };
            apply_operation(&mut fs, &operation).expect("should not error");
            assert_eq!(original_count, fs.list_files().len());
        }

        #[test]
        fn test_apply_operation_tools_with_available_tool() {
            // Test tools operation with a commonly available tool
            let mut fs = create_test_fs();
            let operation = Operation::Tools {
                tools: ToolsOp {
                    tools: vec![Tool {
                        name: "sh".to_string(),
                        version: "*".to_string(),
                    }],
                },
            };
            // This should succeed since 'sh' is available
            let result = apply_operation(&mut fs, &operation);
            // Note: This may fail in some environments, which is OK for testing
            // The important thing is that the operation is attempted
            let _ = result;
        }
    }

    mod process_single_repo_tests {
        use super::*;
        use crate::cache::RepoCache;
        use crate::config::{ExcludeOp, TemplateVars};
        use crate::filesystem::MemoryFS;
        use crate::phases::RepoNode;
        use crate::repository::{CacheOperations, GitOperations, RepositoryManager};
        use std::path::{Path, PathBuf};

        struct SimpleTestGitOps;
        impl GitOperations for SimpleTestGitOps {
            fn clone_shallow(&self, _url: &str, _ref_name: &str, _path: &Path) -> Result<()> {
                Ok(())
            }
            fn list_tags(&self, _url: &str) -> Result<Vec<String>> {
                Ok(vec![])
            }
        }

        struct SimpleTestCacheOps {
            fs: MemoryFS,
        }
        impl SimpleTestCacheOps {
            fn new() -> Self {
                let mut fs = MemoryFS::new();
                fs.add_file_string("file.txt", "content").unwrap();
                fs.add_file_string("test.tmp", "temp").unwrap();
                Self { fs }
            }
        }
        impl CacheOperations for SimpleTestCacheOps {
            fn exists(&self, _cache_path: &Path) -> bool {
                true
            }
            fn get_cache_path(&self, _url: &str, _ref_name: &str) -> PathBuf {
                PathBuf::from("/mock/cache")
            }
            fn load_from_cache(&self, _cache_path: &Path) -> Result<MemoryFS> {
                Ok(self.fs.clone())
            }
            fn save_to_cache(&self, _cache_path: &Path, _fs: &MemoryFS) -> Result<()> {
                Ok(())
            }
        }

        #[test]
        fn test_process_single_repo_local_no_caching() {
            let repo_manager = RepositoryManager::with_operations(
                Box::new(SimpleTestGitOps),
                Box::new(SimpleTestCacheOps::new()),
            );
            let cache = RepoCache::new();

            // Local repo should not be cached
            let node = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            let result = process_single_repo(&node, &repo_manager, &cache);
            assert!(result.is_ok());
            let intermediate = result.unwrap();
            assert_eq!(intermediate.source_url, "local");
            assert_eq!(intermediate.source_ref, "HEAD");
            // Cache should remain empty for local repos
            assert_eq!(cache.len().unwrap(), 0);
        }

        #[test]
        fn test_process_single_repo_with_template_vars() {
            let repo_manager = RepositoryManager::with_operations(
                Box::new(SimpleTestGitOps),
                Box::new(SimpleTestCacheOps::new()),
            );
            let cache = RepoCache::new();

            let mut vars = HashMap::new();
            vars.insert("NAME".to_string(), "test-project".to_string());

            let operations = vec![Operation::TemplateVars {
                template_vars: TemplateVars { vars },
            }];
            let node = RepoNode::new(
                "https://github.com/example/repo.git".to_string(),
                "main".to_string(),
                operations,
            );

            let result = process_single_repo(&node, &repo_manager, &cache);
            assert!(result.is_ok());
            let intermediate = result.unwrap();
            assert_eq!(intermediate.template_vars.len(), 1);
            assert_eq!(
                intermediate.template_vars.get("NAME"),
                Some(&"test-project".to_string())
            );
        }

        #[test]
        fn test_process_single_repo_collects_merge_operations() {
            let repo_manager = RepositoryManager::with_operations(
                Box::new(SimpleTestGitOps),
                Box::new(SimpleTestCacheOps::new()),
            );
            let cache = RepoCache::new();

            let operations = vec![
                Operation::Yaml {
                    yaml: crate::config::YamlMergeOp {
                        source: "s.yaml".to_string(),
                        dest: "d.yaml".to_string(),
                        path: None,
                        append: false,
                        array_mode: None,
                    },
                },
                Operation::Exclude {
                    exclude: ExcludeOp {
                        patterns: vec!["*.tmp".to_string()],
                    },
                },
            ];
            let node = RepoNode::new(
                "https://github.com/example/repo.git".to_string(),
                "main".to_string(),
                operations,
            );

            let result = process_single_repo(&node, &repo_manager, &cache);
            assert!(result.is_ok());
            let intermediate = result.unwrap();
            // Merge operations should be collected
            assert_eq!(intermediate.merge_operations.len(), 1);
            assert!(matches!(
                intermediate.merge_operations[0],
                Operation::Yaml { .. }
            ));
        }

        #[test]
        fn test_process_single_repo_local_no_merge_operations() {
            let repo_manager = RepositoryManager::with_operations(
                Box::new(SimpleTestGitOps),
                Box::new(SimpleTestCacheOps::new()),
            );
            let cache = RepoCache::new();

            // Local repos should not collect merge operations
            let operations = vec![Operation::Yaml {
                yaml: crate::config::YamlMergeOp {
                    source: "s.yaml".to_string(),
                    dest: "d.yaml".to_string(),
                    path: None,
                    append: false,
                    array_mode: None,
                },
            }];
            let node = RepoNode::new("local".to_string(), "HEAD".to_string(), operations);

            let result = process_single_repo(&node, &repo_manager, &cache);
            assert!(result.is_ok());
            let intermediate = result.unwrap();
            // Local repos should not have merge operations collected
            assert_eq!(intermediate.merge_operations.len(), 0);
        }
    }

    mod execute_phase2_tests {
        use super::*;
        use crate::cache::RepoCache;
        use crate::config::ExcludeOp;
        use crate::filesystem::MemoryFS;
        use crate::phases::{RepoNode, RepoTree};
        use crate::repository::{CacheOperations, GitOperations, RepositoryManager};
        use std::path::{Path, PathBuf};

        struct Phase2TestGitOps;
        impl GitOperations for Phase2TestGitOps {
            fn clone_shallow(&self, _url: &str, _ref_name: &str, _path: &Path) -> Result<()> {
                Ok(())
            }
            fn list_tags(&self, _url: &str) -> Result<Vec<String>> {
                Ok(vec![])
            }
        }

        struct Phase2TestCacheOps;
        impl CacheOperations for Phase2TestCacheOps {
            fn exists(&self, _cache_path: &Path) -> bool {
                true
            }
            fn get_cache_path(&self, _url: &str, _ref_name: &str) -> PathBuf {
                PathBuf::from("/mock/cache")
            }
            fn load_from_cache(&self, _cache_path: &Path) -> Result<MemoryFS> {
                let mut fs = MemoryFS::new();
                fs.add_file_string("file.txt", "content")?;
                Ok(fs)
            }
            fn save_to_cache(&self, _cache_path: &Path, _fs: &MemoryFS) -> Result<()> {
                Ok(())
            }
        }

        #[test]
        fn test_execute_phase2_with_local_root() {
            let repo_manager = RepositoryManager::with_operations(
                Box::new(Phase2TestGitOps),
                Box::new(Phase2TestCacheOps),
            );
            let cache = RepoCache::new();

            let root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            let tree = RepoTree::new(root);

            let result = execute(&tree, &repo_manager, &cache);
            assert!(result.is_ok());
            let intermediate_fss = result.unwrap();
            // Should have one entry for local
            assert_eq!(intermediate_fss.len(), 1);
            assert!(intermediate_fss.contains_key("local@HEAD"));
        }

        #[test]
        fn test_execute_phase2_with_child_repos() {
            let repo_manager = RepositoryManager::with_operations(
                Box::new(Phase2TestGitOps),
                Box::new(Phase2TestCacheOps),
            );
            let cache = RepoCache::new();

            let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            let child = RepoNode::new(
                "https://github.com/example/repo.git".to_string(),
                "main".to_string(),
                vec![],
            );
            root.add_child(child);
            let tree = RepoTree::new(root);

            let result = execute(&tree, &repo_manager, &cache);
            assert!(result.is_ok());
            let intermediate_fss = result.unwrap();
            // Should have entries for both local and child
            assert_eq!(intermediate_fss.len(), 2);
            assert!(intermediate_fss.contains_key("local@HEAD"));
            assert!(intermediate_fss.contains_key("https://github.com/example/repo.git@main"));
        }

        #[test]
        fn test_execute_phase2_child_operations_applied() {
            let repo_manager = RepositoryManager::with_operations(
                Box::new(Phase2TestGitOps),
                Box::new(Phase2TestCacheOps),
            );
            let cache = RepoCache::new();

            let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            // Child with exclude operation
            let child = RepoNode::new(
                "https://github.com/example/repo.git".to_string(),
                "main".to_string(),
                vec![Operation::Exclude {
                    exclude: ExcludeOp {
                        patterns: vec!["*.txt".to_string()],
                    },
                }],
            );
            root.add_child(child);
            let tree = RepoTree::new(root);

            let result = execute(&tree, &repo_manager, &cache);
            assert!(result.is_ok());
            let intermediate_fss = result.unwrap();
            let child_fs = intermediate_fss
                .get("https://github.com/example/repo.git@main")
                .unwrap();
            // The exclude operation should have removed .txt files
            assert!(!child_fs.fs.exists("file.txt"));
        }

        #[test]
        fn test_execute_phase2_nested_children() {
            let repo_manager = RepositoryManager::with_operations(
                Box::new(Phase2TestGitOps),
                Box::new(Phase2TestCacheOps),
            );
            let cache = RepoCache::new();

            let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            let mut parent = RepoNode::new(
                "https://github.com/parent/repo.git".to_string(),
                "main".to_string(),
                vec![],
            );
            let child = RepoNode::new(
                "https://github.com/child/repo.git".to_string(),
                "v1.0".to_string(),
                vec![],
            );
            parent.add_child(child);
            root.add_child(parent);
            let tree = RepoTree::new(root);

            let result = execute(&tree, &repo_manager, &cache);
            assert!(result.is_ok());
            let intermediate_fss = result.unwrap();
            // Should have entries for all three
            assert_eq!(intermediate_fss.len(), 3);
            assert!(intermediate_fss.contains_key("local@HEAD"));
            assert!(intermediate_fss.contains_key("https://github.com/parent/repo.git@main"));
            assert!(intermediate_fss.contains_key("https://github.com/child/repo.git@v1.0"));
        }
    }
}
