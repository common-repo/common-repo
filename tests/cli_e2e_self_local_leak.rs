//! Regression test for GitHub issue #361 / #350: when `.common-repo.yaml`
//! declares a `self:` block, `apply` must write only the `self:` output to
//! the working directory. The top-level (source API) pipeline output is
//! built in memory for consumers but must not leak onto local disk.

mod common;

use common::expected_fixture::run_expected_fixtures;
use std::path::Path;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn self_block_does_not_leak_source_api_locally() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("testdata")
        .join("self-local-leak");
    run_expected_fixtures(&fixture);
}
