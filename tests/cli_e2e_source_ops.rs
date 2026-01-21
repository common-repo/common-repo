//! E2E tests for source repository operation semantics.
//!
//! These tests verify that source repositories can define their "public API" of
//! files using include/exclude/rename operations in their .common-repo.yaml,
//! and that these operations are respected by consumers.
//!
//! ## Confirmed semantics being tested:
//!
//! - `include` in source: Defines the set of files exposed to consumers (allowlist)
//! - `exclude` in source: Removes files from the exposed set
//! - `rename` in source: Renames files before consumers see them
//! - Config files (.common-repo.yaml, .commonrepo.yaml): Auto-excluded from source repos
//! - Operation order: Source ops first, then consumer's `with:` ops

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
    // Initialize git repo with main as default branch
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(dir.path())
        .output()?;

    // Configure git user for commits
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir.path())
        .output()?;
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir.path())
        .output()?;
    // Disable commit signing for tests
    Command::new("git")
        .args(["config", "commit.gpgsign", "false"])
        .current_dir(dir.path())
        .output()?;

    // Create files
    for (path, content) in files {
        let file = dir.child(path);
        // Ensure parent directories exist
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(dir.path().join(parent))?;
            }
        }
        file.write_str(content)?;
    }

    // Add and commit all files
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
// Bug: Source's .common-repo.yaml should NOT be copied to consumer
// =============================================================================

/// Test that source repository's .common-repo.yaml is NOT copied to consumer.
///
/// This is a critical security/correctness issue: the source's config file
/// should never overwrite or appear in the consumer's repository.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_source_config_file_not_copied() {
    // Create source repository
    let source_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &source_repo,
        &[
            (
                ".common-repo.yaml",
                "# Source repo config\n- include:\n    patterns: [\"**/*\"]\n",
            ),
            ("README.md", "# Source README\n"),
            ("src/lib.rs", "// Source library\n"),
        ],
    )
    .unwrap();

    // Create consumer repository
    let consumer = assert_fs::TempDir::new().unwrap();
    let source_url = format!("file://{}", source_repo.path().display());

    // Consumer config references the source repo
    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"# Consumer config - should NOT be overwritten
- repo:
    url: "{}"
    ref: main
"#,
            source_url
        ))
        .unwrap();

    // Create consumer's own README that should be preserved
    consumer
        .child("README.md")
        .write_str("# Consumer README - should be overwritten by source\n")
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    // Apply the configuration
    cmd.current_dir(consumer.path())
        .arg("apply")
        .arg("--verbose")
        .assert()
        .success();

    // The consumer's .common-repo.yaml should still contain "Consumer config"
    // NOT "Source repo config"
    let config_content = std::fs::read_to_string(consumer.child(".common-repo.yaml").path())
        .expect("Config file should exist");

    assert!(
        config_content.contains("Consumer config"),
        "Consumer's .common-repo.yaml should NOT be overwritten by source's config.\n\
         Found content: {}",
        config_content
    );

    assert!(
        !config_content.contains("Source repo config"),
        "Source's .common-repo.yaml content should NOT appear in consumer.\n\
         Found content: {}",
        config_content
    );

    // Source files should be copied
    consumer
        .child("src/lib.rs")
        .assert(predicate::path::exists());
}

// =============================================================================
// Bug: Source's include operator should be respected
// =============================================================================

/// Test that source repository's `include` operator filters which files consumers receive.
///
/// When a source repo has `include: [file1.txt, file2.txt]`, only those files
/// should be exposed to consumers - not the entire repository.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_source_include_operator_respected() {
    // Create source repository with include filter
    let source_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &source_repo,
        &[
            // Source config says: only expose public-file.txt and docs/
            (
                ".common-repo.yaml",
                r#"# Source declares its public API
- include:
    patterns:
      - "public-file.txt"
      - "docs/**"
"#,
            ),
            ("public-file.txt", "This should be copied\n"),
            ("private-file.txt", "This should NOT be copied\n"),
            ("docs/guide.md", "Documentation\n"),
            ("src/internal.rs", "Internal code - should NOT be copied\n"),
        ],
    )
    .unwrap();

    // Create consumer repository
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
        .arg("apply")
        .arg("--verbose")
        .assert()
        .success();

    // Files matching source's include SHOULD be copied
    consumer
        .child("public-file.txt")
        .assert(predicate::path::exists());
    consumer
        .child("docs/guide.md")
        .assert(predicate::path::exists());

    // Files NOT matching source's include should NOT be copied
    consumer
        .child("private-file.txt")
        .assert(predicate::path::missing());
    consumer
        .child("src/internal.rs")
        .assert(predicate::path::missing());
}

// =============================================================================
// Bug: Source's exclude operator should be respected
// =============================================================================

/// Test that source repository's `exclude` operator removes files from the exposed set.
///
/// When a source repo has `exclude: [secret.txt]`, that file should not be
/// exposed to consumers even if it would otherwise be included.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_source_exclude_operator_respected() {
    // Create source repository with exclude filter
    let source_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &source_repo,
        &[
            // Source excludes internal files
            (
                ".common-repo.yaml",
                r#"# Source excludes internal files
- exclude:
    patterns:
      - "internal/**"
      - "secret.txt"
"#,
            ),
            ("README.md", "Public readme\n"),
            ("config.yaml", "Public config\n"),
            ("secret.txt", "SECRET DATA - should NOT be copied\n"),
            (
                "internal/notes.md",
                "Internal notes - should NOT be copied\n",
            ),
        ],
    )
    .unwrap();

    // Create consumer repository
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
        .arg("apply")
        .arg("--verbose")
        .assert()
        .success();

    // Non-excluded files SHOULD be copied
    consumer
        .child("README.md")
        .assert(predicate::path::exists());
    consumer
        .child("config.yaml")
        .assert(predicate::path::exists());

    // Excluded files should NOT be copied
    consumer
        .child("secret.txt")
        .assert(predicate::path::missing());
    consumer
        .child("internal/notes.md")
        .assert(predicate::path::missing());
}

// =============================================================================
// Operation order: Source ops then consumer with: ops
// =============================================================================

/// Test that operation order is: source ops first, then consumer's with: clause.
///
/// Consumer's `with:` clause should further filter files from the source's exposed set.
/// This test verifies that even when consumer uses a broad include, only source's
/// exposed files are available.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_operation_order_source_then_consumer() {
    // Create source repository that only exposes public/ directory
    let source_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &source_repo,
        &[
            (
                ".common-repo.yaml",
                r#"# Source only exposes public/ directory
- include:
    patterns:
      - "public/**"
"#,
            ),
            ("public/readme.txt", "Public file\n"),
            ("public/data.json", "Public data\n"),
            (
                "private/secret.txt",
                "Private - should NOT be available to consumer\n",
            ),
            (
                "internal/notes.md",
                "Internal - should NOT be available to consumer\n",
            ),
        ],
    )
    .unwrap();

    // Consumer uses broad include - but should only get source's exposed files
    let consumer = assert_fs::TempDir::new().unwrap();
    let source_url = format!("file://{}", source_repo.path().display());

    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
    with:
      # Consumer asks for ALL files - but should only get source's exposed set
      - include:
          patterns: ["**/*"]
"#,
            source_url
        ))
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(consumer.path())
        .arg("apply")
        .arg("--verbose")
        .assert()
        .success();

    // Files in source's include SHOULD be copied
    consumer
        .child("public/readme.txt")
        .assert(predicate::path::exists());
    consumer
        .child("public/data.json")
        .assert(predicate::path::exists());

    // Files NOT in source's include should NOT be available to consumer
    // even though consumer's with: would match them
    consumer
        .child("private/secret.txt")
        .assert(predicate::path::missing());
    consumer
        .child("internal/notes.md")
        .assert(predicate::path::missing());
}

// =============================================================================
// Source's rename operator should be respected
// =============================================================================

/// Test that source repository's `rename` operator transforms file paths.
///
/// When a source repo renames files, consumers should see the renamed paths.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_source_rename_operator_respected() {
    // Create source repository with rename
    let source_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &source_repo,
        &[
            (
                ".common-repo.yaml",
                r#"# Source renames template files
- rename:
    pattern: "templates/(.*)\\.template"
    replacement: "$1"
"#,
            ),
            (
                "templates/config.yaml.template",
                "# Template config\nkey: value\n",
            ),
            ("templates/readme.md.template", "# Template README\n"),
        ],
    )
    .unwrap();

    // Create consumer repository
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
        .arg("apply")
        .arg("--verbose")
        .assert()
        .success();

    // Files should be renamed according to source's rename operation
    // templates/config.yaml.template -> config.yaml
    consumer
        .child("config.yaml")
        .assert(predicate::path::exists());
    // templates/readme.md.template -> readme.md
    consumer
        .child("readme.md")
        .assert(predicate::path::exists());

    // Original template paths should NOT exist
    consumer
        .child("templates/config.yaml.template")
        .assert(predicate::path::missing());
    consumer
        .child("templates/readme.md.template")
        .assert(predicate::path::missing());
}

// =============================================================================
// Combined operations: include + exclude + rename
// =============================================================================

/// Test that multiple source operations work together correctly.
///
/// Tests include -> exclude -> rename pipeline in source repo.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_source_combined_operations() {
    // Create source repository with combined operations
    let source_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &source_repo,
        &[
            (
                ".common-repo.yaml",
                r#"# Combined operations
- include:
    patterns: ["configs/**", "docs/**"]
- exclude:
    patterns: ["configs/internal.yaml"]
- rename:
    pattern: "configs/(.*)"
    replacement: "settings/$1"
"#,
            ),
            ("configs/app.yaml", "app config\n"),
            ("configs/internal.yaml", "internal - should be excluded\n"),
            ("docs/readme.md", "documentation\n"),
            ("src/main.rs", "code - not in include\n"),
        ],
    )
    .unwrap();

    // Create consumer repository
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
        .arg("apply")
        .arg("--verbose")
        .assert()
        .success();

    // configs/app.yaml should be renamed to settings/app.yaml
    consumer
        .child("settings/app.yaml")
        .assert(predicate::path::exists());

    // docs/readme.md should be copied (included, not renamed)
    consumer
        .child("docs/readme.md")
        .assert(predicate::path::exists());

    // configs/internal.yaml was excluded
    consumer
        .child("settings/internal.yaml")
        .assert(predicate::path::missing());
    consumer
        .child("configs/internal.yaml")
        .assert(predicate::path::missing());

    // src/main.rs was not in include
    consumer
        .child("src/main.rs")
        .assert(predicate::path::missing());
}

// =============================================================================
// Alternate config filename: .commonrepo.yaml
// =============================================================================

/// Test that .commonrepo.yaml (alternate filename) is also auto-excluded from source.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_alternate_config_filename_not_copied() {
    // Create source repository with alternate config filename
    let source_repo = assert_fs::TempDir::new().unwrap();

    // Initialize git repo manually since we're using alternate config name
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(source_repo.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(source_repo.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(source_repo.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "commit.gpgsign", "false"])
        .current_dir(source_repo.path())
        .output()
        .unwrap();

    // Use alternate config filename
    source_repo
        .child(".commonrepo.yaml")
        .write_str("# Alternate config\n- include:\n    patterns: [\"**/*\"]\n")
        .unwrap();
    source_repo
        .child("data.txt")
        .write_str("Some data\n")
        .unwrap();

    Command::new("git")
        .args(["add", "."])
        .current_dir(source_repo.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(source_repo.path())
        .output()
        .unwrap();

    // Create consumer repository
    let consumer = assert_fs::TempDir::new().unwrap();
    let source_url = format!("file://{}", source_repo.path().display());

    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"# Consumer using primary config name
- repo:
    url: "{}"
    ref: main
"#,
            source_url
        ))
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(consumer.path())
        .arg("apply")
        .arg("--verbose")
        .assert()
        .success();

    // .commonrepo.yaml should NOT be copied from source
    consumer
        .child(".commonrepo.yaml")
        .assert(predicate::path::missing());

    // Consumer's .common-repo.yaml should still exist with consumer content
    let config_content = std::fs::read_to_string(consumer.child(".common-repo.yaml").path())
        .expect("Config should exist");
    assert!(config_content.contains("Consumer using primary config name"));

    // Other files should be copied
    consumer.child("data.txt").assert(predicate::path::exists());
}
