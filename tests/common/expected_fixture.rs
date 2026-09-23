//! Byte-exact apply verification using `<name>.expected/` fixtures.
//!
//! ## Convention
//!
//! A fixture root contains zero or more sibling directories whose names end in
//! `.expected`. Each is a self-contained mini-fixture: a `.common-repo.yaml`
//! input and the byte-exact files that `common-repo apply` should produce when
//! run against that config.
//!
//! An optional sibling `<name>.input/` directory (for example `local.input/`
//! next to `local.expected/`) seeds the working directory before apply runs.
//! Every file under it is copied into the tempdir at the same relative path.
//! Use it to model files the consumer already has on disk. Seeded files
//! that apply leaves in place must also appear under `<name>.expected/`,
//! because the final tree is compared in full. `<name>.input/` must not
//! contain `.common-repo.yaml`: the input config always lives in
//! `<name>.expected/`, and the runner panics if it finds one in `.input/`.
//! `.git/` and `.common-repo-cache/` under `<name>.input/` are not seeded
//! (they are ignored prefixes).
//!
//! Other siblings of the fixture root (typically used as upstreams) are
//! referenced from the input config via the `__FIXTURE__` placeholder. The
//! runner substitutes it with the absolute path of the fixture root before
//! running apply.
//!
//! ## What the runner does
//!
//! For each `<name>.expected/`:
//!
//! 1. Create a fresh tempdir.
//! 2. If `<name>.input/` exists, copy every file in it into the tempdir,
//!    preserving relative paths.
//! 3. Read `<name>.expected/.common-repo.yaml`, substitute `__FIXTURE__` with
//!    the absolute fixture-root path, write the result to the tempdir.
//! 4. Run `common-repo apply` in the tempdir.
//! 5. Walk `<name>.expected/` (excluding `.common-repo.yaml`, which we
//!    templated and is allowed to differ) and assert every file is
//!    byte-identical to the corresponding file in the tempdir.
//! 6. Walk the tempdir and fail if any file outside the expected set was
//!    produced (catches over-creation).
//!
//! `.git/` and `.common-repo-cache/` paths are ignored on both sides.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::TempDir;

const FIXTURE_PLACEHOLDER: &str = "__FIXTURE__";
pub const CONFIG_FILE: &str = ".common-repo.yaml";
const IGNORED_PREFIXES: &[&str] = &[".git/", ".common-repo-cache/"];

/// Discover every `<name>.expected/` directory directly under `fixture_root`,
/// run apply against each, and assert the resulting tempdir matches byte-for-byte.
/// Panics on any mismatch with a unified-style summary.
pub fn run_expected_fixtures(fixture_root: &Path) {
    let cases = discover_expected_dirs(fixture_root);
    assert!(
        !cases.is_empty(),
        "no `*.expected/` directories found under {}",
        fixture_root.display()
    );
    for expected_dir in cases {
        run_one(fixture_root, &expected_dir);
    }
}

fn discover_expected_dirs(fixture_root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let entries = fs::read_dir(fixture_root).unwrap_or_else(|e| {
        panic!(
            "failed to read fixture root {}: {e}",
            fixture_root.display()
        )
    });
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if name.ends_with(".expected") {
            out.push(path);
        }
    }
    out.sort();
    out
}

fn run_one(fixture_root: &Path, expected_dir: &Path) {
    let temp = TempDir::new().expect("failed to create tempdir");
    seed_input_files(expected_dir, temp.path());
    write_templated_config(fixture_root, expected_dir, temp.path());

    let mut cmd = cargo_bin_cmd!("common-repo");
    cmd.current_dir(temp.path()).arg("apply").assert().success();

    assert_tree_matches(temp.path(), expected_dir);
}

/// Copy every file of the optional sibling `<name>.input/` directory into
/// `dest_dir`, preserving relative paths. `<name>` is `expected_dir` with its
/// `.expected` suffix replaced by `.input`. A missing `.input/` directory is
/// not an error: the case simply starts from an empty working directory.
///
/// Panics if `.input/` contains `.common-repo.yaml`; the config belongs in
/// `<name>.expected/`.
///
/// Unit tests for this helper live in `tests/cli_e2e_expected_fixture.rs`
/// so they compile into one test binary rather than every crate that
/// declares `mod common;`.
pub fn seed_input_files(expected_dir: &Path, dest_dir: &Path) {
    let Some(input_dir) = input_dir_for(expected_dir).filter(|d| d.is_dir()) else {
        return;
    };
    let paths = collect_relative_paths(&input_dir);
    assert!(
        !paths.contains(Path::new(CONFIG_FILE)),
        "{} must not contain {CONFIG_FILE}: the input config lives in {}",
        input_dir.display(),
        expected_dir.display()
    );
    for rel in paths {
        let src = input_dir.join(&rel);
        let dest = dest_dir.join(&rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|e| panic!("failed to create {}: {e}", parent.display()));
        }
        fs::copy(&src, &dest).unwrap_or_else(|e| {
            panic!(
                "failed to copy input file {} to {}: {e}",
                src.display(),
                dest.display()
            )
        });
    }
}

/// Resolve the `<name>.input/` sibling of a `<name>.expected/` directory.
fn input_dir_for(expected_dir: &Path) -> Option<PathBuf> {
    let name = expected_dir.file_name()?.to_str()?;
    let stem = name.strip_suffix(".expected")?;
    Some(expected_dir.with_file_name(format!("{stem}.input")))
}

fn write_templated_config(fixture_root: &Path, expected_dir: &Path, dest_dir: &Path) {
    let src = expected_dir.join(CONFIG_FILE);
    let raw = fs::read_to_string(&src)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", src.display()));
    let absolute_root = fixture_root.canonicalize().unwrap_or_else(|e| {
        panic!(
            "failed to canonicalize fixture root {}: {e}",
            fixture_root.display()
        )
    });
    let abs_str = absolute_root
        .to_str()
        .expect("fixture root path is not valid UTF-8");
    let templated = raw.replace(FIXTURE_PLACEHOLDER, abs_str);
    fs::write(dest_dir.join(CONFIG_FILE), templated)
        .unwrap_or_else(|e| panic!("failed to write tempdir {CONFIG_FILE}: {e}"));
}

fn assert_tree_matches(actual_dir: &Path, expected_dir: &Path) {
    let expected_files = collect_relative_paths(expected_dir);
    let actual_files = collect_relative_paths(actual_dir);
    let config_rel = PathBuf::from(CONFIG_FILE);

    let mut errors: Vec<String> = Vec::new();

    for rel in &expected_files {
        if rel == &config_rel {
            continue;
        }
        let expected_path = expected_dir.join(rel);
        let actual_path = actual_dir.join(rel);
        if !actual_path.exists() {
            errors.push(format!(
                "missing: {} — expected file was not produced by apply",
                rel.display()
            ));
            continue;
        }
        let expected_bytes = fs::read(&expected_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", expected_path.display()));
        let actual_bytes = fs::read(&actual_path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", actual_path.display()));
        if expected_bytes != actual_bytes {
            errors.push(format!(
                "content mismatch: {}\n  expected ({} bytes): {}\n  actual   ({} bytes): {}",
                rel.display(),
                expected_bytes.len(),
                summarize_bytes(&expected_bytes),
                actual_bytes.len(),
                summarize_bytes(&actual_bytes),
            ));
        }
    }

    for rel in &actual_files {
        if rel == &config_rel || expected_files.contains(rel) {
            continue;
        }
        errors.push(format!(
            "unexpected: {} — apply produced a file not listed in the expected fixture",
            rel.display()
        ));
    }

    if !errors.is_empty() {
        panic!(
            "expected fixture mismatch in {}:\n  {}",
            expected_dir.display(),
            errors.join("\n  ")
        );
    }
}

pub fn collect_relative_paths(root: &Path) -> BTreeSet<PathBuf> {
    let mut out = BTreeSet::new();
    walk(root, root, &mut out);
    out
}

fn walk(root: &Path, dir: &Path, out: &mut BTreeSet<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(it) => it,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let rel = match path.strip_prefix(root) {
            Ok(r) => r.to_path_buf(),
            Err(_) => continue,
        };
        if is_ignored(&rel) {
            continue;
        }
        if path.is_dir() {
            walk(root, &path, out);
        } else if path.is_file() {
            out.insert(rel);
        }
    }
}

fn is_ignored(rel: &Path) -> bool {
    let s = rel.to_string_lossy();
    IGNORED_PREFIXES
        .iter()
        .any(|prefix| s.starts_with(prefix) || s == prefix.trim_end_matches('/'))
}

fn summarize_bytes(bytes: &[u8]) -> String {
    const MAX: usize = 200;
    let text = String::from_utf8_lossy(bytes);
    if text.len() <= MAX {
        format!("{text:?}")
    } else {
        format!("{:?}…", &text[..MAX])
    }
}
