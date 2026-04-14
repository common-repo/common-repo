//! E2E tests for dotfile handling.
//!
//! common-repo's core purpose is managing dotfiles and workflow configs.
//! These tests verify that dotfiles are properly loaded and accessible
//! to all operations.

mod common;

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;
use common::init_test_git_repo;
use predicates::prelude::*;

/// Local dotfiles should be accessible to self: block operations.
///
/// A self: block with include should be able to pull in a local dotfile
/// from the working directory and rename it to the output.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_self_block_includes_local_dotfile() {
    let consumer = assert_fs::TempDir::new().unwrap();

    consumer
        .child("src/.pre-commit-config.yaml")
        .write_str("repos:\n  - repo: local\n    hooks:\n      - id: trailing-whitespace\n")
        .unwrap();

    consumer
        .child(".common-repo.yaml")
        .write_str(
            r#"- self:
  - include: ["src/**", "src/.*", "src/.*/**"]
  - rename:
      - from: "^src/(.*)$"
        to: "$1"
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.arg("apply")
        .current_dir(consumer.path())
        .assert()
        .success();

    let output_path = consumer.path().join(".pre-commit-config.yaml");
    assert!(
        output_path.exists(),
        "Local dotfile src/.pre-commit-config.yaml should be included and \
         renamed to .pre-commit-config.yaml in output.\n\
         Files present: {:?}",
        std::fs::read_dir(consumer.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name())
            .collect::<Vec<_>>()
    );

    let content = std::fs::read_to_string(&output_path).unwrap();
    assert!(
        content.contains("trailing-whitespace"),
        "Output should contain the local dotfile content.\nActual: {}",
        content
    );
}

/// Local dotfiles should survive the composite merge in a self: block.
///
/// When a self: block includes a local dotfile and also pulls in an upstream
/// repo, the local dotfile should be present in the final output even if the
/// upstream doesn't provide that file.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_self_block_local_dotfile_survives_with_upstream() {
    let upstream = assert_fs::TempDir::new().unwrap();
    init_test_git_repo(
        &upstream,
        &[("README.md", "# Upstream readme\n")],
        Some("v1.0.0"),
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    let upstream_url = format!("file://{}", upstream.path().display());

    consumer
        .child("src/.eslintrc.yaml")
        .write_str("rules:\n  no-console: error\n")
        .unwrap();

    let config = format!(
        r#"- self:
  - include: ["src/**", "src/.*", "src/.*/**"]
  - rename:
      - from: "^src/(.*)$"
        to: "$1"
  - repo:
      url: "{}"
      ref: v1.0.0
"#,
        upstream_url
    );
    consumer
        .child(".common-repo.yaml")
        .write_str(&config)
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.arg("apply")
        .current_dir(consumer.path())
        .assert()
        .success();

    let eslintrc = consumer.path().join(".eslintrc.yaml");
    assert!(
        eslintrc.exists(),
        "Local dotfile .eslintrc.yaml should be present in output after \
         self: block with upstream repo.\n\
         Files present: {:?}",
        std::fs::read_dir(consumer.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name())
            .collect::<Vec<_>>()
    );

    let content = std::fs::read_to_string(&eslintrc).unwrap();
    assert!(
        content.contains("no-console"),
        "Dotfile content should be preserved.\nActual: {}",
        content
    );
}

/// Upstream dotfiles should be delivered to the consumer.
///
/// When an upstream repo includes dotfiles (e.g., .editorconfig), they
/// should appear in the consumer's output.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_upstream_dotfiles_delivered_to_consumer() {
    let upstream = assert_fs::TempDir::new().unwrap();
    init_test_git_repo(
        &upstream,
        &[
            (
                ".common-repo.yaml",
                "- include: [\"**\", \".*\", \".*/**\"]\n",
            ),
            (
                ".editorconfig",
                "root = true\n\n[*]\nindent_style = space\n",
            ),
            ("README.md", "# Test\n"),
        ],
        None,
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    let upstream_url = format!("file://{}", upstream.path().display());

    let config = format!(
        r#"- repo:
    url: "{}"
    ref: main
"#,
        upstream_url
    );
    consumer
        .child(".common-repo.yaml")
        .write_str(&config)
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.arg("apply")
        .current_dir(consumer.path())
        .assert()
        .success();

    // Non-dotfile should be present
    consumer
        .child("README.md")
        .assert(predicate::path::exists());

    // Dotfile from upstream should also be present
    let editorconfig = consumer.path().join(".editorconfig");
    assert!(
        editorconfig.exists(),
        "Upstream dotfile .editorconfig should be delivered to consumer.\n\
         Files present: {:?}",
        std::fs::read_dir(consumer.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name())
            .collect::<Vec<_>>()
    );

    let content = std::fs::read_to_string(&editorconfig).unwrap();
    assert!(
        content.contains("indent_style"),
        "Upstream dotfile content should be present.\nActual: {}",
        content
    );
}
