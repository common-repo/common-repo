//! # Info Command Implementation
//!
//! This module implements the `info` subcommand, which displays information
//! about the current `.common-repo.yaml` configuration file.
//!
//! ## Functionality
//!
//! - **Configuration Overview**: Displays the configuration file path and basic statistics
//! - **Repository Information**: Lists all inherited repositories with their refs and cache status
//! - **Operation Breakdown**: Counts and displays operations by type
//!
//! This command is a safe, read-only operation that does not modify any files.

use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

use common_repo::config;
use common_repo::repository::RepositoryManager;

/// Show information about a repository or the current configuration
#[derive(Args, Debug)]
pub struct InfoArgs {
    /// Path to the .common-repo.yaml configuration file.
    #[arg(short, long, value_name = "FILE", default_value = ".common-repo.yaml")]
    pub config: PathBuf,

    /// The root directory for the repository cache.
    ///
    /// Defaults to the system cache directory (`~/.cache/common-repo` on Linux,
    /// `~/Library/Caches/common-repo` on macOS).
    /// Can also be set with the `COMMON_REPO_CACHE` environment variable.
    #[arg(long, value_name = "DIR", env = "COMMON_REPO_CACHE")]
    pub cache_root: Option<PathBuf>,
}

/// Execute the `info` command.
///
/// This function handles the logic for the `info` subcommand. It loads the
/// configuration file and displays comprehensive information about it.
pub fn execute(args: InfoArgs) -> Result<()> {
    let config_path = &args.config;
    println!("📋 Configuration: {}", config_path.display());

    // Load configuration
    let schema = config::from_file(config_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to load config from {}: {}",
            config_path.display(),
            e
        )
    })?;

    // Initialize repository manager for cache checking
    let cache_root = args
        .cache_root
        .unwrap_or_else(common_repo::defaults::default_cache_root);

    let repo_manager = RepositoryManager::new(cache_root);

    // Count operations by type
    let operation_counts = count_operations(&schema);

    // Extract repository operations
    let repo_operations: Vec<_> = schema
        .iter()
        .filter_map(|op| match op {
            config::Operation::Repo { repo } => Some(repo.clone()),
            _ => None,
        })
        .collect();

    // Check cache status for each repository
    let repo_cache_status = check_repo_cache_status(&repo_operations, &repo_manager);

    // Format and display output
    display_info(&operation_counts, &repo_operations, &repo_cache_status);

    Ok(())
}

/// Count operations by type from the configuration schema.
fn count_operations(schema: &config::Schema) -> OperationCounts {
    let mut counts = OperationCounts::default();

    for operation in schema {
        match operation {
            config::Operation::Repo { .. } => counts.repo += 1,
            config::Operation::Include { .. } => counts.include += 1,
            config::Operation::Exclude { .. } => counts.exclude += 1,
            config::Operation::Rename { .. } => counts.rename += 1,
            config::Operation::Template { .. } => counts.template += 1,
            config::Operation::Tools { .. } => counts.tools += 1,
            config::Operation::TemplateVars { .. } => counts.template_vars += 1,
            config::Operation::Yaml { .. } => counts.yaml += 1,
            config::Operation::Json { .. } => counts.json += 1,
            config::Operation::Toml { .. } => counts.toml += 1,
            config::Operation::Ini { .. } => counts.ini += 1,
            config::Operation::Markdown { .. } => counts.markdown += 1,
        }
    }

    counts
}

/// Check cache status for each repository operation.
fn check_repo_cache_status(
    repo_operations: &[config::RepoOp],
    repo_manager: &RepositoryManager,
) -> Vec<bool> {
    repo_operations
        .iter()
        .map(|repo| {
            if let Some(path) = &repo.path {
                repo_manager.is_cached_with_path(&repo.url, &repo.r#ref, Some(path))
            } else {
                repo_manager.is_cached(&repo.url, &repo.r#ref)
            }
        })
        .collect()
}

/// Display the configuration information.
fn display_info(
    operation_counts: &OperationCounts,
    repo_operations: &[config::RepoOp],
    repo_cache_status: &[bool],
) {
    // Display repository information
    println!("\nInherited repositories: {}", repo_operations.len());
    for (repo, &is_cached) in repo_operations.iter().zip(repo_cache_status.iter()) {
        let cache_status = if is_cached {
            "(cached)"
        } else {
            "(not cached)"
        };
        let path_info = if let Some(path) = &repo.path {
            format!(" path:{}", path)
        } else {
            String::new()
        };
        println!(
            "  • {} @ {}{} {}",
            repo.url, repo.r#ref, path_info, cache_status
        );
    }

    // Display operation counts
    let total_operations: usize = operation_counts.repo
        + operation_counts.include
        + operation_counts.exclude
        + operation_counts.rename
        + operation_counts.template
        + operation_counts.tools
        + operation_counts.template_vars
        + operation_counts.yaml
        + operation_counts.json
        + operation_counts.toml
        + operation_counts.ini
        + operation_counts.markdown;

    println!("\nOperations: {}", total_operations);

    if operation_counts.repo > 0 {
        println!("  • {} repo operations", operation_counts.repo);
    }
    if operation_counts.include > 0 {
        println!("  • {} include operations", operation_counts.include);
    }
    if operation_counts.exclude > 0 {
        println!("  • {} exclude operations", operation_counts.exclude);
    }
    if operation_counts.rename > 0 {
        println!("  • {} rename operations", operation_counts.rename);
    }
    if operation_counts.template > 0 {
        println!("  • {} template operations", operation_counts.template);
    }
    if operation_counts.tools > 0 {
        println!("  • {} tools operations", operation_counts.tools);
    }
    if operation_counts.template_vars > 0 {
        println!(
            "  • {} template_vars operations",
            operation_counts.template_vars
        );
    }
    if operation_counts.yaml > 0 {
        println!("  • {} yaml merge operations", operation_counts.yaml);
    }
    if operation_counts.json > 0 {
        println!("  • {} json merge operations", operation_counts.json);
    }
    if operation_counts.toml > 0 {
        println!("  • {} toml merge operations", operation_counts.toml);
    }
    if operation_counts.ini > 0 {
        println!("  • {} ini merge operations", operation_counts.ini);
    }
    if operation_counts.markdown > 0 {
        println!(
            "  • {} markdown merge operations",
            operation_counts.markdown
        );
    }
}

/// Structure to hold operation counts by type.
#[derive(Debug, Default)]
struct OperationCounts {
    repo: usize,
    include: usize,
    exclude: usize,
    rename: usize,
    template: usize,
    tools: usize,
    template_vars: usize,
    yaml: usize,
    json: usize,
    toml: usize,
    ini: usize,
    markdown: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use common_repo::config::{ExcludeOp, IncludeOp, Operation, RepoOp};
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_count_operations() {
        let schema = vec![
            Operation::Repo {
                repo: RepoOp {
                    url: "https://example.com/repo".to_string(),
                    r#ref: "main".to_string(),
                    path: None,
                    with: vec![],
                },
            },
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
            Operation::Include {
                include: IncludeOp {
                    patterns: vec!["*.md".to_string()],
                },
            },
        ];

        let counts = count_operations(&schema);

        assert_eq!(counts.repo, 1);
        assert_eq!(counts.include, 2);
        assert_eq!(counts.exclude, 1);
        assert_eq!(counts.rename, 0);
        assert_eq!(counts.template, 0);
        assert_eq!(counts.tools, 0);
        assert_eq!(counts.template_vars, 0);
        assert_eq!(counts.yaml, 0);
        assert_eq!(counts.json, 0);
        assert_eq!(counts.toml, 0);
        assert_eq!(counts.ini, 0);
        assert_eq!(counts.markdown, 0);
    }

    #[test]
    fn test_execute_missing_config() {
        let args = InfoArgs {
            config: PathBuf::from("/nonexistent/config.yaml"),
            cache_root: None,
        };

        let result = execute(args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to load config"));
    }

    #[test]
    fn test_execute_with_temp_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        // Create a simple config file (YAML array format)
        let config_content = r#"
- repo:
    url: https://example.com/repo
    ref: main
- include:
    patterns: ["*.rs"]
- exclude:
    patterns: ["*.tmp"]
"#;

        std::fs::write(&config_path, config_content).unwrap();

        let args = InfoArgs {
            config: config_path,
            cache_root: Some(temp_dir.path().to_path_buf()),
        };

        // This should succeed (though it will print output)
        let result = execute(args);
        assert!(result.is_ok());
    }
}
