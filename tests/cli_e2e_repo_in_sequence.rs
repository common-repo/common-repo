//! E2E tests for repo-in-sequence resolution (Phase B).
//!
//! These tests validate that `repo:` operations inside `self:` blocks resolve
//! inline at their declaration position in the sequential pass, not eagerly
//! in Phase 1. This is the load-bearing acceptance test for the Phase B
//! architectural change.

mod common;

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;
use common::init_test_git_repo;

/// Reproduce the concrete bug from the Phase B plan.
///
/// The scenario: a consumer's `self:` block uses include + rename to pull local
/// files from a `src/` prefix, then uses `repo:` to merge in an upstream that
/// declares a YAML auto-merge. The auto-merge should fire at the `repo:`
/// declaration point (after include/rename have run), merging upstream content
/// into the renamed local file.
///
/// Setup:
/// - Upstream repo exposes `config.yaml` via YAML auto-merge
/// - Consumer has `src/config.yaml` with local settings
/// - Consumer `self:` block: include src/** → rename src/ prefix off → repo: upstream
///
/// Expected behavior (after fix):
/// 1. `include ["src/**"]` pulls local files matching src/** into the FS
/// 2. `rename ^src/(.*)$ → $1` strips the src/ prefix (config.yaml at root)
/// 3. `repo: upstream` resolves inline — auto-merge fires, merging upstream's
///    config.yaml into the local config.yaml
///
/// Current (broken) behavior:
/// - `repo:` resolves in Phase 1, auto-merge fires in Phase 5 step 4
///   (before consumer ops), then `include ["src/**"]` destroys the merged
///   result because it doesn't match the src/** pattern.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_self_block_repo_inline_with_auto_merge() {
    // Upstream repo: exposes config.yaml via YAML auto-merge
    let upstream = assert_fs::TempDir::new().unwrap();
    init_test_git_repo(
        &upstream,
        &[
            (
                "config.yaml",
                "settings:\n  from_upstream: true\n  log_level: info\n",
            ),
            (
                ".common-repo.yaml",
                "- include: [\"**\"]\n- yaml:\n    auto-merge: config.yaml\n",
            ),
        ],
        Some("v1.0.0"),
    )
    .unwrap();

    // Consumer: self block with include → rename → repo
    let consumer = assert_fs::TempDir::new().unwrap();
    let upstream_url = format!("file://{}", upstream.path().display());

    // Local config under src/ (will be renamed to root by the self block)
    consumer
        .child("src/config.yaml")
        .write_str("settings:\n  local_only: true\n  app_name: my-app\n")
        .unwrap();

    let config = format!(
        r#"- self:
  - include: ["src/**"]
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

    // config.yaml should exist at root (renamed from src/config.yaml)
    let config_path = consumer.path().join("config.yaml");
    assert!(
        config_path.exists(),
        "config.yaml should exist in consumer output (renamed from src/config.yaml)"
    );

    let content = std::fs::read_to_string(&config_path).unwrap();

    // Should contain the LOCAL settings (from src/config.yaml after rename)
    assert!(
        content.contains("local_only"),
        "Should contain local settings from src/config.yaml.\nActual content:\n{}",
        content
    );

    // Should ALSO contain the UPSTREAM settings (from auto-merge)
    // This is the assertion that fails with current code — the auto-merge
    // content is destroyed by the include ["src/**"] operation because
    // repo: resolves eagerly in Phase 1, not at its declaration position.
    assert!(
        content.contains("from_upstream"),
        "Should contain upstream settings via auto-merge at repo: declaration point.\n\
         This fails because repo: resolves eagerly in Phase 1, and the \
         auto-merge fires before include/rename run in the consumer ops pass.\n\
         Actual content:\n{}",
        content
    );
}
