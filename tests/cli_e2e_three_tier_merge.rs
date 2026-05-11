//! Three-tier auto-merge regression: upstream contributes a yaml file plus
//! an extra file, source adds a yaml fragment plus a third file, and the
//! consumer holds a base yaml at the same path. Expected output: merged
//! yaml carries all three tiers' sections; both extra files are present.

mod common;

use common::expected_fixture::run_expected_fixtures;
use std::path::Path;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn consumer_merges_three_tiers() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("testdata")
        .join("three-tier-merge");
    run_expected_fixtures(&fixture);
}
