//! # Update Command Implementation
//!
//! This module implements the `update` subcommand, which provides functionality
//! for updating the Git references (e.g., tags, branches) of inherited
//! repositories in the `.common-repo.yaml` configuration file.
//!
//! ## Functionality
//!
//! - **Update Checking**: The command first checks for available updates by
//!   querying the remote Git repositories, similar to the `check --updates`
//!   command.
//!
//! - **Update Modes**:
//!   - By default, or with the `--compatible` flag, it only considers updates
//!     that are compatible (minor or patch version increases).
//!   - With the `--latest` flag, it will also include updates with breaking
//!     changes (major version increases).
//!
//! - **Interactive Confirmation**: Before making any changes, it presents a
//!   summary of the proposed updates and prompts the user for confirmation.
//!   This can be bypassed with the `--yes` flag.
//!
//! - **Configuration Modification**: If confirmed, the command modifies the
//!   `.common-repo.yaml` file in place, updating the `ref` for each repository
//!   to the selected newer version.
//!
//! - **Dry Run**: A `--dry-run` mode is available to show what would be updated
//!   without actually modifying the configuration file.

use anyhow::Result;
use clap::Args;
use std::fs;
use std::path::PathBuf;

use common_repo::config;
use common_repo::repository::RepositoryManager;
use common_repo::version;

/// Update repository refs to newer versions
#[derive(Args, Debug)]
pub struct UpdateArgs {
    /// Path to the .common-repo.yaml configuration file to update.
    #[arg(short, long, value_name = "FILE", default_value = ".common-repo.yaml")]
    pub config: PathBuf,

    /// The root directory for the repository cache.
    ///
    /// If not provided, it defaults to the system's cache directory.
    /// Can also be set with the `COMMON_REPO_CACHE` environment variable.
    #[arg(long, value_name = "DIR", env = "COMMON_REPO_CACHE")]
    pub cache_root: Option<PathBuf>,

    /// If set, the command will update to the latest compatible versions
    /// (minor and patch updates only). This is the default behavior.
    #[arg(long)]
    pub compatible: bool,

    /// If set, the command will update to the latest available versions,
    /// including those with breaking changes (major version updates).
    #[arg(long)]
    pub latest: bool,

    /// If set, the command will not ask for confirmation before updating all
    /// eligible repositories.
    #[arg(long)]
    pub yes: bool,

    /// If set, the command will show what would be updated without making any
    /// changes to the configuration file.
    #[arg(long)]
    pub dry_run: bool,
}

/// Execute the `update` command.
///
/// This function handles the logic for the `update` subcommand. It checks for
/// repository updates, prompts the user for confirmation, and then modifies the
/// `.common-repo.yaml` file to update the repository references.
pub fn execute(args: UpdateArgs) -> Result<()> {
    // Load configuration
    let config_path = &args.config;
    println!("Loading configuration from: {}", config_path.display());

    let mut schema = config::from_file(config_path).map_err(|e| {
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

    // Check for updates
    println!("Checking for repository updates...");
    let updates = version::check_updates(&schema, &repo_manager)?;

    if updates.is_empty() {
        println!("✅ No repositories found that can be checked for updates.");
        return Ok(());
    }

    // Filter updates based on flags
    let relevant_updates: Vec<_> = updates
        .into_iter()
        .filter(|update| {
            if args.latest {
                // Show all updates when --latest is used
                update.breaking_changes || update.compatible_updates
            } else if args.compatible {
                // Show only compatible updates when --compatible is used
                update.compatible_updates && !update.breaking_changes
            } else {
                // Default: show compatible updates only
                update.compatible_updates && !update.breaking_changes
            }
        })
        .collect();

    if relevant_updates.is_empty() {
        if args.latest {
            println!("✅ All repositories are already at the latest versions!");
        } else {
            println!("✅ No compatible updates available. Use --latest to see breaking changes.");
        }
        return Ok(());
    }

    // Display available updates
    println!("\n📦 Available Updates:");
    println!("{} repositories can be updated\n", relevant_updates.len());

    for update in &relevant_updates {
        println!("🔄 {} (current: {})", update.url, update.current_ref);

        if let Some(latest) = &update.latest_version {
            println!("   Latest: {}", latest);

            if update.breaking_changes {
                println!("   ⚠️  BREAKING CHANGES (major version update)");
            } else if update.compatible_updates {
                println!("   ✅ Compatible update");
            }
        }

        println!();
    }

    if args.dry_run {
        println!("ℹ️  Dry run mode - no changes will be made.");
        return Ok(());
    }

    // Confirm updates unless --yes flag is used
    if !args.yes {
        println!("Do you want to update these repositories? (y/N): ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input != "y" && input != "yes" {
            println!("Update cancelled.");
            return Ok(());
        }
    }

    // Perform updates
    println!("\n🔄 Updating repositories...");
    let mut updated_count = 0;

    for update in relevant_updates {
        if let Some(latest_version) = update.latest_version {
            // Find and update the repository in the schema
            if update_repo_ref(&mut schema, &update.url, &latest_version)? {
                println!(
                    "✅ Updated {}: {} → {}",
                    update.url, update.current_ref, latest_version
                );
                updated_count += 1;
            }
        }
    }

    if updated_count > 0 {
        // Write back the updated configuration
        let yaml_content = serde_yaml::to_string(&schema)
            .map_err(|e| anyhow::anyhow!("Failed to serialize updated config: {}", e))?;

        fs::write(config_path, yaml_content).map_err(|e| {
            anyhow::anyhow!(
                "Failed to write updated config to {}: {}",
                config_path.display(),
                e
            )
        })?;

        println!(
            "\n✅ Successfully updated {} repositories in {}",
            updated_count,
            config_path.display()
        );
        println!("💡 Run 'common-repo apply' to apply the updated configuration.");
    } else {
        println!("\nℹ️  No repositories were updated.");
    }

    Ok(())
}

/// Update the ref for a specific repository URL in the schema
fn update_repo_ref(schema: &mut config::Schema, url: &str, new_ref: &str) -> Result<bool> {
    for operation in schema {
        if let config::Operation::Repo { repo } = operation {
            if repo.url == url {
                repo.r#ref = new_ref.to_string();
                return Ok(true);
            }
        }
    }
    Ok(false)
}
