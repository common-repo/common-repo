//! E2E tests for upstream repository operation semantics.
//!
//! These tests verify that upstream repositories can define their "public API" of
//! files using include/exclude/rename operations in their .common-repo.yaml,
//! and that these operations are respected by consumers.
//!
//! ## Confirmed semantics being tested:
//!
//! - `include` in upstream: Defines the set of files exposed to consumers (allowlist)
//! - `exclude` in upstream: Removes files from the exposed set
//! - `rename` in upstream: Renames files before consumers see them
//! - Config files (.common-repo.yaml, .commonrepo.yaml): Auto-excluded from upstream repos
//! - Operation order: Upstream ops first, then consumer's `with:` ops

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

    // Add and commit all files (--no-verify skips pre-commit hooks that
    // would fail in temp directories without a .pre-commit-config.yaml)
    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()?;
    let commit_output = Command::new("git")
        .args(["commit", "--no-verify", "-m", "Initial commit"])
        .current_dir(dir.path())
        .output()?;
    assert!(
        commit_output.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&commit_output.stderr)
    );

    Ok(())
}

// =============================================================================
// Bug: Upstream's .common-repo.yaml should NOT be copied to consumer
// =============================================================================

/// Test that upstream repository's .common-repo.yaml is NOT copied to consumer.
///
/// This is a critical security/correctness issue: the upstream's config file
/// should never overwrite or appear in the consumer's repository.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_upstream_config_file_not_copied() {
    // Create upstream repository
    let upstream_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &upstream_repo,
        &[
            (
                ".common-repo.yaml",
                "# Upstream repo config\n- include: [\"**/*\"]\n",
            ),
            ("README.md", "# Upstream README\n"),
            ("src/lib.rs", "// Upstream library\n"),
        ],
    )
    .unwrap();

    // Create consumer repository
    let consumer = assert_fs::TempDir::new().unwrap();
    let upstream_url = format!("file://{}", upstream_repo.path().display());

    // Consumer config references the upstream repo
    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"# Consumer config - should NOT be overwritten
- repo:
    url: "{}"
    ref: main
"#,
            upstream_url
        ))
        .unwrap();

    // Create consumer's own README that should be preserved
    consumer
        .child("README.md")
        .write_str("# Consumer README - should be overwritten by upstream\n")
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    // Apply the configuration
    cmd.current_dir(consumer.path())
        .arg("apply")
        .arg("--verbose")
        .assert()
        .success();

    // The consumer's .common-repo.yaml should still contain "Consumer config"
    // NOT "Upstream repo config"
    let config_content = std::fs::read_to_string(consumer.child(".common-repo.yaml").path())
        .expect("Config file should exist");

    assert!(
        config_content.contains("Consumer config"),
        "Consumer's .common-repo.yaml should NOT be overwritten by upstream's config.\n\
         Found content: {}",
        config_content
    );

    assert!(
        !config_content.contains("Upstream repo config"),
        "Upstream's .common-repo.yaml content should NOT appear in consumer.\n\
         Found content: {}",
        config_content
    );

    // Upstream files should be copied
    consumer
        .child("src/lib.rs")
        .assert(predicate::path::exists());
}

// =============================================================================
// Bug: Upstream's include operator should be respected
// =============================================================================

/// Test that upstream repository's `include` operator filters which files consumers receive.
///
/// When an upstream repo has `include: [file1.txt, file2.txt]`, only those files
/// should be exposed to consumers - not the entire repository.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_upstream_include_operator_respected() {
    // Create upstream repository with include filter
    let upstream_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &upstream_repo,
        &[
            // Upstream config says: only expose public-file.txt and docs/
            (
                ".common-repo.yaml",
                r#"# Upstream declares its public API
- include:
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
    let upstream_url = format!("file://{}", upstream_repo.path().display());

    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
"#,
            upstream_url
        ))
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(consumer.path())
        .arg("apply")
        .arg("--verbose")
        .assert()
        .success();

    // Files matching upstream's include SHOULD be copied
    consumer
        .child("public-file.txt")
        .assert(predicate::path::exists());
    consumer
        .child("docs/guide.md")
        .assert(predicate::path::exists());

    // Files NOT matching upstream's include should NOT be copied
    consumer
        .child("private-file.txt")
        .assert(predicate::path::missing());
    consumer
        .child("src/internal.rs")
        .assert(predicate::path::missing());
}

// =============================================================================
// Bug: Upstream's exclude operator should be respected
// =============================================================================

/// Test that upstream repository's `exclude` operator removes files from the exposed set.
///
/// When an upstream repo has `exclude: [secret.txt]`, that file should not be
/// exposed to consumers even if it would otherwise be included.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_upstream_exclude_operator_respected() {
    // Create upstream repository with exclude filter
    let upstream_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &upstream_repo,
        &[
            // Upstream excludes internal files
            (
                ".common-repo.yaml",
                r#"# Upstream excludes internal files
- exclude:
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
    let upstream_url = format!("file://{}", upstream_repo.path().display());

    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
"#,
            upstream_url
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
// Operation order: Upstream ops then consumer with: ops
// =============================================================================

/// Test that operation order is: upstream ops first, then consumer's with: clause.
///
/// Consumer's `with:` clause should further filter files from the upstream's exposed set.
/// This test verifies that even when consumer uses a broad include, only upstream's
/// exposed files are available.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_operation_order_upstream_then_consumer() {
    // Create upstream repository that only exposes public/ directory
    let upstream_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &upstream_repo,
        &[
            (
                ".common-repo.yaml",
                r#"# Upstream only exposes public/ directory
- include:
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

    // Consumer uses broad include - but should only get upstream's exposed files
    let consumer = assert_fs::TempDir::new().unwrap();
    let upstream_url = format!("file://{}", upstream_repo.path().display());

    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
    with:
      # Consumer asks for ALL files - but should only get upstream's exposed set
      - include: ["**/*"]
"#,
            upstream_url
        ))
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(consumer.path())
        .arg("apply")
        .arg("--verbose")
        .assert()
        .success();

    // Files in upstream's include SHOULD be copied
    consumer
        .child("public/readme.txt")
        .assert(predicate::path::exists());
    consumer
        .child("public/data.json")
        .assert(predicate::path::exists());

    // Files NOT in upstream's include should NOT be available to consumer
    // even though consumer's with: would match them
    consumer
        .child("private/secret.txt")
        .assert(predicate::path::missing());
    consumer
        .child("internal/notes.md")
        .assert(predicate::path::missing());
}

// =============================================================================
// Upstream's rename operator should be respected
// =============================================================================

/// Test that upstream repository's `rename` operator transforms file paths.
///
/// When an upstream repo renames files, consumers should see the renamed paths.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_upstream_rename_operator_respected() {
    // Create upstream repository with rename
    let upstream_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &upstream_repo,
        &[
            (
                ".common-repo.yaml",
                r#"# Upstream renames template files
- rename:
    - from: "templates/(.*)\\.template"
      to: "$1"
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
    let upstream_url = format!("file://{}", upstream_repo.path().display());

    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
"#,
            upstream_url
        ))
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(consumer.path())
        .arg("apply")
        .arg("--verbose")
        .assert()
        .success();

    // Files should be renamed according to upstream's rename operation
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
// Combined upstream operations: include + exclude + rename
// =============================================================================

/// Test that multiple upstream operations work together correctly.
///
/// Tests include + exclude pipeline in upstream repo.
/// Note: Rename is tested separately in test_upstream_rename_operator_respected
/// due to a config parser issue when combining all three operations.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_upstream_combined_operations() {
    // Create upstream repository with combined include + exclude
    let upstream_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &upstream_repo,
        &[
            (
                ".common-repo.yaml",
                r#"# Combined include and exclude
- include:
    - "configs/**"
    - "docs/**"
- exclude:
    - "configs/internal.yaml"
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
    let upstream_url = format!("file://{}", upstream_repo.path().display());

    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
"#,
            upstream_url
        ))
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(consumer.path())
        .arg("apply")
        .arg("--verbose")
        .assert()
        .success();

    // configs/app.yaml should be copied (included, not excluded)
    consumer
        .child("configs/app.yaml")
        .assert(predicate::path::exists());

    // docs/readme.md should be copied (included)
    consumer
        .child("docs/readme.md")
        .assert(predicate::path::exists());

    // configs/internal.yaml was excluded
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

/// Test that .commonrepo.yaml (alternate filename) is also auto-excluded from upstream.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_alternate_config_filename_not_copied() {
    // Create upstream repository with alternate config filename
    let upstream_repo = assert_fs::TempDir::new().unwrap();

    // Initialize git repo manually since we're using alternate config name
    Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(upstream_repo.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(upstream_repo.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(upstream_repo.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "commit.gpgsign", "false"])
        .current_dir(upstream_repo.path())
        .output()
        .unwrap();

    // Use alternate config filename
    upstream_repo
        .child(".commonrepo.yaml")
        .write_str("# Alternate config\n- include: [\"**/*\"]\n")
        .unwrap();
    upstream_repo
        .child("data.txt")
        .write_str("Some data\n")
        .unwrap();

    Command::new("git")
        .args(["add", "."])
        .current_dir(upstream_repo.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(upstream_repo.path())
        .output()
        .unwrap();

    // Create consumer repository
    let consumer = assert_fs::TempDir::new().unwrap();
    let upstream_url = format!("file://{}", upstream_repo.path().display());

    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"# Consumer using primary config name
- repo:
    url: "{}"
    ref: main
"#,
            upstream_url
        ))
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(consumer.path())
        .arg("apply")
        .arg("--verbose")
        .assert()
        .success();

    // .commonrepo.yaml should NOT be copied from upstream
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

// =============================================================================
// Real-world test: shakefu/vibes repository
// =============================================================================

/// Test against the real shakefu/vibes repository (pinned to v0.1.0-test tag).
///
/// This repo has an include filter that only exposes CLAUDE.md and .mcp.json.
/// This test verifies that:
/// 1. Only included files are copied (CLAUDE.md, .mcp.json)
/// 2. The upstream's .common-repo.yaml is NOT copied to consumer
/// 3. Files not in the include list are not copied (LICENSE, README.md, etc.)
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_real_world_shakefu_vibes_repo() {
    // Create consumer repository
    let consumer = assert_fs::TempDir::new().unwrap();

    // Consumer config references the real shakefu/vibes repo
    consumer
        .child(".common-repo.yaml")
        .write_str(
            r#"# Consumer config - testing against real shakefu/vibes repo
- repo:
    url: "https://github.com/shakefu/vibes"
    ref: v0.1.0-test
"#,
        )
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    // Apply the configuration
    let assert = cmd
        .current_dir(consumer.path())
        .arg("apply")
        .arg("--verbose")
        .assert();

    assert.success();

    // Files in upstream's include SHOULD be copied
    consumer
        .child("CLAUDE.md")
        .assert(predicate::path::exists());
    consumer
        .child(".mcp.json")
        .assert(predicate::path::exists());

    // Upstream's .common-repo.yaml should NOT be copied
    // Consumer's config should still contain "Consumer config"
    let config_content = std::fs::read_to_string(consumer.child(".common-repo.yaml").path())
        .expect("Config file should exist");
    assert!(
        config_content.contains("Consumer config"),
        "Consumer's .common-repo.yaml should NOT be overwritten.\nFound: {}",
        config_content
    );

    // Files NOT in upstream's include should NOT be copied
    consumer.child("LICENSE").assert(predicate::path::missing());
    consumer
        .child("README.md")
        .assert(predicate::path::missing());
    consumer
        .child(".pre-commit-config.yaml")
        .assert(predicate::path::missing());
}

// =============================================================================
// Bug #226: ls should respect exclude filters
// =============================================================================

/// Test that `ls` respects upstream-declared exclude filters.
///
/// When an upstream repo excludes files in its .common-repo.yaml, those files
/// should not appear in the `ls` output.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ls_respects_upstream_declared_excludes() {
    let upstream_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &upstream_repo,
        &[
            (
                ".common-repo.yaml",
                r#"- exclude:
    - "go.mod"
    - "go.sum"
    - "cmd/**"
"#,
            ),
            ("go.mod", "module example.com/test\n"),
            ("go.sum", "hash\n"),
            ("cmd/app/main.go", "package main\n"),
            ("lib/utils.go", "package lib\n"),
            ("README.md", "# Test\n"),
        ],
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    let upstream_url = format!("file://{}", upstream_repo.path().display());

    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
"#,
            upstream_url
        ))
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    let output = cmd
        .current_dir(consumer.path())
        .arg("ls")
        .output()
        .expect("failed to execute ls");

    assert!(
        output.status.success(),
        "ls command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = String::from_utf8_lossy(&output.stdout);

    // Files NOT excluded should appear
    assert!(
        output.contains("lib/utils.go"),
        "ls should show lib/utils.go"
    );
    assert!(output.contains("README.md"), "ls should show README.md");

    // Excluded files should NOT appear
    assert!(
        !output.contains("go.mod"),
        "ls should not show upstream-excluded go.mod, got:\n{output}"
    );
    assert!(
        !output.contains("go.sum"),
        "ls should not show upstream-excluded go.sum, got:\n{output}"
    );
    assert!(
        !output.contains("cmd/app/main.go"),
        "ls should not show upstream-excluded cmd/app/main.go, got:\n{output}"
    );
}

/// Test that `ls` respects consumer top-level exclude filters.
///
/// When the consumer's .common-repo.yaml has a top-level exclude, those files
/// should not appear in the `ls` output.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ls_respects_consumer_top_level_excludes() {
    let upstream_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &upstream_repo,
        &[
            ("lib/utils.go", "package lib\n"),
            ("README.md", "# Test\n"),
            ("Makefile", "all:\n"),
        ],
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    let upstream_url = format!("file://{}", upstream_repo.path().display());

    // Consumer excludes Makefile at top level
    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
- exclude:
    - "Makefile"
"#,
            upstream_url
        ))
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    let output = cmd
        .current_dir(consumer.path())
        .arg("ls")
        .output()
        .expect("failed to execute ls");

    assert!(
        output.status.success(),
        "ls command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.contains("lib/utils.go"),
        "ls should show lib/utils.go"
    );
    assert!(output.contains("README.md"), "ls should show README.md");
    assert!(
        !output.contains("Makefile"),
        "ls should not show consumer-excluded Makefile, got:\n{output}"
    );
}

/// Test that `ls` respects consumer with-clause exclude filters.
///
/// When the consumer uses `with:` to exclude files from an upstream, those files
/// should not appear in the `ls` output.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_ls_respects_consumer_with_clause_excludes() {
    let upstream_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &upstream_repo,
        &[
            ("lib/utils.go", "package lib\n"),
            ("README.md", "# Test\n"),
            ("LICENSE", "MIT\n"),
        ],
    )
    .unwrap();

    let consumer = assert_fs::TempDir::new().unwrap();
    let upstream_url = format!("file://{}", upstream_repo.path().display());

    // Consumer uses with: clause to exclude LICENSE
    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
    with:
      - exclude:
          - "LICENSE"
"#,
            upstream_url
        ))
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    let output = cmd
        .current_dir(consumer.path())
        .arg("ls")
        .output()
        .expect("failed to execute ls");

    assert!(
        output.status.success(),
        "ls command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.contains("lib/utils.go"),
        "ls should show lib/utils.go"
    );
    assert!(output.contains("README.md"), "ls should show README.md");
    assert!(
        !output.contains("LICENSE"),
        "ls should not show with-clause-excluded LICENSE, got:\n{output}"
    );
}

// =============================================================================
// Bug #249: Upstream-declared template vars baked into cache — consumer overrides ignored
// =============================================================================

/// Test that consumer's `with:` template-vars override upstream's default template-vars.
///
/// When an upstream repo declares `template:` and `template-vars:`, and a consumer
/// overrides some vars via `with: template-vars:`, the consumer's values should
/// take precedence over the upstream's defaults.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_consumer_with_template_vars_override_upstream_defaults() {
    // Create upstream repository with template + template-vars
    let upstream_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &upstream_repo,
        &[
            (
                ".common-repo.yaml",
                r#"- include:
    - "src/**"
    - "src/.*"
    - "src/.*/**"
- template:
    - "src/.github/workflows/release.yaml"
- template-vars:
    GH_APP_OWNER: christmas-island
    GH_APP_ID_VAR: CHRISTMAS_ISLAND_APP_ID
- rename:
    - from: "^src/(.*)"
      to: "$1"
"#,
            ),
            (
                "src/.github/workflows/release.yaml",
                "owner: ${GH_APP_OWNER:-christmas-island}\napp_id: ${GH_APP_ID_VAR}\n",
            ),
        ],
    )
    .unwrap();

    // Create consumer that overrides GH_APP_OWNER via with: clause
    let consumer = assert_fs::TempDir::new().unwrap();
    let upstream_url = format!("file://{}", upstream_repo.path().display());

    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- repo:
    url: "{}"
    ref: main
    with:
      - template-vars:
          GH_APP_OWNER: my-cool-org
"#,
            upstream_url
        ))
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(consumer.path())
        .arg("apply")
        .arg("--verbose")
        .assert()
        .success();

    // Verify the template was expanded with consumer's override
    let workflow_path = consumer.child(".github/workflows/release.yaml");
    workflow_path.assert(predicate::path::exists());

    let content = std::fs::read_to_string(workflow_path.path()).unwrap();

    // Consumer override should win for GH_APP_OWNER
    assert!(
        content.contains("owner: my-cool-org"),
        "Consumer's template-vars override should be applied.\n\
         Expected 'owner: my-cool-org' but got:\n{}",
        content
    );

    // Upstream defaults should be preserved for non-overridden vars
    assert!(
        content.contains("app_id: CHRISTMAS_ISLAND_APP_ID"),
        "Upstream's non-overridden template-vars should be preserved.\n\
         Expected 'app_id: CHRISTMAS_ISLAND_APP_ID' but got:\n{}",
        content
    );
}

/// Test that consumer's top-level template-vars override upstream's defaults.
///
/// When a consumer has top-level `template-vars:` (not in `with:`), these should
/// override the upstream's default values during Phase 4 composite construction.
#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_consumer_top_level_template_vars_override_upstream_defaults() {
    // Create upstream repository with template + template-vars
    let upstream_repo = assert_fs::TempDir::new().unwrap();
    init_git_repo(
        &upstream_repo,
        &[
            (
                ".common-repo.yaml",
                r#"- include:
    - "configs/**"
- template:
    - "configs/app.yaml"
- template-vars:
    APP_NAME: default-app
    APP_PORT: "8080"
"#,
            ),
            ("configs/app.yaml", "name: ${APP_NAME}\nport: ${APP_PORT}\n"),
        ],
    )
    .unwrap();

    // Create consumer with top-level template-vars override
    let consumer = assert_fs::TempDir::new().unwrap();
    let upstream_url = format!("file://{}", upstream_repo.path().display());

    consumer
        .child(".common-repo.yaml")
        .write_str(&format!(
            r#"- template-vars:
    APP_NAME: my-custom-app
- repo:
    url: "{}"
    ref: main
"#,
            upstream_url
        ))
        .unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(consumer.path())
        .arg("apply")
        .arg("--verbose")
        .assert()
        .success();

    // Verify the template was expanded with consumer's override
    let config_path = consumer.child("configs/app.yaml");
    config_path.assert(predicate::path::exists());

    let content = std::fs::read_to_string(config_path.path()).unwrap();

    // Consumer override should win for APP_NAME
    assert!(
        content.contains("name: my-custom-app"),
        "Consumer's top-level template-vars override should be applied.\n\
         Expected 'name: my-custom-app' but got:\n{}",
        content
    );

    // Upstream defaults should be preserved for non-overridden vars
    assert!(
        content.contains("port: 8080"),
        "Upstream's non-overridden template-vars should be preserved.\n\
         Expected 'port: 8080' but got:\n{}",
        content
    );
}
