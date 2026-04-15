//! Implementation of the 6 phases of the common-repo pull operation.
//!
//! ## Overview
//!
//! The pull operation follows 6 phases:
//! 1. Discovery and Cloning - Fetch all inherited repos in parallel (with automatic caching)
//! 2. Processing Individual Repos - Apply operations to each repo
//! 3. Determining Operation Order - Calculate deterministic merge order
//! 4. Composite Filesystem Construction - Merge all intermediate filesystems, collect deferred merge ops
//! 5. Local File Merging - Combine with local files (composite wins), execute deferred merges, consumer operations in declaration order
//! 6. Writing to Disk - Write final result to host filesystem
//!
//! Note: Caching happens automatically during Phase 1 via RepositoryManager, so there is no
//! separate cache update phase.
//!
//! Each phase depends only on the previous phases and the foundation layers (0-2).

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use log::warn;

use crate::config::Operation;
use crate::filesystem::MemoryFS;

/// Compute a unique lookup key from a repo's URL, ref, and operations.
///
/// The key format is `url@ref` for repos without operations, or
/// `url#ops-<hash>@ref` for repos with operations. This ensures that the
/// same repository referenced with different operations (e.g., different
/// `with:` template-vars) gets distinct keys in the intermediate filesystem
/// map and operation order.
///
/// Used by [`RepoNode::node_key`] and [`ClonedRepo::node_key`] for lookup
/// consistency, and by [`orchestrator::resolve_repo_inline`] when building
/// keys for nested `repo:` references without allocating a temporary struct.
pub(crate) fn compute_repo_key(url: &str, ref_: &str, operations: &[Operation]) -> String {
    if operations.is_empty() {
        return format!("{url}@{ref_}");
    }

    if let Some(fingerprint) = ops_fingerprint(operations) {
        format!("{url}#{fingerprint}@{ref_}")
    } else {
        warn!("Failed to serialize operations for repo key ({url}@{ref_}), using fallback key");
        format!("{url}@{ref_}")
    }
}

/// Compute a fingerprint string from a list of operations.
///
/// Serializes the operations to YAML and hashes the result, producing
/// a hex string like `ops-0123456789abcdef`. Returns `None` if
/// serialization fails.
pub(crate) fn ops_fingerprint(operations: &[Operation]) -> Option<String> {
    let serialized = serde_yaml::to_string(operations).ok()?;
    let mut hasher = DefaultHasher::new();
    serialized.hash(&mut hasher);
    Some(format!("ops-{:016x}", hasher.finish()))
}

// Phase modules - internal implementations
pub(crate) mod composite;
pub(crate) mod discovery;
pub(crate) mod local_merge;
pub(crate) mod ordering;
pub(crate) mod processing;
pub(crate) mod write;

// Public orchestrator for coordinating all phases
pub mod orchestrator;

// Re-export phase modules for internal use (crate-only aliases)
pub(crate) use composite as phase4;
pub(crate) use discovery as phase1;
pub(crate) use local_merge as phase5;
#[allow(unused_imports)]
pub(crate) use ordering as phase3;
pub(crate) use processing as phase2;
pub(crate) use write as phase6;

// Public re-exports for CLI commands
/// Discover all repositories in the inheritance tree.
///
/// This is re-exported for CLI command use (tree, validate).
pub use discovery::discover_repos;

/// Repository tree node representing inheritance hierarchy
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoNode {
    /// Repository URL. For local nodes after discovery, this is the canonical
    /// absolute path produced by `fs::canonicalize`.
    pub url: String,
    /// Git reference (tag, branch, commit). Empty string for local nodes.
    pub ref_: String,
    /// Original URL as written in the config (only populated for local nodes
    /// — preserves `./foo` spelling for error messages and display).
    pub original_url: Option<String>,
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
            original_url: None,
            children: Vec::new(),
            operations,
        }
    }

    pub fn add_child(&mut self, child: RepoNode) {
        self.children.push(child);
    }

    /// Generate a unique key for this node that includes the operations fingerprint.
    ///
    /// Delegates to [`compute_repo_key`] for the actual key format.
    pub fn node_key(&self) -> String {
        compute_repo_key(&self.url, &self.ref_, &self.operations)
    }

    /// Returns true when this node references a local filesystem path.
    ///
    /// Uses `original_url` when present (which it is for local nodes after
    /// discovery canonicalises the path); falls back to `url` for nodes that
    /// haven't been through discovery yet.
    pub fn is_local(&self) -> bool {
        let s = self.original_url.as_deref().unwrap_or(self.url.as_str());
        crate::repository::is_local_url(s)
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
    // Used in tests; available for future use in cycle detection API
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn would_create_cycle(&self, url: &str, ref_: &str) -> bool {
        self.all_repos
            .contains(&(url.to_string(), ref_.to_string()))
    }

    /// Collect all deferred merge operations from upstream nodes in the tree.
    ///
    /// Walks all non-root nodes and returns their deferred operations. These
    /// represent merge operations declared by upstream repositories, used by
    /// the provenance check to determine which merge sources are "known."
    pub fn collect_upstream_deferred_ops(&self) -> Vec<Operation> {
        let mut ops = Vec::new();
        for child in &self.root.children {
            Self::collect_deferred_ops_recursive(child, &mut ops);
        }
        ops
    }

    fn collect_deferred_ops_recursive(node: &RepoNode, ops: &mut Vec<Operation>) {
        for op in &node.operations {
            if op.is_deferred() {
                ops.push(op.clone());
            }
        }
        for child in &node.children {
            Self::collect_deferred_ops_recursive(child, ops);
        }
    }
}

/// A repository that has been cloned but not yet processed into an IntermediateFS.
///
/// Holds the raw filesystem content (with config files already stripped) plus
/// the metadata and operations needed for on-demand processing. This enables
/// lazy resolution: Phase 1 clones eagerly, but processing into an IntermediateFS
/// can happen on-demand when a `repo:` operation fires in the sequential pass.
#[derive(Debug, Clone)]
pub struct ClonedRepo {
    /// Raw filesystem content from the cloned repository (config files removed)
    pub(crate) fs: MemoryFS,
    /// Repository URL. For local nodes this is the canonical absolute path
    /// produced by `fs::canonicalize`.
    pub(crate) url: String,
    /// Git reference (tag, branch, commit). Empty string for local nodes.
    pub(crate) ref_: String,
    /// Original URL as written in the config (only populated for local nodes
    /// — preserves `./foo` / `../foo` spelling for orchestrator lookup, which
    /// sees the unresolved spelling in Operation::Repo entries).
    pub(crate) original_url: Option<String>,
    /// Operations to apply when this repo is processed (from `with:` clause +
    /// upstream filtering + deferred ops)
    pub(crate) operations: Vec<Operation>,
    /// Keys of child repos discovered during Phase 1 (from the repo's own
    /// `.common-repo.yaml` `repo:` entries). Used by `resolve_repo_inline` to
    /// integrate nested repos that were extracted as tree children rather than
    /// kept in `operations`.
    pub(crate) children_keys: Vec<String>,
}

impl ClonedRepo {
    pub fn new(fs: MemoryFS, url: String, ref_: String, operations: Vec<Operation>) -> Self {
        Self {
            fs,
            url,
            ref_,
            original_url: None,
            operations,
            children_keys: Vec::new(),
        }
    }

    /// Generate the same unique key as [`RepoNode::node_key`] for lookup consistency.
    ///
    /// Delegates to [`compute_repo_key`] for the actual key format.
    pub fn node_key(&self) -> String {
        compute_repo_key(&self.url, &self.ref_, &self.operations)
    }
}

/// Intermediate filesystem wrapper with metadata
#[derive(Debug, Clone)]
pub struct IntermediateFS {
    /// The processed filesystem
    pub fs: MemoryFS,
    /// Repository URL this FS came from (for debugging/tracking)
    pub upstream_url: String,
    /// Git reference used
    pub upstream_ref: String,
    /// Template variables collected from this repository's operations
    pub template_vars: HashMap<String, String>,
    /// Merge operations to be applied during Phase 4 composition
    pub merge_operations: Vec<Operation>,
}

impl IntermediateFS {
    pub fn new(fs: MemoryFS, upstream_url: String, upstream_ref: String) -> Self {
        Self {
            fs,
            upstream_url,
            upstream_ref,
            template_vars: HashMap::new(),
            merge_operations: Vec::new(),
        }
    }

    pub fn new_with_vars(
        fs: MemoryFS,
        upstream_url: String,
        upstream_ref: String,
        template_vars: HashMap<String, String>,
    ) -> Self {
        Self {
            fs,
            upstream_url,
            upstream_ref,
            template_vars,
            merge_operations: Vec::new(),
        }
    }

    pub fn new_with_vars_and_merges(
        fs: MemoryFS,
        upstream_url: String,
        upstream_ref: String,
        template_vars: HashMap<String, String>,
        merge_operations: Vec<Operation>,
    ) -> Self {
        Self {
            fs,
            upstream_url,
            upstream_ref,
            template_vars,
            merge_operations,
        }
    }
}

/// Operation order for deterministic merging
#[derive(Debug, Clone)]
pub struct OperationOrder {
    /// Ordered list of repository keys in the correct merge order
    /// Format: `url@ref` (e.g., `https://github.com/user/repo@main`)
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

#[cfg(test)]
mod phase_tests {
    use super::*;

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

        #[test]
        fn test_collect_upstream_deferred_ops_empty_tree() {
            let root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            let tree = RepoTree::new(root);

            let deferred = tree.collect_upstream_deferred_ops();
            assert!(deferred.is_empty());
        }

        #[test]
        fn test_collect_upstream_deferred_ops_with_deferred_child() {
            use crate::config::{JsonMergeOp, Operation};

            let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            let child = RepoNode::new(
                "https://github.com/upstream.git".to_string(),
                "main".to_string(),
                vec![Operation::Json {
                    json: JsonMergeOp {
                        source: Some("fragment.json".to_string()),
                        dest: Some("package.json".to_string()),
                        defer: Some(true),
                        ..Default::default()
                    },
                }],
            );
            root.add_child(child);
            let tree = RepoTree::new(root);

            let deferred = tree.collect_upstream_deferred_ops();
            assert_eq!(deferred.len(), 1);
            assert!(matches!(&deferred[0], Operation::Json { .. }));
        }

        #[test]
        fn test_collect_upstream_deferred_ops_skips_non_deferred() {
            use crate::config::{ExcludeOp, JsonMergeOp, Operation};

            let mut root = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            let child = RepoNode::new(
                "https://github.com/upstream.git".to_string(),
                "main".to_string(),
                vec![
                    // Non-deferred merge op
                    Operation::Json {
                        json: JsonMergeOp {
                            source: Some("local.json".to_string()),
                            dest: Some("output.json".to_string()),
                            ..Default::default()
                        },
                    },
                    // Non-merge op
                    Operation::Exclude {
                        exclude: ExcludeOp {
                            patterns: vec!["*.tmp".to_string()],
                        },
                    },
                ],
            );
            root.add_child(child);
            let tree = RepoTree::new(root);

            let deferred = tree.collect_upstream_deferred_ops();
            assert!(deferred.is_empty());
        }

        #[test]
        fn repo_node_is_local_uses_original_url_when_present() {
            // Local node after discovery: url = canonical abs, original_url = "./foo"
            let node = RepoNode {
                url: "/abs/foo".to_string(),
                ref_: String::new(),
                original_url: Some("./foo".to_string()),
                operations: vec![],
                children: vec![],
            };
            assert!(node.is_local());
        }

        #[test]
        fn repo_node_is_local_false_for_local_sentinel() {
            let node = RepoNode::new("local".to_string(), "HEAD".to_string(), vec![]);
            assert!(!node.is_local());
        }

        #[test]
        fn repo_node_is_local_false_for_git_url() {
            let node = RepoNode::new(
                "https://github.com/foo/bar".to_string(),
                "main".to_string(),
                vec![],
            );
            assert!(!node.is_local());
        }
    }
}
