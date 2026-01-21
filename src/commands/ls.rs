//! # Ls Command Implementation
//!
//! This module implements the `ls` subcommand, which lists the files that would
//! be created or modified when applying a `.common-repo.yaml` configuration.
//!
//! ## Functionality
//!
//! - **File Listing**: Shows all files that would result from applying the configuration
//! - **Pattern Filtering**: Supports glob patterns to filter the output
//! - **Detailed Output**: Optional long format showing file sizes and permissions
//! - **Sorting**: Files can be sorted by name, size, or path
//!
//! This command is a safe, read-only operation that does not modify any files.
//! It runs phases 1-5 of the pipeline without writing to disk (phase 6).

use anyhow::Result;
use clap::{Args, ValueEnum};
use std::path::PathBuf;

use common_repo::cache::RepoCache;
use common_repo::config;
use common_repo::defaults::DEFAULT_CONFIG_FILENAME;
use common_repo::phases::orchestrator;
use common_repo::repository::RepositoryManager;

/// List files that would be created/modified by the configuration
#[derive(Args, Debug)]
pub struct LsArgs {
    /// Path to the .common-repo.yaml configuration file.
    #[arg(short, long, value_name = "FILE", default_value = DEFAULT_CONFIG_FILENAME)]
    pub config: PathBuf,

    /// The root directory for the repository cache.
    ///
    /// Defaults to the system cache directory (`~/.cache/common-repo` on Linux,
    /// `~/Library/Caches/common-repo` on macOS).
    /// Can also be set with the `COMMON_REPO_CACHE` environment variable.
    #[arg(long, value_name = "DIR", env = "COMMON_REPO_CACHE")]
    pub cache_root: Option<PathBuf>,

    /// The working directory for local file operations.
    ///
    /// If not provided, it defaults to the current working directory.
    #[arg(long, value_name = "DIR")]
    pub working_dir: Option<PathBuf>,

    /// Filter files by glob pattern (e.g., "*.rs", "src/**/*.ts").
    #[arg(short, long, value_name = "PATTERN")]
    pub pattern: Option<String>,

    /// Use long listing format showing size and permissions.
    #[arg(short, long)]
    pub long: bool,

    /// Sort order for file listing.
    #[arg(short, long, value_enum, default_value = "name")]
    pub sort: SortOrder,

    /// Show only the total count of files.
    #[arg(long)]
    pub count: bool,

    /// Reverse the sort order.
    #[arg(short, long)]
    pub reverse: bool,
}

/// Sort order options for file listing
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum SortOrder {
    /// Sort alphabetically by file name
    #[default]
    Name,
    /// Sort by file size
    Size,
    /// Sort by full path
    Path,
}

/// Execute the `ls` command.
///
/// This function handles the logic for the `ls` subcommand. It runs phases 1-5
/// of the pipeline to build the final filesystem, then lists the files without
/// writing them to disk.
pub fn execute(args: LsArgs) -> Result<()> {
    let config_path = &args.config;

    // Validate config file exists
    if !config_path.exists() {
        return Err(common_repo::suggestions::config_not_found(config_path));
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
    let cache_root = args
        .cache_root
        .unwrap_or_else(common_repo::defaults::default_cache_root);

    let repo_manager = RepositoryManager::new(cache_root);
    let repo_cache = RepoCache::new();

    // Execute phases 1-5 (skip phase 6 - writing to disk)
    let working_dir = args
        .working_dir
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));
    let final_fs = orchestrator::execute_pull(
        &schema,
        &repo_manager,
        &repo_cache,
        &working_dir,
        None, // Don't write to disk
    )
    .map_err(|e| anyhow::anyhow!("Failed to process configuration: {}", e))?;

    // Collect files with their metadata
    let mut files: Vec<FileInfo> = final_fs
        .files()
        .map(|(path, file)| FileInfo {
            path: path.clone(),
            size: file.content.len(),
            permissions: file.permissions,
        })
        .collect();

    // Apply pattern filter if specified
    if let Some(pattern) = &args.pattern {
        let glob_pattern = glob::Pattern::new(pattern)
            .map_err(|e| anyhow::anyhow!("Invalid glob pattern '{}': {}", pattern, e))?;
        files.retain(|f| f.path.to_str().is_some_and(|s| glob_pattern.matches(s)));
    }

    // Sort files
    match args.sort {
        SortOrder::Name => {
            files.sort_by(|a, b| {
                let name_a = a.path.file_name().unwrap_or_default();
                let name_b = b.path.file_name().unwrap_or_default();
                name_a.cmp(name_b)
            });
        }
        SortOrder::Size => {
            files.sort_by_key(|f| f.size);
        }
        SortOrder::Path => {
            files.sort_by(|a, b| a.path.cmp(&b.path));
        }
    }

    // Reverse if requested
    if args.reverse {
        files.reverse();
    }

    // Output
    if args.count {
        println!("{}", files.len());
        return Ok(());
    }

    if files.is_empty() {
        println!("No files would be created.");
        return Ok(());
    }

    if args.long {
        // Long format: permissions size path
        for file in &files {
            println!(
                "{} {:>8} {}",
                format_permissions(file.permissions),
                format_size(file.size),
                file.path.display()
            );
        }
    } else {
        // Simple format: just paths
        for file in &files {
            println!("{}", file.path.display());
        }
    }

    // Summary
    let total_size: usize = files.iter().map(|f| f.size).sum();
    println!();
    println!("{} file(s), {} total", files.len(), format_size(total_size));

    Ok(())
}

/// File information for listing
struct FileInfo {
    path: PathBuf,
    size: usize,
    permissions: u32,
}

/// Format file permissions in Unix-style (e.g., "rw-r--r--")
fn format_permissions(mode: u32) -> String {
    let mut result = String::with_capacity(9);

    // Owner permissions
    result.push(if mode & 0o400 != 0 { 'r' } else { '-' });
    result.push(if mode & 0o200 != 0 { 'w' } else { '-' });
    result.push(if mode & 0o100 != 0 { 'x' } else { '-' });

    // Group permissions
    result.push(if mode & 0o040 != 0 { 'r' } else { '-' });
    result.push(if mode & 0o020 != 0 { 'w' } else { '-' });
    result.push(if mode & 0o010 != 0 { 'x' } else { '-' });

    // Other permissions
    result.push(if mode & 0o004 != 0 { 'r' } else { '-' });
    result.push(if mode & 0o002 != 0 { 'w' } else { '-' });
    result.push(if mode & 0o001 != 0 { 'x' } else { '-' });

    result
}

/// Format file size in human-readable format
fn format_size(size: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = KB * 1024;
    const GB: usize = MB * 1024;

    if size >= GB {
        format!("{:.1}G", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1}M", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1}K", size as f64 / KB as f64)
    } else {
        format!("{}B", size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_format_permissions() {
        assert_eq!(format_permissions(0o644), "rw-r--r--");
        assert_eq!(format_permissions(0o755), "rwxr-xr-x");
        assert_eq!(format_permissions(0o700), "rwx------");
        assert_eq!(format_permissions(0o777), "rwxrwxrwx");
        assert_eq!(format_permissions(0o000), "---------");
        assert_eq!(format_permissions(0o600), "rw-------");
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0B");
        assert_eq!(format_size(100), "100B");
        assert_eq!(format_size(1023), "1023B");
        assert_eq!(format_size(1024), "1.0K");
        assert_eq!(format_size(1536), "1.5K");
        assert_eq!(format_size(1048576), "1.0M");
        assert_eq!(format_size(1073741824), "1.0G");
    }

    #[test]
    fn test_execute_missing_config() {
        let temp_dir = TempDir::new().unwrap();
        let args = LsArgs {
            config: PathBuf::from("/nonexistent/config.yaml"),
            cache_root: None,
            working_dir: Some(temp_dir.path().to_path_buf()),
            pattern: None,
            long: false,
            sort: SortOrder::Name,
            count: false,
            reverse: false,
        };

        let result = execute(args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Configuration file not found"));
    }

    #[test]
    fn test_execute_with_simple_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        // Create a simple config file that includes all files
        let config_content = r#"
- include: ["**/*"]
"#;

        fs::write(&config_path, config_content).unwrap();

        // Create some test files in the working directory
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let args = LsArgs {
            config: config_path,
            cache_root: Some(temp_dir.path().join("cache")),
            working_dir: Some(temp_dir.path().to_path_buf()),
            pattern: None,
            long: false,
            sort: SortOrder::Name,
            count: false,
            reverse: false,
        };

        // This should succeed (output goes to stdout)
        let result = execute(args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_with_count_flag() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        // Create a simple config file
        let config_content = r#"
- include: ["**/*"]
"#;

        fs::write(&config_path, config_content).unwrap();

        let args = LsArgs {
            config: config_path,
            cache_root: Some(temp_dir.path().join("cache")),
            working_dir: Some(temp_dir.path().to_path_buf()),
            pattern: None,
            long: false,
            sort: SortOrder::Name,
            count: true,
            reverse: false,
        };

        let result = execute(args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_with_long_format() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let config_content = r#"
- include: ["**/*"]
"#;

        fs::write(&config_path, config_content).unwrap();

        let args = LsArgs {
            config: config_path,
            cache_root: Some(temp_dir.path().join("cache")),
            working_dir: Some(temp_dir.path().to_path_buf()),
            pattern: None,
            long: true,
            sort: SortOrder::Size,
            count: false,
            reverse: true,
        };

        let result = execute(args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_with_invalid_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let config_content = r#"
- include: ["**/*"]
"#;

        fs::write(&config_path, config_content).unwrap();

        // Create a test file so the pipeline produces output
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "content").unwrap();

        let args = LsArgs {
            config: config_path,
            cache_root: Some(temp_dir.path().join("cache")),
            working_dir: Some(temp_dir.path().to_path_buf()),
            pattern: Some("[invalid".to_string()), // Invalid glob pattern
            long: false,
            sort: SortOrder::Name,
            count: false,
            reverse: false,
        };

        let result = execute(args);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid glob pattern"));
    }
}
