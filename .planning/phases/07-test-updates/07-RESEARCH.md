# Phase 7: Test Updates - Research

**Researched:** 2026-03-17
**Domain:** Rust test file renaming and assertion updates
**Confidence:** HIGH

## Summary

Phase 7 updates test files and assertions to reflect the "source" to "upstream" terminology rename completed in Phases 1-6. The scope is narrow and well-defined: one test file rename (`cli_e2e_source_ops.rs` -> `cli_e2e_upstream_ops.rs`), plus updating doc comments, code comments, variable names, function names, and string literals within that file and any other test files that reference "source" where it means the upstream repository.

There are also test assertions in `cli_e2e_update.rs` that check CLI output strings which were changed in Phase 6 (e.g., "Filtering sources matching" became "Filtering upstreams matching"). These assertions will fail if not updated.

**Primary recommendation:** Rename the file with `git mv`, then do a systematic find-and-replace of terminology within the file. Update `cli_e2e_update.rs` CLI output assertions. Do NOT touch merge operator `source:` references, `source_file` variables referring to file paths, or testdata YAML `source:` fields.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| TEST-01 | Update test file names that reference "source" (e.g., `cli_e2e_source_ops.rs`) | File identified: `tests/cli_e2e_source_ops.rs` -> `tests/cli_e2e_upstream_ops.rs`. Use `git mv` for rename. |
| TEST-02 | Update test assertions and string literals to match new terminology | Doc comments, code comments, variable names (`source_repo` -> `upstream_repo`, `source_url` -> `upstream_url`), function names (`test_source_*` -> `test_upstream_*`), and section banner comments all need updating. Also `cli_e2e_update.rs` has 5 assertions checking old CLI output strings. |
</phase_requirements>

## Architecture Patterns

### File Rename with git mv

Use `git mv` to rename `tests/cli_e2e_source_ops.rs` to `tests/cli_e2e_upstream_ops.rs`. This preserves git history tracking for the file.

### Scope of Changes in cli_e2e_source_ops.rs

The file is 1078 lines. Changes needed:

1. **Module doc comment** (lines 1-13): "source repository" -> "upstream repository", "source repos" -> "upstream repos", "Source ops" -> "Upstream ops"
2. **Variable names**: `source_repo` -> `upstream_repo` (~20 occurrences), `source_url` -> `upstream_url` (~14 occurrences)
3. **Function names**: `test_source_*` -> `test_upstream_*` (6 functions), `test_ls_respects_source_declared_excludes` -> `test_ls_respects_upstream_declared_excludes`
4. **Doc comments on functions**: "source repository's" -> "upstream repository's", "source repo" -> "upstream repo"
5. **Code comments**: "Create source repository" -> "Create upstream repository", "source's include" -> "upstream's include", etc.
6. **Section banner comments**: "Source's .common-repo.yaml" -> "Upstream's .common-repo.yaml", etc.
7. **String literals in assertions**: `"Source repo config"` and `"Source's .common-repo.yaml"` in assertion messages -- these are test-internal messages, not CLI output, so update them
8. **YAML content strings**: `"# Source repo config\n"` -- this is written to a temp file as test data content, rename to `"# Upstream repo config\n"` and update the assertion that checks for it

### Scope of Changes in cli_e2e_update.rs

5 assertions check CLI output strings that were changed in Phase 6:

| Line | Old String | New String |
|------|-----------|------------|
| 533 | `"Filter sources by glob pattern"` | `"Filter upstreams by glob pattern"` |
| 567 | `"Filtering sources matching"` | `"Filtering upstreams matching"` |
| 598 | `"Filtering sources matching"` | `"Filtering upstreams matching"` |
| 635 | `"Filtering sources matching: gitlab.com/*, github.com/common-repo/*"` | `"Filtering upstreams matching: gitlab.com/*, github.com/common-repo/*"` |
| 669 | `"Filtering sources matching"` | `"Filtering upstreams matching"` |

### Scope of Changes in cli_e2e_defer.rs

Line 4 doc comment: "source repositories" -> "upstream repositories" (one occurrence)

### What NOT to Change

These "source" references must be preserved:

| File | Reference | Reason |
|------|-----------|--------|
| `tests/merge_source_field_guard.rs` | All `source` references | Guards merge operator `source:` field (fragment path) |
| `tests/cli_e2e_defer.rs` | `source: settings.yaml` etc. | YAML config `source:` field for merge operators |
| `tests/cli_e2e_info.rs` | `source: config.yml` etc. | YAML config `source:` field for merge operators |
| `tests/cli_e2e_diff.rs` | `source_dir` variable | Refers to a file directory, not the repository role |
| `tests/cli_e2e_source_ops.rs` | `"# Source repo config"` string literal | This is a content marker written INTO a temp .yaml file -- it IS safe to rename since we also update the assertion checking for it |
| `tests/testdata/*/` | `source:` in YAML files | Merge operator fragment paths |
| `tests/snapshots/` | `source:` metadata | Insta snapshot metadata, not repository terminology |

## Common Pitfalls

### Pitfall 1: Renaming merge operator source: field references
**What goes wrong:** Accidentally renaming `source:` in YAML test data or merge guard tests
**How to avoid:** Only touch `cli_e2e_source_ops.rs`, `cli_e2e_update.rs`, and `cli_e2e_defer.rs` (doc comment only). Leave all other test files untouched.

### Pitfall 2: Breaking test assertions by changing content markers
**What goes wrong:** Changing the string `"# Source repo config"` written to a temp YAML file without also updating the assertion that checks for `"Source repo config"` in the output
**How to avoid:** Always change content marker AND its assertion together. In `test_source_config_file_not_copied`, line 98 writes the marker and lines 151-152 assert on it.

### Pitfall 3: Missing CLI output assertion updates in cli_e2e_update.rs
**What goes wrong:** Tests pass for `cli_e2e_upstream_ops.rs` but `cli_e2e_update.rs` integration tests fail because they still assert on old "Filtering sources" output
**How to avoid:** Update all 5 "sources" -> "upstreams" assertions in `cli_e2e_update.rs`

### Pitfall 4: Cargo test binary caching after file rename
**What goes wrong:** Old `cli_e2e_source_ops` binary artifacts remain in `target/` after rename
**How to avoid:** Not a real problem -- cargo will build the new file and the old binary simply won't be used. No cleanup needed.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| File rename | Manual copy + delete | `git mv` | Preserves history |

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo-nextest |
| Config file | `.config/nextest.toml` |
| Quick run command | `cargo nextest run cli_e2e_upstream_ops` |
| Full suite command | `./script/test` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TEST-01 | Test file renamed | smoke | `ls tests/cli_e2e_upstream_ops.rs && ! ls tests/cli_e2e_source_ops.rs` | Will exist after rename |
| TEST-02 | Assertions match new terminology | integration | `cargo nextest run --features integration-tests cli_e2e_upstream_ops` | Will exist after rename |

### Sampling Rate
- **Per task commit:** `cargo nextest run cli_e2e_upstream_ops` (unit tests only, no integration flag)
- **Per wave merge:** `./script/test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
None -- existing test infrastructure covers all phase requirements. The tests themselves ARE the deliverable.

## Sources

### Primary (HIGH confidence)
- Direct inspection of `tests/cli_e2e_source_ops.rs` (1078 lines, all occurrences catalogued)
- Direct inspection of `tests/cli_e2e_update.rs` (5 assertions with old CLI strings)
- Direct inspection of `tests/cli_e2e_defer.rs` (1 doc comment reference)
- Direct inspection of `src/commands/update.rs` (confirms Phase 6 changed "sources" to "upstreams")
- `grep` across all test files for "source" references (30 files matched, categorized)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - direct codebase inspection, no external dependencies
- Architecture: HIGH - straightforward rename operations
- Pitfalls: HIGH - all edge cases identified by reading the actual test code

**Research date:** 2026-03-17
**Valid until:** Until phase is complete (no external dependencies that could change)
