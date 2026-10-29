//! Byte-exact fixtures for issue #376: an auto-merge file the consumer has
//! never edited must stay byte-identical to the copy it came from.
//!
//! Each case runs `apply` (the first apply must produce the expected tree
//! byte for byte), then `diff` (a freshly applied tree must read as clean,
//! not as drift), then `apply` again (merging a file with itself is a no-op,
//! so the bytes must not move). The two-upstream chain case covers an
//! intermediate repo whose own file sits at a path a farther upstream
//! auto-merges; the rest cover a single upstream per format. Every input
//! file is written the way a person writes it — header comments, SHA-pin
//! comments, flow lists, inline tables, non-writer indentation — because the
//! bug is exactly that the merge writers reformat those on a no-op merge.

mod common;

use common::expected_fixture::{run_expected_fixtures_with, Options};
use std::path::Path;

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn fresh_auto_merge_files_are_stable_across_diff_and_reapply() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("testdata")
        .join("auto-merge-stable");
    let options = Options {
        reapply: true,
        diff_after_apply: true,
    };
    run_expected_fixtures_with(&fixture, options);
}
