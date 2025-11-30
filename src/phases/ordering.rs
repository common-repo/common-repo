//! Phase 3: Determining Operation Order
//!
//! This is the third phase of the `common-repo` execution pipeline. Its main
//! responsibility is to determine the order in which repositories should be
//! merged to achieve deterministic, predictable results.
//!
//! ## Process
//!
//! 1.  **Depth-First Traversal**: The process traverses the `RepoTree` from the
//!     root, visiting children before their parents (post-order traversal).
//!
//! 2.  **Dependency Ordering**: By visiting children first, we ensure that
//!     dependencies (inherited repositories) are processed before the
//!     repositories that depend on them. This guarantees that base configurations
//!     are applied before derived ones.
//!
//! 3.  **Visited Tracking**: A `HashSet` tracks visited nodes to prevent
//!     processing the same repository twice (though this shouldn't happen in a
//!     proper tree structure).
//!
//! This phase produces an `OperationOrder` containing repository keys in the
//! correct merge order, ready for Phase 4's composite filesystem construction.

use std::collections::HashSet;

use super::{OperationOrder, RepoNode, RepoTree};
use crate::error::Result;

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
fn build_order_recursive(node: &RepoNode, order: &mut Vec<String>, visited: &mut HashSet<String>) {
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

#[cfg(test)]
mod tests {
    use super::{execute, RepoNode, RepoTree};

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
        let order = execute(&tree).unwrap();

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
        let order = execute(&tree).unwrap();

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
        let order = execute(&tree).unwrap();

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
        let order = execute(&tree).unwrap();

        // Should only contain local
        assert_eq!(order.len(), 1);
        assert_eq!(order.order[0], "local@HEAD");
    }
}
