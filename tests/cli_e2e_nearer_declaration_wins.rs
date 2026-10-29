//! End-to-end regression tests for issue #383 (precedence side): when
//! several upstreams declare an auto-merge for the same path, the
//! declaration nearest the consumer wins — the intermediate's own
//! re-declaration beats the farther upstream's, reversing the old
//! farthest-upstream-wins precedence. A differing duplicate is no longer
//! silent: the tool warns which declaration it uses.
//!
//! Chains under `tests/testdata/nearer-declaration-wins/`: base declares
//! `append_unique`, mid re-declares `merge_by_key` without excluding base's
//! file, and a consumer holding a stale copy proves the keyed mode ran
//! (items updated in place, consumer-only entries preserved) instead of
//! `append_unique` (edited items duplicated). YAML, JSON and XML variants
//! cover the merge operations of all three formats. The runner re-applies
//! each case, so every `*.expected/` is also a fixed point.

mod common;

use common::expected_fixture::{run_expected_fixtures_with, Options};
use std::path::Path;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn expected_fixtures_match_byte_exactly_and_are_idempotent() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("testdata")
        .join("nearer-declaration-wins");
    // `..Default::default()` rather than listing every field: #387 adds
    // `diff_after_apply` to `Options`, and an exhaustive initializer here
    // breaks the build whichever of the two branches lands second. The
    // allow covers the window where `reapply` is still the only field.
    #[allow(clippy::needless_update)]
    run_expected_fixtures_with(
        &fixture,
        Options {
            reapply: true,
            ..Default::default()
        },
    );
}
