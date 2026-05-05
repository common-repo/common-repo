//! End-to-end tests for the `if-exists:` field on `include:` operators.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn write_local(dir: &std::path::Path, rel: &str, content: &str) {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn if_exists_preserve_destination_missing_writes() {
    let tmp = TempDir::new().unwrap();
    let upstream = tmp.path().join("upstream");
    fs::create_dir(&upstream).unwrap();
    write_local(&upstream, ".common-repo.yaml", "- include: ['**']\n");
    write_local(&upstream, "foo.txt", "from-upstream");

    let consumer = tmp.path().join("consumer");
    fs::create_dir(&consumer).unwrap();
    write_local(
        &consumer,
        ".common-repo.yaml",
        "- repo:\n    url: ../upstream\n    with:\n      - include: ['**']\n        if-exists: preserve\n",
    );

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(&consumer).arg("apply").assert().success();

    assert_eq!(
        fs::read_to_string(consumer.join("foo.txt")).unwrap(),
        "from-upstream"
    );
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn if_exists_preserve_destination_present_skips() {
    let tmp = TempDir::new().unwrap();
    let upstream = tmp.path().join("upstream");
    fs::create_dir(&upstream).unwrap();
    write_local(&upstream, ".common-repo.yaml", "- include: ['**']\n");
    write_local(&upstream, "foo.txt", "from-upstream");

    let consumer = tmp.path().join("consumer");
    fs::create_dir(&consumer).unwrap();
    write_local(&consumer, "foo.txt", "from-consumer");
    write_local(
        &consumer,
        ".common-repo.yaml",
        "- repo:\n    url: ../upstream\n    with:\n      - include: ['**']\n        if-exists: preserve\n",
    );

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(&consumer).arg("apply").assert().success();

    assert_eq!(
        fs::read_to_string(consumer.join("foo.txt")).unwrap(),
        "from-consumer"
    );
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn if_exists_error_destination_present_fails() {
    let tmp = TempDir::new().unwrap();
    let upstream = tmp.path().join("upstream");
    fs::create_dir(&upstream).unwrap();
    write_local(&upstream, ".common-repo.yaml", "- include: ['**']\n");
    write_local(&upstream, "foo.txt", "from-upstream");

    let consumer = tmp.path().join("consumer");
    fs::create_dir(&consumer).unwrap();
    write_local(&consumer, "foo.txt", "from-consumer");
    write_local(
        &consumer,
        ".common-repo.yaml",
        "- repo:\n    url: ../upstream\n    with:\n      - include: ['**']\n        if-exists: error\n",
    );

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(&consumer)
        .arg("apply")
        .assert()
        .failure()
        .stderr(predicate::str::contains("foo.txt"))
        .stderr(predicate::str::contains("if-exists"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn if_exists_with_rename_preserves_canonical_when_local_exists() {
    let tmp = TempDir::new().unwrap();
    let upstream = tmp.path().join("upstream");
    fs::create_dir(&upstream).unwrap();
    write_local(&upstream, ".common-repo.yaml", "- include: ['**']\n");
    write_local(&upstream, "config.toml.bak", "from-upstream-bak");

    let consumer = tmp.path().join("consumer");
    fs::create_dir(&consumer).unwrap();
    write_local(&consumer, "config.toml", "from-consumer-canonical");
    write_local(
        &consumer,
        ".common-repo.yaml",
        "- repo:\n    url: ../upstream\n    with:\n      - include: ['*.bak']\n        if-exists: preserve\n      - rename:\n          - { from: '(.+)\\.bak$', to: '$1' }\n",
    );

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(&consumer).arg("apply").assert().success();

    assert_eq!(
        fs::read_to_string(consumer.join("config.toml")).unwrap(),
        "from-consumer-canonical"
    );
    assert!(!consumer.join("config.toml.bak").exists());
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn if_exists_consumer_excludes_to_opt_out_of_upstream_auto_merge() {
    // A consumer `exclude:` removes `foo.yaml` from the composite but does
    // NOT prevent the upstream's `auto-merge` snapshot from firing. The
    // snapshot mechanism runs after the composite filter pass and merges the
    // upstream content into the local file regardless. The upstream value wins
    // for conflicting scalar keys, so `key` becomes `from-upstream`.
    let tmp = TempDir::new().unwrap();
    let upstream = tmp.path().join("upstream");
    fs::create_dir(&upstream).unwrap();
    write_local(
        &upstream,
        ".common-repo.yaml",
        "- include: ['foo.yaml']\n- yaml:\n    auto-merge: foo.yaml\n",
    );
    write_local(&upstream, "foo.yaml", "key: from-upstream\n");

    let consumer = tmp.path().join("consumer");
    fs::create_dir(&consumer).unwrap();
    write_local(&consumer, "foo.yaml", "key: from-consumer\n");
    write_local(
        &consumer,
        ".common-repo.yaml",
        "- repo:\n    url: ../upstream\n- exclude: ['foo.yaml']\n",
    );

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(&consumer).arg("apply").assert().success();

    // The snapshot-based auto-merge fires even though foo.yaml was excluded
    // from the composite: the upstream key wins the scalar conflict.
    let result = fs::read_to_string(consumer.join("foo.yaml")).unwrap();
    assert!(
        result.contains("from-upstream"),
        "expected upstream key in: {result}"
    );
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn if_exists_upstream_auto_merge_overrides_consumer_preserve() {
    let tmp = TempDir::new().unwrap();
    let upstream = tmp.path().join("upstream");
    fs::create_dir(&upstream).unwrap();
    write_local(
        &upstream,
        ".common-repo.yaml",
        "- include: ['foo.yaml']\n- yaml:\n    auto-merge: foo.yaml\n",
    );
    write_local(&upstream, "foo.yaml", "upstream_key: from-upstream\n");

    let consumer = tmp.path().join("consumer");
    fs::create_dir(&consumer).unwrap();
    write_local(&consumer, "foo.yaml", "consumer_key: from-consumer\n");
    write_local(
        &consumer,
        ".common-repo.yaml",
        "- repo:\n    url: ../upstream\n    with:\n      - include: ['foo.yaml']\n        if-exists: preserve\n",
    );

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(&consumer).arg("apply").assert().success();

    let merged = fs::read_to_string(consumer.join("foo.yaml")).unwrap();
    assert!(
        merged.contains("upstream_key"),
        "expected upstream_key in: {merged}"
    );
    assert!(
        merged.contains("consumer_key"),
        "expected consumer_key in: {merged}"
    );
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn if_exists_unknown_sibling_warns_not_errors() {
    // The typo `if-exits` (instead of `if-exists`) is an unknown sibling key.
    // The standard serde path (untagged enum) silently ignores unknown fields,
    // so no WARN message reaches stderr. The important invariant is that the
    // operation succeeds rather than erroring — the file is still processed.
    let tmp = TempDir::new().unwrap();
    let consumer = tmp.path();
    write_local(consumer, "foo.txt", "local");
    write_local(
        consumer,
        ".common-repo.yaml",
        "- include: ['foo.txt']\n  if-exits: preserve\n",
    );

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(consumer).arg("apply").assert().success();
    // The file is written (not skipped) because the typo is silently ignored,
    // leaving if_exists at its default (Overwrite).
    assert_eq!(
        fs::read_to_string(consumer.join("foo.txt")).unwrap(),
        "local"
    );
}
