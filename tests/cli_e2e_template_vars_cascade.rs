//! Template-vars cascade across multi-level inheritance. Every case renders
//! the same `settings.conf` template (`log_level`, `timeout`, `region`) from
//! a different arrangement of upstream tiers and pins the value each
//! variable resolves to.
//!
//! Own blocks win. A repo's own `template-vars` blocks, including the
//! `with:` blocks a consumer appends to that repo, beat every value the repo
//! inherits, no matter where the blocks appear in its config file.
//!
//! Later sibling wins. Among sibling `repo:` entries in one file, the later
//! entry's whole resolved result overrides the earlier one's, so a value the
//! later sibling only inherited from its own ancestor still beats a value the
//! earlier sibling defined itself.
//!
//! Earlier sibling gap-fills. An earlier sibling's values apply wherever no
//! later sibling defines them.
//!
//! Ancestors gap-fill. Within one `repo:` chain, the nearer repo overrides
//! the farther one, and ancestor values only fill in variables nothing nearer
//! defines.

mod common;

use common::expected_fixture::run_expected_fixtures;
use std::path::Path;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn consumer_resolves_template_vars_cascade() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("testdata")
        .join("template-vars-cascade");
    run_expected_fixtures(&fixture);
}
