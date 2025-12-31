//! End-to-end tests for the `common-repo ls` command.
//!
//! These tests verify the CLI behavior of the `ls` command by invoking
//! the binary directly and checking its output.

mod common;
use common::prelude::*;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ls_help() {
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.arg("ls")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "List files that would be created/modified",
        ));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ls_missing_config() {
    let fixture = TestFixture::new();

    fixture
        .command()
        .arg("ls")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Configuration file not found"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ls_with_simple_config() {
    let fixture = TestFixture::new()
        .with_minimal_config()
        .with_file("test.txt", "hello world");

    fixture
        .command()
        .arg("ls")
        .arg("--working-dir")
        .arg(fixture.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("test.txt"))
        .stdout(predicate::str::contains("file(s)"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ls_with_count_flag() {
    let fixture = TestFixture::new()
        .with_minimal_config()
        .with_file("file1.txt", "content1")
        .with_file("file2.txt", "content2");

    fixture
        .command()
        .arg("ls")
        .arg("--working-dir")
        .arg(fixture.path())
        .arg("--count")
        .assert()
        .success()
        // Output should be just a number (the count)
        .stdout(predicate::str::is_match(r"^\d+\n$").unwrap());
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ls_with_long_format() {
    let fixture = TestFixture::new()
        .with_minimal_config()
        .with_file("test.txt", "hello");

    fixture
        .command()
        .arg("ls")
        .arg("--working-dir")
        .arg(fixture.path())
        .arg("--long")
        .assert()
        .success()
        // Long format should show permissions (rw-r--r--)
        .stdout(predicate::str::contains("rw-"))
        .stdout(predicate::str::contains("test.txt"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ls_with_pattern_filter() {
    let fixture = TestFixture::new()
        .with_minimal_config()
        .with_file("file.txt", "text")
        .with_file("file.rs", "rust");

    // Filter to only .rs files
    fixture
        .command()
        .arg("ls")
        .arg("--working-dir")
        .arg(fixture.path())
        .arg("--pattern")
        .arg("*.rs")
        .assert()
        .success()
        .stdout(predicate::str::contains("file.rs"))
        .stdout(predicate::str::contains("file.txt").not());
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ls_with_sort_by_path() {
    let fixture = TestFixture::new()
        .with_minimal_config()
        .with_file("zebra.txt", "z")
        .with_file("alpha.txt", "a");

    fixture
        .command()
        .arg("ls")
        .arg("--working-dir")
        .arg(fixture.path())
        .arg("--sort")
        .arg("path")
        .assert()
        .success()
        // Both files should be present
        .stdout(predicate::str::contains("alpha.txt"))
        .stdout(predicate::str::contains("zebra.txt"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ls_with_reverse_sort() {
    let fixture = TestFixture::new()
        .with_minimal_config()
        .with_file("aaa.txt", "a")
        .with_file("zzz.txt", "z");

    fixture
        .command()
        .arg("ls")
        .arg("--working-dir")
        .arg(fixture.path())
        .arg("--sort")
        .arg("name")
        .arg("--reverse")
        .assert()
        .success();
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ls_empty_result() {
    let fixture = TestFixture::new().with_config(
        r#"
- include:
    patterns: ["*.nonexistent"]
"#,
    );

    fixture
        .command()
        .arg("ls")
        .arg("--working-dir")
        .arg(fixture.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No files would be created"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ls_invalid_pattern() {
    let fixture = TestFixture::new()
        .with_minimal_config()
        .with_file("test.txt", "content");

    fixture
        .command()
        .arg("ls")
        .arg("--working-dir")
        .arg(fixture.path())
        .arg("--pattern")
        .arg("[invalid") // Invalid glob pattern
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid glob pattern"));
}
