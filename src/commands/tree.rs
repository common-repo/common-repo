//! # Tree Command Implementation
//!
//! This module implements the `tree` subcommand, which displays the repository
//! inheritance tree in a hierarchical format.
//!
//! ## Functionality
//!
//! - **Repository Tree Visualization**: Displays the inheritance hierarchy of repositories
//! - **Depth Control**: Supports `--depth` flag to limit tree depth
//! - **URL and Ref Display**: Shows repository URLs and their references
//!
//! This command is a safe, read-only operation that does not modify any files.

use anyhow::Result;
use clap::Args;
use ptree::{print_tree, TreeItem};
use std::path::PathBuf;

use common_repo::config;
use common_repo::phases::{discover_repos, RepoNode};
use common_repo::repository::RepositoryManager;

/// Display the repository inheritance tree
#[derive(Args, Debug)]
pub struct TreeArgs {
    /// Path to the .common-repo.yaml configuration file.
    #[arg(short, long, value_name = "FILE", default_value = ".common-repo.yaml")]
    pub config: PathBuf,

    /// The root directory for the repository cache.
    ///
    /// If not provided, it defaults to the system's cache directory
    /// (e.g., `~/.cache/common-repo` on Linux).
    /// Can also be set with the `COMMON_REPO_CACHE` environment variable.
    #[arg(long, value_name = "DIR", env = "COMMON_REPO_CACHE")]
    pub cache_root: Option<PathBuf>,

    /// Maximum depth to display in the tree.
    ///
    /// If not specified, displays the full tree.
    /// Use 0 to show only the root level, 1 to show one level of inheritance, etc.
    #[arg(long, value_name = "NUM")]
    pub depth: Option<usize>,
}

/// Execute the `tree` command.
///
/// This function handles the logic for the `tree` subcommand. It loads the
/// configuration file, discovers the repository inheritance tree, and displays
/// it in a hierarchical format.
pub fn execute(args: TreeArgs) -> Result<()> {
    let config_path = &args.config;
    println!(
        "🌳 Repository inheritance tree for: {}",
        config_path.display()
    );

    // Load configuration
    let schema = config::from_file(config_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to load config from {}: {}",
            config_path.display(),
            e
        )
    })?;

    // Initialize repository manager
    let cache_root = args.cache_root.unwrap_or_else(|| {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".common-repo-cache"))
            .join("common-repo")
    });

    let repo_manager = RepositoryManager::new(cache_root);

    // Discover repository tree
    let repo_tree = discover_repos(&schema, &repo_manager)
        .map_err(|e| anyhow::anyhow!("Failed to discover repository tree: {}", e))?;

    // Build and display tree
    let tree_root = build_tree_node(&repo_tree.root, args.depth.unwrap_or(usize::MAX), 0);
    print_tree(&tree_root).map_err(|e| anyhow::anyhow!("Failed to display tree: {}", e))?;

    Ok(())
}

/// Build a tree node from a repository node
fn build_tree_node(repo_node: &RepoNode, max_depth: usize, current_depth: usize) -> TreeNode {
    let label = format!("{} @ {}", repo_node.url, repo_node.ref_);

    if current_depth >= max_depth || repo_node.children.is_empty() {
        TreeNode {
            label,
            children: vec![],
        }
    } else {
        let children = repo_node
            .children
            .iter()
            .map(|child| build_tree_node(child, max_depth, current_depth + 1))
            .collect();
        TreeNode { label, children }
    }
}

/// Tree node structure for ptree visualization
#[derive(Clone)]
struct TreeNode {
    label: String,
    children: Vec<TreeNode>,
}

impl TreeItem for TreeNode {
    type Child = TreeNode;

    fn write_self<W: std::io::Write>(
        &self,
        f: &mut W,
        _style: &ptree::Style,
    ) -> std::io::Result<()> {
        write!(f, "{}", self.label)
    }

    fn children(&self) -> std::borrow::Cow<'_, [Self::Child]> {
        std::borrow::Cow::Borrowed(&self.children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_execute_missing_config() {
        let args = TreeArgs {
            config: PathBuf::from("/nonexistent/config.yaml"),
            cache_root: None,
            depth: None,
        };

        let result = execute(args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to load config"));
    }

    /// Test tree command with repository configuration.
    /// This test requires network access to fetch repositories.
    #[test]
    #[cfg_attr(not(feature = "integration-tests"), ignore)]
    fn test_execute_with_simple_config() {
        // Skip if network tests are disabled
        if std::env::var("SKIP_NETWORK_TESTS").is_ok() {
            println!("Skipping network test");
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        // Use a real, small test repository
        let config_content = r#"
- repo:
    url: https://github.com/octocat/Hello-World
    ref: master
"#;

        std::fs::write(&config_path, config_content).unwrap();

        let args = TreeArgs {
            config: config_path,
            cache_root: Some(temp_dir.path().to_path_buf()),
            depth: Some(1),
        };

        // This should succeed (though it will print output)
        let result = execute(args);
        assert!(result.is_ok());
    }
}
