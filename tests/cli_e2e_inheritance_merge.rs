//! End-to-end tests for YAML merge inheritance across a 4-level local-filesystem
//! chain. The fixture at `tests/testdata/inheritance-merge/` defines three upstream
//! levels (each with a deferred YAML auto-merge op and `array_mode: append_unique`)
//! and one pure consumer. These tests `apply` at each depth and parse the resulting
//! `merge.yaml` to assert the merged shape.

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn copy_tree(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let src_p = entry.path();
        let dst_p = dst.join(entry.file_name());
        if src_p.is_dir() {
            copy_tree(&src_p, &dst_p);
        } else {
            fs::copy(&src_p, &dst_p).unwrap();
        }
    }
}

/// Copy the fixture tree into a fresh tempdir. Returns the tempdir guard
/// and the path to the staged fixture root (which contains the four
/// upstream-*/ and consumer/ subdirectories).
fn stage_fixture() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let fixture_src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("testdata")
        .join("inheritance-merge");
    let staged = tmp.path().join("inheritance-merge");
    copy_tree(&fixture_src, &staged);
    (tmp, staged)
}

/// Run `common-repo apply` in `dir`; panic if it fails.
fn apply_in(dir: &Path) {
    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(dir).arg("apply").assert().success();
}

/// Read and parse `dir/merge.yaml` as a `serde_yaml::Value`.
fn read_merge_yaml(dir: &Path) -> serde_yaml::Value {
    let text = fs::read_to_string(dir.join("merge.yaml"))
        .unwrap_or_else(|e| panic!("failed to read {}/merge.yaml: {e}", dir.display()));
    serde_yaml::from_str(&text)
        .unwrap_or_else(|e| panic!("failed to parse {}/merge.yaml as YAML: {e}", dir.display()))
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn apply_at_base_is_local_noop() {
    let (_tmp, fixture) = stage_fixture();
    let base = fixture.join("upstream-base");

    let before = read_merge_yaml(&base);

    apply_in(&base);

    let after = read_merge_yaml(&base);
    assert_eq!(
        before, after,
        "A deferred YAML op at the chain root must be a no-op locally — no upstream to merge from"
    );

    let top = after
        .as_mapping()
        .expect("merge.yaml should be a YAML mapping")
        .get(serde_yaml::Value::String("top_level_key".into()))
        .expect("top_level_key should exist");
    assert_eq!(
        top.as_f64(),
        Some(1.0),
        "upstream-base's top_level_key should be 1.0 after no-op"
    );
}

#[test]
#[cfg_attr(not(feature = "integration-tests"), ignore)]
fn merge_at_intermediate_combines_base() {
    let (_tmp, fixture) = stage_fixture();
    let dir = fixture.join("upstream-intermediate");

    apply_in(&dir);

    let merged = read_merge_yaml(&dir);
    let map = merged
        .as_mapping()
        .expect("merge.yaml should be a YAML mapping");

    // Observed on first run: apply at upstream-intermediate resolves the upstream
    // contribution after the local one, so upstream-base wins scalar conflicts and
    // sequence order is intermediate then base.
    let top = map
        .get(serde_yaml::Value::String("top_level_key".into()))
        .expect("top_level_key should exist");
    assert_eq!(
        top.as_f64(),
        Some(1.0),
        "top_level_key should reflect upstream-base winning the conflict, got {top:?}"
    );

    let list = map
        .get(serde_yaml::Value::String("list_within_key".into()))
        .and_then(|v| v.as_sequence())
        .expect("list_within_key should be a sequence");
    let list_strs: Vec<&str> = list.iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(
        list_strs,
        vec!["beta", "fnord", "alpha"],
        "list_within_key should accumulate intermediate then base with de-duplication, got {list_strs:?}"
    );

    let nested = map
        .get(serde_yaml::Value::String("nested_list_keys".into()))
        .and_then(|v| v.as_sequence())
        .expect("nested_list_keys should be a sequence");
    let repo_names: Vec<&str> = nested
        .iter()
        .filter_map(|v| {
            v.as_mapping()?
                .get(serde_yaml::Value::String("repo".into()))?
                .as_str()
        })
        .collect();
    assert_eq!(
        repo_names,
        vec!["nested_list_beta", "nested_list_alpha"],
        "nested_list_keys entries should accumulate intermediate then base, got {repo_names:?}"
    );
}
