//! Update command - Update repository refs to newer versions

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
    /// Path to .common-repo.yaml configuration file
    #[arg(short, long, value_name = "FILE", default_value = ".common-repo.yaml")]
    config: PathBuf,

    /// Cache directory for repositories
    #[arg(long, value_name = "DIR", env = "COMMON_REPO_CACHE")]
    cache_root: Option<PathBuf>,

    /// Update to latest compatible versions (minor/patch updates only)
    #[arg(long)]
    compatible: bool,

    /// Update to latest versions including breaking changes
    #[arg(long)]
    latest: bool,

    /// Don't ask for confirmation, update all eligible repositories
    #[arg(long)]
    yes: bool,

    /// Dry run - show what would be updated without making changes
    #[arg(long)]
    dry_run: bool,
}

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
