//! Phase 1: Discovery and Cloning
//!
//! This is the first phase of the `common-repo` execution pipeline. Its primary
//! responsibilities are to discover all inherited repositories and then clone
//! them in parallel.
//!
//! ## Process
//!
//! 1.  **Discovery (`discover_repos`)**: The process begins by parsing the root
//!     `.common-repo.yaml` file. It then recursively fetches the configuration
//!     from each inherited repository, building a complete dependency tree
//!     (`RepoTree`). A breadth-first traversal is used to ensure that all
//!     repositories at a given depth are discovered before moving to the next
//!     level.
//!
//! 2.  **Deferred Operations**: When fetching an upstream repository's config,
//!     deferred operations (marked with `defer: true` or `auto-merge`) are
//!     extracted and prepended to the consumer's `with:` operations. This allows
//!     upstream repos to declare how their files should be merged rather than copied.
//!
//! 3.  **Cycle Detection**: During discovery, the process keeps track of the
//!     inheritance path and will abort if a circular dependency is detected
//!     (e.g., repository A inherits from B, which in turn inherits from A).
//!
//! 4.  **Parallel Cloning (`clone_parallel`)**: Once the complete dependency
//!     tree is built, all the repositories are cloned in parallel to maximize
//!     performance. The `RepositoryManager` is used for this, which automatically
//!     handles on-disk caching to avoid re-downloading repositories that are
//!     already up to date.
//!
//! This phase ensures that all the necessary source material is available locally
//! before the processing and merging phases begin.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use log::warn;
use rayon::prelude::*;

use super::{RepoNode, RepoTree};
use crate::cache::RepoCache;
use crate::config::{Operation, Schema};
use crate::defaults::{ALT_CONFIG_FILENAME, DEFAULT_CONFIG_FILENAME};
use crate::error::{Error, Result};
use crate::repository::RepositoryManager;

/// A deduplicated key used in cycle-detection and the discovery visited set.
///
/// Two local-directory references to the same canonical path are considered
/// the same key regardless of how the path was spelled in the config (e.g.
/// `./alpha` vs `../consumer/../alpha`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum VisitKey {
    Git { url: String, ref_: String },
    Local { canonical: PathBuf },
}

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
    discover_repos_with_parent(config, std::env::current_dir()?.as_path(), repo_manager)
}

/// Discover repositories with an explicit parent directory for local path resolution.
///
/// Like [`discover_repos`] but allows the caller to supply the directory that
/// the config was read from. Relative local-path references in `config` are
/// resolved against `parent_dir`.
pub(crate) fn discover_repos_with_parent(
    config: &Schema,
    parent_dir: &Path,
    repo_manager: &RepositoryManager,
) -> Result<RepoTree> {
    let root_node = process_config_to_node(config, parent_dir)?;
    let root_node =
        discover_inherited_configs(root_node, parent_dir, repo_manager, &mut HashSet::new())?;
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
    parent_dir: &Path,
    repo_manager: &RepositoryManager,
    visited: &mut HashSet<VisitKey>,
) -> Result<RepoNode> {
    let mut new_children = Vec::new();

    for child in node.children {
        if child.url == "local" {
            new_children.push(child);
            continue;
        }

        let visit_key = if child.is_local() {
            VisitKey::Local {
                canonical: PathBuf::from(&child.url),
            }
        } else {
            VisitKey::Git {
                url: child.url.clone(),
                ref_: child.ref_.clone(),
            }
        };

        if visited.contains(&visit_key) {
            continue;
        }
        visited.insert(visit_key.clone());

        // The directory the child's own .common-repo.yaml sits in:
        // for local children, the canonical path on the node; for git
        // children, the cache checkout directory (not threaded here — we
        // use cwd as a best-effort fallback since nested git configs
        // already work without parent-dir resolution).
        let child_parent = if child.is_local() {
            PathBuf::from(&child.url)
        } else {
            parent_dir.to_path_buf()
        };

        match fetch_and_parse_config(&child.url, &child.ref_, repo_manager) {
            Ok(inherited_config) => {
                let upstream_filtering_ops = extract_upstream_operations(&inherited_config);
                let deferred_ops = extract_deferred_operations(&inherited_config);

                let inherited_node = process_config_to_node(&inherited_config, &child_parent)?;
                let inherited_node = discover_inherited_configs(
                    inherited_node,
                    &child_parent,
                    repo_manager,
                    visited,
                )?;

                let mut combined_operations = upstream_filtering_ops;
                combined_operations.extend(deferred_ops);
                combined_operations.extend(child.operations.clone());

                let mut combined_node = RepoNode {
                    url: child.url.clone(),
                    ref_: child.ref_.clone(),
                    original_url: child.original_url.clone(),
                    children: Vec::new(),
                    operations: combined_operations,
                };
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

        visited.remove(&visit_key);
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
    let config_content = match fs.get_file(DEFAULT_CONFIG_FILENAME) {
        Some(file) => file.content.clone(),
        None => {
            // Try .commonrepo.yaml as fallback
            match fs.get_file(ALT_CONFIG_FILENAME) {
                Some(file) => file.content.clone(),
                None => {
                    return Err(Error::ConfigParse {
                        message: format!(
                            "No {} or {} found in repository",
                            DEFAULT_CONFIG_FILENAME, ALT_CONFIG_FILENAME
                        ),
                        hint: Some(format!(
                            "Create a {} file in the repository root",
                            DEFAULT_CONFIG_FILENAME
                        )),
                    });
                }
            }
        }
    };

    // Parse the YAML content
    let yaml_str = String::from_utf8(config_content).map_err(|_| Error::ConfigParse {
        message: format!("Invalid UTF-8 in {}", DEFAULT_CONFIG_FILENAME),
        hint: Some("Ensure the file is saved with UTF-8 encoding".to_string()),
    })?;

    crate::config::parse(&yaml_str)
}

/// Extract deferred operations from an upstream repository's config
///
/// Deferred operations have `defer: true` or `auto-merge` set.
/// These will be applied before the consumer's `with:` operations.
fn extract_deferred_operations(config: &Schema) -> Vec<Operation> {
    config
        .iter()
        .filter(|op| op.is_deferred())
        .cloned()
        .collect()
}

/// Extract upstream operations from an upstream repository's config
///
/// Upstream operations are include, exclude, rename, template, and template-vars
/// operations that define how an upstream repo exposes files to consumers.
/// These are applied BEFORE deferred operations and consumer's with: clause.
fn extract_upstream_operations(config: &Schema) -> Vec<Operation> {
    config
        .iter()
        .filter(|op| {
            matches!(
                op,
                Operation::Include { .. }
                    | Operation::Exclude { .. }
                    | Operation::Rename { .. }
                    | Operation::Template { .. }
                    | Operation::TemplateVars { .. }
            )
        })
        .cloned()
        .collect()
}

/// Detect cycles in the repository dependency tree
///
/// A cycle occurs when a repository appears multiple times in a single dependency path
/// (from root to leaf). Multiple branches can reference the same repo - that's allowed.
fn detect_cycles(node: &RepoNode, path: &mut Vec<VisitKey>) -> Result<()> {
    if node.url != "local" {
        let key = if node.is_local() {
            VisitKey::Local {
                canonical: PathBuf::from(&node.url),
            }
        } else {
            VisitKey::Git {
                url: node.url.clone(),
                ref_: node.ref_.clone(),
            }
        };

        if path.contains(&key) {
            let mut cycle_path: Vec<String> = path
                .iter()
                .map(|k| match k {
                    VisitKey::Git { url, ref_ } => format!("{url}@{ref_}"),
                    VisitKey::Local { canonical } => canonical.to_string_lossy().into_owned(),
                })
                .collect();
            cycle_path.push(match &key {
                VisitKey::Git { url, ref_ } => format!("{url}@{ref_}"),
                VisitKey::Local { canonical } => canonical.to_string_lossy().into_owned(),
            });
            return Err(Error::CycleDetected {
                cycle: cycle_path.join(" -> "),
            });
        }

        path.push(key);
    }

    for child in &node.children {
        detect_cycles(child, path)?;
    }

    if node.url != "local" {
        path.pop();
    }

    Ok(())
}

/// Convert a configuration into a repository node
///
/// Extracts repo operations as child nodes and keeps other operations in the root node.
/// Relative and absolute local-path URLs are canonicalised against `defining_config_dir`.
fn process_config_to_node(config: &Schema, defining_config_dir: &Path) -> Result<RepoNode> {
    let mut repo_operations = Vec::new();
    let mut other_operations = Vec::new();

    for operation in config {
        match operation {
            Operation::Repo { repo } => repo_operations.push(repo.clone()),
            _ => other_operations.push(operation.clone()),
        }
    }

    let mut root_node = RepoNode::new("local".to_string(), "HEAD".to_string(), other_operations);

    for repo_op in repo_operations {
        if repo_op.url == "local" {
            return Err(Error::CycleDetected {
                cycle: format!("{}@{}", repo_op.url, repo_op.r#ref.as_deref().unwrap_or("")),
            });
        }

        let child_operations = repo_op.with.clone();

        let child_node = if repo_op.is_local() {
            // Resolve the URL against defining_config_dir
            let candidate = if repo_op.url.starts_with('/') {
                PathBuf::from(&repo_op.url)
            } else {
                defining_config_dir.join(&repo_op.url)
            };
            let canonical =
                std::fs::canonicalize(&candidate).map_err(|e| Error::LocalPathNotFound {
                    original: repo_op.url.clone(),
                    attempted: candidate,
                    source: e,
                })?;
            if !canonical.is_dir() {
                return Err(Error::LocalPathNotDirectory { path: canonical });
            }

            RepoNode {
                url: canonical.to_string_lossy().into_owned(),
                ref_: String::new(),
                original_url: Some(repo_op.url.clone()),
                children: Vec::new(),
                operations: child_operations,
            }
        } else {
            RepoNode::new(
                repo_op.url,
                repo_op.r#ref.unwrap_or_default(),
                child_operations,
            )
        };

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
/// Clones all repositories at the same depth level in parallel using rayon.
/// This is the default behavior - no CLI flag needed to enable it.
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

        // Clone all repos at current depth level in parallel
        // Collect errors from all parallel operations
        let errors: Mutex<Vec<Error>> = Mutex::new(Vec::new());

        repos_to_clone.par_iter().for_each(|(url, ref_)| {
            // Try to fetch the repository
            if let Err(e) = repo_manager.fetch_repository(url, ref_) {
                // Check if this is a network-related error and if we have a cached version
                let is_network_error =
                    matches!(e, Error::GitClone { .. }) || matches!(e, Error::Network { .. });

                if is_network_error && repo_manager.is_cached(url, ref_) {
                    // Fall back to cached version with warning
                    warn!(
                        "Network fetch failed for {}@{}, falling back to cached version",
                        url, ref_
                    );
                    // Continue - the repository is already cached and will be used
                } else {
                    // Either not a network error, or no cache available - collect the error
                    errors.lock().unwrap().push(e);
                }
            }
        });

        // Return the first error if any occurred
        let collected_errors = errors.into_inner().unwrap();
        if let Some(first_error) = collected_errors.into_iter().next() {
            return Err(first_error);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ExcludeOp, IncludeOp, RepoOp};
    use crate::filesystem::MemoryFS;
    use crate::repository::{CacheOperations, GitOperations};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    // ========================================================================
    // Mock implementations for testing
    // ========================================================================

    /// Mock git operations for testing
    struct MockGitOperations {
        clone_calls: Arc<Mutex<Vec<(String, String, PathBuf)>>>,
        should_fail: bool,
        fail_with_network_error: bool,
        error_message: String,
    }

    impl MockGitOperations {
        fn new() -> Self {
            Self {
                clone_calls: Arc::new(Mutex::new(Vec::new())),
                should_fail: false,
                fail_with_network_error: false,
                error_message: String::new(),
            }
        }

        fn with_network_error(message: String) -> Self {
            Self {
                clone_calls: Arc::new(Mutex::new(Vec::new())),
                should_fail: true,
                fail_with_network_error: true,
                error_message: message,
            }
        }

        fn with_non_network_error(message: String) -> Self {
            Self {
                clone_calls: Arc::new(Mutex::new(Vec::new())),
                should_fail: true,
                fail_with_network_error: false,
                error_message: message,
            }
        }
    }

    impl GitOperations for MockGitOperations {
        fn clone_shallow(&self, url: &str, ref_name: &str, target_dir: &Path) -> Result<()> {
            self.clone_calls.lock().unwrap().push((
                url.to_string(),
                ref_name.to_string(),
                target_dir.to_path_buf(),
            ));
            if self.should_fail {
                if self.fail_with_network_error {
                    Err(Error::GitClone {
                        url: url.to_string(),
                        r#ref: ref_name.to_string(),
                        message: self.error_message.clone(),
                        hint: None,
                    })
                } else {
                    Err(Error::ConfigParse {
                        message: self.error_message.clone(),
                        hint: None,
                    })
                }
            } else {
                Ok(())
            }
        }

        fn list_tags(&self, _url: &str) -> Result<Vec<String>> {
            Ok(vec!["v1.0.0".to_string(), "v2.0.0".to_string()])
        }
    }

    /// Mock cache operations for testing
    struct MockCacheOperations {
        cached_repos: Arc<Mutex<Vec<PathBuf>>>,
        cache_root: PathBuf,
        filesystem: MemoryFS,
    }

    impl MockCacheOperations {
        fn new() -> Self {
            Self {
                cached_repos: Arc::new(Mutex::new(Vec::new())),
                cache_root: PathBuf::from("/mock/cache"),
                filesystem: MemoryFS::new(),
            }
        }

        fn with_cached(paths: Vec<PathBuf>) -> Self {
            Self {
                cached_repos: Arc::new(Mutex::new(paths)),
                cache_root: PathBuf::from("/mock/cache"),
                filesystem: MemoryFS::new(),
            }
        }
    }

    impl CacheOperations for MockCacheOperations {
        fn exists(&self, cache_path: &Path) -> bool {
            self.cached_repos
                .lock()
                .unwrap()
                .contains(&cache_path.to_path_buf())
        }

        fn get_cache_path(&self, url: &str, ref_name: &str) -> PathBuf {
            self.cache_root
                .join(format!("{}-{}", url.replace(['/', ':'], "-"), ref_name))
        }

        fn load_from_cache(&self, _cache_path: &Path) -> Result<MemoryFS> {
            Ok(self.filesystem.clone())
        }

        fn save_to_cache(&self, cache_path: &Path, _fs: &MemoryFS) -> Result<()> {
            self.cached_repos
                .lock()
                .unwrap()
                .push(cache_path.to_path_buf());
            Ok(())
        }
    }

    // ========================================================================
    // Tests for process_config_to_node
    // ========================================================================

    #[test]
    fn test_process_config_to_node_empty_config() {
        let config: Schema = vec![];
        let result = process_config_to_node(&config, std::env::current_dir().unwrap().as_path());

        assert!(result.is_ok());
        let node = result.unwrap();
        assert_eq!(node.url, "local");
        assert_eq!(node.ref_, "HEAD");
        assert!(node.children.is_empty());
        assert!(node.operations.is_empty());
    }

    #[test]
    fn test_process_config_to_node_non_repo_operations() {
        let config: Schema = vec![
            Operation::Include {
                include: IncludeOp {
                    patterns: vec!["*.rs".to_string()],
                },
            },
            Operation::Exclude {
                exclude: ExcludeOp {
                    patterns: vec!["*.tmp".to_string()],
                },
            },
        ];

        let result = process_config_to_node(&config, std::env::current_dir().unwrap().as_path());
        assert!(result.is_ok());

        let node = result.unwrap();
        assert_eq!(node.url, "local");
        assert!(node.children.is_empty());
        assert_eq!(node.operations.len(), 2);
    }

    #[test]
    fn test_process_config_to_node_repo_operations_become_children() {
        let config: Schema = vec![Operation::Repo {
            repo: RepoOp {
                url: "https://github.com/example/repo".to_string(),
                r#ref: Some("main".to_string()),
                path: None,
                with: vec![],
            },
        }];

        let result = process_config_to_node(&config, std::env::current_dir().unwrap().as_path());
        assert!(result.is_ok());

        let node = result.unwrap();
        assert_eq!(node.url, "local");
        assert_eq!(node.children.len(), 1);
        assert_eq!(node.children[0].url, "https://github.com/example/repo");
        assert_eq!(node.children[0].ref_, "main");
        assert!(node.operations.is_empty());
    }

    #[test]
    fn test_process_config_to_node_mixed_operations() {
        let config: Schema = vec![
            Operation::Include {
                include: IncludeOp {
                    patterns: vec!["*.rs".to_string()],
                },
            },
            Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/example/repo1".to_string(),
                    r#ref: Some("v1.0".to_string()),
                    path: None,
                    with: vec![Operation::Exclude {
                        exclude: ExcludeOp {
                            patterns: vec!["tests/**".to_string()],
                        },
                    }],
                },
            },
            Operation::Exclude {
                exclude: ExcludeOp {
                    patterns: vec!["*.tmp".to_string()],
                },
            },
            Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/example/repo2".to_string(),
                    r#ref: Some("main".to_string()),
                    path: None,
                    with: vec![],
                },
            },
        ];

        let result = process_config_to_node(&config, std::env::current_dir().unwrap().as_path());
        assert!(result.is_ok());

        let node = result.unwrap();
        assert_eq!(node.url, "local");
        // Non-repo operations stay in root
        assert_eq!(node.operations.len(), 2);
        // Repo operations become children
        assert_eq!(node.children.len(), 2);
        assert_eq!(node.children[0].url, "https://github.com/example/repo1");
        assert_eq!(node.children[0].ref_, "v1.0");
        // The with clause operations are preserved
        assert_eq!(node.children[0].operations.len(), 1);
        assert_eq!(node.children[1].url, "https://github.com/example/repo2");
    }

    #[test]
    fn test_process_config_to_node_local_url_error() {
        // Using "local" as a repo URL should trigger a cycle detection error
        let config: Schema = vec![Operation::Repo {
            repo: RepoOp {
                url: "local".to_string(),
                r#ref: Some("HEAD".to_string()),
                path: None,
                with: vec![],
            },
        }];

        let result = process_config_to_node(&config, std::env::current_dir().unwrap().as_path());
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(matches!(error, Error::CycleDetected { .. }));
        assert!(error.to_string().contains("local@HEAD"));
    }

    // ========================================================================
    // Tests for detect_cycles
    // ========================================================================

    #[test]
    fn test_detect_cycles_no_cycle_single_node() {
        let node = RepoNode::new(
            "https://github.com/example/repo".to_string(),
            "main".to_string(),
            vec![],
        );

        let result = detect_cycles(&node, &mut Vec::new());
        assert!(result.is_ok());
    }

    #[test]
    fn test_detect_cycles_no_cycle_linear_chain() {
        let mut grandchild = RepoNode::new(
            "https://github.com/example/repo-c".to_string(),
            "main".to_string(),
            vec![],
        );
        grandchild.children = vec![];

        let mut child = RepoNode::new(
            "https://github.com/example/repo-b".to_string(),
            "main".to_string(),
            vec![],
        );
        child.children = vec![grandchild];

        let mut root = RepoNode::new(
            "https://github.com/example/repo-a".to_string(),
            "main".to_string(),
            vec![],
        );
        root.children = vec![child];

        let result = detect_cycles(&root, &mut Vec::new());
        assert!(result.is_ok());
    }

    #[test]
    fn test_detect_cycles_no_cycle_same_repo_different_branches() {
        // Same repo can appear in different branches - that's allowed
        let child1 = RepoNode::new(
            "https://github.com/example/shared".to_string(),
            "main".to_string(),
            vec![],
        );
        let child2 = RepoNode::new(
            "https://github.com/example/shared".to_string(),
            "main".to_string(),
            vec![],
        );

        let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
        root.children = vec![child1, child2];

        let result = detect_cycles(&root, &mut Vec::new());
        assert!(result.is_ok());
    }

    #[test]
    fn test_detect_cycles_direct_cycle() {
        // Create a cycle: A -> B -> A
        let mut node_a = RepoNode::new(
            "https://github.com/example/repo-a".to_string(),
            "main".to_string(),
            vec![],
        );

        let mut node_b = RepoNode::new(
            "https://github.com/example/repo-b".to_string(),
            "main".to_string(),
            vec![],
        );

        // B points back to A (cycle)
        let node_a_copy = RepoNode::new(
            "https://github.com/example/repo-a".to_string(),
            "main".to_string(),
            vec![],
        );
        node_b.children = vec![node_a_copy];

        node_a.children = vec![node_b];

        let result = detect_cycles(&node_a, &mut Vec::new());
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(matches!(error, Error::CycleDetected { .. }));
        let error_msg = error.to_string();
        assert!(error_msg.contains("Cycle detected"));
        assert!(error_msg.contains("repo-a"));
        assert!(error_msg.contains("repo-b"));
    }

    #[test]
    fn test_detect_cycles_longer_cycle() {
        // Create a longer cycle: A -> B -> C -> A
        let node_a_copy = RepoNode::new(
            "https://github.com/example/repo-a".to_string(),
            "main".to_string(),
            vec![],
        );

        let mut node_c = RepoNode::new(
            "https://github.com/example/repo-c".to_string(),
            "main".to_string(),
            vec![],
        );
        node_c.children = vec![node_a_copy]; // C -> A (cycle)

        let mut node_b = RepoNode::new(
            "https://github.com/example/repo-b".to_string(),
            "main".to_string(),
            vec![],
        );
        node_b.children = vec![node_c]; // B -> C

        let mut node_a = RepoNode::new(
            "https://github.com/example/repo-a".to_string(),
            "main".to_string(),
            vec![],
        );
        node_a.children = vec![node_b]; // A -> B

        let result = detect_cycles(&node_a, &mut Vec::new());
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(matches!(error, Error::CycleDetected { .. }));
    }

    #[test]
    fn test_detect_cycles_skips_local_root() {
        // Local root nodes should be skipped in cycle detection
        let child = RepoNode::new(
            "https://github.com/example/repo".to_string(),
            "main".to_string(),
            vec![],
        );

        let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
        root.children = vec![child];

        let result = detect_cycles(&root, &mut Vec::new());
        assert!(result.is_ok());
    }

    #[test]
    fn test_detect_cycles_different_refs_not_a_cycle() {
        // Same URL but different refs are considered different repos
        let child = RepoNode::new(
            "https://github.com/example/repo".to_string(),
            "v2.0".to_string(), // Different ref
            vec![],
        );

        let mut parent = RepoNode::new(
            "https://github.com/example/repo".to_string(),
            "v1.0".to_string(),
            vec![],
        );
        parent.children = vec![child];

        let result = detect_cycles(&parent, &mut Vec::new());
        assert!(result.is_ok());
    }

    // ========================================================================
    // Tests for clone_parallel
    // ========================================================================

    #[test]
    fn test_clone_parallel_empty_tree() {
        let root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
        let tree = RepoTree::new(root);

        let git_ops = Box::new(MockGitOperations::new());
        let cache_ops = Box::new(MockCacheOperations::new());
        let repo_manager = RepositoryManager::with_operations(git_ops, cache_ops);
        let cache = RepoCache::new();

        let result = clone_parallel(&tree, &repo_manager, &cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_clone_parallel_single_repo() {
        let child = RepoNode::new(
            "https://github.com/example/repo".to_string(),
            "main".to_string(),
            vec![],
        );
        let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
        root.children = vec![child];
        let tree = RepoTree::new(root);

        let git_ops = Box::new(MockGitOperations::new());
        let clone_calls = git_ops.clone_calls.clone();
        let cache_ops = Box::new(MockCacheOperations::new());
        let repo_manager = RepositoryManager::with_operations(git_ops, cache_ops);
        let cache = RepoCache::new();

        let result = clone_parallel(&tree, &repo_manager, &cache);
        assert!(result.is_ok());

        let calls = clone_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "https://github.com/example/repo");
        assert_eq!(calls[0].1, "main");
    }

    #[test]
    fn test_clone_parallel_multiple_repos_at_same_level() {
        let child1 = RepoNode::new(
            "https://github.com/example/repo1".to_string(),
            "main".to_string(),
            vec![],
        );
        let child2 = RepoNode::new(
            "https://github.com/example/repo2".to_string(),
            "v1.0".to_string(),
            vec![],
        );
        let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
        root.children = vec![child1, child2];
        let tree = RepoTree::new(root);

        let git_ops = Box::new(MockGitOperations::new());
        let clone_calls = git_ops.clone_calls.clone();
        let cache_ops = Box::new(MockCacheOperations::new());
        let repo_manager = RepositoryManager::with_operations(git_ops, cache_ops);
        let cache = RepoCache::new();

        let result = clone_parallel(&tree, &repo_manager, &cache);
        assert!(result.is_ok());

        let calls = clone_calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
    }

    #[test]
    fn test_clone_parallel_nested_repos() {
        let grandchild = RepoNode::new(
            "https://github.com/example/repo-c".to_string(),
            "main".to_string(),
            vec![],
        );
        let mut child = RepoNode::new(
            "https://github.com/example/repo-b".to_string(),
            "main".to_string(),
            vec![],
        );
        child.children = vec![grandchild];

        let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
        root.children = vec![child];
        let tree = RepoTree::new(root);

        let git_ops = Box::new(MockGitOperations::new());
        let clone_calls = git_ops.clone_calls.clone();
        let cache_ops = Box::new(MockCacheOperations::new());
        let repo_manager = RepositoryManager::with_operations(git_ops, cache_ops);
        let cache = RepoCache::new();

        let result = clone_parallel(&tree, &repo_manager, &cache);
        assert!(result.is_ok());

        let calls = clone_calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        // First level cloned first
        assert_eq!(calls[0].0, "https://github.com/example/repo-b");
        // Then second level
        assert_eq!(calls[1].0, "https://github.com/example/repo-c");
    }

    #[test]
    fn test_clone_parallel_network_error_with_cache_fallback() {
        let child = RepoNode::new(
            "https://github.com/example/repo".to_string(),
            "main".to_string(),
            vec![],
        );
        let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
        root.children = vec![child];
        let tree = RepoTree::new(root);

        let git_ops = Box::new(MockGitOperations::with_network_error(
            "Connection refused".to_string(),
        ));
        // Pre-populate cache so fallback works
        let cache_path = PathBuf::from("/mock/cache/https---github.com-example-repo-main");
        let cache_ops = Box::new(MockCacheOperations::with_cached(vec![cache_path]));
        let repo_manager = RepositoryManager::with_operations(git_ops, cache_ops);
        let cache = RepoCache::new();

        // Should succeed because we fall back to cache
        let result = clone_parallel(&tree, &repo_manager, &cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_clone_parallel_network_error_without_cache_fails() {
        let child = RepoNode::new(
            "https://github.com/example/repo".to_string(),
            "main".to_string(),
            vec![],
        );
        let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
        root.children = vec![child];
        let tree = RepoTree::new(root);

        let git_ops = Box::new(MockGitOperations::with_network_error(
            "Connection refused".to_string(),
        ));
        // No cache available
        let cache_ops = Box::new(MockCacheOperations::new());
        let repo_manager = RepositoryManager::with_operations(git_ops, cache_ops);
        let cache = RepoCache::new();

        // Should fail because no cache fallback available
        let result = clone_parallel(&tree, &repo_manager, &cache);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(matches!(error, Error::GitClone { .. }));
    }

    #[test]
    fn test_clone_parallel_non_network_error_propagates() {
        let child = RepoNode::new(
            "https://github.com/example/repo".to_string(),
            "main".to_string(),
            vec![],
        );
        let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
        root.children = vec![child];
        let tree = RepoTree::new(root);

        let git_ops = Box::new(MockGitOperations::with_non_network_error(
            "Some other error".to_string(),
        ));
        // No cache available - non-network errors should propagate
        let cache_ops = Box::new(MockCacheOperations::new());
        let repo_manager = RepositoryManager::with_operations(git_ops, cache_ops);
        let cache = RepoCache::new();

        // Should fail because non-network errors propagate
        let result = clone_parallel(&tree, &repo_manager, &cache);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(matches!(error, Error::ConfigParse { .. }));
    }

    /// Mock git operations that can selectively fail for specific URLs
    struct SelectiveFailGitOperations {
        clone_calls: Arc<Mutex<Vec<(String, String, PathBuf)>>>,
        fail_urls: Vec<String>,
    }

    impl SelectiveFailGitOperations {
        fn with_fail_urls(urls: Vec<String>) -> Self {
            Self {
                clone_calls: Arc::new(Mutex::new(Vec::new())),
                fail_urls: urls,
            }
        }
    }

    impl GitOperations for SelectiveFailGitOperations {
        fn clone_shallow(&self, url: &str, ref_name: &str, target_dir: &Path) -> Result<()> {
            self.clone_calls.lock().unwrap().push((
                url.to_string(),
                ref_name.to_string(),
                target_dir.to_path_buf(),
            ));
            if self.fail_urls.contains(&url.to_string()) {
                Err(Error::GitClone {
                    url: url.to_string(),
                    r#ref: ref_name.to_string(),
                    message: "Clone failed".to_string(),
                    hint: None,
                })
            } else {
                Ok(())
            }
        }

        fn list_tags(&self, _url: &str) -> Result<Vec<String>> {
            Ok(vec!["v1.0.0".to_string()])
        }
    }

    #[test]
    fn test_clone_parallel_multiple_repos_one_fails() {
        // Test that when multiple repos are cloned in parallel and one fails,
        // we get an error even though others succeed
        let child1 = RepoNode::new(
            "https://github.com/example/repo1".to_string(),
            "main".to_string(),
            vec![],
        );
        let child2 = RepoNode::new(
            "https://github.com/example/repo2".to_string(),
            "main".to_string(),
            vec![],
        );
        let child3 = RepoNode::new(
            "https://github.com/example/repo3".to_string(),
            "main".to_string(),
            vec![],
        );
        let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
        root.children = vec![child1, child2, child3];
        let tree = RepoTree::new(root);

        // Only repo2 fails
        let git_ops = Box::new(SelectiveFailGitOperations::with_fail_urls(vec![
            "https://github.com/example/repo2".to_string(),
        ]));
        let clone_calls = git_ops.clone_calls.clone();
        let cache_ops = Box::new(MockCacheOperations::new());
        let repo_manager = RepositoryManager::with_operations(git_ops, cache_ops);
        let cache = RepoCache::new();

        // Should fail because one repo failed
        let result = clone_parallel(&tree, &repo_manager, &cache);
        assert!(result.is_err());

        // But all repos should have been attempted (parallel execution)
        let calls = clone_calls.lock().unwrap();
        assert_eq!(calls.len(), 3);

        // Error should be from repo2
        let error = result.unwrap_err();
        assert!(
            matches!(error, Error::GitClone { url, .. } if url == "https://github.com/example/repo2")
        );
    }

    // ========================================================================
    // Integration tests for discover_repos
    // ========================================================================

    #[test]
    fn test_discover_repos_empty_config() {
        let config: Schema = vec![];

        let git_ops = Box::new(MockGitOperations::new());
        let cache_ops = Box::new(MockCacheOperations::new());
        let repo_manager = RepositoryManager::with_operations(git_ops, cache_ops);

        let result = discover_repos(&config, &repo_manager);
        assert!(result.is_ok());

        let tree = result.unwrap();
        assert_eq!(tree.root.url, "local");
        assert!(tree.root.children.is_empty());
    }

    #[test]
    fn test_discover_repos_with_operations_only() {
        let config: Schema = vec![
            Operation::Include {
                include: IncludeOp {
                    patterns: vec!["*.rs".to_string()],
                },
            },
            Operation::Exclude {
                exclude: ExcludeOp {
                    patterns: vec!["*.tmp".to_string()],
                },
            },
        ];

        let git_ops = Box::new(MockGitOperations::new());
        let cache_ops = Box::new(MockCacheOperations::new());
        let repo_manager = RepositoryManager::with_operations(git_ops, cache_ops);

        let result = discover_repos(&config, &repo_manager);
        assert!(result.is_ok());

        let tree = result.unwrap();
        assert_eq!(tree.root.url, "local");
        assert!(tree.root.children.is_empty());
        assert_eq!(tree.root.operations.len(), 2);
    }

    // ========================================================================
    // Tests for extract_deferred_operations
    // ========================================================================

    mod extract_deferred_ops_tests {
        use super::*;
        use crate::config::{ExcludeOp, IncludeOp, MarkdownMergeOp, Operation, YamlMergeOp};

        #[test]
        fn test_extract_deferred_operations_empty_config() {
            let config: Schema = vec![];
            let result = extract_deferred_operations(&config);
            assert!(result.is_empty());
        }

        #[test]
        fn test_extract_deferred_operations_no_deferred() {
            let config: Schema = vec![
                Operation::Include {
                    include: IncludeOp {
                        patterns: vec!["*.rs".to_string()],
                    },
                },
                Operation::Exclude {
                    exclude: ExcludeOp {
                        patterns: vec!["*.tmp".to_string()],
                    },
                },
                // Regular merge op (not deferred)
                Operation::Yaml {
                    yaml: YamlMergeOp {
                        source: Some("s.yaml".to_string()),
                        dest: Some("d.yaml".to_string()),
                        ..Default::default()
                    },
                },
            ];
            let result = extract_deferred_operations(&config);
            assert!(result.is_empty());
        }

        #[test]
        fn test_extract_deferred_operations_with_auto_merge() {
            let config: Schema = vec![
                Operation::Include {
                    include: IncludeOp {
                        patterns: vec!["*.rs".to_string()],
                    },
                },
                Operation::Yaml {
                    yaml: YamlMergeOp {
                        auto_merge: Some("config.yaml".to_string()),
                        ..Default::default()
                    },
                },
            ];
            let result = extract_deferred_operations(&config);
            assert_eq!(result.len(), 1);
            assert!(matches!(result[0], Operation::Yaml { .. }));
        }

        #[test]
        fn test_extract_deferred_operations_with_defer_flag() {
            let config: Schema = vec![Operation::Yaml {
                yaml: YamlMergeOp {
                    source: Some("s.yaml".to_string()),
                    dest: Some("d.yaml".to_string()),
                    defer: Some(true),
                    ..Default::default()
                },
            }];
            let result = extract_deferred_operations(&config);
            assert_eq!(result.len(), 1);
        }

        #[test]
        fn test_extract_deferred_operations_multiple() {
            let config: Schema = vec![
                Operation::Include {
                    include: IncludeOp {
                        patterns: vec!["**/*".to_string()],
                    },
                },
                // First deferred op
                Operation::Yaml {
                    yaml: YamlMergeOp {
                        auto_merge: Some("config.yaml".to_string()),
                        ..Default::default()
                    },
                },
                // Non-deferred merge op
                Operation::Yaml {
                    yaml: YamlMergeOp {
                        source: Some("other.yaml".to_string()),
                        dest: Some("out.yaml".to_string()),
                        ..Default::default()
                    },
                },
                // Second deferred op
                Operation::Markdown {
                    markdown: MarkdownMergeOp {
                        auto_merge: Some("CLAUDE.md".to_string()),
                        section: "## Rules".to_string(),
                        ..Default::default()
                    },
                },
            ];
            let result = extract_deferred_operations(&config);
            assert_eq!(result.len(), 2);
            assert!(matches!(result[0], Operation::Yaml { .. }));
            assert!(matches!(result[1], Operation::Markdown { .. }));
        }

        #[test]
        fn test_extract_deferred_operations_preserves_order() {
            let config: Schema = vec![
                Operation::Markdown {
                    markdown: MarkdownMergeOp {
                        auto_merge: Some("A.md".to_string()),
                        section: "## A".to_string(),
                        ..Default::default()
                    },
                },
                Operation::Yaml {
                    yaml: YamlMergeOp {
                        auto_merge: Some("B.yaml".to_string()),
                        ..Default::default()
                    },
                },
                Operation::Markdown {
                    markdown: MarkdownMergeOp {
                        auto_merge: Some("C.md".to_string()),
                        section: "## C".to_string(),
                        ..Default::default()
                    },
                },
            ];
            let result = extract_deferred_operations(&config);
            assert_eq!(result.len(), 3);
            // Check order is preserved
            match &result[0] {
                Operation::Markdown { markdown } => {
                    assert_eq!(markdown.auto_merge, Some("A.md".to_string()));
                }
                _ => panic!("Expected Markdown"),
            }
            match &result[1] {
                Operation::Yaml { yaml } => {
                    assert_eq!(yaml.auto_merge, Some("B.yaml".to_string()));
                }
                _ => panic!("Expected Yaml"),
            }
            match &result[2] {
                Operation::Markdown { markdown } => {
                    assert_eq!(markdown.auto_merge, Some("C.md".to_string()));
                }
                _ => panic!("Expected Markdown"),
            }
        }
    }

    // ========================================================================
    // Tests for extract_upstream_operations
    // ========================================================================

    mod extract_upstream_ops_tests {
        use super::*;
        use crate::config::{JsonMergeOp, RenameOp, TemplateOp, TemplateVars, YamlMergeOp};

        #[test]
        fn test_extracts_include_operations() {
            let config = vec![
                Operation::Include {
                    include: IncludeOp {
                        patterns: vec!["*.md".to_string()],
                    },
                },
                Operation::Repo {
                    repo: RepoOp {
                        url: "example".to_string(),
                        r#ref: Some("main".to_string()),
                        path: None,
                        with: vec![],
                    },
                },
            ];
            let result = extract_upstream_operations(&config);
            assert_eq!(result.len(), 1);
            match &result[0] {
                Operation::Include { include } => {
                    assert_eq!(include.patterns, vec!["*.md".to_string()]);
                }
                _ => panic!("Expected Include operation"),
            }
        }

        #[test]
        fn test_extracts_exclude_operations() {
            let config = vec![
                Operation::Exclude {
                    exclude: ExcludeOp {
                        patterns: vec!["*.bak".to_string()],
                    },
                },
                Operation::Repo {
                    repo: RepoOp {
                        url: "example".to_string(),
                        r#ref: Some("main".to_string()),
                        path: None,
                        with: vec![],
                    },
                },
            ];
            let result = extract_upstream_operations(&config);
            assert_eq!(result.len(), 1);
            match &result[0] {
                Operation::Exclude { exclude } => {
                    assert_eq!(exclude.patterns, vec!["*.bak".to_string()]);
                }
                _ => panic!("Expected Exclude operation"),
            }
        }

        #[test]
        fn test_extracts_rename_operations() {
            let config = vec![Operation::Rename {
                rename: RenameOp { mappings: vec![] },
            }];
            let result = extract_upstream_operations(&config);
            assert_eq!(result.len(), 1);
            matches!(&result[0], Operation::Rename { .. });
        }

        #[test]
        fn test_extracts_all_filtering_operations() {
            let config = vec![
                Operation::Include {
                    include: IncludeOp {
                        patterns: vec!["src/**".to_string()],
                    },
                },
                Operation::Exclude {
                    exclude: ExcludeOp {
                        patterns: vec!["*.test".to_string()],
                    },
                },
                Operation::Rename {
                    rename: RenameOp { mappings: vec![] },
                },
                // Non-filtering operations should be ignored
                Operation::Repo {
                    repo: RepoOp {
                        url: "example".to_string(),
                        r#ref: Some("main".to_string()),
                        path: None,
                        with: vec![],
                    },
                },
            ];
            let result = extract_upstream_operations(&config);
            assert_eq!(result.len(), 3);
        }

        #[test]
        fn test_preserves_order() {
            let config = vec![
                Operation::Include {
                    include: IncludeOp {
                        patterns: vec!["first".to_string()],
                    },
                },
                Operation::Exclude {
                    exclude: ExcludeOp {
                        patterns: vec!["second".to_string()],
                    },
                },
                Operation::Include {
                    include: IncludeOp {
                        patterns: vec!["third".to_string()],
                    },
                },
            ];
            let result = extract_upstream_operations(&config);
            assert_eq!(result.len(), 3);
            // Verify order is preserved
            match &result[0] {
                Operation::Include { include } => {
                    assert_eq!(include.patterns[0], "first");
                }
                _ => panic!("Expected Include"),
            }
            match &result[1] {
                Operation::Exclude { exclude } => {
                    assert_eq!(exclude.patterns[0], "second");
                }
                _ => panic!("Expected Exclude"),
            }
            match &result[2] {
                Operation::Include { include } => {
                    assert_eq!(include.patterns[0], "third");
                }
                _ => panic!("Expected Include"),
            }
        }

        #[test]
        fn test_empty_config_returns_empty() {
            let config: Vec<Operation> = vec![];
            let result = extract_upstream_operations(&config);
            assert!(result.is_empty());
        }

        #[test]
        fn test_ignores_deferred_operations() {
            let config = vec![
                Operation::Yaml {
                    yaml: YamlMergeOp {
                        defer: Some(true),
                        ..Default::default()
                    },
                },
                Operation::Json {
                    json: JsonMergeOp {
                        auto_merge: Some("test.json".to_string()),
                        ..Default::default()
                    },
                },
            ];
            let result = extract_upstream_operations(&config);
            assert!(result.is_empty());
        }

        #[test]
        fn test_extracts_template_operations() {
            let config = vec![
                Operation::Template {
                    template: TemplateOp {
                        patterns: vec!["*.yaml".to_string()],
                    },
                },
                Operation::Repo {
                    repo: RepoOp {
                        url: "example".to_string(),
                        r#ref: Some("main".to_string()),
                        path: None,
                        with: vec![],
                    },
                },
            ];
            let result = extract_upstream_operations(&config);
            assert_eq!(result.len(), 1);
            match &result[0] {
                Operation::Template { template } => {
                    assert_eq!(template.patterns, vec!["*.yaml".to_string()]);
                }
                _ => panic!("Expected Template operation"),
            }
        }

        #[test]
        fn test_extracts_template_vars_operations() {
            let config = vec![
                Operation::TemplateVars {
                    template_vars: TemplateVars {
                        vars: {
                            let mut m = std::collections::HashMap::new();
                            m.insert("APP_ID".to_string(), "MY_APP_ID".to_string());
                            m.insert("APP_KEY".to_string(), "MY_APP_KEY".to_string());
                            m
                        },
                    },
                },
                Operation::Repo {
                    repo: RepoOp {
                        url: "example".to_string(),
                        r#ref: Some("main".to_string()),
                        path: None,
                        with: vec![],
                    },
                },
            ];
            let result = extract_upstream_operations(&config);
            assert_eq!(result.len(), 1);
            match &result[0] {
                Operation::TemplateVars { template_vars } => {
                    assert_eq!(
                        template_vars.vars.get("APP_ID"),
                        Some(&"MY_APP_ID".to_string())
                    );
                    assert_eq!(
                        template_vars.vars.get("APP_KEY"),
                        Some(&"MY_APP_KEY".to_string())
                    );
                }
                _ => panic!("Expected TemplateVars operation"),
            }
        }

        #[test]
        fn test_extracts_all_upstream_operations_including_templates() {
            let config = vec![
                Operation::Include {
                    include: IncludeOp {
                        patterns: vec!["src/**".to_string()],
                    },
                },
                Operation::Template {
                    template: TemplateOp {
                        patterns: vec!["*.yaml".to_string()],
                    },
                },
                Operation::TemplateVars {
                    template_vars: TemplateVars {
                        vars: {
                            let mut m = std::collections::HashMap::new();
                            m.insert("KEY".to_string(), "VALUE".to_string());
                            m
                        },
                    },
                },
                Operation::Exclude {
                    exclude: ExcludeOp {
                        patterns: vec!["*.test".to_string()],
                    },
                },
                Operation::Rename {
                    rename: RenameOp { mappings: vec![] },
                },
                // Non-upstream operations should be ignored
                Operation::Repo {
                    repo: RepoOp {
                        url: "example".to_string(),
                        r#ref: Some("main".to_string()),
                        path: None,
                        with: vec![],
                    },
                },
            ];
            let result = extract_upstream_operations(&config);
            assert_eq!(result.len(), 5);
            assert!(matches!(&result[0], Operation::Include { .. }));
            assert!(matches!(&result[1], Operation::Template { .. }));
            assert!(matches!(&result[2], Operation::TemplateVars { .. }));
            assert!(matches!(&result[3], Operation::Exclude { .. }));
            assert!(matches!(&result[4], Operation::Rename { .. }));
        }
    }

    #[test]
    fn test_extract_upstream_operations_excludes_self() {
        use crate::config::{IncludeOp, Operation, SelfOp};

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

        let upstream_ops = extract_upstream_operations(&config);
        assert_eq!(upstream_ops.len(), 1);
        assert!(matches!(upstream_ops[0], Operation::Include { .. }));
    }

    #[test]
    fn test_process_config_to_node_keeps_self_in_operations() {
        use crate::config::{IncludeOp, Operation, RepoOp, SelfOp};

        let config = vec![
            Operation::Repo {
                repo: RepoOp {
                    url: "https://github.com/example/repo".to_string(),
                    r#ref: Some("main".to_string()),
                    path: None,
                    with: vec![],
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

        let node =
            process_config_to_node(&config, std::env::current_dir().unwrap().as_path()).unwrap();
        assert_eq!(node.children.len(), 1); // one repo child
        assert_eq!(node.operations.len(), 1); // self goes to operations
        assert!(matches!(node.operations[0], Operation::Self_ { .. }));
    }

    #[test]
    fn process_config_to_node_resolves_local_relative_path() {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let alpha_dir = tmp.path().join("alpha");
        let beta_dir = tmp.path().join("beta");
        std::fs::create_dir(&alpha_dir).unwrap();
        std::fs::create_dir(&beta_dir).unwrap();
        std::fs::write(alpha_dir.join(".common-repo.yaml"), b"- include: ['**']\n").unwrap();

        // beta/.common-repo.yaml references ../alpha
        let config: Schema = vec![Operation::Repo {
            repo: RepoOp {
                url: "../alpha".to_string(),
                r#ref: None,
                path: None,
                with: vec![],
            },
        }];

        let node = process_config_to_node(&config, &beta_dir).unwrap();
        assert_eq!(node.children.len(), 1);
        let child = &node.children[0];
        assert_eq!(
            child.url,
            std::fs::canonicalize(&alpha_dir).unwrap().to_string_lossy()
        );
        assert_eq!(child.original_url.as_deref(), Some("../alpha"));
        assert_eq!(child.ref_, "");
    }

    #[test]
    fn process_config_to_node_errors_when_local_path_missing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config: Schema = vec![Operation::Repo {
            repo: RepoOp {
                url: "./nope".to_string(),
                r#ref: None,
                path: None,
                with: vec![],
            },
        }];
        let err = process_config_to_node(&config, tmp.path()).unwrap_err();
        assert!(
            matches!(err, Error::LocalPathNotFound { .. }),
            "expected LocalPathNotFound, got {err:?}"
        );
    }

    #[test]
    fn process_config_to_node_errors_when_local_path_is_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let file = tmp.path().join("a-file.txt");
        std::fs::write(&file, b"x").unwrap();

        let config: Schema = vec![Operation::Repo {
            repo: RepoOp {
                url: "./a-file.txt".to_string(),
                r#ref: None,
                path: None,
                with: vec![],
            },
        }];
        let err = process_config_to_node(&config, tmp.path()).unwrap_err();
        assert!(matches!(err, Error::LocalPathNotDirectory { .. }));
    }

    #[test]
    fn process_config_to_node_absolute_local_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sub = tmp.path().join("abs-target");
        std::fs::create_dir(&sub).unwrap();

        let config: Schema = vec![Operation::Repo {
            repo: RepoOp {
                url: sub.to_string_lossy().into_owned(),
                r#ref: None,
                path: None,
                with: vec![],
            },
        }];
        // parent_dir here is irrelevant for an absolute-path URL
        let node = process_config_to_node(&config, tmp.path()).unwrap();
        assert_eq!(node.children.len(), 1);
        assert_eq!(
            node.children[0].url,
            std::fs::canonicalize(&sub).unwrap().to_string_lossy()
        );
    }
}
