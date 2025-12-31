//! End-to-end tests for the `init` command.
//!
//! These tests invoke the actual CLI binary and validate the behavior of the
//! `init` subcommand from a user's perspective.

use assert_cmd::cargo::cargo_bin_cmd;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_init_minimal_config() {
    let temp = assert_fs::TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("init")
        .arg("--minimal")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "🎯 Initializing common-repo configuration",
        ))
        .stdout(predicate::str::contains("✅ Created .common-repo.yaml"))
        .stdout(predicate::str::contains("💡 Run `common-repo apply`"));

    // Check that the config file was created with expected content
    let config_file = temp.child(".common-repo.yaml");
    config_file.assert(predicate::path::exists());
    config_file.assert(predicate::str::contains("# common-repo configuration"));
    config_file.assert(predicate::str::contains("repo:"));
    config_file.assert(predicate::str::contains("include:"));
    config_file.assert(predicate::str::contains("exclude:"));
    config_file.assert(predicate::str::contains("template:"));
    config_file.assert(predicate::str::contains("template-vars:"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_init_empty_config() {
    let temp = assert_fs::TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("init")
        .arg("--empty")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "🎯 Initializing common-repo configuration",
        ))
        .stdout(predicate::str::contains("✅ Created .common-repo.yaml"));

    // Check that the config file was created with minimal content
    let config_file = temp.child(".common-repo.yaml");
    config_file.assert(predicate::path::exists());
    config_file.assert(predicate::str::contains("# common-repo configuration"));
    config_file.assert(predicate::str::contains(
        "# Add your repository configurations here",
    ));
    // Should not contain repo operations
    config_file.assert(
        predicate::str::contains("# Add your repository configurations here")
            .not()
            .or(predicate::str::contains("repo:").not()),
    );
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_init_template_rust_cli() {
    let temp = assert_fs::TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("init")
        .arg("--template")
        .arg("rust-cli")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "🎯 Initializing common-repo configuration",
        ))
        .stdout(predicate::str::contains("✅ Created .common-repo.yaml"));

    // Check that the config file was created with rust-cli template
    let config_file = temp.child(".common-repo.yaml");
    config_file.assert(predicate::path::exists());
    config_file.assert(predicate::str::contains(
        "# Rust CLI Application Configuration",
    ));
    config_file.assert(predicate::str::contains("common-repo/rust-cli"));
    config_file.assert(predicate::str::contains("common-repo/ci-rust"));
    config_file.assert(predicate::str::contains("common-repo/pre-commit-hooks"));
    config_file.assert(predicate::str::contains("rust_version:"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_init_template_python_django() {
    let temp = assert_fs::TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("init")
        .arg("--template")
        .arg("python-django")
        .assert()
        .success();

    // Check that the config file was created with python-django template
    let config_file = temp.child(".common-repo.yaml");
    config_file.assert(predicate::path::exists());
    config_file.assert(predicate::str::contains(
        "# Python Django Application Configuration",
    ));
    config_file.assert(predicate::str::contains("common-repo/python-django"));
    config_file.assert(predicate::str::contains("common-repo/ci-python"));
    config_file.assert(predicate::str::contains("django_version:"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_init_template_unknown() {
    let temp = assert_fs::TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("init")
        .arg("--template")
        .arg("unknown-template")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Unknown template 'unknown-template'",
        ));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_init_force_overwrite() {
    let temp = assert_fs::TempDir::new().unwrap();
    let config_file = temp.child(".common-repo.yaml");

    // Create existing config file
    config_file.write_str("existing content").unwrap();

    // Try to init without force - should fail
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(temp.path())
        .arg("init")
        .arg("--minimal")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "already exists. Use --force to overwrite",
        ));

    // Now try with force - should succeed
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(temp.path())
        .arg("init")
        .arg("--minimal")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("✅ Created .common-repo.yaml"));

    // Verify the content was overwritten
    config_file.assert(predicate::str::contains("# common-repo configuration"));
    config_file.assert(predicate::str::contains("existing content").not());
}

// Note: Interactive mode requires TTY simulation (e.g., rexpect) because dialoguer
// reads from terminal. This test is disabled until rexpect-based E2E tests are added.
// See: context/init-redesign.json task "add-init-e2e-tests" for testing plan.
// See: https://github.com/console-rs/dialoguer/issues/95
#[test]
#[ignore = "interactive mode requires TTY - use rexpect for E2E testing"]
fn test_init_interactive_mode() {
    let temp = assert_fs::TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path())
        .arg("init")
        .arg("--interactive")
        .assert()
        .success()
        .stdout(predicate::str::contains("Welcome to common-repo!"))
        .stdout(predicate::str::contains(
            "Enter repository URLs to inherit from",
        ))
        .stdout(predicate::str::contains("✅ Created .common-repo.yaml"));

    // Check that the config file was created
    let config_file = temp.child(".common-repo.yaml");
    config_file.assert(predicate::path::exists());
    config_file.assert(predicate::str::contains("# common-repo configuration"));
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn test_init_default_is_minimal() {
    let temp = assert_fs::TempDir::new().unwrap();

    let mut cmd = cargo_bin_cmd!("common-repo");

    cmd.current_dir(temp.path()).arg("init").assert().success();

    // Check that the default creates minimal config
    let config_file = temp.child(".common-repo.yaml");
    config_file.assert(predicate::path::exists());
    config_file.assert(predicate::str::contains("repo:"));
    config_file.assert(predicate::str::contains("include:"));
    config_file.assert(predicate::str::contains("exclude:"));
}
