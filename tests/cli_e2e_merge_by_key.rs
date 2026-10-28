//! End-to-end regression tests for issue #370: a deferred yaml auto-merge with
//! `array_mode: merge_by_key` must update workflow steps in place when the
//! upstream revises them, instead of appending the edited steps as duplicates
//! next to the stale copies (the old `append_unique` failure mode).

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// The upstream's current ci.yaml (v2): the checkout step gained
/// `persist-credentials: false` and the coverage step gained a second
/// hashFiles argument.
const CI_V2: &str = "\
name: CI
on: [push]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683
        with:
          persist-credentials: false
      - name: Check coverage threshold
        if: hashFiles('coverage.out', 'coverprofile.out') != ''
        run: coverage verify --threshold 80
";

/// What the consumer last applied (v1): both steps predate the upstream edits.
const CI_V1: &str = "\
name: CI
on: [push]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683
      - name: Check coverage threshold
        if: hashFiles('coverage.out') != ''
        run: coverage verify --threshold 70
";

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

/// Stage an upstream declaring the keyed deferred auto-merge plus a consumer
/// still holding the v1 merge result. Returns (tempdir guard, consumer path).
fn stage_upstream_and_consumer() -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new().unwrap();
    let upstream = tmp.path().join("upstream");
    let consumer = tmp.path().join("consumer");

    write(
        &upstream.join(".common-repo.yaml"),
        "\
- include: [\"**\"]
- yaml:
    auto-merge: .github/workflows/ci.yaml
    array_mode: merge_by_key
",
    );
    write(&upstream.join(".github/workflows/ci.yaml"), CI_V2);

    write(
        &consumer.join(".common-repo.yaml"),
        "- repo:\n    url: ../upstream\n",
    );
    write(&consumer.join(".github/workflows/ci.yaml"), CI_V1);

    (tmp, consumer)
}

fn apply_in(dir: &Path) {
    cargo_bin_cmd!("common-repo")
        .current_dir(dir)
        .arg("apply")
        .assert()
        .success();
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn upstream_step_edits_replace_stale_steps_instead_of_duplicating() {
    let (_tmp, consumer) = stage_upstream_and_consumer();

    apply_in(&consumer);

    let merged = fs::read_to_string(consumer.join(".github/workflows/ci.yaml")).unwrap();
    let parsed: serde_yaml::Value = serde_yaml::from_str(&merged).unwrap();
    let steps = parsed["jobs"]["test"]["steps"].as_sequence().unwrap();

    // The exact regressions from issue #370: duplicated step names and
    // duplicated checkouts after an upstream bump.
    assert_eq!(
        steps.len(),
        2,
        "steps must not duplicate after upstream revision: {merged}"
    );
    assert_eq!(
        merged.matches("name: Check coverage threshold").count(),
        1,
        "coverage step must appear exactly once: {merged}"
    );
    assert_eq!(
        merged.matches("actions/checkout").count(),
        1,
        "checkout step must appear exactly once: {merged}"
    );
    // The upstream edits actually landed.
    assert!(merged.contains("persist-credentials: false"));
    assert!(merged.contains("hashFiles('coverage.out', 'coverprofile.out')"));
    assert!(merged.contains("coverage verify --threshold 80"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn keyed_merge_is_idempotent_across_reapplies() {
    let (_tmp, consumer) = stage_upstream_and_consumer();

    apply_in(&consumer);
    let first = fs::read_to_string(consumer.join(".github/workflows/ci.yaml")).unwrap();
    apply_in(&consumer);
    let second = fs::read_to_string(consumer.join(".github/workflows/ci.yaml")).unwrap();

    assert_eq!(first, second, "re-applying must be a no-op (fixed point)");
    assert_eq!(second.matches("actions/checkout").count(), 1);
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn consumer_only_steps_survive_the_keyed_merge() {
    let (_tmp, consumer) = stage_upstream_and_consumer();

    // The consumer adds a private step on top of the v1 state.
    let with_private = CI_V1.replace(
        "      - name: Check coverage threshold",
        "      - name: Consumer-only\n        run: ./private.sh\n      - name: Check coverage threshold",
    );
    write(&consumer.join(".github/workflows/ci.yaml"), &with_private);

    apply_in(&consumer);

    let merged = fs::read_to_string(consumer.join(".github/workflows/ci.yaml")).unwrap();
    let parsed: serde_yaml::Value = serde_yaml::from_str(&merged).unwrap();
    let steps = parsed["jobs"]["test"]["steps"].as_sequence().unwrap();
    assert_eq!(
        steps.len(),
        3,
        "private step preserved, no duplicates: {merged}"
    );
    assert!(merged.contains("./private.sh"));
    assert!(merged.contains("coverage verify --threshold 80"));
}
