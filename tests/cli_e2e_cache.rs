//!
//! These tests invoke the actual CLI binary and validate cache command behavior
//! from a user's perspective.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;
use predicates::prelude::*;

/// Test that cache --help flag shows help information
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_help() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("cache")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Manage repository cache"));
}

/// Test that cache list --help shows list-specific help
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_list_help() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("cache")
        .arg("list")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("List all cached repositories"));
}

/// Test that cache list with empty cache directory shows appropriate message
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_list_empty() {
    let temp = assert_fs::TempDir::new().unwrap();
    let cache_dir = temp.child("cache");

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("cache")
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No cached repositories found"));
}

/// Test that cache list with nonexistent cache directory shows appropriate message
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_list_nonexistent() {
    let temp = assert_fs::TempDir::new().unwrap();
    let cache_dir = temp.child("nonexistent-cache");

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("cache")
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cache directory does not exist"));
}

/// Test that cache list with populated cache shows entries
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_list_populated() {
    let temp = assert_fs::TempDir::new().unwrap();
    let cache_dir = temp.child("cache");
    cache_dir.create_dir_all().unwrap();

    // Create a mock cache entry directory
    // Format: {hash}-{ref} or {hash}-{ref}-path-{path}
    let entry1 = cache_dir.child("a1b2c3d4e5f6-main");
    entry1.create_dir_all().unwrap();

    // Add some files to make it look like a real cache entry
    let file1 = entry1.child("README.md");
    file1.write_str("# Test Repo").unwrap();

    let file2 = entry1.child("src/main.rs");
    file2.write_str("fn main() {}").unwrap();

    let entry2 = cache_dir.child("f6e5d4c3b2a1-v1-0-0-path-uv");
    entry2.create_dir_all().unwrap();
    let file3 = entry2.child("main.py");
    file3.write_str("print('hello')").unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("cache")
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cached repositories"))
        .stdout(predicate::str::contains("a1b2c3d4e5f6"))
        .stdout(predicate::str::contains("main"))
        .stdout(predicate::str::contains("v1-0-0"))
        .stdout(predicate::str::contains("uv"))
        .stdout(predicate::str::contains("Total: 2 cached repositories"));
}

/// Test that cache list --verbose shows detailed information
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_list_verbose() {
    let temp = assert_fs::TempDir::new().unwrap();
    let cache_dir = temp.child("cache");
    cache_dir.create_dir_all().unwrap();

    // Create a mock cache entry
    let entry = cache_dir.child("a1b2c3d4e5f6-main");
    entry.create_dir_all().unwrap();
    let file = entry.child("test.txt");
    file.write_str("test content").unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("cache")
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("list")
        .arg("--verbose")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hash:"))
        .stdout(predicate::str::contains("Ref:"))
        .stdout(predicate::str::contains("Size:"))
        .stdout(predicate::str::contains("Files:"));
}

/// Test that cache list --json outputs JSON format
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_list_json() {
    let temp = assert_fs::TempDir::new().unwrap();
    let cache_dir = temp.child("cache");
    cache_dir.create_dir_all().unwrap();

    // Create a mock cache entry
    let entry = cache_dir.child("a1b2c3d4e5f6-main");
    entry.create_dir_all().unwrap();
    let file = entry.child("test.txt");
    file.write_str("test content").unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("cache")
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("list")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"hash\""))
        .stdout(predicate::str::contains("\"ref\""))
        .stdout(predicate::str::contains("\"size\""))
        .stdout(predicate::str::contains("\"file_count\""));
}

/// Test that cache list --json with empty cache outputs empty array
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_list_json_empty() {
    let temp = assert_fs::TempDir::new().unwrap();
    let cache_dir = temp.child("cache");

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("cache")
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("list")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("[]"));
}

/// Test that cache list respects --cache-root flag
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_list_custom_cache_root() {
    let temp = assert_fs::TempDir::new().unwrap();
    let cache_dir = temp.child("custom-cache");
    cache_dir.create_dir_all().unwrap();

    // Create a mock cache entry
    let entry = cache_dir.child("a1b2c3d4e5f6-main");
    entry.create_dir_all().unwrap();
    let file = entry.child("test.txt");
    file.write_str("test").unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("cache")
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("a1b2c3d4e5f6"))
        .stdout(predicate::str::contains("main"));
}

/// Test that cache clean --help shows help information
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_clean_help() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("cache")
        .arg("clean")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Clean cached repositories"));
}

/// Test that cache clean requires at least one filter flag
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_clean_requires_filter() {
    let temp = assert_fs::TempDir::new().unwrap();
    let cache_dir = temp.child("cache");
    cache_dir.create_dir_all().unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("cache")
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("clean")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "At least one filter must be specified",
        ));
}

/// Test cache clean --dry-run shows what would be deleted
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_clean_dry_run() {
    let temp = assert_fs::TempDir::new().unwrap();
    let cache_dir = temp.child("cache");
    cache_dir.create_dir_all().unwrap();

    // Create cache entries
    let entry1 = cache_dir.child("a1b2c3d4e5f6-main");
    entry1.create_dir_all().unwrap();
    let file1 = entry1.child("test.txt");
    file1.write_str("test content").unwrap();

    let entry2 = cache_dir.child("f6e5d4c3b2a1-v1-0-0");
    entry2.create_dir_all().unwrap();
    let file2 = entry2.child("main.py");
    file2.write_str("print('hello')").unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("cache")
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("clean")
        .arg("--all")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cache entries to be deleted"))
        .stdout(predicate::str::contains("Dry run mode"))
        .stdout(predicate::str::contains("a1b2c3d4e5f6"))
        .stdout(predicate::str::contains("f6e5d4c3b2a1"));

    // Verify entries still exist
    assert!(entry1.path().exists());
    assert!(entry2.path().exists());
}

/// Test cache clean --all --yes deletes all entries
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_clean_all_with_yes() {
    let temp = assert_fs::TempDir::new().unwrap();
    let cache_dir = temp.child("cache");
    cache_dir.create_dir_all().unwrap();

    // Create cache entries
    let entry1 = cache_dir.child("a1b2c3d4e5f6-main");
    entry1.create_dir_all().unwrap();
    let file1 = entry1.child("test.txt");
    file1.write_str("test content").unwrap();

    let entry2 = cache_dir.child("f6e5d4c3b2a1-v1-0-0");
    entry2.create_dir_all().unwrap();
    let file2 = entry2.child("main.py");
    file2.write_str("print('hello')").unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("cache")
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("clean")
        .arg("--all")
        .arg("--yes")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cache entries to be deleted"))
        .stdout(predicate::str::contains("Successfully deleted"));

    // Verify entries are deleted
    assert!(!entry1.path().exists());
    assert!(!entry2.path().exists());
}

/// Test cache clean with empty cache
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_clean_empty() {
    let temp = assert_fs::TempDir::new().unwrap();
    let cache_dir = temp.child("cache");
    cache_dir.create_dir_all().unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("cache")
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("clean")
        .arg("--all")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("No cached repositories"));
}

/// Test cache clean with nonexistent cache directory
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_clean_nonexistent() {
    let temp = assert_fs::TempDir::new().unwrap();
    let cache_dir = temp.child("nonexistent-cache");

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("cache")
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("clean")
        .arg("--all")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cache directory does not exist"));
}

/// Test cache clean --older-than with invalid duration format
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_clean_invalid_duration() {
    let temp = assert_fs::TempDir::new().unwrap();
    let cache_dir = temp.child("cache");
    cache_dir.create_dir_all().unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("cache")
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("clean")
        .arg("--older-than")
        .arg("invalid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid duration"));
}

/// Test cache clean confirmation prompt cancels deletion when "n" is entered
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_clean_confirmation_cancel() {
    let temp = assert_fs::TempDir::new().unwrap();
    let cache_dir = temp.child("cache");
    cache_dir.create_dir_all().unwrap();

    // Create cache entry
    let entry = cache_dir.child("a1b2c3d4e5f6-main");
    entry.create_dir_all().unwrap();
    let file = entry.child("test.txt");
    file.write_str("test content").unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("cache")
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("clean")
        .arg("--all")
        .write_stdin("n\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cache entries to be deleted"))
        .stdout(predicate::str::contains("Clean cancelled"));

    // Verify entry still exists
    assert!(entry.path().exists());
}

/// Test cache clean confirmation prompt accepts deletion when "y" is entered
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_clean_confirmation_accept() {
    let temp = assert_fs::TempDir::new().unwrap();
    let cache_dir = temp.child("cache");
    cache_dir.create_dir_all().unwrap();

    // Create cache entry
    let entry = cache_dir.child("a1b2c3d4e5f6-main");
    entry.create_dir_all().unwrap();
    let file = entry.child("test.txt");
    file.write_str("test content").unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("cache")
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("clean")
        .arg("--all")
        .write_stdin("y\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Cache entries to be deleted"))
        .stdout(predicate::str::contains("Successfully deleted"));

    // Verify entry is deleted
    assert!(!entry.path().exists());
}

/// Integration test: Test cache clean with --older-than filter
/// This test verifies that the filtering logic works correctly by using --all
/// Note: Testing time-based filters requires manipulating file times which is complex
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_clean_with_older_than_filter() {
    let temp = assert_fs::TempDir::new().unwrap();
    let cache_dir = temp.child("cache");
    cache_dir.create_dir_all().unwrap();

    // Create cache entries
    let entry1 = cache_dir.child("a1b2c3d4e5f6-main");
    entry1.create_dir_all().unwrap();
    let file1 = entry1.child("test.txt");
    file1.write_str("test content").unwrap();

    let entry2 = cache_dir.child("f6e5d4c3b2a1-v1-0-0");
    entry2.create_dir_all().unwrap();
    let file2 = entry2.child("main.py");
    file2.write_str("print('hello')").unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    // Test that --older-than accepts valid duration format
    // Newly created files won't match, so we expect "No cache entries match"
    cmd.arg("cache")
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("clean")
        .arg("--older-than")
        .arg("1d")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("No cache entries match"));

    // Verify entries still exist
    assert!(entry1.path().exists());
    assert!(entry2.path().exists());
}

/// Integration test: Test cache clean filters multiple entries correctly
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_clean_filters_multiple_entries() {
    let temp = assert_fs::TempDir::new().unwrap();
    let cache_dir = temp.child("cache");
    cache_dir.create_dir_all().unwrap();

    // Create multiple cache entries with different refs and paths
    let entries = vec![
        ("a1b2c3d4e5f6-main", None),
        ("f6e5d4c3b2a1-v1-0-0", None),
        ("1234567890ab-feature-branch-path-uv", Some("uv")),
        ("abcdef123456-v2-0-0-path-src", Some("src")),
    ];

    for (dir_name, _path) in &entries {
        let entry = cache_dir.child(dir_name);
        entry.create_dir_all().unwrap();
        let file = entry.child("test.txt");
        file.write_str("test content").unwrap();
    }

    let mut cmd = cargo_bin_cmd!("common-repo");

    // List entries first to verify they exist
    cmd.arg("cache")
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("Total: 4 cached repositories"));

    // Clean all entries
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.arg("cache")
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("clean")
        .arg("--all")
        .arg("--yes")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Successfully deleted 4 cache entries",
        ));

    // Verify all entries are deleted
    for (dir_name, _path) in &entries {
        assert!(!cache_dir.child(dir_name).path().exists());
    }
}

/// Integration test: Test cache clean with nested directory structures
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_clean_with_nested_directories() {
    let temp = assert_fs::TempDir::new().unwrap();
    let cache_dir = temp.child("cache");
    cache_dir.create_dir_all().unwrap();

    // Create a cache entry with nested directory structure
    let entry = cache_dir.child("a1b2c3d4e5f6-main");
    entry.create_dir_all().unwrap();

    // Create nested directories and files
    let nested_dir = entry.child("src/subdir/deep");
    nested_dir.create_dir_all().unwrap();
    let file1 = entry.child("README.md");
    file1.write_str("# Test Repo").unwrap();
    let file2 = nested_dir.child("file.txt");
    file2.write_str("nested content").unwrap();
    let file3 = entry.child("src/main.rs");
    file3.write_str("fn main() {}").unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    // Clean the entry
    cmd.arg("cache")
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("clean")
        .arg("--all")
        .arg("--yes")
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully deleted"));

    // Verify entire entry directory is deleted (including nested structure)
    assert!(!entry.path().exists());
    assert!(!nested_dir.path().exists());
}

/// Integration test: Test cache clean handles empty cache directories
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_clean_empty_cache_directories() {
    let temp = assert_fs::TempDir::new().unwrap();
    let cache_dir = temp.child("cache");
    cache_dir.create_dir_all().unwrap();

    // Create an empty cache entry directory with valid naming format
    let empty_entry = cache_dir.child("a1b2c3d4e5f6-empty-branch");
    empty_entry.create_dir_all().unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    // Clean should handle empty directories
    cmd.arg("cache")
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("clean")
        .arg("--all")
        .arg("--yes")
        .assert()
        .success();

    // Verify empty entry is deleted
    assert!(!empty_entry.path().exists());
}

/// Integration test: Test cache clean with combined filters
/// This test verifies that multiple filters can be used together
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_cache_clean_combined_filters() {
    let temp = assert_fs::TempDir::new().unwrap();
    let cache_dir = temp.child("cache");
    cache_dir.create_dir_all().unwrap();

    // Create cache entries
    let entry1 = cache_dir.child("a1b2c3d4e5f6-main");
    entry1.create_dir_all().unwrap();
    let file1 = entry1.child("file.txt");
    file1.write_str("content").unwrap();

    let entry2 = cache_dir.child("f6e5d4c3b2a1-v1-0-0");
    entry2.create_dir_all().unwrap();
    let file2 = entry2.child("file.txt");
    file2.write_str("content").unwrap();

    let entry3 = cache_dir.child("1234567890ab-feature");
    entry3.create_dir_all().unwrap();
    let file3 = entry3.child("file.txt");
    file3.write_str("content").unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    // Use both --unused and --older-than filters together
    // This verifies that the command accepts multiple filters
    // Newly created files won't match either filter
    cmd.arg("cache")
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("clean")
        .arg("--unused")
        .arg("--older-than")
        .arg("1d")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("No cache entries match"));

    // Verify entries still exist
    assert!(entry1.path().exists());
    assert!(entry2.path().exists());
    assert!(entry3.path().exists());
}
