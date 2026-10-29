//! End-to-end regression tests for issue #370: a deferred auto-merge with
//! `array_mode: merge_by_key` must update keyed list items in place when the
//! upstream revises them, instead of appending the edited items as duplicates
//! next to the stale copies (the old `append_unique` failure mode).
//!
//! Every scenario is a byte-exact fixture under `tests/testdata/merge-by-key/`
//! run through the shared expected-fixture runner: YAML (workflow steps,
//! pre-commit repos keyed on `repo`, `position: start`), JSON, TOML
//! array-of-tables, and XML (attribute keys). The runner re-applies each case
//! and checks the tree again, so every `*.expected/` there is also asserted
//! to be a fixed point.

mod common;

use common::expected_fixture::{run_expected_fixtures_with, Options};
use std::path::Path;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn expected_fixtures_match_byte_exactly_and_are_idempotent() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("testdata")
        .join("merge-by-key");
    run_expected_fixtures_with(
        &fixture,
        Options {
            reapply: true,
            diff_after_apply: false,
        },
    );
}
