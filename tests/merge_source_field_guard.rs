//! Regression guard: merge operator `source:` field preservation
//!
//! The `source:` field on merge operator structs (YamlMergeOp, JsonMergeOp,
//! TomlMergeOp, IniMergeOp, MarkdownMergeOp) refers to a **fragment file path**,
//! not the repository. It must never be renamed to "upstream" or any other name
//! during terminology refactors. These tests will fail at compile time if the
//! `source` field or `get_source()` method is removed or renamed.

use common_repo::config::{IniMergeOp, JsonMergeOp, MarkdownMergeOp, TomlMergeOp, YamlMergeOp};

/// Verify YamlMergeOp retains its `source` field and `get_source()` method.
///
/// The `source:` field refers to a fragment file path for merging, not the
/// repository. Renaming it would break configuration file parsing.
#[test]
fn yaml_merge_op_source_field_preserved() {
    let op = YamlMergeOp::default().source("fragment.yaml");
    assert_eq!(op.get_source(), Some("fragment.yaml"));
}

/// Verify JsonMergeOp retains its `source` field and `get_source()` method.
///
/// The `source:` field refers to a fragment file path for merging, not the
/// repository. Renaming it would break configuration file parsing.
#[test]
fn json_merge_op_source_field_preserved() {
    let op = JsonMergeOp::default().source("fragment.json");
    assert_eq!(op.get_source(), Some("fragment.json"));
}

/// Verify TomlMergeOp retains its `source` field and `get_source()` method.
///
/// The `source:` field refers to a fragment file path for merging, not the
/// repository. Renaming it would break configuration file parsing.
#[test]
fn toml_merge_op_source_field_preserved() {
    let op = TomlMergeOp::default().source("fragment.toml");
    assert_eq!(op.get_source(), Some("fragment.toml"));
}

/// Verify IniMergeOp retains its `source` field and `get_source()` method.
///
/// The `source:` field refers to a fragment file path for merging, not the
/// repository. Renaming it would break configuration file parsing.
#[test]
fn ini_merge_op_source_field_preserved() {
    let op = IniMergeOp::default().source("fragment.ini");
    assert_eq!(op.get_source(), Some("fragment.ini"));
}

/// Verify MarkdownMergeOp retains its `source` field and `get_source()` method.
///
/// The `source:` field refers to a fragment file path for merging, not the
/// repository. Renaming it would break configuration file parsing.
#[test]
fn markdown_merge_op_source_field_preserved() {
    let op = MarkdownMergeOp::default().source("fragment.md");
    assert_eq!(op.get_source(), Some("fragment.md"));
}
