//! End-to-end tests for the `validate` command.
//!
//! These tests invoke the actual CLI binary and validate the behavior of the
//! `validate` subcommand from a user's perspective.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_valid_config() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

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

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_invalid_yaml() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Use actually invalid YAML syntax (unmatched bracket)
    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: [unclosed
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .failure();
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_invalid_regex_pattern() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- rename:
    mappings:
      - from: "[invalid(regex"
        to: "valid"
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .failure();
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_invalid_glob_pattern() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- include:
    - "[invalid"
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .failure();
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_empty_tools_warning() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- tools:
    tools: []
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    // Should succeed but with warnings
    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_empty_tools_strict_mode() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- tools:
    tools: []
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    // Should fail in strict mode due to warnings
    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .arg("--strict")
        .assert()
        .failure();
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_missing_config_file() {
    let temp = assert_fs::TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg("nonexistent.yaml")
        .assert()
        .failure();
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_default_config_path() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

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

    // Should use default .common-repo.yaml path
    cmd.current_dir(temp.path())
        .arg("validate")
        .assert()
        .success();
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_complex_valid_config() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/common-repo/common-repo.git
    ref: main

- rename:
    mappings:
      - from: "^old_"
        to: "new_"

- include:
    - "*.rs"
    - "Cargo.toml"

- exclude:
    - "target/**"
    - "*.bak"

- tools:
    tools:
      - name: cargo
        version: ">=1.70.0"
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_with_custom_cache_root() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let cache_dir = temp.child("custom-cache");

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

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .arg("--cache-root")
        .arg(cache_dir.path())
        .assert()
        .success();
}
