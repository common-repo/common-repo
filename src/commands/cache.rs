//! # Cache Command Implementation
//!
//! This module implements the `cache` subcommand, which provides functionality
//! for managing the local repository cache.
//!
//! ## Subcommands
//!
//! - **`list`**: Display all cached repositories with their information
//! - **`clean`**: Remove cached repositories based on filters (--all, --unused, --older-than)

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use walkdir::WalkDir;

/// Manage repository cache
#[derive(Args, Debug)]
pub struct CacheArgs {
    /// The root directory for the repository cache.
    ///
    /// If not provided, it defaults to the system's cache directory
    /// (e.g., `~/.cache/common-repo` on Linux).
    /// Can also be set with the `COMMON_REPO_CACHE` environment variable.
    #[arg(long, value_name = "DIR", env = "COMMON_REPO_CACHE")]
    pub cache_root: Option<PathBuf>,

    /// Subcommand to execute
    #[command(subcommand)]
    pub command: CacheSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum CacheSubcommand {
    /// List all cached repositories
    List(ListArgs),
    /// Clean cached repositories
    Clean(CleanArgs),
}

/// Arguments for the cache list command
#[derive(Args, Debug)]
pub struct ListArgs {
    /// Show detailed information including last modified time and file count
    #[arg(long)]
    pub detailed: bool,

    /// Output in JSON format
    #[arg(long)]
    pub json: bool,
}

/// Arguments for the cache clean command
#[derive(Args, Debug)]
pub struct CleanArgs {
    /// Show what would be deleted without actually deleting anything
    #[arg(long)]
    pub dry_run: bool,

    /// Delete all cached repositories
    #[arg(long)]
    pub all: bool,

    /// Delete unused cache entries (older than 30 days)
    #[arg(long)]
    pub unused: bool,

    /// Delete entries older than the specified duration
    ///
    /// Duration format: number followed by unit (s, m, h, d, w)
    /// Examples: "30d", "7d", "1h", "30m", "2w"
    #[arg(long, value_name = "DURATION")]
    pub older_than: Option<String>,

    /// Skip confirmation prompt and delete immediately
    #[arg(long)]
    pub yes: bool,
}

/// Cache entry information
#[derive(Debug, Clone)]
struct CacheEntry {
    hash: String,
    ref_name: String,
    path: Option<String>,
    size: u64,
    file_count: usize,
    last_modified: Option<std::time::SystemTime>,
    dir_path: PathBuf,
}

/// Execute the `cache` command.
pub fn execute(args: CacheArgs) -> Result<()> {
    match args.command {
        CacheSubcommand::List(list_args) => execute_list(args.cache_root, list_args),
        CacheSubcommand::Clean(clean_args) => execute_clean(args.cache_root, clean_args),
    }
}

/// Execute the `cache list` command.
fn execute_list(cache_root: Option<PathBuf>, args: ListArgs) -> Result<()> {
    // Determine cache root path
    let cache_root = cache_root.unwrap_or_else(|| {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".common-repo-cache"))
            .join("common-repo")
    });

    // Check if cache directory exists
    if !cache_root.exists() {
        if args.json {
            println!("[]");
        } else {
            println!("Cache directory does not exist: {}", cache_root.display());
            println!("No cached repositories found.");
        }
        return Ok(());
    }

    // Scan cache directory
    let entries = scan_cache_directory(&cache_root)?;

    if entries.is_empty() {
        if args.json {
            println!("[]");
        } else {
            println!("No cached repositories found in: {}", cache_root.display());
        }
        return Ok(());
    }

    // Display entries
    if args.json {
        display_json(&entries)?;
    } else if args.detailed {
        display_detailed(&entries);
    } else {
        display_table(&entries);
    }

    Ok(())
}

/// Execute the `cache clean` command.
fn execute_clean(cache_root: Option<PathBuf>, args: CleanArgs) -> Result<()> {
    // Determine cache root path
    let cache_root = cache_root.unwrap_or_else(|| {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".common-repo-cache"))
            .join("common-repo")
    });

    // Validate that at least one filter is specified first
    if !args.all && !args.unused && args.older_than.is_none() {
        return Err(common_repo::suggestions::cache_clean_no_filter());
    }

    // Parse and validate older_than duration if provided
    if let Some(ref duration_str) = args.older_than {
        parse_duration(duration_str).with_context(|| {
            format!("Invalid duration format: '{}'. Expected format: number followed by unit (s, m, h, d, w)", duration_str)
        })?;
    }

    // Check if cache directory exists
    if !cache_root.exists() {
        println!("Cache directory does not exist: {}", cache_root.display());
        println!("No cached repositories to clean.");
        return Ok(());
    }

    // Scan cache directory
    let entries = scan_cache_directory(&cache_root)?;

    if entries.is_empty() {
        println!("No cached repositories found in: {}", cache_root.display());
        return Ok(());
    }

    // Filter entries based on flags
    let entries_to_delete = filter_entries_for_cleanup(&entries, &args)?;

    if entries_to_delete.is_empty() {
        println!("No cache entries match the specified criteria.");
        return Ok(());
    }

    // Display what will be deleted
    println!("Cache entries to be deleted:\n");
    let total_size: u64 = entries_to_delete.iter().map(|e| e.size).sum();
    for entry in &entries_to_delete {
        let path_display = entry.path.as_deref().unwrap_or("(none)");
        println!(
            "  {} {} {} ({})",
            &entry.hash[..entry.hash.len().min(16)],
            &entry.ref_name[..entry.ref_name.len().min(20)],
            path_display,
            format_size(entry.size)
        );
    }
    println!(
        "\nTotal: {} entries ({})",
        entries_to_delete.len(),
        format_size(total_size)
    );

    if args.dry_run {
        println!("\n🔎 Dry run mode - no changes were made.");
        return Ok(());
    }

    // Confirm deletion unless --yes flag is used
    if !args.yes {
        print!("\nDo you want to delete these cache entries? (y/N): ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input != "y" && input != "yes" {
            println!("Clean cancelled.");
            return Ok(());
        }
    }

    // Delete directories
    println!("\n🗑️  Deleting cache entries...");
    let mut deleted_count = 0;
    let mut failed_count = 0;

    for entry in &entries_to_delete {
        match fs::remove_dir_all(&entry.dir_path) {
            Ok(_) => {
                deleted_count += 1;
                if args.yes || deleted_count <= 3 {
                    // Show first few deletions, or all if --yes
                    println!("  ✅ Deleted: {}", entry.dir_path.display());
                }
            }
            Err(e) => {
                failed_count += 1;
                eprintln!("  ❌ Failed to delete {}: {}", entry.dir_path.display(), e);
            }
        }
    }

    if deleted_count > 0 {
        println!("\n✅ Successfully deleted {} cache entries.", deleted_count);
    }
    if failed_count > 0 {
        eprintln!("\n⚠️  Failed to delete {} cache entries.", failed_count);
    }

    Ok(())
}

/// Filter cache entries based on clean arguments
fn filter_entries_for_cleanup(entries: &[CacheEntry], args: &CleanArgs) -> Result<Vec<CacheEntry>> {
    let mut filtered = Vec::new();

    // Parse older_than duration if provided (validation already done in execute_clean)
    let older_than_duration = if let Some(ref duration_str) = args.older_than {
        Some(parse_duration(duration_str)?)
    } else {
        None
    };

    let now = SystemTime::now();
    let unused_threshold = Duration::from_secs(30 * 24 * 60 * 60); // 30 days

    for entry in entries {
        let mut should_delete = false;

        if args.all {
            should_delete = true;
        } else {
            if args.unused {
                // Check if entry is unused (older than 30 days)
                if let Some(last_modified) = entry.last_modified {
                    if let Ok(age) = now.duration_since(last_modified) {
                        if age >= unused_threshold {
                            should_delete = true;
                        }
                    }
                } else {
                    // If we can't determine last modified, consider it unused
                    should_delete = true;
                }
            }

            if let Some(threshold) = older_than_duration {
                // Check if entry is older than threshold
                if let Some(last_modified) = entry.last_modified {
                    if let Ok(age) = now.duration_since(last_modified) {
                        if age >= threshold {
                            should_delete = true;
                        }
                    } else {
                        // Entry is from the future (shouldn't happen), but include it
                        should_delete = true;
                    }
                } else {
                    // If we can't determine last modified, include it
                    should_delete = true;
                }
            }
        }

        if should_delete {
            filtered.push(entry.clone());
        }
    }

    Ok(filtered)
}

/// Parse a duration string into a Duration
///
/// Format: number followed by unit (s, m, h, d, w)
/// Examples: "30d", "7d", "1h", "30m", "2w"
fn parse_duration(duration_str: &str) -> Result<Duration> {
    let duration_str = duration_str.trim().to_lowercase();

    if duration_str.is_empty() {
        return Err(anyhow::anyhow!("Duration string cannot be empty"));
    }

    // Find the split point between number and unit
    let mut split_idx = duration_str.len();
    for (i, c) in duration_str.char_indices() {
        if !c.is_ascii_digit() && c != '.' {
            split_idx = i;
            break;
        }
    }

    if split_idx == 0 {
        return Err(anyhow::anyhow!("Duration must start with a number"));
    }

    let number_str = &duration_str[..split_idx];
    let unit_str = &duration_str[split_idx..];

    let number: f64 = number_str
        .parse()
        .with_context(|| format!("Invalid number in duration: '{}'", number_str))?;

    let seconds = match unit_str {
        "s" | "sec" | "second" | "seconds" => number,
        "m" | "min" | "minute" | "minutes" => number * 60.0,
        "h" | "hr" | "hour" | "hours" => number * 3600.0,
        "d" | "day" | "days" => number * 86400.0,
        "w" | "week" | "weeks" => number * 604800.0,
        _ => {
            return Err(anyhow::anyhow!(
                "Invalid duration unit: '{}'. Valid units: s, m, h, d, w",
                unit_str
            ));
        }
    };

    Ok(Duration::from_secs(seconds as u64))
}

/// Scan the cache directory and parse all cache entries
fn scan_cache_directory(cache_root: &Path) -> Result<Vec<CacheEntry>> {
    let mut entries = Vec::new();

    let dir_entries = fs::read_dir(cache_root)?;

    for entry in dir_entries {
        let entry = entry?;
        let path = entry.path();

        // Only process directories
        if !path.is_dir() {
            continue;
        }

        // Parse directory name
        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
            if let Some(cache_entry) = parse_cache_directory_name(dir_name, &path) {
                entries.push(cache_entry);
            }
        }
    }

    // Sort by hash, then ref, then path
    entries.sort_by(|a, b| {
        a.hash
            .cmp(&b.hash)
            .then_with(|| a.ref_name.cmp(&b.ref_name))
            .then_with(|| {
                a.path
                    .as_ref()
                    .unwrap_or(&String::new())
                    .cmp(b.path.as_ref().unwrap_or(&String::new()))
            })
    });

    Ok(entries)
}

/// Parse a cache directory name to extract hash, ref, and optional path
///
/// Format: `{hash}-{ref}` or `{hash}-{ref}-path-{path}`
/// The hash is a hex string (from DefaultHasher u64, typically 1-16 hex digits)
/// The ref can contain dashes (since `/` is replaced with `-`)
/// The path has `/` and `.` replaced with `-`
fn parse_cache_directory_name(dir_name: &str, dir_path: &Path) -> Option<CacheEntry> {
    // Check for path suffix: `-path-{path}`
    let (base_name, path) = if dir_name.ends_with("-path-") {
        // Edge case: ends with "-path-" but no actual path
        (dir_name, None)
    } else if dir_name.contains("-path-") {
        // Split on "-path-" to separate base from path
        let parts: Vec<&str> = dir_name.splitn(2, "-path-").collect();
        if parts.len() == 2 {
            (parts[0], Some(parts[1].to_string()))
        } else {
            (dir_name, None)
        }
    } else {
        (dir_name, None)
    };

    // Split base name into hash and ref
    // Format: {hash}-{ref}
    // The hash is hex (0-9a-f), ref can contain dashes
    // Find the first dash - everything before is hash, everything after is ref
    if let Some(dash_pos) = base_name.find('-') {
        let hash = base_name[..dash_pos].to_string();
        let ref_name = base_name[dash_pos + 1..].to_string();

        // Validate hash is hex (basic check)
        if hash.is_empty() || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }

        // Calculate directory size and metadata
        let (size, file_count, last_modified) = calculate_directory_info(dir_path);

        Some(CacheEntry {
            hash,
            ref_name,
            path,
            size,
            file_count,
            last_modified,
            dir_path: dir_path.to_path_buf(),
        })
    } else {
        // No dash found - invalid format
        None
    }
}

/// Calculate directory size, file count, and last modified time
fn calculate_directory_info(dir_path: &Path) -> (u64, usize, Option<std::time::SystemTime>) {
    let mut total_size = 0u64;
    let mut file_count = 0usize;
    let mut latest_mtime = None;

    for entry in WalkDir::new(dir_path).into_iter().flatten() {
        let metadata = entry.metadata();

        if let Ok(meta) = metadata {
            if meta.is_file() {
                total_size += meta.len();
                file_count += 1;

                // Track latest modification time
                if let Ok(mtime) = meta.modified() {
                    latest_mtime = Some(
                        latest_mtime
                            .map(|prev: std::time::SystemTime| prev.max(mtime))
                            .unwrap_or(mtime),
                    );
                }
            }
        }
    }

    (total_size, file_count, latest_mtime)
}

/// Display cache entries in table format
fn display_table(entries: &[CacheEntry]) {
    println!("Cached repositories:\n");
    println!("{:<16} {:<20} {:<30} {:>12}", "HASH", "REF", "PATH", "SIZE");
    println!("{}", "-".repeat(80));

    for entry in entries {
        let path_display = entry.path.as_deref().unwrap_or("(none)");
        let size_display = format_size(entry.size);
        println!(
            "{:<16} {:<20} {:<30} {:>12}",
            &entry.hash[..entry.hash.len().min(16)],
            &entry.ref_name[..entry.ref_name.len().min(20)],
            &path_display[..path_display.len().min(30)],
            size_display
        );
    }

    println!("\nTotal: {} cached repositories", entries.len());
}

/// Display cache entries in detailed format
fn display_detailed(entries: &[CacheEntry]) {
    println!("Cached repositories:\n");

    for (i, entry) in entries.iter().enumerate() {
        println!("Entry {}:", i + 1);
        println!("  Hash: {}", entry.hash);
        println!("  Ref: {}", entry.ref_name);
        if let Some(path) = &entry.path {
            println!("  Path: {}", path);
        } else {
            println!("  Path: (none)");
        }
        println!("  Size: {}", format_size(entry.size));
        println!("  Files: {}", entry.file_count);
        if let Some(mtime) = entry.last_modified {
            if let Ok(datetime) = mtime.duration_since(std::time::UNIX_EPOCH) {
                println!("  Last Modified: {} seconds ago", datetime.as_secs());
            } else {
                println!("  Last Modified: (unknown)");
            }
        } else {
            println!("  Last Modified: (unknown)");
        }
        if i < entries.len() - 1 {
            println!();
        }
    }

    println!("\nTotal: {} cached repositories", entries.len());
}

/// Display cache entries in JSON format
fn display_json(entries: &[CacheEntry]) -> Result<()> {
    use std::collections::HashMap;

    let json_entries: Vec<HashMap<&str, serde_json::Value>> = entries
        .iter()
        .map(|e| {
            let mut map = HashMap::new();
            map.insert("hash", serde_json::Value::String(e.hash.clone()));
            map.insert("ref", serde_json::Value::String(e.ref_name.clone()));
            map.insert(
                "path",
                e.path
                    .as_ref()
                    .map(|p| serde_json::Value::String(p.clone()))
                    .unwrap_or(serde_json::Value::Null),
            );
            map.insert(
                "size",
                serde_json::Value::Number(serde_json::Number::from(e.size)),
            );
            map.insert(
                "file_count",
                serde_json::Value::Number(serde_json::Number::from(e.file_count)),
            );
            if let Some(mtime) = e.last_modified {
                if let Ok(duration) = mtime.duration_since(std::time::UNIX_EPOCH) {
                    map.insert(
                        "last_modified",
                        serde_json::Value::Number(serde_json::Number::from(duration.as_secs())),
                    );
                }
            }
            map
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&json_entries)?);
    Ok(())
}

/// Format size in human-readable format
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[unit_idx])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cache_directory_name_simple() {
        let dir_name = "a1b2c3d4e5f6-main";
        let temp_dir = std::env::temp_dir();
        let dir_path = temp_dir.join(dir_name);

        let entry = parse_cache_directory_name(dir_name, &dir_path);
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.hash, "a1b2c3d4e5f6");
        assert_eq!(entry.ref_name, "main");
        assert_eq!(entry.path, None);
    }

    #[test]
    fn test_parse_cache_directory_name_with_path() {
        let dir_name = "a1b2c3d4e5f6-main-path-uv";
        let temp_dir = std::env::temp_dir();
        let dir_path = temp_dir.join(dir_name);

        let entry = parse_cache_directory_name(dir_name, &dir_path);
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.hash, "a1b2c3d4e5f6");
        assert_eq!(entry.ref_name, "main");
        assert_eq!(entry.path, Some("uv".to_string()));
    }

    #[test]
    fn test_parse_cache_directory_name_with_dashes_in_ref() {
        let dir_name = "a1b2c3d4e5f6-feature-some-branch";
        let temp_dir = std::env::temp_dir();
        let dir_path = temp_dir.join(dir_name);

        let entry = parse_cache_directory_name(dir_name, &dir_path);
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.hash, "a1b2c3d4e5f6");
        assert_eq!(entry.ref_name, "feature-some-branch");
        assert_eq!(entry.path, None);
    }

    #[test]
    fn test_parse_cache_directory_name_with_path_and_dashes() {
        let dir_name = "a1b2c3d4e5f6-v1-0-0-path-src-python";
        let temp_dir = std::env::temp_dir();
        let dir_path = temp_dir.join(dir_name);

        let entry = parse_cache_directory_name(dir_name, &dir_path);
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.hash, "a1b2c3d4e5f6");
        assert_eq!(entry.ref_name, "v1-0-0");
        assert_eq!(entry.path, Some("src-python".to_string()));
    }

    #[test]
    fn test_parse_cache_directory_name_invalid_no_dash() {
        let dir_name = "a1b2c3d4e5f6";
        let temp_dir = std::env::temp_dir();
        let dir_path = temp_dir.join(dir_name);

        let entry = parse_cache_directory_name(dir_name, &dir_path);
        assert!(entry.is_none());
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1073741824), "1.00 GB");
    }

    #[test]
    fn test_parse_duration_seconds() {
        assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_duration("1sec").unwrap(), Duration::from_secs(1));
        assert_eq!(parse_duration("60second").unwrap(), Duration::from_secs(60));
        assert_eq!(
            parse_duration("120seconds").unwrap(),
            Duration::from_secs(120)
        );
    }

    #[test]
    fn test_parse_duration_minutes() {
        assert_eq!(parse_duration("30m").unwrap(), Duration::from_secs(1800));
        assert_eq!(parse_duration("1min").unwrap(), Duration::from_secs(60));
        assert_eq!(
            parse_duration("60minute").unwrap(),
            Duration::from_secs(3600)
        );
        assert_eq!(
            parse_duration("2minutes").unwrap(),
            Duration::from_secs(120)
        );
    }

    #[test]
    fn test_parse_duration_hours() {
        assert_eq!(parse_duration("1h").unwrap(), Duration::from_secs(3600));
        assert_eq!(parse_duration("2hr").unwrap(), Duration::from_secs(7200));
        assert_eq!(
            parse_duration("24hour").unwrap(),
            Duration::from_secs(86400)
        );
        assert_eq!(
            parse_duration("12hours").unwrap(),
            Duration::from_secs(43200)
        );
    }

    #[test]
    fn test_parse_duration_days() {
        assert_eq!(parse_duration("1d").unwrap(), Duration::from_secs(86400));
        assert_eq!(parse_duration("7day").unwrap(), Duration::from_secs(604800));
        assert_eq!(
            parse_duration("30days").unwrap(),
            Duration::from_secs(2592000)
        );
    }

    #[test]
    fn test_parse_duration_weeks() {
        assert_eq!(parse_duration("1w").unwrap(), Duration::from_secs(604800));
        assert_eq!(
            parse_duration("2week").unwrap(),
            Duration::from_secs(1209600)
        );
        assert_eq!(
            parse_duration("4weeks").unwrap(),
            Duration::from_secs(2419200)
        );
    }

    #[test]
    fn test_parse_duration_decimal() {
        assert_eq!(parse_duration("1.5h").unwrap(), Duration::from_secs(5400));
        assert_eq!(parse_duration("0.5d").unwrap(), Duration::from_secs(43200));
    }

    #[test]
    fn test_parse_duration_case_insensitive() {
        assert_eq!(parse_duration("30D").unwrap(), Duration::from_secs(2592000));
        assert_eq!(parse_duration("7W").unwrap(), Duration::from_secs(4233600));
        assert_eq!(parse_duration("1H").unwrap(), Duration::from_secs(3600));
    }

    #[test]
    fn test_parse_duration_whitespace() {
        assert_eq!(
            parse_duration(" 30d ").unwrap(),
            Duration::from_secs(2592000)
        );
        assert_eq!(parse_duration("7d\n").unwrap(), Duration::from_secs(604800));
    }

    #[test]
    fn test_parse_duration_invalid_empty() {
        assert!(parse_duration("").is_err());
    }

    #[test]
    fn test_parse_duration_invalid_no_number() {
        assert!(parse_duration("d").is_err());
        assert!(parse_duration("h").is_err());
    }

    #[test]
    fn test_parse_duration_invalid_unit() {
        assert!(parse_duration("30x").is_err());
        assert!(parse_duration("30invalid").is_err());
    }

    #[test]
    fn test_parse_duration_invalid_number() {
        assert!(parse_duration("abc30d").is_err());
        assert!(parse_duration("not-a-number-d").is_err());
    }

    #[test]
    fn test_filter_entries_for_cleanup_all() {
        let temp_dir = std::env::temp_dir();
        let entries = vec![
            CacheEntry {
                hash: "a1b2c3d4e5f6".to_string(),
                ref_name: "main".to_string(),
                path: None,
                size: 100,
                file_count: 1,
                last_modified: Some(SystemTime::now()),
                dir_path: temp_dir.join("entry1"),
            },
            CacheEntry {
                hash: "f6e5d4c3b2a1".to_string(),
                ref_name: "v1.0.0".to_string(),
                path: Some("uv".to_string()),
                size: 200,
                file_count: 2,
                last_modified: Some(SystemTime::now()),
                dir_path: temp_dir.join("entry2"),
            },
        ];

        let args = CleanArgs {
            dry_run: false,
            all: true,
            unused: false,
            older_than: None,
            yes: false,
        };

        let filtered = filter_entries_for_cleanup(&entries, &args).unwrap();
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_entries_for_cleanup_unused() {
        let temp_dir = std::env::temp_dir();
        let now = SystemTime::now();
        let old_time = now - Duration::from_secs(31 * 24 * 60 * 60); // 31 days ago
        let recent_time = now - Duration::from_secs(24 * 60 * 60); // 1 day ago

        let entries = vec![
            CacheEntry {
                hash: "old".to_string(),
                ref_name: "main".to_string(),
                path: None,
                size: 100,
                file_count: 1,
                last_modified: Some(old_time),
                dir_path: temp_dir.join("old_entry"),
            },
            CacheEntry {
                hash: "recent".to_string(),
                ref_name: "main".to_string(),
                path: None,
                size: 100,
                file_count: 1,
                last_modified: Some(recent_time),
                dir_path: temp_dir.join("recent_entry"),
            },
            CacheEntry {
                hash: "no_time".to_string(),
                ref_name: "main".to_string(),
                path: None,
                size: 100,
                file_count: 1,
                last_modified: None,
                dir_path: temp_dir.join("no_time_entry"),
            },
        ];

        let args = CleanArgs {
            dry_run: false,
            all: false,
            unused: true,
            older_than: None,
            yes: false,
        };

        let filtered = filter_entries_for_cleanup(&entries, &args).unwrap();
        // Should include old entry and entry with no modification time
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|e| e.hash == "old"));
        assert!(filtered.iter().any(|e| e.hash == "no_time"));
        assert!(!filtered.iter().any(|e| e.hash == "recent"));
    }

    #[test]
    fn test_filter_entries_for_cleanup_older_than() {
        let temp_dir = std::env::temp_dir();
        let now = SystemTime::now();
        let old_time = now - Duration::from_secs(2 * 60 * 60); // 2 hours ago
        let recent_time = now - Duration::from_secs(30 * 60); // 30 minutes ago

        let entries = vec![
            CacheEntry {
                hash: "old".to_string(),
                ref_name: "main".to_string(),
                path: None,
                size: 100,
                file_count: 1,
                last_modified: Some(old_time),
                dir_path: temp_dir.join("old_entry"),
            },
            CacheEntry {
                hash: "recent".to_string(),
                ref_name: "main".to_string(),
                path: None,
                size: 100,
                file_count: 1,
                last_modified: Some(recent_time),
                dir_path: temp_dir.join("recent_entry"),
            },
        ];

        let args = CleanArgs {
            dry_run: false,
            all: false,
            unused: false,
            older_than: Some("1h".to_string()),
            yes: false,
        };

        let filtered = filter_entries_for_cleanup(&entries, &args).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].hash, "old");
    }

    #[test]
    fn test_filter_entries_for_cleanup_combined_filters() {
        let temp_dir = std::env::temp_dir();
        let now = SystemTime::now();
        let old_time = now - Duration::from_secs(31 * 24 * 60 * 60); // 31 days ago
        let recent_time = now - Duration::from_secs(24 * 60 * 60); // 1 day ago

        let entries = vec![
            CacheEntry {
                hash: "old_unused".to_string(),
                ref_name: "main".to_string(),
                path: None,
                size: 100,
                file_count: 1,
                last_modified: Some(old_time),
                dir_path: temp_dir.join("old_unused"),
            },
            CacheEntry {
                hash: "recent".to_string(),
                ref_name: "main".to_string(),
                path: None,
                size: 100,
                file_count: 1,
                last_modified: Some(recent_time),
                dir_path: temp_dir.join("recent"),
            },
        ];

        // Test with both --unused and --older-than (should match if either condition is true)
        let args = CleanArgs {
            dry_run: false,
            all: false,
            unused: true,
            older_than: Some("1h".to_string()),
            yes: false,
        };

        let filtered = filter_entries_for_cleanup(&entries, &args).unwrap();
        // Both entries should match: old_unused matches --unused, recent matches --older-than (1 day > 1 hour)
        assert_eq!(filtered.len(), 2);
    }

    // Test removed: validation now happens in execute_clean before filter_entries_for_cleanup is called
}
