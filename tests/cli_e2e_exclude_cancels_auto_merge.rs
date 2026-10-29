//! End-to-end regression tests for issue #383 (exclude side): a repository
//! that excludes a path owns that path. No deferred auto-merge declared by
//! any of its upstreams for that path runs on it:
//!
//! - a consumer-side `exclude` cancels inherited declarations for the path,
//!   so the consumer's local file is never merged into (the behavior the
//!   authoring guide's "Consumer Override" section used to describe);
//! - an intermediate's `with: exclude` cancels the declaration at the point
//!   it takes over the file, so a farther upstream's declaration does not
//!   travel to the intermediate's consumers — a fresh consumer gets the
//!   intermediate's file byte for byte and a stale consumer's outdated
//!   entries are replaced, not preserved by the upstream's array mode.
//!
//! Every scenario is a byte-exact fixture under
//! `tests/testdata/exclude-cancels-upstream-auto-merge/` run through the
//! shared expected-fixture runner with the re-apply check on: YAML
//! (`.github/workflows/ci.yaml` chains), JSON (`services.json`) and XML
//! (`config.xml`) upstreams alike, since the cancellation is keyed on the
//! declared path, not the format.

mod common;

use common::expected_fixture::{run_expected_fixtures_with, Options};
use std::path::Path;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn expected_fixtures_match_byte_exactly_and_are_idempotent() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("testdata")
        .join("exclude-cancels-upstream-auto-merge");
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
