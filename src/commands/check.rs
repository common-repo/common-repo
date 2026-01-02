//! # Check Command Implementation
//!
//! This module implements the `check` subcommand, which provides functionality
//! for validating the `.common-repo.yaml` configuration file and checking for
//! available updates in the inherited repositories.
//!
//! ## Functionality
//!
//! - **Configuration Validation**: By default, the command parses the configuration
//!   file to ensure it is syntactically correct and conforms to the defined
//!   schema. It reports a summary of the loaded configuration, including the
//!   number of repositories and other operations.
//!
//! - **Update Checking**: When the `--updates` flag is provided, the command
//!   queries the remote Git repositories to check for newer versions (tags)
//!   that are compatible with semantic versioning. It then displays a summary
//!   of available updates, categorizing them as either compatible or containing
//!   breaking changes.
//!
//! This command is a safe, read-only operation that does not modify any files.

use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

use common_repo::config;
use common_repo::repository::RepositoryManager;
use common_repo::version;

/// Check for repository updates and configuration validity
#[derive(Args, Debug)]
pub struct CheckArgs {
    /// Path to the .common-repo.yaml configuration file to check.
    #[arg(short, long, value_name = "FILE", default_value = ".common-repo.yaml")]
    pub config: PathBuf,

    /// The root directory for the repository cache.
    ///
    /// Defaults to the system cache directory (`~/.cache/common-repo` on Linux,
    /// `~/Library/Caches/common-repo` on macOS).
    /// Can also be set with the `COMMON_REPO_CACHE` environment variable.
    #[arg(long, value_name = "DIR", env = "COMMON_REPO_CACHE")]
    pub cache_root: Option<PathBuf>,

    /// If set, the command will check for newer versions of the inherited
    /// repositories.
    #[arg(long)]
    pub updates: bool,
}

/// Execute the `check` command.
///
/// This function handles the logic for the `check` subcommand. It either
/// performs a basic configuration validation or, if the `--updates` flag is
/// present, checks for newer versions of the repositories defined in the config.
pub fn execute(args: CheckArgs) -> Result<()> {
    // Load configuration
    let config_path = &args.config;
    println!("Loading configuration from: {}", config_path.display());

    let schema = config::from_file(config_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to load config from {}: {}",
            config_path.display(),
            e
        )
    })?;

    // Initialize repository manager
    let cache_root = args
        .cache_root
        .unwrap_or_else(common_repo::defaults::default_cache_root);

    let repo_manager = RepositoryManager::new(cache_root);

    if args.updates {
        // Check for updates
        println!("Checking for repository updates...");
        let updates = version::check_updates(&schema, &repo_manager)?;

        if updates.is_empty() {
            println!("✅ No repositories found that can be checked for updates.");
            return Ok(());
        }

        let mut has_updates = false;
        let mut has_breaking = false;

        for update in &updates {
            if update.breaking_changes || update.compatible_updates {
                has_updates = true;
                if update.breaking_changes {
                    has_breaking = true;
                }
            }
        }

        if !has_updates {
            println!("✅ All repositories are up to date!");
            return Ok(());
        }

        // Display update information
        println!("\n📦 Repository Update Summary:");
        println!(
            "{} repositories checked, {} have available updates\n",
            updates.len(),
            updates
                .iter()
                .filter(|u| u.breaking_changes || u.compatible_updates)
                .count()
        );

        for update in updates {
            if update.breaking_changes || update.compatible_updates {
                println!("🔄 {} (current: {})", update.url, update.current_ref);

                if let Some(latest) = &update.latest_version {
                    println!("   Latest: {}", latest);

                    if update.breaking_changes {
                        println!("   ⚠️  BREAKING CHANGES available (major version update)");
                    } else if update.compatible_updates {
                        println!("   ✅ Compatible updates available");
                    }
                }

                println!();
            }
        }

        if has_breaking {
            println!(
                "⚠️  Some repositories have breaking changes. Review carefully before updating."
            );
            println!(
                "   Use 'common-repo update' to update repository refs in your configuration."
            );
        }
    } else {
        // Basic configuration check
        println!("✅ Configuration loaded successfully");
        println!("   Operations: {}", schema.len());

        // Count repositories
        let repo_count = schema
            .iter()
            .filter(|op| matches!(op, config::Operation::Repo { .. }))
            .count();
        println!("   Repositories: {}", repo_count);

        // Count other operations
        let other_ops = schema.len() - repo_count;
        println!("   Other operations: {}", other_ops);

        println!("\n💡 Tip: Use --updates to check for repository version updates");
    }

    Ok(())
}
