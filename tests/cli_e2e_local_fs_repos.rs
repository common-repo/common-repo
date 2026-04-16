//! End-to-end tests for local filesystem repo references.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn apply_warns_on_local_url_with_ref() {
    let tmp = TempDir::new().unwrap();
    let sibling = tmp.path().join("upstream");
    fs::create_dir(&sibling).unwrap();
    fs::write(sibling.join(".common-repo.yaml"), b"- include: ['**']\n").unwrap();
    fs::write(sibling.join("payload.txt"), b"hello").unwrap();

    let consumer = tmp.path().join("consumer");
    fs::create_dir(&consumer).unwrap();
    fs::write(
        consumer.join(".common-repo.yaml"),
        b"- repo:\n    url: ../upstream\n    ref: main\n",
    )
    .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(&consumer)
        .arg("--verbose")
        .arg("apply")
        .assert()
        .success()
        .stderr(predicate::str::contains("ignored on local-path"));
    assert!(consumer.join("payload.txt").exists());
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn apply_errors_on_missing_local_dir() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join(".common-repo.yaml"),
        b"- repo:\n    url: ./does-not-exist\n",
    )
    .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(tmp.path())
        .arg("apply")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Local path not found"))
        .stderr(predicate::str::contains("missing ref").not());
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn apply_accepts_absolute_local_path() {
    let tmp = TempDir::new().unwrap();
    let abs = tmp.path().join("abs-upstream");
    fs::create_dir(&abs).unwrap();
    fs::write(abs.join(".common-repo.yaml"), b"- include: ['**']\n").unwrap();
    fs::write(abs.join("payload.txt"), b"abs").unwrap();

    let consumer = tmp.path().join("consumer");
    fs::create_dir(&consumer).unwrap();
    let abs_str = abs.to_string_lossy();
    fs::write(
        consumer.join(".common-repo.yaml"),
        format!("- repo:\n    url: {abs_str}\n").as_bytes(),
    )
    .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(&consumer).arg("apply").assert().success();
    assert!(consumer.join("payload.txt").exists());
}
