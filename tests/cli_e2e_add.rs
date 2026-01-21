//! End-to-end tests for the `add` command.
//!
//! These tests invoke the actual CLI binary and validate the behavior of the
//! `add` subcommand from a user's perspective.
//!
//! Note: The default `add` behavior prompts for confirmation when no config exists.
//! Most tests use --yes to skip the interactive prompt.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_add_quiet_creates_config() {
    let temp = assert_fs::TempDir::new().unwrap();

    // Add with --yes to a directory without config
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("add")
        .arg("--yes")
        .arg("rust-lang/rust-clippy")
        .assert()
        .success()
        .stdout(predicate::str::contains("Fetching tags from"))
        .stdout(predicate::str::contains("✅ Created .common-repo.yaml"));

    // Check that the config file was created with the repo
    let config_file = temp.child(".common-repo.yaml");
    config_file.assert(predicate::path::exists());
    config_file.assert(predicate::str::contains(
        "url: https://github.com/rust-lang/rust-clippy",
    ));
    config_file.assert(predicate::str::contains("ref:"));
    config_file.assert(predicate::str::contains("- include:"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_add_to_existing_config() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Create existing config
    config_file
        .write_str(
            r#"# common-repo configuration

- repo:
    url: https://github.com/existing/repo
    ref: v1.0.0

- include:
    - "**/*"
"#,
        )
        .unwrap();

    // Add another repo
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("add")
        .arg("rust-lang/rust-clippy")
        .assert()
        .success()
        .stdout(predicate::str::contains("Fetching tags from"))
        .stdout(predicate::str::contains("✅ Added"));

    // Verify both repos are in the config
    config_file.assert(predicate::str::contains(
        "url: https://github.com/existing/repo",
    ));
    config_file.assert(predicate::str::contains(
        "url: https://github.com/rust-lang/rust-clippy",
    ));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_add_github_shorthand_expansion() {
    let temp = assert_fs::TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("add")
        .arg("--yes")
        .arg("rust-lang/rust-clippy")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Fetching tags from https://github.com/rust-lang/rust-clippy",
        ));

    // Check URL was expanded in config
    let config_file = temp.child(".common-repo.yaml");
    config_file.assert(predicate::str::contains(
        "url: https://github.com/rust-lang/rust-clippy",
    ));
}

#[test]
fn test_add_without_uri_shows_error() {
    let temp = assert_fs::TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("add")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_add_help() {
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.arg("add")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Add a repository"))
        .stdout(predicate::str::contains("--yes"));
}
