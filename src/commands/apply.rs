//! Apply command implementation
//!
//! The apply command executes the full 6-phase pipeline:
//! 1. Discovery and cloning of inherited repos
//! 2. Processing individual repos into intermediate filesystems
//! 3. Determining operation order
//! 4. Constructing composite filesystem
//! 5. Merging with local files
//! 6. Writing to disk

use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

/// Arguments for the apply command
#[derive(Args, Debug)]
pub struct ApplyArgs {
    /// Path to config file
    #[arg(short, long, value_name = "PATH", env = "COMMON_REPO_CONFIG")]
    pub config: Option<PathBuf>,

    /// Output directory (defaults to current directory)
    #[arg(short, long, value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Cache root directory
    #[arg(long, value_name = "PATH", env = "COMMON_REPO_CACHE")]
    pub cache_root: Option<PathBuf>,

    /// Show what would be done without making changes
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Overwrite files without confirmation
    #[arg(short, long)]
    pub force: bool,

    /// Show detailed progress information
    #[arg(short, long)]
    pub verbose: bool,

    /// Bypass cache and fetch fresh clones
    #[arg(long)]
    pub no_cache: bool,

    /// Suppress all output except errors
    #[arg(short, long)]
    pub quiet: bool,
}

/// Execute the apply command
pub fn execute(args: ApplyArgs) -> Result<()> {
    use common_repo::cache::RepoCache;
    use common_repo::config::from_file;
    use common_repo::phases::orchestrator;
    use common_repo::repository::RepositoryManager;
    use std::time::Instant;

    let start_time = Instant::now();

    // Determine config file path
    let config_path = args
        .config
        .unwrap_or_else(|| PathBuf::from(".common-repo.yaml"));

    // Validate config file exists
    if !config_path.exists() {
        anyhow::bail!("Configuration file not found: {}", config_path.display());
    }

    // Determine output directory
    let output_dir = args
        .output
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    // Determine cache root
    let cache_root = args.cache_root.unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".common-repo").join("cache")
    });

    // Print header
    if !args.quiet {
        println!("🔍 Common Repository Apply");
        println!();

        if args.dry_run {
            println!("🔎 DRY RUN MODE - No changes will be made");
            println!();
        }
    }

    // Parse configuration
    if !args.quiet && args.verbose {
        println!("📋 Parsing configuration: {}", config_path.display());
    }
    let config = from_file(&config_path)?;

    // Setup repository manager and cache
    let repo_manager = RepositoryManager::new(cache_root.clone());
    let repo_cache = RepoCache::new();

    // Execute the 6-phase pipeline
    let result = orchestrator::execute_pull(
        &config,
        &repo_manager,
        &repo_cache,
        &std::env::current_dir().expect("Failed to get current directory"),
        if args.dry_run {
            None
        } else {
            Some(&output_dir)
        },
    );

    match result {
        Ok(final_fs) => {
            let duration = start_time.elapsed();

            if !args.quiet {
                println!("✅ Applied successfully in {:.2}s", duration.as_secs_f64());

                // Report statistics
                let file_count = final_fs.len();
                if file_count > 0 {
                    println!("   {} files processed", file_count);

                    if !args.dry_run {
                        println!("   Files written to: {}", output_dir.display());
                    }
                }
            }

            Ok(())
        }
        Err(e) => {
            if !args.quiet {
                println!("❌ Apply failed");
                println!();
            }
            Err(e.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_execute_missing_config() {
        let args = ApplyArgs {
            config: Some(PathBuf::from("/nonexistent/config.yaml")),
            output: None,
            cache_root: None,
            dry_run: false,
            force: false,
            verbose: false,
            no_cache: false,
            quiet: true,
        };

        let result = execute(args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Configuration file not found"));
    }

    #[test]
    fn test_execute_with_valid_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".common-repo.yaml");

        // Create a minimal valid config file
        fs::write(&config_path, "- include:\n    patterns: ['**/*']").unwrap();

        let args = ApplyArgs {
            config: Some(config_path),
            output: Some(temp_dir.path().to_path_buf()),
            cache_root: Some(temp_dir.path().join("cache")),
            dry_run: true,
            force: false,
            verbose: false,
            no_cache: false,
            quiet: true,
        };

        let result = execute(args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_default_config_path() {
        // Test that config defaults to .common-repo.yaml
        let args = ApplyArgs {
            config: None,
            output: None,
            cache_root: None,
            dry_run: true,
            force: false,
            verbose: false,
            no_cache: false,
            quiet: true,
        };

        // This will fail because .common-repo.yaml doesn't exist in test directory
        let result = execute(args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains(".common-repo.yaml"));
    }

    #[test]
    fn test_dry_run_mode() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".common-repo.yaml");
        fs::write(&config_path, "- include:\n    patterns: ['**/*']").unwrap();

        let args = ApplyArgs {
            config: Some(config_path),
            output: Some(temp_dir.path().to_path_buf()),
            cache_root: Some(temp_dir.path().join("cache")),
            dry_run: true,
            force: false,
            verbose: false,
            no_cache: false,
            quiet: true,
        };

        // Dry run should succeed without making changes
        let result = execute(args);
        assert!(result.is_ok());
    }
}
