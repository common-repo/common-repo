//! End-to-end tests for the `init --interactive` command using TTY simulation.
//!
//! These tests use the `rexpect` crate to simulate an interactive terminal session,
//! which is required because `dialoguer` prompts need a real TTY.
//!
//! **Platform limitation**: `rexpect` only works on Unix-like systems (Linux, macOS, WSL).
//! These tests are automatically skipped on Windows.
//!
//! See: <https://github.com/console-rs/dialoguer/issues/95>

#![cfg(unix)]

use std::fs;
use std::process::Command;

use rexpect::session::{spawn_command, PtySession};
use tempfile::TempDir;

/// Get the path to the `common-repo` binary.
fn get_binary_path() -> std::path::PathBuf {
    // First try the release binary
    let release_path = std::path::Path::new("target/release/common-repo");
    if release_path.exists() {
        return release_path.to_path_buf();
    }

    // Fall back to debug binary
    let debug_path = std::path::Path::new("target/debug/common-repo");
    if debug_path.exists() {
        return debug_path.to_path_buf();
    }

    // Build the binary if neither exists
    let status = Command::new("cargo")
        .args(["build", "--bin", "common-repo"])
        .status()
        .expect("Failed to build binary");
    assert!(status.success(), "Failed to build common-repo binary");

    debug_path.to_path_buf()
}

/// Create a new PTY session running `common-repo init -i` in the given directory.
fn spawn_interactive_init(temp_dir: &TempDir) -> Result<PtySession, rexpect::error::Error> {
    let binary = get_binary_path();
    let binary_path = binary
        .canonicalize()
        .expect("Failed to get absolute binary path");

    let mut cmd = Command::new(&binary_path);
    cmd.arg("init")
        .arg("--interactive")
        .current_dir(temp_dir.path());

    spawn_command(cmd, Some(30_000)) // 30 second timeout
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_interactive_empty_input_creates_empty_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let mut session =
        spawn_interactive_init(&temp_dir).expect("Failed to spawn interactive session");

    // Wait for the welcome message and initial prompt
    session
        .exp_string("Welcome to common-repo!")
        .expect("Should see welcome message");
    session
        .exp_string("Repository URL")
        .expect("Should see first prompt");

    // Press Enter without any input to finish immediately
    session.send_line("").expect("Failed to send empty line");

    // Verify it creates an empty config
    session
        .exp_string("No repositories added")
        .expect("Should see empty message");
    session
        .exp_string("Created .common-repo.yaml")
        .expect("Should see success message");

    // Wait for process to exit
    session.exp_eof().expect("Process should exit");

    // Verify the config file was created with empty content
    let config_path = temp_dir.path().join(".common-repo.yaml");
    assert!(config_path.exists(), "Config file should exist");

    let content = fs::read_to_string(&config_path).expect("Failed to read config");
    assert!(
        content.contains("# common-repo configuration"),
        "Should have header comment"
    );
    assert!(
        content.contains("# Add your repository configurations here"),
        "Should have empty config placeholder"
    );
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_interactive_github_shorthand_expansion() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let mut session =
        spawn_interactive_init(&temp_dir).expect("Failed to spawn interactive session");

    // Wait for the welcome message and initial prompt
    session
        .exp_string("Welcome to common-repo!")
        .expect("Should see welcome message");
    session
        .exp_string("Repository URL")
        .expect("Should see first prompt");

    // Enter GitHub shorthand
    session
        .send_line("rust-lang/rust")
        .expect("Failed to send shorthand");

    // Verify URL expansion in the fetching message
    session
        .exp_string("https://github.com/rust-lang/rust")
        .expect("Should expand shorthand to full URL");

    // Wait for tag fetching to complete (either success or fallback to main)
    // The output will show "found <version>" or "no tags found" or "failed"
    session
        .exp_regex("(found|no.*tags|failed)")
        .expect("Should see fetch result");

    // Wait for the next prompt
    session
        .exp_string("Add another repository")
        .expect("Should see second prompt");

    // Finish the wizard
    session.send_line("").expect("Failed to send empty line");

    // Verify success
    session
        .exp_string("Created .common-repo.yaml")
        .expect("Should see success message");

    session.exp_eof().expect("Process should exit");

    // Verify the config file contains the expanded URL
    let config_path = temp_dir.path().join(".common-repo.yaml");
    let content = fs::read_to_string(&config_path).expect("Failed to read config");
    assert!(
        content.contains("https://github.com/rust-lang/rust"),
        "Should contain expanded GitHub URL"
    );
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_interactive_full_url_with_semver() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let mut session =
        spawn_interactive_init(&temp_dir).expect("Failed to spawn interactive session");

    // Wait for the welcome message and initial prompt
    session
        .exp_string("Welcome to common-repo!")
        .expect("Should see welcome message");
    session
        .exp_string("Repository URL")
        .expect("Should see first prompt");

    // Enter a well-known repo that has semver tags
    // Using serde_json since it's a stable, popular crate with clear semver tags
    session
        .send_line("https://github.com/serde-rs/json")
        .expect("Failed to send URL");

    // Verify it fetches tags and finds a version
    session
        .exp_string("Fetching tags from")
        .expect("Should see fetch message");

    // The output should show version detection (found v1.x.x or similar)
    // or fallback to main if network issues
    session
        .exp_regex("(found v|no.*tags|failed|main)")
        .expect("Should see version or fallback");

    // Wait for the next prompt
    session
        .exp_string("Add another repository")
        .expect("Should see second prompt");

    // Finish the wizard
    session.send_line("").expect("Failed to send empty line");

    // Verify success
    session
        .exp_string("Created .common-repo.yaml")
        .expect("Should see success message");

    session.exp_eof().expect("Process should exit");

    // Verify the config file contains the repo with a ref
    let config_path = temp_dir.path().join(".common-repo.yaml");
    let content = fs::read_to_string(&config_path).expect("Failed to read config");
    assert!(
        content.contains("url: https://github.com/serde-rs/json"),
        "Should contain the repo URL"
    );
    assert!(content.contains("ref:"), "Should have a ref field");
    assert!(
        content.contains("- include:"),
        "Should have include patterns"
    );
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_interactive_invalid_url_fallback_to_main() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let mut session =
        spawn_interactive_init(&temp_dir).expect("Failed to spawn interactive session");

    // Wait for the welcome message and initial prompt
    session
        .exp_string("Welcome to common-repo!")
        .expect("Should see welcome message");
    session
        .exp_string("Repository URL")
        .expect("Should see first prompt");

    // Enter an invalid local file:// URL that will fail quickly
    // This avoids network timeouts with unreachable remote URLs
    session
        .send_line("file:///nonexistent/path/that/does/not/exist")
        .expect("Failed to send invalid URL");

    // Should see fetch attempt and error
    session
        .exp_string("Fetching tags from")
        .expect("Should see fetch message");

    // Should fall back to main branch on error (git will fail quickly for local paths)
    session
        .exp_regex("(failed|Error|main)")
        .expect("Should see error or fallback");

    // Wait for the next prompt
    session
        .exp_string("Add another repository")
        .expect("Should see second prompt");

    // Finish the wizard
    session.send_line("").expect("Failed to send empty line");

    // Verify success
    session
        .exp_string("Created .common-repo.yaml")
        .expect("Should see success message");

    session.exp_eof().expect("Process should exit");

    // Verify the config file contains the repo with main as ref
    let config_path = temp_dir.path().join(".common-repo.yaml");
    let content = fs::read_to_string(&config_path).expect("Failed to read config");
    assert!(
        content.contains("file:///nonexistent/path/that/does/not/exist"),
        "Should contain the repo URL"
    );
    assert!(
        content.contains("ref: main"),
        "Should fallback to main branch"
    );
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_interactive_multiple_repos() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let mut session =
        spawn_interactive_init(&temp_dir).expect("Failed to spawn interactive session");

    // Wait for the welcome message and initial prompt
    session
        .exp_string("Welcome to common-repo!")
        .expect("Should see welcome message");
    session
        .exp_string("Repository URL")
        .expect("Should see first prompt");

    // Enter first repo using shorthand
    session
        .send_line("rust-lang/log")
        .expect("Failed to send first repo");

    // Wait for fetch to complete
    session
        .exp_regex("(found|no.*tags|failed)")
        .expect("Should see first fetch result");

    // Wait for second prompt
    session
        .exp_string("Add another repository")
        .expect("Should see second prompt");

    // Enter second repo
    session
        .send_line("rust-lang/regex")
        .expect("Failed to send second repo");

    // Wait for fetch to complete
    session
        .exp_regex("(found|no.*tags|failed)")
        .expect("Should see second fetch result");

    // Wait for third prompt
    session
        .exp_string("Add another repository")
        .expect("Should see third prompt");

    // Finish the wizard
    session.send_line("").expect("Failed to send empty line");

    // Verify success
    session
        .exp_string("Created .common-repo.yaml")
        .expect("Should see success message");

    session.exp_eof().expect("Process should exit");

    // Verify the config file contains both repos
    let config_path = temp_dir.path().join(".common-repo.yaml");
    let content = fs::read_to_string(&config_path).expect("Failed to read config");
    assert!(
        content.contains("https://github.com/rust-lang/log"),
        "Should contain first repo"
    );
    assert!(
        content.contains("https://github.com/rust-lang/regex"),
        "Should contain second repo"
    );
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_interactive_force_overwrite() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create an existing config file
    let config_path = temp_dir.path().join(".common-repo.yaml");
    fs::write(&config_path, "# existing content\n").expect("Failed to create existing config");

    // First, try without --force (should fail)
    let binary = get_binary_path();
    let binary_path = binary
        .canonicalize()
        .expect("Failed to get absolute binary path");

    let output = Command::new(&binary_path)
        .arg("init")
        .arg("--interactive")
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to run command");

    assert!(
        !output.status.success(),
        "Should fail without --force when file exists"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("already exists"),
        "Should mention file exists"
    );

    // Now try with --force
    let mut cmd = Command::new(&binary_path);
    cmd.arg("init")
        .arg("--interactive")
        .arg("--force")
        .current_dir(temp_dir.path());

    let mut session =
        spawn_command(cmd, Some(30_000)).expect("Failed to spawn interactive session");

    // Wait for the welcome message
    session
        .exp_string("Welcome to common-repo!")
        .expect("Should see welcome message");

    // Finish immediately
    session
        .exp_string("Repository URL")
        .expect("Should see prompt");
    session.send_line("").expect("Failed to send empty line");

    // Verify success
    session
        .exp_string("Created .common-repo.yaml")
        .expect("Should see success message");

    session.exp_eof().expect("Process should exit");

    // Verify the file was overwritten
    let content = fs::read_to_string(&config_path).expect("Failed to read config");
    assert!(
        !content.contains("# existing content"),
        "Should have overwritten existing content"
    );
    assert!(
        content.contains("# common-repo configuration"),
        "Should have new content"
    );
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_interactive_precommit_config_generation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let mut session =
        spawn_interactive_init(&temp_dir).expect("Failed to spawn interactive session");

    // Wait for the welcome message and initial prompt
    session
        .exp_string("Welcome to common-repo!")
        .expect("Should see welcome message");
    session
        .exp_string("Repository URL")
        .expect("Should see first prompt");

    // Skip adding repos
    session.send_line("").expect("Failed to send empty line");

    // Should see the pre-commit hooks prompt
    session
        .exp_string("Set up pre-commit hooks?")
        .expect("Should see pre-commit prompt");

    // Accept the default (yes)
    session.send_line("").expect("Failed to accept pre-commit");

    // Should create the pre-commit config
    session
        .exp_string("Created .pre-commit-config.yaml")
        .expect("Should see pre-commit config created");

    // Wait for final success message
    session
        .exp_string("Created .common-repo.yaml")
        .expect("Should see success message");

    session.exp_eof().expect("Process should exit");

    // Verify both config files were created
    let common_repo_config = temp_dir.path().join(".common-repo.yaml");
    assert!(
        common_repo_config.exists(),
        "common-repo config should exist"
    );

    let precommit_config = temp_dir.path().join(".pre-commit-config.yaml");
    assert!(precommit_config.exists(), "pre-commit config should exist");

    let content = fs::read_to_string(&precommit_config).expect("Failed to read pre-commit config");
    assert!(content.contains("repos:"), "Should have repos section");
    assert!(
        content.contains("pre-commit/pre-commit-hooks"),
        "Should reference pre-commit-hooks"
    );
    assert!(
        content.contains("trailing-whitespace"),
        "Should include trailing-whitespace hook"
    );
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_interactive_precommit_decline() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let mut session =
        spawn_interactive_init(&temp_dir).expect("Failed to spawn interactive session");

    // Wait for the welcome message and initial prompt
    session
        .exp_string("Welcome to common-repo!")
        .expect("Should see welcome message");
    session
        .exp_string("Repository URL")
        .expect("Should see first prompt");

    // Skip adding repos
    session.send_line("").expect("Failed to send empty line");

    // Should see the pre-commit hooks prompt
    session
        .exp_string("Set up pre-commit hooks?")
        .expect("Should see pre-commit prompt");

    // Decline
    session
        .send_line("n")
        .expect("Failed to decline pre-commit");

    // Should NOT create the pre-commit config (shouldn't see that message)
    // Just proceed to final success
    session
        .exp_string("Created .common-repo.yaml")
        .expect("Should see success message");

    session.exp_eof().expect("Process should exit");

    // Verify only the common-repo config was created
    let common_repo_config = temp_dir.path().join(".common-repo.yaml");
    assert!(
        common_repo_config.exists(),
        "common-repo config should exist"
    );

    let precommit_config = temp_dir.path().join(".pre-commit-config.yaml");
    assert!(
        !precommit_config.exists(),
        "pre-commit config should NOT exist when declined"
    );
}
