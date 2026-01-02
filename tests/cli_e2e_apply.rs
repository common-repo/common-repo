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
- include:
    patterns: ["README.md"]
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
- include:
    patterns: ["**/*"]
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
- include:
    patterns: ["**/*"]
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
- include:
    patterns: ["**/*"]
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
- include:
    patterns: ["**/*"]
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
- include:
    patterns: ["**/*"]
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
- include:
    patterns: ["**/*"]
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
- include:
    patterns: ["**/*"]
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
- include:
    patterns: ["**/*"]
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
- include:
    patterns: ["**/*"]
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
      - include:
          patterns: ["**/*"]
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
