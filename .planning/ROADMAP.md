# Roadmap: Upstream Terminology Rename

## Overview

Systematically replace "source repo" terminology with "upstream repo" across the common-repo Rust codebase. The work proceeds from foundational config structs outward through code identifiers, operations terminology, merge operator guards, comments, CLI output, and finally tests. Each phase delivers a verifiable slice of the rename that can be checked independently before moving to the next.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Config Structs** - Rename config struct fields from "source" to "upstream" (hard rename, no backwards compat) ✓ 2026-03-17
- [x] **Phase 2: Core Code Rename** - Rename struct fields, variable names, and function names across all key source files ✓ 2026-03-17
- [x] **Phase 3: Operations Terminology** - Rename source-declared, source filtering, source authors, and source_ops to upstream equivalents ✓ 2026-03-17
- [ ] **Phase 4: Operators Preservation** - Verify and guard that merge operator source: field is preserved unchanged
- [ ] **Phase 5: Code Comments** - Update all code comments referencing "source repo" to "upstream repo"
- [ ] **Phase 6: CLI and Error Output** - Update help text, user-facing messages, and error messages to use "upstream"
- [ ] **Phase 7: Test Updates** - Rename test files and update test assertions to match new terminology
- [ ] **Phase 8: Final Validation** - All tests pass and CI is green after the complete rename

## Phase Details

### Phase 1: Config Structs
**Goal**: Config struct fields use "upstream" terminology, establishing the foundation for all downstream code changes
**Depends on**: Nothing (first phase)
**Requirements**: CONF-01, CONF-02
**Success Criteria** (what must be TRUE):
  1. All config struct fields that previously referenced "source repo" now use "upstream" naming
  2. No backwards compatibility shims or deprecation warnings exist for renamed fields
  3. Code compiles after the config rename (downstream breakage is expected and fixed in later phases)
**Plans**: 1 plan

Plans:
- [x] 01-01-PLAN.md -- Update config struct doc comments to use "upstream" terminology

### Phase 2: Core Code Rename
**Goal**: All Rust identifiers (struct fields, variables, function names) across key source files use "upstream" instead of "source repo"
**Depends on**: Phase 1
**Requirements**: CODE-01
**Success Criteria** (what must be TRUE):
  1. No struct field, variable, or function name contains "source_repo" or similar where it refers to the upstream repository
  2. Files src/phases/discovery.rs, src/phases/processing.rs, src/phases/composite.rs, src/config.rs, src/error.rs use "upstream" identifiers
  3. Code compiles cleanly after all identifier renames
**Plans**: 1 plan

Plans:
- [x] 02-01-PLAN.md — Rename IntermediateFS source_url/source_ref fields to upstream_url/upstream_ref

### Phase 3: Operations Terminology
**Goal**: Rename source-declared, source filtering, source authors, and source_ops to upstream equivalents
**Depends on**: Phase 2
**Requirements**: CODE-03, CODE-04, CODE-05, CODE-06
**Success Criteria** (what must be TRUE):
  1. "source-declared" operations are now called "upstream-declared" in all code paths
  2. "source filtering" is now "upstream filtering" in all code paths
  3. "source authors" is now "upstream authors" in all code paths
  4. "source_ops" / "source ops" references are now "upstream_ops" / "upstream ops"
  5. Code compiles and the renamed operations concepts function identically
**Plans**: 1 plan

Plans:
- [x] 03-01-PLAN.md -- Rename all operations terminology in discovery.rs and processing.rs

### Phase 4: Operators Preservation
**Goal**: Merge operator source: field is confirmed preserved and protected from accidental rename
**Depends on**: Phase 3
**Requirements**: CODE-07
**Success Criteria** (what must be TRUE):
  1. The `source:` field in yaml, json, toml, ini, and markdown merge operators remains named "source"
  2. No code change in phases 1-3 accidentally renamed the merge operator source: field
  3. Merge operators function correctly with their source: field pointing to fragment file paths
**Plans**: 1 plan

Plans:
- [ ] 04-01-PLAN.md -- Audit source: field preservation and add regression test guard

### Phase 5: Code Comments
**Goal**: All code comments throughout the codebase use "upstream repo" instead of "source repo"
**Depends on**: Phase 4
**Requirements**: CODE-02
**Success Criteria** (what must be TRUE):
  1. No code comment refers to "source repo" or "source repository" where it means the upstream repository
  2. Comments about the merge operator source: field still correctly reference "source" (since that field is unchanged)
  3. Comments are accurate and consistent with the renamed identifiers
**Plans**: 2 plans

Plans:
- [ ] 05-01-PLAN.md -- Update comments in src/phases/ files (discovery.rs, processing.rs, composite.rs)
- [ ] 05-02-PLAN.md -- Update repo-role comments in src/config.rs (preserve merge operator source: field comments)

### Phase 6: CLI and Error Output
**Goal**: All user-facing text (help, output messages, error messages) uses "upstream" terminology
**Depends on**: Phase 5
**Requirements**: CLI-01, CLI-02, CLI-03
**Success Criteria** (what must be TRUE):
  1. Running `common-repo --help` and subcommand help shows "upstream" instead of "source repo"
  2. Normal operation output messages reference "upstream" repository, not "source"
  3. Error messages triggered by invalid config or missing repos say "upstream" not "source repo"
**Plans**: 1 plan

Plans:
- [ ] 06-01-PLAN.md -- Update CLI help text and output messages, verify no remaining source repo references

### Phase 7: Test Updates
**Goal**: Test files and assertions reflect the new "upstream" terminology
**Depends on**: Phase 6
**Requirements**: TEST-01, TEST-02
**Success Criteria** (what must be TRUE):
  1. Test file `cli_e2e_source_ops.rs` is renamed to reflect "upstream" terminology
  2. All test string literals and assertions match the new "upstream" output and identifiers
  3. Test intent and coverage remain identical -- no tests removed or weakened
**Plans:** 2 plans

Plans:
- [ ] 07-01-PLAN.md -- Rename test file and update all source-to-upstream terminology in cli_e2e_source_ops.rs
- [ ] 07-02-PLAN.md -- Update CLI output assertions in cli_e2e_update.rs and doc comment in cli_e2e_defer.rs

### Phase 8: Final Validation
**Goal**: The complete rename is verified: all tests pass and CI checks are green
**Depends on**: Phase 7
**Requirements**: TEST-03, TEST-04
**Success Criteria** (what must be TRUE):
  1. `./script/test` passes with zero failures
  2. `./script/ci` passes (fmt, clippy, pre-commit, prose checks all green)
  3. No remaining references to "source repo" in code identifiers, comments, or CLI output (except merge operator source: field)
**Plans**: 2 plans

Plans:
- [ ] 08-01-PLAN.md — Run full test suite and CI checks, fix any failures
- [ ] 08-02-PLAN.md — Final grep audit for remaining "source repo" references

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Config Structs | 1/1 | Complete | 2026-03-17 |
| 2. Core Code Rename | 1/1 | Complete | 2026-03-17 |
| 3. Operations Terminology | 1/1 | Complete | 2026-03-17 |
| 4. Operators Preservation | 0/1 | Not started | - |
| 5. Code Comments | 0/2 | Not started | - |
| 6. CLI and Error Output | 0/1 | Not started | - |
| 7. Test Updates | 0/2 | Not started | - |
| 8. Final Validation | 0/2 | Not started | - |
