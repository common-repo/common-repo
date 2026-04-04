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
use common_repo::defaults::DEFAULT_CONFIG_FILENAME;
use common_repo::repository::RepositoryManager;
use common_repo::version;

/// Update repository refs to newer versions
#[derive(Args, Debug)]
pub struct UpdateArgs {
    /// Path to the .common-repo.yaml configuration file to update.
    #[arg(short, long, value_name = "FILE", default_value = DEFAULT_CONFIG_FILENAME)]
    pub config: PathBuf,

    /// The root directory for the repository cache.
    ///
    /// Defaults to the system cache directory (`~/.cache/common-repo` on Linux,
    /// `~/Library/Caches/common-repo` on macOS).
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

    /// Filter upstreams by glob pattern (matches against url/path, scheme stripped).
    ///
    /// The pattern is matched against a normalized string combining the repository
    /// URL (without scheme) and optional path. Multiple filters use OR logic.
    ///
    /// Examples:
    ///   --filter "github.com/org/*"
    ///   --filter "*/*/ci-*" --filter "*/*/linter-*"
    #[arg(long, value_name = "GLOB")]
    pub filter: Vec<String>,
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

    // Show filter status if filters are active
    if !args.filter.is_empty() {
        let patterns = args.filter.join(", ");
        println!("Filtering upstreams matching: {}", patterns);
    }

    // Check for updates (with optional filtering)
    println!("Checking for repository updates...");
    let update_result = version::check_updates_filtered(&schema, &repo_manager, &args.filter)?;
    let updates = update_result.updates;
    let filtered_out = update_result.filtered_out_count;

    if updates.is_empty() {
        if filtered_out > 0 {
            println!(
                "✅ No repositories found that match the filter ({} filtered out).",
                filtered_out
            );
        } else {
            println!("✅ No repositories found that can be checked for updates.");
        }
        return Ok(());
    }

    // Filter updates based on flags (--compatible/--latest)
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
    if filtered_out > 0 {
        println!(
            "{} repositories can be updated ({} filtered out)\n",
            relevant_updates.len(),
            filtered_out
        );
    } else {
        println!("{} repositories can be updated\n", relevant_updates.len());
    }

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

    // Perform updates via surgical text replacement to preserve YAML structure,
    // ordering, comments, and formatting. This avoids the serde round-trip which
    // normalizes shorthand syntax, adds null keys, and reorders fields.
    // Also fixes updating all occurrences (e.g. repo used in both self: and top-level).
    let mut yaml_content = fs::read_to_string(config_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to read config from {}: {}",
            config_path.display(),
            e
        )
    })?;

    println!("\n🔄 Updating repositories...");
    let mut updated_count = 0;

    for update in relevant_updates {
        if let Some(latest_version) = update.latest_version {
            let replacements =
                update_ref_in_text(&yaml_content, &update.url, &update.current_ref, &latest_version);
            if replacements > 0 {
                // Apply the text replacement
                yaml_content =
                    apply_ref_update(&yaml_content, &update.url, &update.current_ref, &latest_version);
                println!(
                    "✅ Updated {} ({} occurrence{}): {} → {}",
                    update.url,
                    replacements,
                    if replacements == 1 { "" } else { "s" },
                    update.current_ref,
                    latest_version
                );
                updated_count += 1;
            }
        }
    }

    if updated_count > 0 {
        fs::write(config_path, &yaml_content).map_err(|e| {
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

/// Count how many ref occurrences would be updated for a given repo URL.
fn update_ref_in_text(content: &str, url: &str, current_ref: &str, _new_ref: &str) -> usize {
    let lines: Vec<&str> = content.lines().collect();
    let mut count = 0;

    for i in 0..lines.len() {
        let trimmed = lines[i].trim();
        // Match a url: line containing this repo URL
        if trimmed.starts_with("url:") && trimmed.contains(url) {
            // Search nearby lines (within 5 lines) for the ref: field
            for j in (i.saturating_sub(5))..=(i + 5).min(lines.len() - 1) {
                let ref_trimmed = lines[j].trim();
                if ref_trimmed.starts_with("ref:") && ref_trimmed.contains(current_ref) {
                    count += 1;
                    break;
                }
            }
        }
    }
    count
}

/// Surgically replace ref values in YAML text for a given repo URL.
/// Finds all `url:` lines matching the repo, then locates the adjacent `ref:` line
/// and replaces only the version value, preserving all formatting and structure.
fn apply_ref_update(content: &str, url: &str, current_ref: &str, new_ref: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result_lines: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    // Track which ref lines we've already updated to avoid double-counting
    let mut updated_ref_lines: Vec<usize> = Vec::new();

    for i in 0..lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.starts_with("url:") && trimmed.contains(url) {
            // Search nearby lines for the ref: field
            for j in (i.saturating_sub(5))..=(i + 5).min(lines.len() - 1) {
                if updated_ref_lines.contains(&j) {
                    continue;
                }
                let ref_trimmed = lines[j].trim();
                if ref_trimmed.starts_with("ref:") && ref_trimmed.contains(current_ref) {
                    // Replace only the ref value, preserving indentation and quoting
                    result_lines[j] = lines[j].replacen(current_ref, new_ref, 1);
                    updated_ref_lines.push(j);
                    break;
                }
            }
        }
    }

    let mut result = result_lines.join("\n");
    // Preserve trailing newline if the original content had one
    if content.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }
    result
}
