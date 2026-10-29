//! Self-test for the `*.expected/` byte-exact fixture runner.
//!
//! See `tests/common/expected_fixture.rs` for the convention. This file
//! exercises the runner against a minimal fixture under
//! `tests/testdata/expected-fixture-selftest/` to confirm:
//!
//! - The runner discovers `<name>.expected/` directories.
//! - It substitutes the `__FIXTURE__` placeholder in the input config.
//! - It runs `common-repo apply` and asserts byte-exact output.
//! - `<name>.input/` seeding copies nested files, is a no-op when absent,
//!   and rejects a `.common-repo.yaml` in the input directory.
//! - `Options { reapply: true, diff_after_apply: false }` runs `apply` a second time and fails when
//!   the tree drifts (`reapply-drift/`, a nested fixture root whose only
//!   case appends on every apply).

mod common;

use common::expected_fixture::{
    collect_relative_paths, run_expected_fixtures, run_expected_fixtures_with, seed_input_files,
    Options, CONFIG_FILE,
};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn expected_fixture_runner_runs_minimal_fixture() {
    run_expected_fixtures(&selftest_root());
}

fn selftest_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("testdata")
        .join("expected-fixture-selftest")
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn reapply_option_passes_when_every_case_is_a_fixed_point() {
    run_expected_fixtures_with(
        &selftest_root(),
        Options {
            reapply: true,
            diff_after_apply: false,
        },
    );
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
#[should_panic(expected = "after re-apply")]
fn reapply_option_fails_when_a_second_apply_changes_the_tree() {
    // Without the option the case passes: the first apply lands on the
    // expected bytes. The second apply appends `b` again.
    let drift = selftest_root().join("reapply-drift");
    run_expected_fixtures(&drift);
    run_expected_fixtures_with(
        &drift,
        Options {
            reapply: true,
            diff_after_apply: false,
        },
    );
}

/// Create `<stem>.expected/` and `<stem>.input/` under `fixture` and
/// return their paths in that order.
fn case_dirs(fixture: &Path, stem: &str) -> (PathBuf, PathBuf) {
    let expected_dir = fixture.join(format!("{stem}.expected"));
    let input_dir = fixture.join(format!("{stem}.input"));
    fs::create_dir_all(&expected_dir).unwrap();
    fs::create_dir_all(&input_dir).unwrap();
    (expected_dir, input_dir)
}

#[test]
fn seed_input_files_copies_nested_files() {
    let fixture = TempDir::new().unwrap();
    let (expected_dir, input_dir) = case_dirs(fixture.path(), "case");
    fs::create_dir_all(input_dir.join("nested/dir")).unwrap();
    fs::write(input_dir.join("nested/dir/seed.txt"), b"seeded").unwrap();

    let dest = TempDir::new().unwrap();
    seed_input_files(&expected_dir, dest.path());

    assert_eq!(
        fs::read(dest.path().join("nested/dir/seed.txt")).unwrap(),
        b"seeded"
    );
}

#[test]
fn seed_input_files_without_input_dir_is_a_no_op() {
    let fixture = TempDir::new().unwrap();
    let (expected_dir, input_dir) = case_dirs(fixture.path(), "case");
    fs::remove_dir(&input_dir).unwrap();

    let dest = TempDir::new().unwrap();
    seed_input_files(&expected_dir, dest.path());

    assert!(collect_relative_paths(dest.path()).is_empty());
}

#[test]
#[should_panic(expected = "must not contain .common-repo.yaml")]
fn seed_input_files_rejects_config_in_input_dir() {
    let fixture = TempDir::new().unwrap();
    let (expected_dir, input_dir) = case_dirs(fixture.path(), "case");
    fs::write(input_dir.join(CONFIG_FILE), b"- include: ['**']\n").unwrap();

    let dest = TempDir::new().unwrap();
    seed_input_files(&expected_dir, dest.path());
}
