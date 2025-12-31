//! End-to-end tests for the `init` command.
//!
//! These tests invoke the actual CLI binary and validate the behavior of the
//! `init` subcommand from a user's perspective.
//!
//! Note: The default `init` behavior is now an interactive wizard that requires
//! TTY simulation. Most interactive tests are in `cli_e2e_init_interactive.rs`.
//! This file tests the non-interactive modes (URI argument, force flag, etc).

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_init_with_uri_positional_arg() {
    let temp = assert_fs::TempDir::new().unwrap();

    // Use a known repo with semver tags
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("init")
        .arg("https://github.com/rust-lang/rust-clippy")
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
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_init_with_uri_github_shorthand() {
    let temp = assert_fs::TempDir::new().unwrap();

    // Use GitHub shorthand format
    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("init")
        .arg("rust-lang/rust-clippy")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Fetching tags from https://github.com/rust-lang/rust-clippy",
        ));

    // Check that the config file was created with expanded URL
    let config_file = temp.child(".common-repo.yaml");
    config_file.assert(predicate::path::exists());
    config_file.assert(predicate::str::contains(
        "url: https://github.com/rust-lang/rust-clippy",
    ));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_init_with_uri_force_overwrite() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Create existing config file
    config_file.write_str("existing content").unwrap();

    // Try to init without force - should fail
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(temp.path())
        .arg("init")
        .arg("rust-lang/rust-clippy")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "already exists. Use --force to overwrite",
        ));

    // Now try with force - should succeed
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(temp.path())
        .arg("init")
        .arg("rust-lang/rust-clippy")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("Fetching tags from"))
        .stdout(predicate::str::contains("✅ Created .common-repo.yaml"));

    // Verify the content was overwritten
    config_file.assert(predicate::str::contains("# common-repo configuration"));
    config_file.assert(predicate::str::contains("existing content").not());
    config_file.assert(predicate::str::contains(
        "url: https://github.com/rust-lang/rust-clippy",
    ));
}
