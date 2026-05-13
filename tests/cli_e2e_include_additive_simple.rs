//! Minimal additive-include regression: a source pipeline with two
//! sequential `include:` operators (each picking one of three files) must
//! yield a composite containing both included files and not the third.

mod common;

use common::expected_fixture::run_expected_fixtures;
use std::path::Path;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn two_includes_are_additive() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("testdata")
        .join("include-additive-simple");
    run_expected_fixtures(&fixture);
}
