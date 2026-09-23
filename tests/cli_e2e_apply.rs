//! End-to-end tests for the `apply` command
//!
//! These tests invoke the actual CLI binary and validate its behavior
//! from a user's perspective.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;
use predicates::prelude::*;

/// Test that --help flag shows help information
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_help() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("apply")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Apply the .common-repo.yaml configuration",
        ));
}

/// Test that missing config file produces an error
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_missing_config() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("apply")
        .arg("--config")
        .arg("/nonexistent/config.yaml")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Configuration file not found"));
}

/// Test that missing default config file produces an error
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_missing_default_config() {
    let temp = assert_fs::TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("apply")
        .assert()
        .failure()
        .stderr(predicate::str::contains(".common-repo.yaml"));
}

/// Test that apply succeeds with valid minimal config
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_valid_config() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Write minimal valid config
    config_file
        .write_str(
            r#"
- include: ["README.md"]
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .arg("--dry-run")
        .arg("--quiet")
        .assert()
        .success();
}

/// Test that --dry-run flag shows dry run message
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_dry_run() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- include: ["**/*"]
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .arg("--dry-run")
        .assert()
        .success()
        .stderr(predicate::str::contains("DRY RUN MODE"));
}

/// Test that --verbose flag shows parsing information
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_verbose() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- include: ["**/*"]
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .arg("--dry-run")
        .arg("--verbose")
        .assert()
        .success()
        .stderr(predicate::str::contains("📋 Parsing configuration"));
}

/// Test that --force flag is accepted
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_force() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- include: ["**/*"]
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .arg("--dry-run")
        .arg("--force")
        .assert()
        .success()
        .stderr(predicate::str::contains("Applied successfully"));
}

/// Test that --no-cache flag is accepted
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_no_cache() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- include: ["**/*"]
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .arg("--dry-run")
        .arg("--no-cache")
        .assert()
        .success()
        .stderr(predicate::str::contains("Applied successfully"));
}

/// Test that --quiet flag suppresses output
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_quiet() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- include: ["**/*"]
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .arg("--dry-run")
        .arg("--quiet")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

/// Test that custom output directory is accepted
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_custom_output() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let output_dir = temp.child("output");

    config_file
        .write_str(
            r#"
- include: ["**/*"]
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .arg("--output")
        .arg(output_dir.path())
        .arg("--dry-run")
        .assert()
        .success()
        .stderr(predicate::str::contains("Applied successfully"));
}

/// Test that custom cache root is accepted
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_custom_cache_root() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let cache_dir = temp.child("cache");

    config_file
        .write_str(
            r#"
- include: ["**/*"]
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("--dry-run")
        .assert()
        .success()
        .stderr(predicate::str::contains("Applied successfully"));
}

/// Test that invalid YAML config produces an error
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_invalid_yaml() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Write invalid YAML
    config_file.write_str("invalid: yaml: content:").unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("YAML parsing error"));
}

/// Test the main binary --version flag
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_version() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("common-repo"));
}

/// Test the main binary --help flag
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_main_help() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Manage repository configuration inheritance",
        ));
}

/// Test that COMMON_REPO_CONFIG environment variable works
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_env_config() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child("custom-config.yaml");

    config_file
        .write_str(
            r#"
- include: ["**/*"]
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.env("COMMON_REPO_CONFIG", config_file.path())
        .arg("apply")
        .arg("--dry-run")
        .arg("--quiet")
        .assert()
        .success();
}

/// Test that COMMON_REPO_CACHE environment variable works
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_env_cache() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let cache_dir = temp.child("env-cache");

    config_file
        .write_str(
            r#"
- include: ["**/*"]
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.env("COMMON_REPO_CACHE", cache_dir.path())
        .arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .arg("--dry-run")
        .assert()
        .success()
        .stderr(predicate::str::contains("Applied successfully"));
}

/// Test that apply with multiple repo operations shows appropriate error for invalid URLs
/// This tests that the parallel cloning error handling works correctly
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_multiple_repos_invalid_urls() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Config with multiple repo operations that should fail
    // This tests the parallel error collection mechanism
    config_file
        .write_str(
            r#"
- repo:
    url: "https://invalid-domain-that-does-not-exist.example/repo1"
    ref: "main"
- repo:
    url: "https://invalid-domain-that-does-not-exist.example/repo2"
    ref: "main"
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    // Should fail with error about cloning
    cmd.arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("clone").or(predicate::str::contains("Git")));
}

/// Test that apply with nested repo inheritance produces appropriate error for invalid URLs
/// This tests multi-level parallel cloning behavior
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_nested_repo_inheritance_invalid() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Config with nested repo operations
    config_file
        .write_str(
            r#"
- repo:
    url: "https://invalid-domain-that-does-not-exist.example/parent"
    ref: "main"
    with:
      - include: ["**/*"]
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    // Should fail with error about cloning
    cmd.arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("clone").or(predicate::str::contains("Git")));
}

/// Seed a working directory with `src/test.txt` and a sibling local upstream
/// (`.common-repo.yaml` including everything, plus `tool.txt`). Returns the
/// working directory, the upstream directory, and the upstream's absolute
/// path as a string for use in a `repo:` URL.
fn self_block_fixture() -> (assert_fs::TempDir, assert_fs::TempDir, String) {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("src/test.txt")
        .write_str("local source file\n")
        .unwrap();

    let upstream = assert_fs::TempDir::new().unwrap();
    upstream
        .child(".common-repo.yaml")
        .write_str("- include: ['**']\n")
        .unwrap();
    upstream
        .child("tool.txt")
        .write_str("tool from self upstream\n")
        .unwrap();

    let upstream_path = upstream
        .path()
        .canonicalize()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    (temp, upstream, upstream_path)
}

/// With a `self:` block, `apply` tells the user that only the `self:` output
/// is written, in both normal and dry-run mode.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_with_self_block_reports_self_output_only() {
    let (temp, _upstream, upstream_path) = self_block_fixture();

    temp.child(".common-repo.yaml")
        .write_str(&format!(
            r#"
- self:
    - repo:
        url: {upstream_path}
- include: ["src/**"]
- rename:
    - "^src/(.*)$": "$1"
"#
        ))
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(temp.path())
        .arg("apply")
        .arg("--dry-run")
        .assert()
        .success()
        .stderr(predicate::str::contains("self: block present"));

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(temp.path())
        .arg("apply")
        .assert()
        .success()
        .stderr(predicate::str::contains("Applied successfully"))
        .stderr(predicate::str::contains("self: block present"));

    temp.child("tool.txt").assert(predicate::path::exists());
    temp.child("test.txt").assert(predicate::path::missing());
}

/// Without a `self:` block, `apply` does not print the `self:` note.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_without_self_block_has_no_self_note() {
    let (temp, _upstream, _upstream_path) = self_block_fixture();

    temp.child(".common-repo.yaml")
        .write_str(
            r#"
- include: ["src/**"]
- rename:
    - "^src/(.*)$": "$1"
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(temp.path())
        .arg("apply")
        .assert()
        .success()
        .stderr(predicate::str::contains("Applied successfully"))
        .stderr(predicate::str::contains("self: block present").not());

    temp.child("test.txt").assert(predicate::path::exists());
}
