//! End-to-end tests for the `info` command.
//!
//! These tests invoke the actual CLI binary and validate the behavior of the
//! `info` subcommand from a user's perspective.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;
use predicates::prelude::*;

/// Test that info --help flag shows help information
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_info_help() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("info")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Show information about a repository or the current configuration",
        ));
}

/// Test that info with a minimal config shows basic information
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_info_minimal_config() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- include: ["*.rs"]
- exclude: ["*.tmp"]
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("info")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("📋 Configuration:"))
        .stdout(predicate::str::contains("Inherited repositories: 0"))
        .stdout(predicate::str::contains("Operations: 2"))
        .stdout(predicate::str::contains("1 include operations"))
        .stdout(predicate::str::contains("1 exclude operations"));
}

/// Test that info with a config containing repositories shows repository information
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_info_config_with_repositories() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: main
- include: ["*.rs"]
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("info")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("📋 Configuration:"))
        .stdout(predicate::str::contains("Inherited repositories: 1"))
        .stdout(predicate::str::contains(
            "https://github.com/common-repo/common-repo.git @ main",
        ))
        .stdout(predicate::str::contains("Operations: 2"))
        .stdout(predicate::str::contains("1 repo operations"))
        .stdout(predicate::str::contains("1 include operations"));
}

/// Test that info with path-filtered repositories shows path information
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_info_config_with_path_filtered_repos() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: main
    path: src
- repo:
    url: https://github.com/common-repo/rust-cli.git
    ref: v1.0.0
    path: templates
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("info")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("📋 Configuration:"))
        .stdout(predicate::str::contains("Inherited repositories: 2"))
        .stdout(predicate::str::contains(
            "https://github.com/common-repo/common-repo.git @ main path:src",
        ))
        .stdout(predicate::str::contains(
            "https://github.com/common-repo/rust-cli.git @ v1.0.0 path:templates",
        ));
}

/// Test that info with various operation types shows correct counts
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_info_various_operations() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: main
- include: ["*.rs", "*.toml"]
- exclude: ["*.tmp"]
- rename:
    mappings:
      - from: "old_(.+)"
        to: "new_$1"
- template: ["*.md"]
- tools:
    tools:
      - name: cargo
        version: "*"
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("info")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Operations: 6"))
        .stdout(predicate::str::contains("1 repo operations"))
        .stdout(predicate::str::contains("1 include operations"))
        .stdout(predicate::str::contains("1 exclude operations"))
        .stdout(predicate::str::contains("1 rename operations"))
        .stdout(predicate::str::contains("1 template operations"))
        .stdout(predicate::str::contains("1 tools operations"));
}

/// Test that info with missing config file fails appropriately
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_info_missing_config() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child("nonexistent.yaml");

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("info")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to load config"));
}

/// Test that info with invalid YAML fails appropriately
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_info_invalid_yaml() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: [unclosed bracket
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("info")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to load config"));
}

/// Test that info with merge operations shows correct counts
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_info_merge_operations() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- yaml:
    source: config.yml
    dest: .github/workflows/ci.yml
- json:
    source: package.json
    dest: package.json
    path: dependencies
- toml:
    source: Cargo.toml
    dest: Cargo.toml
    path: dependencies
- ini:
    source: config.ini
    dest: config.ini
- markdown:
    source: README.md
    dest: README.md
    section: Features
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("info")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Operations: 5"))
        .stdout(predicate::str::contains("1 yaml merge operations"))
        .stdout(predicate::str::contains("1 json merge operations"))
        .stdout(predicate::str::contains("1 toml merge operations"))
        .stdout(predicate::str::contains("1 ini merge operations"))
        .stdout(predicate::str::contains("1 markdown merge operations"));
}

/// Test that info with template_vars shows correct count
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_info_template_vars() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- template_vars:
    vars:
      VERSION: "1.0.0"
      AUTHOR: "test"
- template: ["*.md"]
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("info")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Operations: 2"))
        .stdout(predicate::str::contains("1 template operations"))
        .stdout(predicate::str::contains("1 template_vars operations"));
}
