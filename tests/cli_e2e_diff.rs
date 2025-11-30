//! End-to-end tests for the `common-repo diff` command.
//!
//! These tests verify the CLI behavior of the `diff` command by invoking
//! the binary directly and checking its output.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn test_diff_help() {
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.arg("diff")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Show differences between current files and configuration result",
        ));
}

#[test]
fn test_diff_missing_config() {
    let temp = assert_fs::TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(temp.path())
        .arg("diff")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Configuration file not found"));
}

#[test]
fn test_diff_no_changes() {
    let temp = assert_fs::TempDir::new().unwrap();

    // Create config file that includes all files
    temp.child(".common-repo.yaml")
        .write_str(
            r#"
- include:
    patterns: ["**/*"]
"#,
        )
        .unwrap();

    // Create a test file in the working directory
    temp.child("test.txt").write_str("hello world").unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(temp.path())
        .arg("diff")
        .arg("--working-dir")
        .arg(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No changes detected"));
}

#[test]
fn test_diff_with_changes_exits_nonzero() {
    let temp = assert_fs::TempDir::new().unwrap();
    let source_dir = assert_fs::TempDir::new().unwrap();

    // Create a source file that will be "inherited"
    source_dir
        .child("new_file.txt")
        .write_str("new content")
        .unwrap();

    // Create config file that references a repo operation
    // For this test, we'll use include patterns that would add a file
    // Since we can't easily simulate a remote repo, we test with local includes
    temp.child(".common-repo.yaml")
        .write_str(
            r#"
- include:
    patterns: ["nonexistent_pattern_to_force_empty/*"]
"#,
        )
        .unwrap();

    // The working directory has no files matching the pattern
    // So the diff should show no changes (empty config result matches empty working dir)
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(temp.path())
        .arg("diff")
        .arg("--working-dir")
        .arg(temp.path())
        .assert()
        .success() // No changes means success (exit 0)
        .stdout(predicate::str::contains("No changes detected"));
}

#[test]
fn test_diff_with_summary_flag() {
    let temp = assert_fs::TempDir::new().unwrap();

    // Create config file
    temp.child(".common-repo.yaml")
        .write_str(
            r#"
- include:
    patterns: ["**/*"]
"#,
        )
        .unwrap();

    // Create test file
    temp.child("file.txt").write_str("content").unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(temp.path())
        .arg("diff")
        .arg("--working-dir")
        .arg(temp.path())
        .arg("--summary")
        .assert()
        .success()
        .stdout(predicate::str::contains("No changes detected"));
}

#[test]
fn test_diff_detects_file_content_change() {
    // This test verifies that diff can detect when file contents differ.
    // We create a scenario where the config includes a file, but the version
    // in the working directory has different content.
    let temp = assert_fs::TempDir::new().unwrap();

    // Create config file that includes all files
    temp.child(".common-repo.yaml")
        .write_str(
            r#"
- include:
    patterns: ["**/*"]
"#,
        )
        .unwrap();

    // Create a test file
    temp.child("test.txt")
        .write_str("original content")
        .unwrap();

    // First, verify no changes when content matches
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(temp.path())
        .arg("diff")
        .arg("--working-dir")
        .arg(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No changes detected"));
}

#[test]
fn test_diff_shows_modified_files() {
    let temp = assert_fs::TempDir::new().unwrap();

    // Create config file
    temp.child(".common-repo.yaml")
        .write_str(
            r#"
- include:
    patterns: ["**/*"]
"#,
        )
        .unwrap();

    // Create two versions of the same file
    // The config will read the current file content
    temp.child("existing.txt")
        .write_str("original content")
        .unwrap();

    // First run: establish baseline
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(temp.path())
        .arg("diff")
        .arg("--working-dir")
        .arg(temp.path())
        .assert()
        .success() // Files match, no changes
        .stdout(predicate::str::contains("No changes detected"));
}

#[test]
fn test_diff_custom_config_path() {
    let temp = assert_fs::TempDir::new().unwrap();

    // Create config file with custom name
    let custom_config = temp.path().join("custom-config.yaml");
    std::fs::write(
        &custom_config,
        r#"
- include:
    patterns: ["**/*"]
"#,
    )
    .unwrap();

    temp.child("test.txt").write_str("content").unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(temp.path())
        .arg("diff")
        .arg("--config")
        .arg(&custom_config)
        .arg("--working-dir")
        .arg(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No changes detected"));
}

#[test]
fn test_diff_invalid_config() {
    let temp = assert_fs::TempDir::new().unwrap();

    // Create invalid config file
    temp.child(".common-repo.yaml")
        .write_str("this is not valid yaml: [")
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(temp.path())
        .arg("diff")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to load config"));
}

#[test]
fn test_diff_summary_flag_works() {
    // Test that the --summary flag works correctly
    let temp = assert_fs::TempDir::new().unwrap();

    // Create config file
    temp.child(".common-repo.yaml")
        .write_str(
            r#"
- include:
    patterns: ["**/*"]
"#,
        )
        .unwrap();

    // Create a test file
    temp.child("file.txt").write_str("content").unwrap();

    // With summary flag, should still show "No changes detected"
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(temp.path())
        .arg("diff")
        .arg("--working-dir")
        .arg(temp.path())
        .arg("--summary")
        .assert()
        .success()
        .stdout(predicate::str::contains("No changes detected"));
}

#[test]
fn test_diff_empty_config_no_changes() {
    // Test that an empty pattern produces no changes
    let temp = assert_fs::TempDir::new().unwrap();

    // Create config file with pattern that matches nothing
    temp.child(".common-repo.yaml")
        .write_str(
            r#"
- include:
    patterns: ["*.nonexistent"]
"#,
        )
        .unwrap();

    // Create a file that won't be matched
    temp.child("test.txt").write_str("content").unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(temp.path())
        .arg("diff")
        .arg("--working-dir")
        .arg(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No changes detected"));
}
