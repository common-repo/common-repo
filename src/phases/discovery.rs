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
//! 2.  **Cycle Detection**: During discovery, the process keeps track of the
//!     inheritance path and will abort if a circular dependency is detected
//!     (e.g., repository A inherits from B, which in turn inherits from A).
//!
//! 3.  **Parallel Cloning (`clone_parallel`)**: Once the complete dependency
//!     tree is built, all the repositories are cloned in parallel to maximize
//!     performance. The `RepositoryManager` is used for this, which automatically
//!     handles on-disk caching to avoid re-downloading repositories that are
//!     already up to date.
//!
//! This phase ensures that all the necessary source material is available locally
//! before the processing and merging phases begin.

use std::collections::HashSet;

use log::warn;

use super::{RepoNode, RepoTree};
use crate::cache::RepoCache;
use crate::config::{Operation, Schema};
use crate::error::{Error, Result};
use crate::repository::RepositoryManager;

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
                    warn!(
                        "Network fetch failed for {}@{}, falling back to cached version",
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
