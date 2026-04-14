//! E2E tests for sequential consumer operation execution.
//!
//! These tests validate that consumer operations (merges and filters) execute
//! in YAML declaration order, not grouped by type.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;

/// Helper to initialize a local git repository for use as a test upstream.
fn init_test_git_repo(
    dir: &assert_fs::TempDir,
    files: &[(&str, &str)],
) -> Result<(), Box<dyn std::error::Error>> {
    std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir.path())
        .output()?;
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir.path())
        .output()?;
    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir.path())
        .output()?;
    std::process::Command::new("git")
        .args(["config", "commit.gpgsign", "false"])
        .current_dir(dir.path())
        .output()?;
    std::process::Command::new("git")
        .args(["config", "core.hooksPath", "/dev/null"])
        .current_dir(dir.path())
        .output()?;
    for (path, content) in files {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(dir.path().join(parent))?;
            }
        }
        dir.child(path).write_str(content)?;
    }
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()?;
    std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(dir.path())
        .output()?;
    Ok(())
}

/// Sequential execution: merge runs first (succeeds), then exclude removes
/// the merge source file. This validates that declaration order is respected
/// when merge appears before exclude.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_sequential_merge_then_exclude_via_cli() {
    let upstream = assert_fs::TempDir::new().unwrap();
    init_test_git_repo(
        &upstream,
        &[("fragment.json", r#"{"from_upstream": true}"#)],
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    consumer
        .child("config.json")
        .write_str(r#"{"local": true}"#)
        .unwrap();

    let upstream_url = format!("file://{}", upstream.path().display());
    let config = format!(
        r#"- repo:
    url: "{}"
    ref: main
- json:
    source: fragment.json
    dest: config.json
- exclude:
    - fragment.json
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

    // Merge ran: config.json has merged content from both local and upstream
    let content = std::fs::read_to_string(consumer.path().join("config.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["local"], true);
    assert_eq!(json["from_upstream"], true);

    // Exclude ran after merge: fragment.json is gone
    assert!(!consumer.path().join("fragment.json").exists());
}

/// Sequential execution: rename strips a directory prefix, then merge operates
/// on the renamed paths, then exclude cleans up. This validates that rename
/// affects subsequent operations in declaration order.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_sequential_rename_then_merge_via_cli() {
    let upstream = assert_fs::TempDir::new().unwrap();
    init_test_git_repo(
        &upstream,
        &[
            ("src/config.json", r#"{"upstream": true}"#),
            ("src/fragment.json", r#"{"extra": true}"#),
        ],
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();

    let upstream_url = format!("file://{}", upstream.path().display());
    let config = format!(
        r#"- repo:
    url: "{}"
    ref: main
- rename:
    - "^src/(.*)$": "$1"
- json:
    source: fragment.json
    dest: config.json
- exclude:
    - fragment.json
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

    // Rename ran: files are at root, not under src/
    assert!(!consumer.path().join("src/config.json").exists());

    // Merge ran after rename: config.json has upstream base + fragment merged in
    let content = std::fs::read_to_string(consumer.path().join("config.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["upstream"], true);
    assert_eq!(json["extra"], true);

    // Exclude ran last: fragment.json is gone
    assert!(!consumer.path().join("fragment.json").exists());
}

/// Sequential execution: exclude runs first and removes the merge source,
/// so the subsequent merge fails. This is the core behavioral change --
/// the old two-pass model would have run the merge first (succeeding),
/// then the exclude.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_sequential_exclude_before_merge_fails_via_cli() {
    let upstream = assert_fs::TempDir::new().unwrap();
    init_test_git_repo(
        &upstream,
        &[("fragment.json", r#"{"from_upstream": true}"#)],
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    consumer
        .child("config.json")
        .write_str(r#"{"local": true}"#)
        .unwrap();

    let upstream_url = format!("file://{}", upstream.path().display());
    // Exclude appears BEFORE merge -- sequential execution removes fragment.json
    // before the merge can read it as a source.
    let config = format!(
        r#"- repo:
    url: "{}"
    ref: main
- exclude:
    - fragment.json
- json:
    source: fragment.json
    dest: config.json
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
        .failure();
}
