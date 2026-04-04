//! E2E tests for source-declared operator filtering across CLI commands (ls, diff, apply).
//!
//! These tests verify that source-declared operators (include, exclude, rename)
//! and consumer-side operators (with: exclude) are correctly respected by ALL
//! CLI subcommands — not just `apply`.
//!
//! ## Background
//!
//! Issue #226: `common-repo ls` does not respect exclude filters (source-declared
//! or consumer-side), even though `apply` and `diff` correctly filter them.
//! These tests ensure consistent behavior across ls, diff, and apply.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::process::Command;

/// Helper to initialize a git repository with files and commit them.
///
/// Creates a git repo at the given path with the specified files.
/// The repository uses "main" as the default branch name.
fn init_git_repo(
    dir: &assert_fs::TempDir,
    files: &[(&str, &str)],
) -> Result<(), Box<dyn std::error::Error>> {
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir.path())
        .output()?;

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir.path())
        .output()?;
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir.path())
        .output()?;
    Command::new("git")
        .args(["config", "commit.gpgsign", "false"])
        .current_dir(dir.path())
        .output()?;
    // Disable global hooks (e.g. pre-commit) that may interfere with test repos
    Command::new("git")
        .args(["config", "core.hooksPath", "/dev/null"])
        .current_dir(dir.path())
        .output()?;

    for (path, content) in files {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(dir.path().join(parent))?;
            }
        }
        let file = dir.child(path);
        file.write_str(content)?;
    }

    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()?;
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(dir.path())
        .output()?;

    Ok(())
}

// =============================================================================
// Test 1: `ls` should respect source-declared excludes (regression test for #226)
// =============================================================================

/// Related to https://github.com/common-repo/common-repo/issues/226
///
/// `common-repo ls` should NOT list files that are excluded by the source
/// repository's `.common-repo.yaml`. This verifies source-declared excludes
/// are respected by the `ls` command.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ls_respects_source_declared_excludes() {
    // Source repo excludes internal/** and go.mod
    let source_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &source_repo,
        &[
            (
                ".common-repo.yaml",
                r#"- exclude:
    - "internal/**"
    - "go.mod"
"#,
            ),
            ("README.md", "# Public readme\n"),
            ("main.go", "package main\n"),
            ("go.mod", "module example.com/source\n"),
            ("internal/secret.go", "package internal\n"),
            ("internal/helper.go", "package internal\n"),
        ],
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    let source_url = format!("file://{}", source_repo.path().display());

    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
"#,
            source_url
        ))
        .unwrap();

    // ls should NOT show excluded files
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(consumer.path())
        .arg("ls")
        .assert()
        .success()
        .stdout(predicate::str::contains("README.md"))
        .stdout(predicate::str::contains("main.go"))
        // These should NOT appear — excluded by source
        .stdout(predicate::str::contains("go.mod").not())
        .stdout(predicate::str::contains("internal/secret.go").not())
        .stdout(predicate::str::contains("internal/helper.go").not());
}

// =============================================================================
// Test 2: `ls` should respect source-declared includes
// =============================================================================

/// When a source repo uses `include` as an allowlist, `ls` should ONLY show
/// files matching the include patterns — not the entire source repo contents.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ls_respects_source_declared_includes() {
    let source_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &source_repo,
        &[
            (
                ".common-repo.yaml",
                r#"- include:
    - ".github/**"
    - "Dockerfile"
"#,
            ),
            (".github/workflows/ci.yml", "name: CI\n"),
            (".github/dependabot.yml", "version: 2\n"),
            ("Dockerfile", "FROM ubuntu:22.04\n"),
            ("README.md", "# Should not appear\n"),
            ("src/main.rs", "fn main() {}\n"),
            ("LICENSE", "MIT\n"),
        ],
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    let source_url = format!("file://{}", source_repo.path().display());

    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
"#,
            source_url
        ))
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(consumer.path())
        .arg("ls")
        .assert()
        .success()
        // Only included files should appear
        .stdout(predicate::str::contains("ci.yml"))
        .stdout(predicate::str::contains("dependabot.yml"))
        .stdout(predicate::str::contains("Dockerfile"))
        // Non-included files should NOT appear
        .stdout(predicate::str::contains("README.md").not())
        .stdout(predicate::str::contains("main.rs").not())
        .stdout(predicate::str::contains("LICENSE").not());
}

// =============================================================================
// Test 3: `ls` should respect consumer-side `with` excludes
// =============================================================================

/// Consumer-side `with:` exclude patterns should filter files from ls output.
/// This is a consumer-level override on top of whatever the source exposes.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ls_respects_consumer_with_excludes() {
    let source_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &source_repo,
        &[
            ("README.md", "# Readme\n"),
            ("config.yaml", "key: value\n"),
            ("settings.yaml", "debug: false\n"),
            ("Makefile", "all:\n\techo hello\n"),
            ("src/app.go", "package main\n"),
        ],
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    let source_url = format!("file://{}", source_repo.path().display());

    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
    with:
      - exclude: ["*.yaml"]
"#,
            source_url
        ))
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(consumer.path())
        .arg("ls")
        .assert()
        .success()
        // Non-yaml files should appear
        .stdout(predicate::str::contains("README.md"))
        .stdout(predicate::str::contains("Makefile"))
        .stdout(predicate::str::contains("app.go"))
        // YAML files should be excluded by consumer's with: clause
        .stdout(predicate::str::contains("config.yaml").not())
        .stdout(predicate::str::contains("settings.yaml").not());
}

// =============================================================================
// Test 4: `ls` should respect consumer-side top-level excludes
// =============================================================================

/// Regression test for https://github.com/common-repo/common-repo/issues/226
///
/// Consumer config can have top-level exclude operations that apply after
/// the repo operation. These should also filter the ls output.
///
/// This test currently FAILS because `ls` does not apply consumer-side
/// top-level exclude operations that appear after the repo operation.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ls_respects_consumer_toplevel_excludes() {
    let source_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &source_repo,
        &[
            ("README.md", "# Readme\n"),
            ("src/app.go", "package main\n"),
            ("tests/app_test.go", "package main\n"),
            ("tests/integration_test.go", "package main\n"),
        ],
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    let source_url = format!("file://{}", source_repo.path().display());

    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
- exclude:
    - "tests/**"
"#,
            source_url
        ))
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(consumer.path())
        .arg("ls")
        .assert()
        .success()
        .stdout(predicate::str::contains("README.md"))
        .stdout(predicate::str::contains("app.go"))
        // Tests should be excluded by consumer's top-level exclude
        .stdout(predicate::str::contains("app_test.go").not())
        .stdout(predicate::str::contains("integration_test.go").not());
}

// =============================================================================
// Test 5: `diff` respects source-declared excludes
// =============================================================================

/// When the source excludes a file (e.g. go.mod), `diff` should NOT report
/// changes to that file even if the consumer has a different local version.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_diff_respects_source_declared_excludes() {
    let source_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &source_repo,
        &[
            (
                ".common-repo.yaml",
                r#"- exclude:
    - "go.mod"
"#,
            ),
            ("README.md", "# Source readme\n"),
            ("go.mod", "module example.com/source\n"),
        ],
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    let source_url = format!("file://{}", source_repo.path().display());

    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
"#,
            source_url
        ))
        .unwrap();

    // Consumer has its own go.mod with different content
    consumer
        .child("go.mod")
        .write_str("module example.com/consumer\n")
        .unwrap();

    // First apply to get README.md in sync
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(consumer.path())
        .arg("apply")
        .assert()
        .success();

    // Now diff should show no changes (go.mod is excluded, README is in sync)
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(consumer.path())
        .arg("diff")
        .assert()
        .success()
        // go.mod should NOT appear in diff output
        .stdout(predicate::str::contains("go.mod").not());

    // Verify consumer's go.mod was NOT overwritten by apply
    let go_mod = std::fs::read_to_string(consumer.child("go.mod").path()).unwrap();
    assert_eq!(
        go_mod, "module example.com/consumer\n",
        "Consumer's go.mod should be preserved (excluded by source)"
    );
}

// =============================================================================
// Test 6: `apply` respects source-declared excludes
// =============================================================================

/// When the source excludes README.md, `apply` should NOT overwrite the
/// consumer's local README.md even if the source has a different version.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_apply_respects_source_declared_excludes() {
    let source_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &source_repo,
        &[
            (
                ".common-repo.yaml",
                r#"- exclude:
    - "README.md"
"#,
            ),
            ("README.md", "# Source README - should not be copied\n"),
            ("config.yaml", "key: source_value\n"),
        ],
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    let source_url = format!("file://{}", source_repo.path().display());

    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
"#,
            source_url
        ))
        .unwrap();

    // Consumer has its own README.md
    consumer
        .child("README.md")
        .write_str("# My Custom README\n")
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(consumer.path())
        .arg("apply")
        .arg("--verbose")
        .assert()
        .success();

    // Consumer's README should be unchanged (excluded by source)
    let readme = std::fs::read_to_string(consumer.child("README.md").path()).unwrap();
    assert_eq!(
        readme, "# My Custom README\n",
        "Consumer's README.md should NOT be overwritten when excluded by source"
    );

    // Non-excluded files should be copied
    consumer
        .child("config.yaml")
        .assert(predicate::path::exists());
    let config = std::fs::read_to_string(consumer.child("config.yaml").path()).unwrap();
    assert_eq!(config, "key: source_value\n");
}

// =============================================================================
// Test 7: Source-declared rename + exclude interaction
// =============================================================================

/// When a source repo uses both rename and exclude operations, both should
/// apply correctly across all commands (ls, diff, apply).
///
/// Source renames `templates/config.yaml` → `config.yaml` and excludes `internal/**`.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_source_rename_plus_exclude_interaction() {
    let source_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &source_repo,
        &[
            (
                ".common-repo.yaml",
                r#"- rename:
    - from: "templates/(.*)"
      to: "$1"
- exclude:
    - "internal/**"
"#,
            ),
            ("templates/config.yaml", "# Renamed config\nkey: value\n"),
            ("templates/setup.sh", "#!/bin/bash\necho setup\n"),
            ("internal/dev.go", "package internal\n"),
            ("internal/test_helper.go", "package internal\n"),
            ("README.md", "# Public readme\n"),
        ],
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    let source_url = format!("file://{}", source_repo.path().display());

    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
"#,
            source_url
        ))
        .unwrap();

    // --- Test ls ---
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(consumer.path())
        .arg("ls")
        .assert()
        .success()
        // Renamed files should appear with new names
        .stdout(predicate::str::contains("config.yaml"))
        .stdout(predicate::str::contains("setup.sh"))
        .stdout(predicate::str::contains("README.md"))
        // Excluded internal files should NOT appear
        .stdout(predicate::str::contains("internal/dev.go").not())
        .stdout(predicate::str::contains("internal/test_helper.go").not())
        // Original template paths should NOT appear (renamed away)
        .stdout(predicate::str::contains("templates/").not());

    // --- Test apply ---
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(consumer.path())
        .arg("apply")
        .assert()
        .success();

    // Renamed files should exist at their new paths
    consumer
        .child("config.yaml")
        .assert(predicate::path::exists());
    consumer.child("setup.sh").assert(predicate::path::exists());
    consumer
        .child("README.md")
        .assert(predicate::path::exists());

    // Excluded files should NOT exist
    consumer
        .child("internal/dev.go")
        .assert(predicate::path::missing());
    consumer
        .child("internal/test_helper.go")
        .assert(predicate::path::missing());

    // --- Test diff (should show no changes after apply) ---
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(consumer.path())
        .arg("diff")
        .assert()
        .success()
        .stdout(predicate::str::contains("internal/").not());
}

// =============================================================================
// Test 8: Layered filtering — source exclude + consumer `with` exclude
// =============================================================================

/// Both source-level and consumer-level excludes should stack.
/// Source excludes go.mod, consumer additionally excludes LICENSE.
/// Both files should be absent from ls/diff/apply output.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_layered_source_and_consumer_excludes() {
    let source_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &source_repo,
        &[
            (
                ".common-repo.yaml",
                r#"- exclude:
    - "go.mod"
"#,
            ),
            ("README.md", "# Readme\n"),
            ("go.mod", "module example.com/source\n"),
            ("LICENSE", "MIT License\n"),
            ("main.go", "package main\n"),
        ],
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    let source_url = format!("file://{}", source_repo.path().display());

    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
    with:
      - exclude: ["LICENSE"]
"#,
            source_url
        ))
        .unwrap();

    // --- Test ls: both go.mod and LICENSE should be absent ---
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(consumer.path())
        .arg("ls")
        .assert()
        .success()
        .stdout(predicate::str::contains("README.md"))
        .stdout(predicate::str::contains("main.go"))
        // Source-excluded
        .stdout(predicate::str::contains("go.mod").not())
        // Consumer-excluded
        .stdout(predicate::str::contains("LICENSE").not());

    // --- Test apply: neither go.mod nor LICENSE should be created ---
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(consumer.path())
        .arg("apply")
        .assert()
        .success();

    consumer
        .child("README.md")
        .assert(predicate::path::exists());
    consumer.child("main.go").assert(predicate::path::exists());
    consumer.child("go.mod").assert(predicate::path::missing());
    consumer.child("LICENSE").assert(predicate::path::missing());

    // --- Test diff: neither should appear ---
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(consumer.path())
        .arg("diff")
        .assert()
        .success()
        .stdout(predicate::str::contains("go.mod").not())
        .stdout(predicate::str::contains("LICENSE").not());
}

// =============================================================================
// Test 9: `ls --count` with excludes
// =============================================================================

/// The file count from `ls --count` should reflect the filtered set,
/// not the raw unfiltered source file count.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ls_count_respects_excludes() {
    let source_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &source_repo,
        &[
            (
                ".common-repo.yaml",
                r#"- exclude:
    - "internal/**"
    - "go.mod"
    - "go.sum"
"#,
            ),
            ("README.md", "# Readme\n"),
            ("main.go", "package main\n"),
            ("go.mod", "module example.com/source\n"),
            ("go.sum", "h1:abc123\n"),
            ("internal/secret.go", "package internal\n"),
            ("internal/helper.go", "package internal\n"),
        ],
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    let source_url = format!("file://{}", source_repo.path().display());

    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
"#,
            source_url
        ))
        .unwrap();

    // Source has 6 files total, 4 are excluded (go.mod, go.sum, internal/secret.go, internal/helper.go)
    // So only 2 files should be listed: README.md and main.go
    let mut cmd = cargo_bin_cmd!("common-repo");
    let output = cmd
        .current_dir(consumer.path())
        .arg("ls")
        .arg("--count")
        .output()
        .expect("Failed to run ls --count");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let count: usize = stdout
        .trim()
        .parse()
        .unwrap_or_else(|_| panic!("Expected numeric count, got: '{}'", stdout.trim()));

    assert_eq!(
        count, 2,
        "ls --count should report 2 files (README.md, main.go) after excluding \
         go.mod, go.sum, internal/secret.go, internal/helper.go. Got {}",
        count
    );
}

// =============================================================================
// Test 10: Source-declared `include` as allowlist across all commands
// =============================================================================

/// When the source only includes `.github/**`, all three commands (ls, diff,
/// apply) should only process files under `.github/`.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_source_include_allowlist_across_all_commands() {
    let source_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &source_repo,
        &[
            (
                ".common-repo.yaml",
                r#"- include:
    - ".github/**"
"#,
            ),
            (".github/workflows/ci.yml", "name: CI\non: push\n"),
            (".github/workflows/release.yml", "name: Release\n"),
            (".github/dependabot.yml", "version: 2\n"),
            ("README.md", "# Should not be included\n"),
            ("src/main.rs", "fn main() {}\n"),
            ("Cargo.toml", "[package]\nname = \"test\"\n"),
        ],
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    let source_url = format!("file://{}", source_repo.path().display());

    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
"#,
            source_url
        ))
        .unwrap();

    // --- Test ls: only .github/ files ---
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(consumer.path())
        .arg("ls")
        .assert()
        .success()
        .stdout(predicate::str::contains("ci.yml"))
        .stdout(predicate::str::contains("release.yml"))
        .stdout(predicate::str::contains("dependabot.yml"))
        .stdout(predicate::str::contains("README.md").not())
        .stdout(predicate::str::contains("main.rs").not())
        .stdout(predicate::str::contains("Cargo.toml").not());

    // --- Test apply: only .github/ files created ---
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(consumer.path())
        .arg("apply")
        .assert()
        .success();

    consumer
        .child(".github/workflows/ci.yml")
        .assert(predicate::path::exists());
    consumer
        .child(".github/workflows/release.yml")
        .assert(predicate::path::exists());
    consumer
        .child(".github/dependabot.yml")
        .assert(predicate::path::exists());
    consumer
        .child("README.md")
        .assert(predicate::path::missing());
    consumer
        .child("src/main.rs")
        .assert(predicate::path::missing());
    consumer
        .child("Cargo.toml")
        .assert(predicate::path::missing());

    // --- Test diff: should show no changes (only .github files, all in sync) ---
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(consumer.path())
        .arg("diff")
        .assert()
        .success()
        .stdout(predicate::str::contains("README.md").not())
        .stdout(predicate::str::contains("main.rs").not())
        .stdout(predicate::str::contains("Cargo.toml").not());
}

// =============================================================================
// Additional: consistency check — ls output matches apply behavior
// =============================================================================

/// Cross-verify that the set of files reported by `ls` exactly matches
/// the set of files actually created by `apply`. This is a core invariant
/// related to issue #226.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ls_output_matches_apply_result() {
    let source_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &source_repo,
        &[
            (
                ".common-repo.yaml",
                r#"- exclude:
    - "internal/**"
    - "go.mod"
    - "go.sum"
"#,
            ),
            ("README.md", "# Readme\n"),
            ("main.go", "package main\n"),
            ("go.mod", "module example\n"),
            ("go.sum", "hash\n"),
            ("internal/a.go", "package internal\n"),
            ("internal/b.go", "package internal\n"),
        ],
    )
    .unwrap();

    // --- Run ls on a fresh consumer ---
    let consumer_ls = assert_fs::TempDir::new().unwrap();
    let source_url = format!("file://{}", source_repo.path().display());

    consumer_ls
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
"#,
            source_url
        ))
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    let ls_output = cmd
        .current_dir(consumer_ls.path())
        .arg("ls")
        .output()
        .expect("Failed to run ls");

    let ls_stdout = String::from_utf8_lossy(&ls_output.stdout);

    // --- Run apply on a separate consumer ---
    let consumer_apply = assert_fs::TempDir::new().unwrap();

    consumer_apply
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
"#,
            source_url
        ))
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(consumer_apply.path())
        .arg("apply")
        .assert()
        .success();

    // Collect files created by apply (excluding .common-repo.yaml)
    let mut applied_files: Vec<String> = Vec::new();
    fn collect_files(dir: &std::path::Path, base: &std::path::Path, files: &mut Vec<String>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Skip .git if present
                    if path.file_name().is_some_and(|n| n == ".git") {
                        continue;
                    }
                    collect_files(&path, base, files);
                } else {
                    let relative = path
                        .strip_prefix(base)
                        .unwrap()
                        .to_string_lossy()
                        .to_string();
                    if relative != ".common-repo.yaml" {
                        files.push(relative);
                    }
                }
            }
        }
    }
    collect_files(
        consumer_apply.path(),
        consumer_apply.path(),
        &mut applied_files,
    );
    applied_files.sort();

    // Every file created by apply should appear in ls output
    for file in &applied_files {
        assert!(
            ls_stdout.contains(file.as_str()),
            "File '{}' was created by apply but NOT listed by ls.\n\
             ls output:\n{}",
            file,
            ls_stdout
        );
    }

    // Excluded files should NOT appear in ls output
    assert!(
        !ls_stdout.contains("go.mod"),
        "go.mod should not appear in ls output (excluded by source)"
    );
    assert!(
        !ls_stdout.contains("go.sum"),
        "go.sum should not appear in ls output (excluded by source)"
    );
    assert!(
        !ls_stdout.contains("internal/"),
        "internal/ files should not appear in ls output (excluded by source)"
    );
}
