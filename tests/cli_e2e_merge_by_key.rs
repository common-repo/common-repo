//! End-to-end regression tests for issue #370: a deferred auto-merge with
//! `array_mode: merge_by_key` must update keyed list items in place when the
//! upstream revises them, instead of appending the edited items as duplicates
//! next to the stale copies (the old `append_unique` failure mode).
//!
//! Byte-exact scenarios live in `tests/testdata/merge-by-key/` and run through
//! the shared expected-fixture runner — YAML (workflow steps, pre-commit
//! repos keyed on `repo`), JSON, TOML array-of-tables, and XML (attribute
//! keys). The tests here assert the behavioral invariants directly and
//! re-apply each bump case through the CLI to prove idempotence.

mod common;

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("testdata")
        .join("merge-by-key")
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn expected_fixtures_match_byte_exactly() {
    common::expected_fixture::run_expected_fixtures(&fixture_root());
}

/// Seed a consumer from `<case>.input/`, install the case's config with the
/// fixture placeholder resolved, and return the tempdir.
fn stage_case(fixture: &Path, case: &str) -> TempDir {
    let expected_dir = fixture.join(format!("{case}.expected"));
    let temp = TempDir::new().unwrap();
    common::expected_fixture::seed_input_files(&expected_dir, temp.path());
    let config = fs::read_to_string(expected_dir.join(".common-repo.yaml"))
        .unwrap()
        .replace("__FIXTURE__", &fixture.to_string_lossy());
    fs::write(temp.path().join(".common-repo.yaml"), config).unwrap();
    temp
}

fn apply_in(dir: &Path) {
    cargo_bin_cmd!("common-repo")
        .current_dir(dir)
        .arg("apply")
        .assert()
        .success();
}

/// Every file the consumer holds after apply, mapped to its bytes. The
/// config file is excluded: the runner templates it, so in-dir copies may
/// differ from the fixture's.
fn snapshot_tree(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    common::expected_fixture::collect_relative_paths(root)
        .into_iter()
        .filter(|rel| rel != &PathBuf::from(".common-repo.yaml"))
        .map(|rel| {
            let bytes = fs::read(root.join(&rel)).unwrap();
            (rel, bytes)
        })
        .collect()
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn bump_cases_reach_their_expected_state_and_are_idempotent() {
    let fixture = fixture_root();
    for case in [
        "bump-consumer",
        "precommit-bump",
        "json-bump",
        "toml-bump",
        "xml-bump",
    ] {
        let expected_dir = fixture.join(format!("{case}.expected"));
        let temp = stage_case(&fixture, case);

        apply_in(temp.path());
        let after_first = snapshot_tree(temp.path());

        // The first apply lands exactly on the fixture's expectation.
        for (rel, bytes) in &after_first {
            let expected = fs::read(expected_dir.join(rel))
                .unwrap_or_else(|e| panic!("{case}: apply produced unexpected file {rel:?}: {e}"));
            assert_eq!(&expected, bytes, "{case}: {rel:?} differs from fixture");
        }
        let expected_files: Vec<PathBuf> = snapshot_tree(&expected_dir)
            .into_iter()
            .map(|(rel, _)| rel)
            .collect();
        let produced_files: Vec<PathBuf> = after_first.iter().map(|(rel, _)| rel.clone()).collect();
        assert_eq!(
            produced_files, expected_files,
            "{case}: produced tree differs from fixture"
        );

        // A re-apply is a no-op: the keyed merge is a fixed point.
        apply_in(temp.path());
        let after_second = snapshot_tree(temp.path());
        assert_eq!(
            after_first, after_second,
            "{case}: second apply changed the tree"
        );
    }
}

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
