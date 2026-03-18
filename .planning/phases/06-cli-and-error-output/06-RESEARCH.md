# Phase 6: CLI and Error Output - Research

**Researched:** 2026-03-17
**Domain:** Rust CLI output strings, clap help text, error messages
**Confidence:** HIGH

## Summary

Phase 6 requires updating all user-facing text (help, output, errors) from "source" to "upstream" terminology where "source" refers to the upstream repository. The scope is very narrow after thorough codebase investigation.

Only two changes are needed, both in `src/commands/update.rs`: one clap doc comment that generates CLI help text (line 75), and one println output message (line 115). All other "source" references in error messages and config validation refer to the merge operator `source:` field (fragment file path), which is explicitly out of scope per project rules.

**Primary recommendation:** Update the two "source" references in `src/commands/update.rs` and verify no other user-facing strings reference "source repo."

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CLI-01 | Update CLI help text to use "upstream" instead of "source repo" | One clap doc comment in `src/commands/update.rs:75` uses "sources" to mean upstream repositories. All other help text is clean. |
| CLI-02 | Update user-facing output messages to use "upstream" terminology | One println in `src/commands/update.rs:115` says "Filtering sources matching". All other output messages are clean. |
| CLI-03 | Update error messages to use "upstream" terminology | No error messages reference "source repo" (the repository). All "source" in error messages refer to the merge operator `source:` field (fragment file path) -- out of scope. |
</phase_requirements>

## Standard Stack

No additional libraries needed. This phase uses existing Rust standard library and clap.

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap | (existing) | CLI argument parsing, help text generation | Already in use; doc comments on Args fields become help text |

## Architecture Patterns

### Where User-Facing Text Lives

CLI help text is generated from doc comments on clap struct fields:
```
src/cli.rs           -- Top-level CLI struct and subcommand enum (clean)
src/commands/*.rs     -- Subcommand Args structs with doc comment help text
```

Output messages use direct `println!()` calls in command execute functions:
```
src/commands/update.rs  -- Update command output (HAS "sources" reference)
src/commands/check.rs   -- Check command output (clean)
src/commands/apply.rs   -- Apply command output (clean)
src/commands/validate.rs -- Validate command output (clean)
src/commands/diff.rs    -- Diff command output (clean)
src/commands/add.rs     -- Add command output (clean)
src/commands/info.rs    -- Info command output (clean)
src/commands/tree.rs    -- Tree command output (clean)
```

Error messages are defined in:
```
src/error.rs         -- Error enum with Display impl (clean -- no repo "source" refs)
src/config.rs        -- Config validation errors (all "source" refs are merge operator field)
src/merge/*.rs       -- Merge operation errors ("source" = fragment file, not repo)
```

### Snapshot Tests

CLI help output is captured in snapshot tests at `tests/snapshots/`. After changing help text, snapshot tests will need updating (handled in Phase 7, but `cargo test` with `INSTA_UPDATE=1` or `cargo insta review` may be needed during this phase to keep tests passing).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Snapshot updates | Manual snapshot editing | `cargo insta review` or `INSTA_UPDATE=1 cargo test` | Prevents formatting mismatches |

## Common Pitfalls

### Pitfall 1: Over-Renaming Merge Operator "source" References
**What goes wrong:** Accidentally renaming error messages that say "source" when they refer to the merge operator `source:` field (fragment file path), not the repository.
**Why it happens:** The word "source" appears extensively in config.rs and merge/*.rs error messages, but these all refer to the `source:` YAML/TOML/JSON/INI field that specifies a fragment file path.
**How to avoid:** Only rename "source" when it clearly means "upstream repository." The merge operator field references (e.g., "Cannot use auto-merge with explicit source or dest", "YAML merge requires source and dest", "Failed to parse source YAML") must NOT be renamed.
**Warning signs:** If you find yourself editing config.rs merge validation errors or merge/*.rs parse errors, you are touching out-of-scope code.

### Pitfall 2: Breaking Snapshot Tests
**What goes wrong:** Changing help text causes snapshot test failures.
**Why it happens:** `tests/cli_snapshot_tests.rs` captures help output in snapshot files under `tests/snapshots/`.
**How to avoid:** After changing help text, run tests and update snapshots. The update command help is NOT currently snapshotted (only main, apply, check, init, ls help are), but verify this before assuming.

### Pitfall 3: Missing Test URL "source-repo" References
**What goes wrong:** Thinking test data URLs like `https://github.com/source-repo.git` need updating in this phase.
**Why it happens:** These appear in `src/phases/composite.rs` test code.
**How to avoid:** Test data updates are Phase 7 scope. Phase 6 only covers user-facing CLI output.

## Code Examples

### Change 1: CLI Help Text (CLI-01)

File: `src/commands/update.rs`, line 75

Before:
```rust
    /// Filter sources by glob pattern (matches against url/path, scheme stripped).
```

After:
```rust
    /// Filter upstreams by glob pattern (matches against url/path, scheme stripped).
```

### Change 2: Output Message (CLI-02)

File: `src/commands/update.rs`, line 115

Before:
```rust
        println!("Filtering sources matching: {}", patterns);
```

After:
```rust
        println!("Filtering upstreams matching: {}", patterns);
```

## Detailed Audit Results

### Files with "source" -- Confirmed Out of Scope

| File | Lines | Why Out of Scope |
|------|-------|------------------|
| `src/config.rs` | 203, 205, 213, 215, 359, 361, 368, 370, 461, 463, 470, 472, 588, 590, 597, 599, 734, 736, 742, 744 | Merge operator `source:` field validation errors -- refers to fragment file path |
| `src/config.rs` | 2305, 2316, 2327, 2338, 2370 | Test assertions checking merge operator error messages |
| `src/merge/yaml.rs` | 284, 296 | "source validated" / "Failed to parse source YAML" -- fragment file |
| `src/merge/json.rs` | 201, 213 | "source validated" / "Failed to parse source JSON" -- fragment file |
| `src/merge/toml.rs` | 365, 375 | "source validated" / "Failed to parse source TOML" -- fragment file |
| `src/merge/ini.rs` | 196 | "source validated" -- fragment file |
| `src/merge/markdown.rs` | 155 | "source validated" -- fragment file |
| `src/error.rs` | 378, 384 | Test code using "source.txt" as a filename |
| `src/phases/composite.rs` | 864-887 | Test code with "source-repo" URLs -- Phase 7 scope |
| All `src/merge/*.rs` test code | Many lines | `source: Some("source.yaml".to_string())` -- merge operator field in tests |

### Files Confirmed Clean (No Changes Needed)

| File | Status |
|------|--------|
| `src/cli.rs` | No "source" references in help text |
| `src/commands/apply.rs` | Clean |
| `src/commands/check.rs` | Clean |
| `src/commands/diff.rs` | Clean |
| `src/commands/add.rs` | Clean |
| `src/commands/info.rs` | Clean |
| `src/commands/tree.rs` | Clean |
| `src/commands/validate.rs` | Only comment about merge source/dest validation -- out of scope |
| `src/output.rs` | Clean |
| `src/suggestions.rs` | Clean |
| `src/error.rs` | No repo-related "source" in Display impl |

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo-nextest + insta (snapshots) |
| Config file | `.config/nextest.toml` |
| Quick run command | `cargo nextest run` |
| Full suite command | `./script/test` |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CLI-01 | Help text says "upstreams" not "sources" | snapshot | `cargo test cli_snapshot` | Partial -- update help may not be snapshotted |
| CLI-02 | Output message says "upstreams" | manual-only | Run `common-repo update --filter "test"` | No automated test |
| CLI-03 | Error messages use "upstream" | manual-only | Grep audit for remaining "source repo" | No -- verified clean already |

### Sampling Rate
- **Per task commit:** `cargo nextest run`
- **Per wave merge:** `./script/test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
None -- the changes are minimal string updates. Existing snapshot tests will catch regressions if update help is snapshotted. A grep audit verifies completeness.

## Open Questions

1. **Is `common-repo update --help` snapshot tested?**
   - What we know: main, apply, check, init, ls help are snapshotted. Update help may or may not be.
   - What's unclear: Whether changing the filter help text will break a snapshot test.
   - Recommendation: Run `cargo test` after making the change; if snapshots fail, update them with `cargo insta review`.

## Sources

### Primary (HIGH confidence)
- Direct codebase grep audit of all `src/` files for "source" references
- Read of `src/commands/update.rs` lines 75 and 115 confirming the only two changes needed
- Read of `src/config.rs` lines 198-220 confirming merge operator scope
- Read of `src/merge/yaml.rs` lines 280-297 confirming fragment file scope

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - no new libraries needed, trivial string changes
- Architecture: HIGH - direct codebase audit, all files checked
- Pitfalls: HIGH - clear distinction between repo "source" and merge operator "source" established across prior phases

**Research date:** 2026-03-17
**Valid until:** 2026-04-17 (stable -- only string literals involved)
