//! # Sync Check Command Implementation
//!
//! This module implements the `sync-check` subcommand, which detects when local
//! files are out of sync with inherited repositories and optionally auto-fixes
//! small drifts.
//!
//! ## Functionality
//!
//! - **Drift Detection**: Compares current files against what `apply` would generate
//! - **Auto-fix Mode**: Automatically applies changes when drift is below thresholds
//! - **Threshold Configuration**: Configurable limits for auto-fix vs manual update
//! - **Exit Codes**:
//!   - 0: In sync (no changes needed)
//!   - 1: Minor drift (auto-fixed if enabled)
//!   - 2: Major drift (manual update required)
//!
//! This command is designed for use in pre-commit hooks to keep dependencies synced.

use anyhow::Result;
use clap::Args;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use common_repo::cache::RepoCache;
use common_repo::config;
use common_repo::filesystem::MemoryFS;
use common_repo::phases::orchestrator;
use common_repo::repository::RepositoryManager;

/// Check if local files are in sync with inherited repositories
#[derive(Args, Debug)]
pub struct SyncCheckArgs {
    /// Path to the .common-repo.yaml configuration file.
    #[arg(short, long, value_name = "FILE", default_value = ".common-repo.yaml")]
    pub config: PathBuf,

    /// The root directory for the repository cache.
    #[arg(long, value_name = "DIR", env = "COMMON_REPO_CACHE")]
    pub cache_root: Option<PathBuf>,

    /// The working directory to check.
    #[arg(long, value_name = "DIR")]
    pub working_dir: Option<PathBuf>,

    /// Automatically fix small drifts (within thresholds).
    #[arg(long)]
    pub auto_fix: bool,

    /// Maximum number of files to auto-fix (default: 5).
    #[arg(long, default_value = "5")]
    pub max_files: usize,

    /// Maximum total lines changed to auto-fix (default: 50).
    #[arg(long, default_value = "50")]
    pub max_lines: usize,

    /// Show detailed output.
    #[arg(long, short)]
    pub verbose: bool,

    /// Output format (text, json).
    #[arg(long, default_value = "text")]
    pub format: String,
}

/// Result of a sync check
#[derive(Debug)]
pub struct SyncCheckResult {
    /// Files that would be added
    pub added: Vec<FileChange>,
    /// Files that would be modified
    pub modified: Vec<FileChange>,
    /// Files that would be deleted
    pub deleted: Vec<FileChange>,
    /// Whether the drift is within auto-fix thresholds
    pub within_threshold: bool,
    /// Whether auto-fix was applied
    pub auto_fixed: bool,
}

/// A file change with line count information
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub lines_added: usize,
    pub lines_removed: usize,
}

impl SyncCheckResult {
    /// Total number of files changed
    pub fn total_files(&self) -> usize {
        self.added.len() + self.modified.len() + self.deleted.len()
    }

    /// Total lines changed (added + removed)
    pub fn total_lines(&self) -> usize {
        let sum = |changes: &[FileChange]| -> usize {
            changes
                .iter()
                .map(|c| c.lines_added + c.lines_removed)
                .sum()
        };
        sum(&self.added) + sum(&self.modified) + sum(&self.deleted)
    }

    /// Check if there are any changes
    pub fn has_changes(&self) -> bool {
        self.total_files() > 0
    }
}

/// Execute the `sync-check` command.
///
/// Returns:
/// - `Ok(())` with appropriate exit code handling in CLI
/// - Exit code 0: In sync
/// - Exit code 1: Minor drift (auto-fixed or within threshold)
/// - Exit code 2: Major drift (requires manual update)
pub fn execute(args: SyncCheckArgs) -> Result<SyncCheckResult> {
    let config_path = &args.config;

    // Validate config file exists
    if !config_path.exists() {
        anyhow::bail!("Configuration file not found: {}", config_path.display());
    }

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
    let repo_cache = RepoCache::new();

    // Determine working directory
    let working_dir = args
        .working_dir
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    // Execute phases 1-5 (skip phase 6 - writing to disk)
    let final_fs = orchestrator::execute_pull(
        &schema,
        &repo_manager,
        &repo_cache,
        &working_dir,
        None, // Don't write to disk yet
    )
    .map_err(|e| anyhow::anyhow!("Failed to process configuration: {}", e))?;

    // Compute changes with line counts
    let (added, modified, deleted) = compute_changes_with_lines(&final_fs, &working_dir)?;

    // Calculate totals
    let total_files = added.len() + modified.len() + deleted.len();
    let total_lines: usize = added
        .iter()
        .chain(modified.iter())
        .chain(deleted.iter())
        .map(|c| c.lines_added + c.lines_removed)
        .sum();

    let within_threshold = total_files <= args.max_files && total_lines <= args.max_lines;

    let mut result = SyncCheckResult {
        added,
        modified,
        deleted,
        within_threshold,
        auto_fixed: false,
    };

    // Display results based on format
    if args.format == "json" {
        print_json_result(&result);
    } else {
        print_text_result(&result, args.verbose);
    }

    // Auto-fix if enabled and within threshold
    if args.auto_fix && result.has_changes() && within_threshold {
        if args.verbose {
            println!("\nAuto-fixing {} file(s)...", result.total_files());
        }

        // Apply the changes by writing to disk
        orchestrator::execute_pull(&schema, &repo_manager, &repo_cache, &working_dir, Some(&working_dir))
            .map_err(|e| anyhow::anyhow!("Failed to apply auto-fix: {}", e))?;

        result.auto_fixed = true;

        if args.verbose {
            println!("Auto-fix complete. Files have been updated.");
        }
    } else if result.has_changes() && !within_threshold {
        // Major drift - suggest manual update
        println!();
        println!("Drift exceeds auto-fix thresholds ({} files, {} lines).", total_files, total_lines);
        println!("Run 'common-repo apply' to update manually.");
    }

    Ok(result)
}

/// Compute changes with line count information
fn compute_changes_with_lines(
    final_fs: &MemoryFS,
    working_dir: &Path,
) -> Result<(Vec<FileChange>, Vec<FileChange>, Vec<FileChange>)> {
    let mut added = Vec::new();
    let mut modified = Vec::new();
    let deleted = Vec::new(); // TODO: Track deleted files if needed

    // Track paths we've seen in the final filesystem
    let _config_paths: HashSet<PathBuf> = HashSet::new();

    // Check each file in the final filesystem
    for (path, file) in final_fs.files() {
        let full_path = working_dir.join(path);

        if full_path.exists() {
            // File exists - check if content differs
            let existing_content = fs::read(&full_path).map_err(|e| {
                anyhow::anyhow!("Failed to read file {}: {}", full_path.display(), e)
            })?;

            if existing_content != file.content {
                // Calculate line diff
                let (lines_added, lines_removed) =
                    count_line_changes(&existing_content, &file.content);

                modified.push(FileChange {
                    path: path.clone(),
                    lines_added,
                    lines_removed,
                });
            }
        } else {
            // File doesn't exist - would be added
            let lines_added = count_lines(&file.content);

            added.push(FileChange {
                path: path.clone(),
                lines_added,
                lines_removed: 0,
            });
        }
    }

    // Sort by path for consistent output
    added.sort_by(|a, b| a.path.cmp(&b.path));
    modified.sort_by(|a, b| a.path.cmp(&b.path));

    Ok((added, modified, deleted))
}

/// Count the number of lines in content
fn count_lines(content: &[u8]) -> usize {
    if content.is_empty() {
        return 0;
    }
    content.iter().filter(|&&b| b == b'\n').count() + 1
}

/// Count line changes between old and new content (simple diff)
fn count_line_changes(old: &[u8], new: &[u8]) -> (usize, usize) {
    let old_lines: HashSet<_> = old.split(|&b| b == b'\n').collect();
    let new_lines: HashSet<_> = new.split(|&b| b == b'\n').collect();

    let added = new_lines.difference(&old_lines).count();
    let removed = old_lines.difference(&new_lines).count();

    (added, removed)
}

/// Print result in text format
fn print_text_result(result: &SyncCheckResult, verbose: bool) {
    if !result.has_changes() {
        println!("In sync: no changes detected.");
        return;
    }

    println!("Sync check found {} change(s):", result.total_files());

    if verbose {
        if !result.added.is_empty() {
            println!("\nFiles to add:");
            for change in &result.added {
                println!("  + {} (+{} lines)", change.path.display(), change.lines_added);
            }
        }

        if !result.modified.is_empty() {
            println!("\nFiles to modify:");
            for change in &result.modified {
                println!(
                    "  ~ {} (+{}, -{} lines)",
                    change.path.display(),
                    change.lines_added,
                    change.lines_removed
                );
            }
        }

        if !result.deleted.is_empty() {
            println!("\nFiles to delete:");
            for change in &result.deleted {
                println!("  - {} (-{} lines)", change.path.display(), change.lines_removed);
            }
        }
    }

    println!(
        "\nSummary: {} added, {} modified, {} deleted ({} total lines)",
        result.added.len(),
        result.modified.len(),
        result.deleted.len(),
        result.total_lines()
    );

    if result.within_threshold {
        println!("Status: Within auto-fix threshold");
    } else {
        println!("Status: Exceeds auto-fix threshold (manual update recommended)");
    }
}

/// Print result in JSON format
fn print_json_result(result: &SyncCheckResult) {
    let json = serde_json::json!({
        "in_sync": !result.has_changes(),
        "total_files": result.total_files(),
        "total_lines": result.total_lines(),
        "within_threshold": result.within_threshold,
        "auto_fixed": result.auto_fixed,
        "added": result.added.iter().map(|c| {
            serde_json::json!({
                "path": c.path.display().to_string(),
                "lines_added": c.lines_added
            })
        }).collect::<Vec<_>>(),
        "modified": result.modified.iter().map(|c| {
            serde_json::json!({
                "path": c.path.display().to_string(),
                "lines_added": c.lines_added,
                "lines_removed": c.lines_removed
            })
        }).collect::<Vec<_>>(),
        "deleted": result.deleted.iter().map(|c| {
            serde_json::json!({
                "path": c.path.display().to_string(),
                "lines_removed": c.lines_removed
            })
        }).collect::<Vec<_>>()
    });

    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_count_lines_empty() {
        assert_eq!(count_lines(b""), 0);
    }

    #[test]
    fn test_count_lines_single() {
        assert_eq!(count_lines(b"hello"), 1);
    }

    #[test]
    fn test_count_lines_multiple() {
        assert_eq!(count_lines(b"hello\nworld\n"), 3);
    }

    #[test]
    fn test_count_line_changes() {
        let old = b"line1\nline2\nline3";
        let new = b"line1\nmodified\nline3\nline4";

        let (added, removed) = count_line_changes(old, new);
        assert_eq!(added, 2); // modified, line4
        assert_eq!(removed, 1); // line2
    }

    #[test]
    fn test_sync_check_result_totals() {
        let result = SyncCheckResult {
            added: vec![FileChange {
                path: PathBuf::from("a.txt"),
                lines_added: 10,
                lines_removed: 0,
            }],
            modified: vec![FileChange {
                path: PathBuf::from("b.txt"),
                lines_added: 5,
                lines_removed: 3,
            }],
            deleted: vec![],
            within_threshold: true,
            auto_fixed: false,
        };

        assert_eq!(result.total_files(), 2);
        assert_eq!(result.total_lines(), 18); // 10 + 5 + 3
        assert!(result.has_changes());
    }

    #[test]
    fn test_execute_missing_config() {
        let temp_dir = TempDir::new().unwrap();
        let args = SyncCheckArgs {
            config: PathBuf::from("/nonexistent/config.yaml"),
            cache_root: None,
            working_dir: Some(temp_dir.path().to_path_buf()),
            auto_fix: false,
            max_files: 5,
            max_lines: 50,
            verbose: false,
            format: "text".to_string(),
        };

        let result = execute(args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Configuration file not found"));
    }

    #[test]
    fn test_execute_no_changes() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        // Create a simple config file
        let config_content = r#"
- include:
    patterns: ["**/*"]
"#;

        fs::write(&config_path, config_content).unwrap();

        let args = SyncCheckArgs {
            config: config_path,
            cache_root: Some(temp_dir.path().join("cache")),
            working_dir: Some(temp_dir.path().to_path_buf()),
            auto_fix: false,
            max_files: 5,
            max_lines: 50,
            verbose: false,
            format: "text".to_string(),
        };

        let result = execute(args);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.has_changes());
    }
}
