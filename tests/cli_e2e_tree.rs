//! End-to-end tests for the `tree` command.
//!
//! These tests invoke the actual CLI binary and validate the behavior of the
//! `tree` subcommand from a user's perspective.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;
use predicates::prelude::*;

/// Test that tree --help flag shows help information
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_tree_help() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("tree")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Display the repository inheritance tree",
        ));
}

/// Test that tree with a minimal config shows basic tree structure
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_tree_minimal_config() {
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
        .arg("--color=always")
        .arg("tree")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("🌳 Repository inheritance tree"))
        .stdout(predicate::str::contains("local @ HEAD"));
}

/// Test that tree with a config containing repositories shows inheritance tree
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_tree_config_with_repositories() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/example/repo1.git
    ref: main
- repo:
    url: https://github.com/example/repo2.git
    ref: v1.0.0
- include: ["*.rs"]
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("--color=always")
        .arg("tree")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("🌳 Repository inheritance tree"))
        .stdout(predicate::str::contains("local @ HEAD"))
        .stdout(predicate::str::contains(
            "├─ https://github.com/example/repo1.git @ main",
        ))
        .stdout(predicate::str::contains(
            "└─ https://github.com/example/repo2.git @ v1.0.0",
        ));
}

/// Test that tree with path-filtered repositories shows path information
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_tree_config_with_path_filtered_repos() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/example/repo1.git
    ref: main
    path: src
- repo:
    url: https://github.com/example/repo2.git
    ref: v1.0.0
    path: templates
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("--color=always")
        .arg("tree")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("🌳 Repository inheritance tree"))
        .stdout(predicate::str::contains("local @ HEAD"))
        .stdout(predicate::str::contains(
            "├─ https://github.com/example/repo1.git @ main",
        ))
        .stdout(predicate::str::contains(
            "└─ https://github.com/example/repo2.git @ v1.0.0",
        ));
}

/// Test that tree --depth 0 shows only root level
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_tree_depth_zero() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/example/repo1.git
    ref: main
- repo:
    url: https://github.com/example/repo2.git
    ref: v1.0.0
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("--color=always")
        .arg("tree")
        .arg("--config")
        .arg(config_file.path())
        .arg("--depth")
        .arg("0")
        .assert()
        .success()
        .stdout(predicate::str::contains("🌳 Repository inheritance tree"))
        .stdout(predicate::str::contains("local @ HEAD"))
        .stdout(predicate::str::contains("https://github.com/example/").not());
}

/// Test that tree --depth 1 shows one level of inheritance
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_tree_depth_one() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/example/repo1.git
    ref: main
- repo:
    url: https://github.com/example/repo2.git
    ref: v1.0.0
- repo:
    url: https://github.com/example/repo3.git
    ref: develop
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("--color=always")
        .arg("tree")
        .arg("--config")
        .arg(config_file.path())
        .arg("--depth")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("🌳 Repository inheritance tree"))
        .stdout(predicate::str::contains("local @ HEAD"))
        .stdout(predicate::str::contains(
            "├─ https://github.com/example/repo1.git @ main",
        ))
        .stdout(predicate::str::contains(
            "├─ https://github.com/example/repo2.git @ v1.0.0",
        ))
        .stdout(predicate::str::contains(
            "└─ https://github.com/example/repo3.git @ develop",
        ));
}

/// Test that tree with missing config file fails appropriately
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_tree_missing_config() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child("nonexistent.yaml");

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("tree")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to load config"));
}

/// Test that tree with invalid YAML fails appropriately
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_tree_invalid_yaml() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/example/repo.git
    ref: [unclosed bracket
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("tree")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to load config"));
}

/// Test that tree with repositories that have with: clauses shows the tree correctly
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_tree_config_with_with_clauses() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- repo:
    url: https://github.com/example/base-config.git
    ref: main
    with:
      - include: ["*.yaml"]
      - exclude: ["*.tmp"]
- repo:
    url: https://github.com/example/templates.git
    ref: v2.0.0
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("--color=always")
        .arg("tree")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("🌳 Repository inheritance tree"))
        .stdout(predicate::str::contains("local @ HEAD"))
        .stdout(predicate::str::contains(
            "├─ https://github.com/example/base-config.git @ main",
        ))
        .stdout(predicate::str::contains(
            "└─ https://github.com/example/templates.git @ v2.0.0",
        ));
}
