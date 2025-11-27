//! End-to-end tests for the `common-repo ls` command.
//!
//! These tests verify the CLI behavior of the `ls` command by invoking
//! the binary directly and checking its output.

use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

/// Get a Command for the common-repo binary
fn common_repo_cmd() -> Command {
    Command::cargo_bin("common-repo").unwrap()
}

#[test]
fn test_ls_help() {
    common_repo_cmd()
        .arg("ls")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("List files that would be created/modified"));
}

#[test]
fn test_ls_missing_config() {
    let temp = assert_fs::TempDir::new().unwrap();

    common_repo_cmd()
        .current_dir(temp.path())
        .arg("ls")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Configuration file not found"));
}

#[test]
fn test_ls_with_simple_config() {
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
    temp.child("test.txt").write_str("hello world").unwrap();

    common_repo_cmd()
        .current_dir(temp.path())
        .arg("ls")
        .arg("--working-dir")
        .arg(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("test.txt"))
        .stdout(predicate::str::contains("file(s)"));
}

#[test]
fn test_ls_with_count_flag() {
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

    // Create test files
    temp.child("file1.txt").write_str("content1").unwrap();
    temp.child("file2.txt").write_str("content2").unwrap();

    common_repo_cmd()
        .current_dir(temp.path())
        .arg("ls")
        .arg("--working-dir")
        .arg(temp.path())
        .arg("--count")
        .assert()
        .success()
        // Output should be just a number (the count)
        .stdout(predicate::str::is_match(r"^\d+\n$").unwrap());
}

#[test]
fn test_ls_with_long_format() {
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
    temp.child("test.txt").write_str("hello").unwrap();

    common_repo_cmd()
        .current_dir(temp.path())
        .arg("ls")
        .arg("--working-dir")
        .arg(temp.path())
        .arg("--long")
        .assert()
        .success()
        // Long format should show permissions (rw-r--r--)
        .stdout(predicate::str::contains("rw-"))
        .stdout(predicate::str::contains("test.txt"));
}

#[test]
fn test_ls_with_pattern_filter() {
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

    // Create test files
    temp.child("file.txt").write_str("text").unwrap();
    temp.child("file.rs").write_str("rust").unwrap();

    // Filter to only .rs files
    common_repo_cmd()
        .current_dir(temp.path())
        .arg("ls")
        .arg("--working-dir")
        .arg(temp.path())
        .arg("--pattern")
        .arg("*.rs")
        .assert()
        .success()
        .stdout(predicate::str::contains("file.rs"))
        .stdout(predicate::str::contains("file.txt").not());
}

#[test]
fn test_ls_with_sort_by_path() {
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

    // Create test files
    temp.child("zebra.txt").write_str("z").unwrap();
    temp.child("alpha.txt").write_str("a").unwrap();

    common_repo_cmd()
        .current_dir(temp.path())
        .arg("ls")
        .arg("--working-dir")
        .arg(temp.path())
        .arg("--sort")
        .arg("path")
        .assert()
        .success()
        // Both files should be present
        .stdout(predicate::str::contains("alpha.txt"))
        .stdout(predicate::str::contains("zebra.txt"));
}

#[test]
fn test_ls_with_reverse_sort() {
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

    // Create test files
    temp.child("aaa.txt").write_str("a").unwrap();
    temp.child("zzz.txt").write_str("z").unwrap();

    common_repo_cmd()
        .current_dir(temp.path())
        .arg("ls")
        .arg("--working-dir")
        .arg(temp.path())
        .arg("--sort")
        .arg("name")
        .arg("--reverse")
        .assert()
        .success();
}

#[test]
fn test_ls_empty_result() {
    let temp = assert_fs::TempDir::new().unwrap();

    // Create config file that excludes everything
    temp.child(".common-repo.yaml")
        .write_str(
            r#"
- include:
    patterns: ["*.nonexistent"]
"#,
        )
        .unwrap();

    common_repo_cmd()
        .current_dir(temp.path())
        .arg("ls")
        .arg("--working-dir")
        .arg(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No files would be created"));
}

#[test]
fn test_ls_invalid_pattern() {
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

    // Create a test file so the pipeline produces results
    temp.child("test.txt").write_str("content").unwrap();

    common_repo_cmd()
        .current_dir(temp.path())
        .arg("ls")
        .arg("--working-dir")
        .arg(temp.path())
        .arg("--pattern")
        .arg("[invalid") // Invalid glob pattern
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid glob pattern"));
}
