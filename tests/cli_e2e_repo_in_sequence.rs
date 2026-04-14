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

/// Validate that multiple `repo:` operations within a single `self:` block
/// resolve sequentially, each seeing the accumulated state from prior ops.
///
/// Setup:
/// - upstream_a exposes `config.yaml` via YAML auto-merge
/// - upstream_b exposes `config.yaml` via YAML auto-merge
/// - Consumer `self:` block: include src/** → rename src/ prefix off → repo: upstream_a → repo: upstream_b
/// - Consumer has `src/config.yaml` with local settings
///
/// Expected: config.yaml contains local + upstream_a + upstream_b settings.
/// The second repo:'s auto-merge must see the first repo:'s integrated content.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_self_block_multiple_repo_ops_with_auto_merge() {
    let upstream_a = assert_fs::TempDir::new().unwrap();
    init_test_git_repo(
        &upstream_a,
        &[
            ("config.yaml", "settings:\n  from_a: true\n"),
            (
                ".common-repo.yaml",
                "- include: [\"**\"]\n- yaml:\n    auto-merge: config.yaml\n",
            ),
        ],
        Some("v1.0.0"),
    )
    .unwrap();

    let upstream_b = assert_fs::TempDir::new().unwrap();
    init_test_git_repo(
        &upstream_b,
        &[
            ("config.yaml", "settings:\n  from_b: true\n"),
            (
                ".common-repo.yaml",
                "- include: [\"**\"]\n- yaml:\n    auto-merge: config.yaml\n",
            ),
        ],
        Some("v1.0.0"),
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    let url_a = format!("file://{}", upstream_a.path().display());
    let url_b = format!("file://{}", upstream_b.path().display());

    consumer
        .child("src/config.yaml")
        .write_str("settings:\n  local: true\n")
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
  - repo:
      url: "{}"
      ref: v1.0.0
"#,
        url_a, url_b
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

    let config_path = consumer.path().join("config.yaml");
    assert!(
        config_path.exists(),
        "config.yaml should exist after rename from src/config.yaml"
    );

    let content = std::fs::read_to_string(&config_path).unwrap();

    assert!(
        content.contains("local"),
        "Should contain local settings.\nActual content:\n{}",
        content
    );
    assert!(
        content.contains("from_a"),
        "Should contain upstream_a settings via first repo: auto-merge.\nActual content:\n{}",
        content
    );
    assert!(
        content.contains("from_b"),
        "Should contain upstream_b settings via second repo: auto-merge.\nActual content:\n{}",
        content
    );
}

/// Validate that a top-level `repo:` (not inside `self:`) still works.
///
/// This is a backward-compatibility guard: the Phase B refactor must not
/// break the original non-self batch pipeline.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_top_level_repo_still_works() {
    let upstream = assert_fs::TempDir::new().unwrap();
    init_test_git_repo(
        &upstream,
        &[
            ("README.md", "# Upstream README\n"),
            ("src/lib.rs", "pub fn hello() {}\n"),
            (".common-repo.yaml", "- include: [\"**\"]\n"),
        ],
        Some("v1.0.0"),
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    let upstream_url = format!("file://{}", upstream.path().display());

    let config = format!(
        r#"- repo:
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

    let readme = consumer.path().join("README.md");
    assert!(
        readme.exists(),
        "README.md should exist from top-level repo: resolution"
    );

    let lib = consumer.path().join("src/lib.rs");
    assert!(
        lib.exists(),
        "src/lib.rs should exist from top-level repo: resolution"
    );
}

/// Validate recursive `repo:` resolution within a `self:` block.
///
/// Setup:
/// - grandchild repo provides `shared_lib.rs`
/// - child repo provides `app.rs` and references grandchild via `repo:`
/// - consumer `self:` block: include src/** → rename → repo: child
///
/// Expected: consumer output contains local file, child's file, and
/// grandchild's file — proving that nested repo: chains resolve fully.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_nested_repo_in_self_block() {
    let grandchild = assert_fs::TempDir::new().unwrap();
    init_test_git_repo(
        &grandchild,
        &[
            ("shared_lib.rs", "pub fn shared() {}\n"),
            (".common-repo.yaml", "- include: [\"**\"]\n"),
        ],
        Some("v1.0.0"),
    )
    .unwrap();

    let grandchild_url = format!("file://{}", grandchild.path().display());

    let child = assert_fs::TempDir::new().unwrap();
    let child_config = format!(
        "- include: [\"**\"]\n- repo:\n    url: \"{}\"\n    ref: v1.0.0\n",
        grandchild_url
    );
    init_test_git_repo(
        &child,
        &[
            ("app.rs", "pub fn app() {}\n"),
            (".common-repo.yaml", &child_config),
        ],
        Some("v1.0.0"),
    )
    .unwrap();

    let child_url = format!("file://{}", child.path().display());

    let consumer = assert_fs::TempDir::new().unwrap();
    consumer
        .child("src/readme.txt")
        .write_str("Local readme content\n")
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
        child_url
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

    let readme = consumer.path().join("readme.txt");
    assert!(
        readme.exists(),
        "readme.txt should exist (local file after rename from src/readme.txt)"
    );

    let app = consumer.path().join("app.rs");
    assert!(
        app.exists(),
        "app.rs should exist from child repo: resolution"
    );

    let shared = consumer.path().join("shared_lib.rs");
    assert!(
        shared.exists(),
        "shared_lib.rs should exist from grandchild repo: (nested resolution)"
    );
}
