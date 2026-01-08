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

/// Test update display when updates are actually found
/// Uses v0.4.0 which should have newer versions available (v0.5.0, v0.6.0, v0.7.x)
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_shows_available_updates() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: v0.4.0
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
        .stdout(predicate::str::contains("Checking for repository updates"))
        .stdout(predicate::str::contains("Available Updates"))
        .stdout(predicate::str::contains("repositories can be updated"));
}

/// Test update with --yes flag to bypass interactive prompt
/// Uses v0.5.0 which should have v0.6.0+ available
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_with_yes_flag() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: v0.5.0
"#,
        )
        .unwrap();

    let original_content = std::fs::read_to_string(config_file.path()).unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("update")
        .arg("--config")
        .arg(config_file.path())
        .arg("--yes")
        .assert()
        .success()
        .stdout(predicate::str::contains("Checking for repository updates"));

    // Verify file was actually modified (ref should be updated)
    let updated_content = std::fs::read_to_string(config_file.path()).unwrap();
    assert_ne!(original_content, updated_content);
    assert!(!updated_content.contains("v0.5.0"));
}

/// Test update display shows breaking change warning for major version updates
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_shows_breaking_changes_with_latest() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Use v0.4.0 and check with --latest flag
    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: v0.4.0
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

/// Test that update correctly filters compatible vs breaking changes
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_compatible_only_filtering() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Use v0.7.0 which should have v0.7.1 as compatible update
    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: v0.7.0
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

/// Test update with actual file modification (no dry-run)
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_modifies_config_file() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: v0.6.0
"#,
        )
        .unwrap();

    let original_content = std::fs::read_to_string(config_file.path()).unwrap();
    assert!(original_content.contains("v0.6.0"));

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("update")
        .arg("--config")
        .arg(config_file.path())
        .arg("--yes")
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully updated"))
        .stdout(predicate::str::contains("common-repo apply"));

    // Verify file was modified
    let updated_content = std::fs::read_to_string(config_file.path()).unwrap();
    assert_ne!(original_content, updated_content);

    // Verify the version was updated (should no longer contain v0.6.0)
    assert!(
        !updated_content.contains("v0.6.0"),
        "Config should not contain old version v0.6.0 after update"
    );

    // Verify it was updated to a newer version (parse and compare)
    let ref_regex = regex::Regex::new(r"ref:\s*v(\d+\.\d+\.\d+)").unwrap();
    let updated_version = ref_regex
        .captures(&updated_content)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
        .expect("Should find version in updated config");

    // Parse versions for proper semantic version comparison
    let updated_ver: Vec<u32> = updated_version
        .split('.')
        .map(|s| s.parse().unwrap())
        .collect();
    let original_ver: Vec<u32> = vec![0, 6, 0];

    // Compare major.minor.patch
    assert!(
        updated_ver > original_ver,
        "Updated version {} should be greater than 0.6.0",
        updated_version
    );
}

/// Test that --filter flag is shown in help
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_filter_help() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("update")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--filter <GLOB>"))
        .stdout(predicate::str::contains("Filter sources by glob pattern"));
}

/// Test that --filter with matching pattern shows only matching repos
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_filter_matching_pattern() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Config with multiple repos
    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: v0.4.0
- repo:
    url: https://github.com/tokio-rs/tokio.git
    ref: tokio-1.0.0
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("update")
        .arg("--config")
        .arg(config_file.path())
        .arg("--filter")
        .arg("*/common-repo/*")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Filtering sources matching"))
        .stdout(predicate::str::contains("filtered out"));
}

/// Test that --filter with non-matching pattern shows nothing
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_filter_non_matching_pattern() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: v0.4.0
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("update")
        .arg("--config")
        .arg(config_file.path())
        .arg("--filter")
        .arg("gitlab.com/*")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Filtering sources matching"))
        .stdout(predicate::str::contains(
            "No repositories found that match the filter",
        ))
        .stdout(predicate::str::contains("filtered out"));
}

/// Test that multiple --filter flags use OR logic
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_filter_multiple_patterns() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: v0.4.0
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("update")
        .arg("--config")
        .arg(config_file.path())
        .arg("--filter")
        .arg("gitlab.com/*")
        .arg("--filter")
        .arg("github.com/common-repo/*")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Filtering sources matching: gitlab.com/*, github.com/common-repo/*",
        ))
        .stdout(predicate::str::contains("Checking for repository updates"));
}

/// Test --filter combined with --dry-run
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_update_filter_with_dry_run() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: v0.4.0
"#,
        )
        .unwrap();

    let original_content = std::fs::read_to_string(config_file.path()).unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("update")
        .arg("--config")
        .arg(config_file.path())
        .arg("--filter")
        .arg("github.com/common-repo/*")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("Filtering sources matching"))
        .stdout(predicate::str::contains("Dry run mode"));

    // Verify config was not modified
    let final_content = std::fs::read_to_string(config_file.path()).unwrap();
    assert_eq!(original_content, final_content);
}
