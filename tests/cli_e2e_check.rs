//! End-to-end tests for the `check` command
//!
//! These tests invoke the actual CLI binary and validate its behavior
//! from a user's perspective.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;
use predicates::prelude::*;

/// Test that --help flag shows help information
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_check_help() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("check")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Check configuration validity and check for repository updates",
        ));
}

/// Test that missing config file produces an error
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_check_missing_config() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("check")
        .arg("--config")
        .arg("/nonexistent/config.yaml")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to load config"));
}

/// Test that missing default config file produces an error
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_check_missing_default_config() {
    let temp = assert_fs::TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains(".common-repo.yaml"));
}

/// Test that check succeeds with valid minimal config (no repos)
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_check_valid_config_no_repos() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Write minimal valid config with no repositories
    config_file
        .write_str(
            r#"
- include:
    patterns: ["README.md"]
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("check")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Configuration loaded successfully",
        ))
        .stdout(predicate::str::contains("Repositories: 0"));
}

/// Test that check succeeds with valid config containing repositories
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_check_valid_config_with_repos() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Write config with a repository
    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: main
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("check")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Configuration loaded successfully",
        ))
        .stdout(predicate::str::contains("Repositories: 1"));
}

/// Test that check shows operation counts correctly
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_check_shows_operation_counts() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Write config with multiple operations
    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: main
- include:
    patterns: ["**/*.md"]
- exclude:
    patterns: ["vendor/**"]
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("check")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Operations: 3"))
        .stdout(predicate::str::contains("Repositories: 1"))
        .stdout(predicate::str::contains("Other operations: 2"));
}

/// Test that check shows tip about --updates flag
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_check_shows_updates_tip() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- include:
    patterns: ["*.md"]
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("check")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Use --updates to check for repository version updates",
        ));
}

/// Test that invalid YAML config produces an error
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_check_invalid_yaml() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Write invalid YAML
    config_file.write_str("invalid: yaml: content:").unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("check")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to load config"));
}

/// Test that custom cache root is accepted
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_check_custom_cache_root() {
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

    cmd.arg("check")
        .arg("--config")
        .arg(config_file.path())
        .arg("--cache-root")
        .arg(cache_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Configuration loaded successfully",
        ));
}

/// Test that COMMON_REPO_CACHE environment variable works
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_check_env_cache() {
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
        .arg("check")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Configuration loaded successfully",
        ));
}

/// Test --updates flag with config that has no repositories
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_check_updates_no_repos() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- include:
    patterns: ["*.md"]
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("check")
        .arg("--config")
        .arg(config_file.path())
        .arg("--updates")
        .assert()
        .success()
        .stdout(predicate::str::contains("Checking for repository updates"))
        .stdout(predicate::str::contains(
            "No repositories found that can be checked for updates",
        ));
}

/// Test --updates flag with repository (network-dependent, checks actual repo)
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_check_updates_with_real_repo() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Use a real repository with semantic versioning
    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: v0.1.0
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("check")
        .arg("--config")
        .arg(config_file.path())
        .arg("--updates")
        .assert()
        .success()
        .stdout(predicate::str::contains("Checking for repository updates"));
    // Note: We can't assert on specific output since it depends on current repo state
    // But we can verify the command runs successfully
}

/// Test --updates flag with current version (should show "up to date" or updates available)
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_check_updates_with_latest_version() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Use main branch (should be up to date)
    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: main
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("check")
        .arg("--config")
        .arg(config_file.path())
        .arg("--updates")
        .assert()
        .success()
        .stdout(predicate::str::contains("Checking for repository updates"));
}

/// Test --updates with multiple repositories
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_check_updates_multiple_repos() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Use multiple repositories
    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: v0.1.0
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: main
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("check")
        .arg("--config")
        .arg(config_file.path())
        .arg("--updates")
        .assert()
        .success()
        .stdout(predicate::str::contains("Checking for repository updates"));
}

/// Test --updates output format when updates are checked
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_check_updates_output_format() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: v0.1.0
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    let output = cmd
        .arg("check")
        .arg("--config")
        .arg(config_file.path())
        .arg("--updates")
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify that the command produces output
    assert!(
        stdout.contains("Checking for repository updates")
            || stdout.contains("All repositories are up to date")
            || stdout.contains("No repositories found")
    );
}
