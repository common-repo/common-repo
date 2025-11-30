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
