//!
//! These tests invoke the actual CLI binary and validate its behavior
//! from a user's perspective.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;
use predicates::prelude::*;

/// Test that --help flag shows help information
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_help() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("update")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Update repository refs to newer versions",
        ));
}

/// Test that missing config file produces an error
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_missing_config() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("update")
        .arg("--config")
        .arg("/nonexistent/config.yaml")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to load config"));
}

/// Test that missing default config file produces an error
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_missing_default_config() {
    let temp = assert_fs::TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("update")
        .assert()
        .failure()
        .stderr(predicate::str::contains(".common-repo.yaml"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_no_repos() {
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

    cmd.arg("update")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Checking for repository updates"))
        .stdout(predicate::str::contains(
            "No repositories found that can be checked for updates",
        ));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_detects_outdated_refs() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: common-repo-v0.1.0
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("update")
        .arg("--config")
        .arg(config_file.path())
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Checking for repository updates"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_with_subpath_reference() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: common-repo-v0.1.0
    path: tests/testdata/simulated-repo-2
    with:
      - include: ["**/*"]
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("update")
        .arg("--config")
        .arg(config_file.path())
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Checking for repository updates"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_dry_run() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: common-repo-v0.1.0
"#,
        )
        .unwrap();

    let original_content = std::fs::read_to_string(config_file.path()).unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("update")
        .arg("--config")
        .arg(config_file.path())
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Checking for repository updates"));

    let final_content = std::fs::read_to_string(config_file.path()).unwrap();
    assert_eq!(original_content, final_content);
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_compatible_flag() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: common-repo-v0.1.0
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("update")
        .arg("--config")
        .arg(config_file.path())
        .arg("--compatible")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Checking for repository updates"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_latest_flag() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: common-repo-v0.1.0
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("update")
        .arg("--config")
        .arg(config_file.path())
        .arg("--latest")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Checking for repository updates"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_custom_cache_root() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let cache_dir = temp.child("cache");

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

    cmd.arg("update")
        .arg("--config")
        .arg(config_file.path())
        .arg("--cache-root")
        .arg(cache_dir.path())
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Checking for repository updates"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_with_current_version() {
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

    cmd.arg("update")
        .arg("--config")
        .arg(config_file.path())
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Checking for repository updates"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_invalid_yaml() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Write invalid YAML
    config_file.write_str("invalid: yaml: content:").unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("update")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to load config"));
}

/// Test update refs behavior with the update-refs-test fixture
/// This test uses the fixture that references common-repo-v0.3.0 with a subpath.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_refs_fixture_subpath() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Load the update-refs-test fixture content
    let fixture_content = include_str!("testdata/update-refs-test/.common-repo.yaml");
    config_file.write_str(fixture_content).unwrap();

    let original_content = std::fs::read_to_string(config_file.path()).unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("update")
        .arg("--config")
        .arg(config_file.path())
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Checking for repository updates"));

    // Verify config file was not modified with --dry-run
    let final_content = std::fs::read_to_string(config_file.path()).unwrap();
    assert_eq!(original_content, final_content);
}
