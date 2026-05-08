//! Self-test for the `*.expected/` byte-exact fixture runner.
//!
//! See `tests/common/expected_fixture.rs` for the convention. This file
//! exercises the runner against a minimal fixture under
//! `tests/testdata/expected-fixture-selftest/` to confirm:
//!
//! - The runner discovers `<name>.expected/` directories.
//! - It substitutes the `__FIXTURE__` placeholder in the input config.
//! - It runs `common-repo apply` and asserts byte-exact output.

mod common;

use common::expected_fixture::run_expected_fixtures;
use std::path::Path;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn expected_fixture_runner_runs_minimal_fixture() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("testdata")
        .join("expected-fixture-selftest");
    run_expected_fixtures(&fixture);
}
