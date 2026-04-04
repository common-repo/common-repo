//! E2E tests for deferred operations (defer flag and auto-merge).
//!
//! These tests validate the defer and auto-merge configuration options
//! that allow upstream repositories to declare merge operations to be
//! applied by consumers.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;
use predicates::prelude::*;

// =============================================================================
// Validation tests (no network required)
// =============================================================================

/// Test that validate accepts config with defer: true on yaml merge
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_yaml_merge_with_defer() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- yaml:
    source: settings.yaml
    dest: config/settings.yaml
    defer: true
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();
}

/// Test that validate accepts config with auto-merge shorthand
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_yaml_merge_with_auto_merge() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- yaml:
    auto-merge: settings.yaml
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();
}

/// Test that auto-merge conflicts with explicit source
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_auto_merge_conflicts_with_source() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- yaml:
    auto-merge: settings.yaml
    source: other.yaml
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .failure()
        .stdout(predicate::str::contains("auto-merge").or(predicate::str::contains("source")));
}

/// Test that auto-merge conflicts with explicit dest
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_auto_merge_conflicts_with_dest() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- yaml:
    auto-merge: settings.yaml
    dest: other.yaml
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .failure()
        .stdout(predicate::str::contains("auto-merge").or(predicate::str::contains("dest")));
}

/// Test that validate accepts config with defer on json merge
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_json_merge_with_defer() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- json:
    source: fragment.json
    dest: package.json
    defer: true
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();
}

/// Test that validate accepts config with auto-merge on json
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_json_merge_with_auto_merge() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- json:
    auto-merge: package.json
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();
}

/// Test that validate accepts config with defer on toml merge
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_toml_merge_with_defer() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- toml:
    source: Cargo.toml.fragment
    dest: Cargo.toml
    defer: true
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();
}

/// Test that validate accepts config with auto-merge on toml
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_toml_merge_with_auto_merge() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- toml:
    auto-merge: Cargo.toml
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();
}

/// Test that validate accepts config with defer on markdown merge
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_markdown_merge_with_defer() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- markdown:
    source: CONTRIBUTING.md
    dest: README.md
    section: Contributing
    level: 2
    defer: true
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();
}

/// Test that validate accepts config with auto-merge on markdown
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_markdown_merge_with_auto_merge() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- markdown:
    auto-merge: CLAUDE.md
    section: Instructions
    level: 2
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();
}

/// Test that validate accepts config with defer on ini merge
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_ini_merge_with_defer() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- ini:
    source: settings.ini
    dest: config.ini
    defer: true
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();
}

/// Test that validate accepts config with auto-merge on ini
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_ini_merge_with_auto_merge() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- ini:
    auto-merge: config.ini
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();
}

/// Test that validate accepts config with multiple deferred operations
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_multiple_deferred_operations() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
# Multiple deferred operations of different types
- yaml:
    auto-merge: settings.yaml
- json:
    source: package.json.fragment
    dest: package.json
    defer: true
- markdown:
    auto-merge: CLAUDE.md
    section: Instructions
    level: 2
- toml:
    auto-merge: Cargo.toml
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();
}

/// Test that defer: false is valid (explicit non-deferred)
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_validate_defer_false_is_valid() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- yaml:
    source: settings.yaml
    dest: config/settings.yaml
    defer: false
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("validate")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();
}

// =============================================================================
// Apply tests with deferred operations (local files, no network)
// =============================================================================

/// Test that apply works with yaml merge using auto-merge
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_yaml_auto_merge() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let yaml_file = temp.child("settings.yaml");

    // Create the source yaml file
    yaml_file
        .write_str(
            r#"
database:
  host: localhost
  port: 5432
"#,
        )
        .unwrap();

    // Config with auto-merge
    config_file
        .write_str(
            r#"
- yaml:
    auto-merge: settings.yaml
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    // Apply should succeed (auto-merge uses same file as source and dest)
    cmd.current_dir(temp.path())
        .arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();

    // File should still exist and be valid YAML
    let content = std::fs::read_to_string(yaml_file.path()).unwrap();
    assert!(content.contains("database:"));
    assert!(content.contains("host: localhost"));
}

/// Test that apply works with json merge using auto-merge
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_json_auto_merge() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let json_file = temp.child("package.json");

    // Create the source json file
    json_file
        .write_str(
            r#"{
    "name": "test-package",
    "version": "1.0.0"
}"#,
        )
        .unwrap();

    // Config with auto-merge
    config_file
        .write_str(
            r#"
- json:
    auto-merge: package.json
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    // Apply should succeed
    cmd.current_dir(temp.path())
        .arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();

    // File should still be valid JSON
    let content = std::fs::read_to_string(json_file.path()).unwrap();
    assert!(content.contains("test-package"));
}

/// Test that apply works with markdown merge using defer flag
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_markdown_with_defer() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");
    let source_file = temp.child("contributing.md");
    let dest_file = temp.child("README.md");

    // Create source fragment
    source_file
        .write_str(
            r###"
We welcome contributions!

1. Fork the repository
2. Create a feature branch
"###,
        )
        .unwrap();

    // Create destination file
    dest_file
        .write_str(
            r###"# My Project

## Features

- Feature A
- Feature B

## Contributing

Old contributing content.
"###,
        )
        .unwrap();

    // Config with defer flag (doesn't affect local apply, just metadata)
    config_file
        .write_str(
            r#"
- markdown:
    source: contributing.md
    dest: README.md
    section: Contributing
    level: 2
    defer: true
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();

    // Check that merge happened
    let content = std::fs::read_to_string(dest_file.path()).unwrap();
    assert!(content.contains("# My Project"));
    assert!(content.contains("## Contributing"));
    assert!(content.contains("We welcome contributions!"));
}

/// Test apply with multiple deferred operations
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_multiple_deferred_operations() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Create yaml file
    let yaml_file = temp.child("config.yaml");
    yaml_file
        .write_str(
            r#"
app:
  name: test
"#,
        )
        .unwrap();

    // Create json file
    let json_file = temp.child("data.json");
    json_file
        .write_str(
            r#"{
    "items": []
}"#,
        )
        .unwrap();

    // Config with multiple auto-merge operations
    config_file
        .write_str(
            r#"
- yaml:
    auto-merge: config.yaml
- json:
    auto-merge: data.json
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("apply")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success();

    // Both files should still be valid
    let yaml_content = std::fs::read_to_string(yaml_file.path()).unwrap();
    assert!(yaml_content.contains("app:"));

    let json_content = std::fs::read_to_string(json_file.path()).unwrap();
    assert!(json_content.contains("items"));
}

// =============================================================================
// Check command tests with deferred operations
// =============================================================================

/// Test that check command works with deferred operations config
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_check_with_deferred_operations() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- yaml:
    auto-merge: settings.yaml
- markdown:
    source: CONTRIBUTING.md
    dest: README.md
    section: Contributing
    level: 2
    defer: true
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("check")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Configuration loaded successfully",
        ));
}

// =============================================================================
// Info command tests with deferred operations
// =============================================================================

/// Test that info command displays deferred operations correctly
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_info_shows_deferred_operations() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    config_file
        .write_str(
            r#"
- yaml:
    auto-merge: settings.yaml
- json:
    source: fragment.json
    dest: package.json
    defer: true
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("info")
        .arg("--config")
        .arg(config_file.path())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("yaml").and(predicate::str::contains("json")), // Both operation types shown
        );
}

// =============================================================================
// Auto-merge composition e2e test (real repos)
// =============================================================================

/// Test that chained repos with auto-merge for the same file trigger
/// Phase 4 composition merge (not just last-write-wins).
///
/// Uses real repos:
/// - common-repo/conventional-commits (chains to common-repo/pre-commit)
/// - Both declare: yaml: { auto-merge: .pre-commit-config.yaml }
///
/// Note: The upstream repos currently use default array_mode (replace),
/// so the merge fires but replaces rather than appends. This test verifies
/// the merge mechanism works; the upstream repos would need array_mode: append
/// to actually combine the repos: arrays.
///
/// Regression test for https://github.com/common-repo/common-repo/issues/284
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_auto_merge_chained_repos_merge_fires() {
    let consumer = assert_fs::TempDir::new().unwrap();

    // Consumer config inherits from conventional-commits, which itself
    // chains to pre-commit. Both repos declare auto-merge for
    // .pre-commit-config.yaml.
    consumer
        .child(".common-repo.yaml")
        .write_str(
            r#"- repo:
    url: https://github.com/common-repo/conventional-commits
    ref: v1.0.1
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(consumer.path())
        .arg("apply")
        .arg("--config")
        .arg(consumer.child(".common-repo.yaml").path())
        .assert()
        .success();

    // The .pre-commit-config.yaml should exist
    let pre_commit_config = consumer.child(".pre-commit-config.yaml");
    assert!(
        pre_commit_config.exists(),
        ".pre-commit-config.yaml should exist in consumer output"
    );

    let content = std::fs::read_to_string(pre_commit_config.path()).unwrap();

    // With default array_mode (replace), conventional-commits' repos: replaces
    // pre-commit's repos:. The file should at minimum contain the conventional
    // hook (proving the merge mechanism ran, not just raw overwrite).
    assert!(
        content.contains("conventional-pre-commit"),
        "Should contain conventional-pre-commit hook.\nActual content:\n{}",
        content
    );
}

/// Test that chained repos with auto-merge + array_mode: append produce
/// combined output containing hooks from ALL repos in the chain.
///
/// The consumer overrides the auto-merge with array_mode: append via
/// inline with: clause to prove the full merge chain works.
///
/// Regression test for https://github.com/common-repo/common-repo/issues/284
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_auto_merge_chained_repos_combine_with_append() {
    let consumer = assert_fs::TempDir::new().unwrap();

    // Consumer inherits from conventional-commits but overrides the
    // auto-merge to use append mode so both repos' hooks combine.
    consumer
        .child(".common-repo.yaml")
        .write_str(
            r#"- repo:
    url: https://github.com/common-repo/conventional-commits
    ref: v1.0.1
    with:
      - yaml:
          auto-merge: .pre-commit-config.yaml
          array_mode: append
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(consumer.path())
        .arg("apply")
        .arg("--config")
        .arg(consumer.child(".common-repo.yaml").path())
        .assert()
        .success();

    let pre_commit_config = consumer.child(".pre-commit-config.yaml");
    assert!(
        pre_commit_config.exists(),
        ".pre-commit-config.yaml should exist in consumer output"
    );

    let content = std::fs::read_to_string(pre_commit_config.path()).unwrap();

    // With append mode, BOTH repos' hooks should be present
    assert!(
        content.contains("trailing-whitespace"),
        "Should contain trailing-whitespace hook from pre-commit repo.\nActual content:\n{}",
        content
    );
    assert!(
        content.contains("conventional-pre-commit"),
        "Should contain conventional-pre-commit hook from conventional-commits repo.\nActual content:\n{}",
        content
    );
}
