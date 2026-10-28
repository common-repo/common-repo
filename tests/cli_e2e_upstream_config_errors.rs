//! End-to-end tests for upstream `.common-repo.yaml` files that fail to parse.
//!
//! An upstream without a config file is legitimate and contributes whatever
//! the consumer's `with:` clause asks for. An upstream whose config exists but
//! does not parse must fail the run instead of being treated as empty, so a
//! consumer running an older binary notices when an upstream adopts syntax it
//! does not understand.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Create `upstream/` with the given config and a payload file, and a
/// sibling `consumer/` that inherits from `../upstream`.
fn setup(tmp: &Path, upstream_config: Option<&str>) -> std::path::PathBuf {
    let upstream = tmp.join("upstream");
    fs::create_dir(&upstream).unwrap();
    fs::write(upstream.join("payload.txt"), b"hello").unwrap();
    if let Some(config) = upstream_config {
        fs::write(upstream.join(".common-repo.yaml"), config).unwrap();
    }

    let consumer = tmp.join("consumer");
    fs::create_dir(&consumer).unwrap();
    fs::write(
        consumer.join(".common-repo.yaml"),
        b"- repo:\n    url: ../upstream\n    with:\n      - include: ['**']\n",
    )
    .unwrap();
    consumer
}

const BOGUS_ARRAY_MODE: &str = "- include: ['**']
- yaml:
    source: a.yaml
    dest: a.yaml
    array_mode: bogus_mode
";

const UNKNOWN_OPERATOR: &str = "- include: ['**']
- bogus_op:
    foo: bar
";

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn apply_fails_when_upstream_config_has_unknown_enum_value() {
    let tmp = TempDir::new().unwrap();
    let consumer = setup(tmp.path(), Some(BOGUS_ARRAY_MODE));

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(&consumer)
        .arg("apply")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("../upstream"))
        .stderr(predicate::str::contains("unknown variant `bogus_mode`"));
    assert!(
        !consumer.join("payload.txt").exists(),
        "a failed apply must not write files"
    );
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn apply_fails_when_upstream_config_has_unknown_operator() {
    let tmp = TempDir::new().unwrap();
    let consumer = setup(tmp.path(), Some(UNKNOWN_OPERATOR));

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(&consumer)
        .arg("apply")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("../upstream"))
        .stderr(predicate::str::contains("Unknown operation type: bogus_op"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn apply_succeeds_when_upstream_has_no_config_file() {
    let tmp = TempDir::new().unwrap();
    let consumer = setup(tmp.path(), None);

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(&consumer).arg("apply").assert().code(0);
    assert_eq!(
        fs::read_to_string(consumer.join("payload.txt")).unwrap(),
        "hello"
    );
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn diff_fails_when_upstream_config_does_not_parse() {
    let tmp = TempDir::new().unwrap();
    let consumer = setup(tmp.path(), Some(BOGUS_ARRAY_MODE));

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(&consumer)
        .arg("diff")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("../upstream"))
        .stderr(predicate::str::contains("unknown variant `bogus_mode`"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn validate_fails_when_upstream_config_does_not_parse() {
    let tmp = TempDir::new().unwrap();
    let consumer = setup(tmp.path(), Some(BOGUS_ARRAY_MODE));

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(&consumer)
        .arg("validate")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("../upstream"))
        .stdout(predicate::str::contains("unknown variant `bogus_mode`"));
}
