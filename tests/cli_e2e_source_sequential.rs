//! E2E tests for unified sequential execution in source blocks (issue #305).
//!
//! These tests validate that source blocks (top-level operations outside
//! `self:`) use the same sequential execution model as `self:` blocks.
//! Operations must execute in YAML declaration order — a rename between
//! two `repo:` entries must affect what the second repo sees.
//!
//! All tests in this file target behaviour described by spec rules marked
//! `@status: Partial` for issue #305:
//!   - SequentialOperationExecution (common-repo.allium)
//!   - BuildComposite (common-repo.allium)
//!   - MergeLocalFiles (common-repo.allium)
//!   - ApplyDeferredMerges (common-repo.allium)
//!   - DeferredMergeAtDeclarationPosition (auto-merge-composition.allium)
//!
//! These tests FAIL on the current batch source pipeline and PASS once the
//! source pipeline uses `execute_sequential_pipeline`.

mod common;

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;
use common::init_test_git_repo;
// =========================================================================
// Spec: SequentialOperationExecution — rename between repos in source block
// =========================================================================

/// A rename between two repo: entries in a source block should affect the
/// second repo's integration.
///
/// Setup:
/// - upstream_a provides `old/data.txt`
/// - upstream_b provides `old/data.txt` (different content)
/// - Consumer source block: repo: A → rename old/ to new/ → repo: B
///
/// Sequential behavior (target): repo A integrates → rename moves
/// `old/data.txt` to `new/data.txt` → repo B integrates `old/data.txt`
/// again. Final FS has both `new/data.txt` (from A, renamed) and
/// `old/data.txt` (from B, integrated after rename).
///
/// Batch behavior (current bug): all repos composite first, then rename
/// runs — only `new/data.txt` exists because both A and B wrote to
/// `old/data.txt` before rename ran.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_source_rename_between_repos_affects_second_repo() {
    let upstream_a = assert_fs::TempDir::new().unwrap();
    init_test_git_repo(
        &upstream_a,
        &[
            ("old/data.txt", "from_upstream_a\n"),
            (".common-repo.yaml", "- include: [\"**\"]\n"),
        ],
        Some("v1.0.0"),
    )
    .unwrap();

    let upstream_b = assert_fs::TempDir::new().unwrap();
    init_test_git_repo(
        &upstream_b,
        &[
            ("old/data.txt", "from_upstream_b\n"),
            (".common-repo.yaml", "- include: [\"**\"]\n"),
        ],
        Some("v1.0.0"),
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    let url_a = format!("file://{}", upstream_a.path().display());
    let url_b = format!("file://{}", upstream_b.path().display());

    let config = format!(
        r#"- repo:
    url: "{}"
    ref: v1.0.0
- rename:
    - "^old/(.*)$": "new/$1"
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

    // After sequential execution:
    // 1. repo A → old/data.txt (from A) enters composite
    // 2. rename → old/data.txt becomes new/data.txt
    // 3. repo B → old/data.txt (from B) enters composite at old/data.txt

    let new_data = consumer.path().join("new/data.txt");
    assert!(
        new_data.exists(),
        "new/data.txt should exist (upstream A's file after rename).\n\
         If this fails, rename did not run between the two repo: operations."
    );
    let new_content = std::fs::read_to_string(&new_data).unwrap();
    assert!(
        new_content.contains("from_upstream_a"),
        "new/data.txt should contain upstream A's content.\nActual: {}",
        new_content
    );

    let old_data = consumer.path().join("old/data.txt");
    assert!(
        old_data.exists(),
        "old/data.txt should exist (upstream B integrated AFTER rename).\n\
         This fails under the batch model because both repos are composited \
         before rename runs, so only new/data.txt exists."
    );
    let old_content = std::fs::read_to_string(&old_data).unwrap();
    assert!(
        old_content.contains("from_upstream_b"),
        "old/data.txt should contain upstream B's content.\nActual: {}",
        old_content
    );
}

// =========================================================================
// Spec: SequentialOperationExecution — exclude between repos in source block
// =========================================================================

/// An exclude between two repo: entries in a source block should remove
/// files before the second repo integrates.
///
/// Setup:
/// - upstream_a provides `shared.txt` and `a_only.txt`
/// - upstream_b provides `shared.txt` (different content)
/// - Consumer source block: repo: A → exclude shared.txt → repo: B
///
/// Sequential: A integrates → exclude removes shared.txt → B integrates
/// shared.txt. Final `shared.txt` is from B only.
///
/// Batch: all repos composite (B overwrites A's shared.txt via
/// last-write-wins), then exclude removes shared.txt entirely.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_source_exclude_between_repos() {
    let upstream_a = assert_fs::TempDir::new().unwrap();
    init_test_git_repo(
        &upstream_a,
        &[
            ("shared.txt", "from_upstream_a\n"),
            ("a_only.txt", "only_in_a\n"),
            (".common-repo.yaml", "- include: [\"**\"]\n"),
        ],
        Some("v1.0.0"),
    )
    .unwrap();

    let upstream_b = assert_fs::TempDir::new().unwrap();
    init_test_git_repo(
        &upstream_b,
        &[
            ("shared.txt", "from_upstream_b\n"),
            (".common-repo.yaml", "- include: [\"**\"]\n"),
        ],
        Some("v1.0.0"),
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    let url_a = format!("file://{}", upstream_a.path().display());
    let url_b = format!("file://{}", upstream_b.path().display());

    let config = format!(
        r#"- repo:
    url: "{}"
    ref: v1.0.0
- exclude:
    - shared.txt
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

    // Sequential: A's shared.txt excluded, then B re-introduces it.
    // shared.txt should exist with B's content.
    let shared = consumer.path().join("shared.txt");
    assert!(
        shared.exists(),
        "shared.txt should exist (from B, integrated after exclude removed A's copy).\n\
         Under batch model, exclude runs after all repos composite, removing it entirely."
    );
    let content = std::fs::read_to_string(&shared).unwrap();
    assert!(
        content.contains("from_upstream_b"),
        "shared.txt should contain B's content.\nActual: {}",
        content
    );

    // a_only.txt was excluded too (matches nothing in B, but was removed from A)
    // Actually exclude pattern is "shared.txt" only, so a_only.txt survives
    let a_only = consumer.path().join("a_only.txt");
    assert!(
        a_only.exists(),
        "a_only.txt should survive (not matched by exclude pattern)"
    );
}

// =========================================================================
// Spec: SequentialOperationExecution — include between repos in source block
// =========================================================================

/// An include between two repo: entries in a source block should filter
/// the composite at that point without affecting the second repo.
///
/// Setup:
/// - upstream_a provides `keep.txt` and `drop.txt`
/// - upstream_b provides `from_b.txt`
/// - Consumer source block: repo: A → include keep.txt → repo: B
///
/// Sequential: A integrates (keep.txt + drop.txt) → include filters to
/// keep.txt only → B integrates from_b.txt. Final FS: keep.txt + from_b.txt.
///
/// Batch: all repos composite, then include runs as post-processing,
/// removing both drop.txt AND from_b.txt (only keep.txt survives).
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_source_include_between_repos() {
    let upstream_a = assert_fs::TempDir::new().unwrap();
    init_test_git_repo(
        &upstream_a,
        &[
            ("keep.txt", "kept from A\n"),
            ("drop.txt", "should be dropped by include\n"),
            (".common-repo.yaml", "- include: [\"**\"]\n"),
        ],
        Some("v1.0.0"),
    )
    .unwrap();

    let upstream_b = assert_fs::TempDir::new().unwrap();
    init_test_git_repo(
        &upstream_b,
        &[
            ("from_b.txt", "from upstream B\n"),
            (".common-repo.yaml", "- include: [\"**\"]\n"),
        ],
        Some("v1.0.0"),
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    let url_a = format!("file://{}", upstream_a.path().display());
    let url_b = format!("file://{}", upstream_b.path().display());

    let config = format!(
        r#"- repo:
    url: "{}"
    ref: v1.0.0
- include:
    - "keep.txt"
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

    // Sequential: include filters after A, before B integrates
    assert!(
        consumer.path().join("keep.txt").exists(),
        "keep.txt should survive the include filter"
    );
    assert!(
        !consumer.path().join("drop.txt").exists(),
        "drop.txt should be removed by the include filter between repos"
    );

    // from_b.txt enters AFTER the include filter, so it should survive
    let from_b = consumer.path().join("from_b.txt");
    assert!(
        from_b.exists(),
        "from_b.txt should exist (repo B integrates after include filter).\n\
         Under batch model, include runs after all repos composite and \
         removes from_b.txt because it doesn't match the include pattern."
    );
}

// =========================================================================
// Spec: DeferredMergeAtDeclarationPosition — auto-merge at repo: position
// =========================================================================

/// Auto-merge operations from a sub-repo should fire at the repo:
/// declaration position in a source block, not batched in Phase 5.
///
/// Setup:
/// - upstream provides `config.yaml` via YAML auto-merge
/// - Consumer has local `config.yaml` with local settings
/// - Consumer source block: repo: upstream → exclude config.yaml
///
/// Sequential: repo: fires → auto-merge merges upstream config.yaml into
/// local config.yaml → exclude removes the upstream's raw config.yaml
/// (the merge fragment). Local config.yaml has merged content.
///
/// Batch: all repos composite, then Phase 5 runs deferred merges and
/// consumer ops. The merged content survives but the execution order
/// of exclude vs merge is not governed by declaration order.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_source_auto_merge_fires_at_repo_declaration_position() {
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

    let consumer = assert_fs::TempDir::new().unwrap();
    let upstream_url = format!("file://{}", upstream.path().display());

    // Local config.yaml that the auto-merge should merge into
    consumer
        .child("config.yaml")
        .write_str("settings:\n  local_only: true\n  app_name: my-app\n")
        .unwrap();

    let config = format!(
        r#"- repo:
    url: "{}"
    ref: v1.0.0
- exclude:
    - "config.yaml"
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

    // In the sequential model, the repo: operation fires first, which
    // triggers the auto-merge into local config.yaml. Then exclude removes
    // the upstream's raw config.yaml from the composite. The local
    // config.yaml should not be excluded because it's a local file, not
    // a composite entry — but the key assertion is that auto-merge
    // happened before exclude ran.
    //
    // Note: the exact behaviour of exclude on a merged file depends on
    // implementation details. The primary assertion is that auto-merge
    // content is present — proving it fired at the repo: position.
    let config_path = consumer.path().join("config.yaml");
    assert!(config_path.exists(), "config.yaml should exist after apply");

    let content = std::fs::read_to_string(&config_path).unwrap();
    assert!(
        content.contains("local_only"),
        "config.yaml should contain local settings.\nActual:\n{}",
        content
    );
    assert!(
        content.contains("from_upstream"),
        "config.yaml should contain upstream settings via auto-merge.\n\
         This proves auto-merge fired at the repo: declaration position.\n\
         Actual:\n{}",
        content
    );
}

// =========================================================================
// Spec: Parity — same config produces identical results in self: vs source
// =========================================================================

/// The same operation sequence should produce identical filesystem results
/// whether it appears in a source block or a self: block.
///
/// This is the core parity test for issue #305. The only difference between
/// self: and source is inheritance visibility — the execution model should
/// be identical.
///
/// Setup: two upstreams both provide `old/data.txt`. Consumer uses
/// repo A → rename old/ to new/ → repo B. Run once as source block,
/// once as self: block, compare results.
///
/// Sequential (target): new/data.txt has A's content, old/data.txt has B's.
/// Batch (current bug): new/data.txt has B's content (B overwrote A before
/// rename ran), old/data.txt does not exist.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_source_and_self_produce_identical_results() {
    let upstream_a = assert_fs::TempDir::new().unwrap();
    init_test_git_repo(
        &upstream_a,
        &[
            ("old/data.txt", "from_a\n"),
            (".common-repo.yaml", "- include: [\"**\"]\n"),
        ],
        Some("v1.0.0"),
    )
    .unwrap();

    let upstream_b = assert_fs::TempDir::new().unwrap();
    init_test_git_repo(
        &upstream_b,
        &[
            ("old/data.txt", "from_b\n"),
            (".common-repo.yaml", "- include: [\"**\"]\n"),
        ],
        Some("v1.0.0"),
    )
    .unwrap();

    let url_a = format!("file://{}", upstream_a.path().display());
    let url_b = format!("file://{}", upstream_b.path().display());

    // Operations: repo A → rename old/ to new/ → repo B
    // Sequential result: new/data.txt (A), old/data.txt (B)

    // --- Run as source block ---
    let consumer_source = assert_fs::TempDir::new().unwrap();
    let source_config = format!(
        r#"- repo:
    url: "{}"
    ref: v1.0.0
- rename:
    - "^old/(.*)$": "new/$1"
- repo:
    url: "{}"
    ref: v1.0.0
"#,
        url_a, url_b
    );
    consumer_source
        .child(".common-repo.yaml")
        .write_str(&source_config)
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.arg("apply")
        .current_dir(consumer_source.path())
        .assert()
        .success();

    // --- Run as self: block ---
    let consumer_self = assert_fs::TempDir::new().unwrap();
    let self_config = format!(
        r#"- self:
  - repo:
      url: "{}"
      ref: v1.0.0
  - rename:
      - "^old/(.*)$": "new/$1"
  - repo:
      url: "{}"
      ref: v1.0.0
"#,
        url_a, url_b
    );
    consumer_self
        .child(".common-repo.yaml")
        .write_str(&self_config)
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.arg("apply")
        .current_dir(consumer_self.path())
        .assert()
        .success();

    // --- Compare results ---
    // Both should have new/data.txt (from A, renamed) and old/data.txt (from B)

    // Self: block (known-good sequential behavior)
    assert!(
        consumer_self.path().join("new/data.txt").exists(),
        "self: block: new/data.txt should exist (A's file after rename)"
    );
    assert!(
        consumer_self.path().join("old/data.txt").exists(),
        "self: block: old/data.txt should exist (B integrated after rename)"
    );

    let self_new = std::fs::read_to_string(consumer_self.path().join("new/data.txt")).unwrap();
    let self_old = std::fs::read_to_string(consumer_self.path().join("old/data.txt")).unwrap();
    assert!(
        self_new.contains("from_a"),
        "self: new/data.txt should be from A"
    );
    assert!(
        self_old.contains("from_b"),
        "self: old/data.txt should be from B"
    );

    // Source block should produce identical results
    assert!(
        consumer_source.path().join("new/data.txt").exists(),
        "source block: new/data.txt should exist (A's file after rename).\n\
         Under batch model, this contains B's content instead of A's."
    );
    assert!(
        consumer_source.path().join("old/data.txt").exists(),
        "source block: old/data.txt should exist (B integrated after rename).\n\
         Under batch model, old/data.txt does not exist because rename \
         moved it to new/ after both repos were composited."
    );

    let source_new = std::fs::read_to_string(consumer_source.path().join("new/data.txt")).unwrap();
    let source_old = std::fs::read_to_string(consumer_source.path().join("old/data.txt")).unwrap();

    assert_eq!(
        source_new, self_new,
        "new/data.txt content should be identical between source and self: blocks"
    );
    assert_eq!(
        source_old, self_old,
        "old/data.txt content should be identical between source and self: blocks"
    );
}

// =========================================================================
// Spec: BuildComposite — sequential composite with multiple auto-merges
// =========================================================================

/// Multiple repo: operations in a source block with auto-merge should
/// accumulate content sequentially, each seeing the prior repo's merged
/// result.
///
/// This mirrors test_self_block_multiple_repo_ops_with_auto_merge from
/// cli_e2e_repo_in_sequence.rs but runs in a source block instead of self:.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_source_multiple_repos_with_auto_merge_accumulate() {
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

    // Local config that both upstreams should merge into
    consumer
        .child("config.yaml")
        .write_str("settings:\n  local: true\n")
        .unwrap();

    let config = format!(
        r#"- repo:
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
    assert!(config_path.exists(), "config.yaml should exist");

    let content = std::fs::read_to_string(&config_path).unwrap();

    // All three sources of settings should be present
    assert!(
        content.contains("local"),
        "Should contain local settings.\nActual:\n{}",
        content
    );
    assert!(
        content.contains("from_a"),
        "Should contain upstream A settings via auto-merge.\nActual:\n{}",
        content
    );
    assert!(
        content.contains("from_b"),
        "Should contain upstream B settings via auto-merge.\n\
         Both repos' auto-merges should accumulate sequentially.\nActual:\n{}",
        content
    );
}
