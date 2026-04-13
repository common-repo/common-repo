# Sequential Operations — Session Context

This document is the entry point for any fresh session working on the `sequential-operations` task. Read this first before touching any plan file.

## The Bug (One Paragraph)

Operations in `.common-repo.yaml` configs must execute in YAML declaration order. Today they don't: Phase 5 runs all merges first, then all filters; and `repo:` operations inside `self:`/source blocks are resolved up-front in Phase 1 rather than at their position in the sequence. The combined effect is that consumer filters (include/exclude/rename) destroy content that upstream repos contributed via deferred auto-merges — because the merges land in the composite before the filters run, and the filters cut them out. The fix requires sequential declaration-order execution everywhere: `self:` blocks, top-level source definitions, and `repo: with:` clauses.

## Concrete Bug Scenario (from `context/prompt.md`)

```yaml
- self:
  - include: ["src/**"]
  - template: ["src/.github/workflows/release.yaml"]
  - template-vars:
      GH_APP_ID_SECRET: COMMON_REPO_BOT_CLIENT_ID
  - rename:
      - "^src/(.*)$": "$1"
  - repo:
      url: https://github.com/common-repo/conventional-commits
      ref: v1.0.3
```

The `conventional-commits` repo specifies a deferred YAML auto-merge for `.pre-commit-config.yaml` with `array_mode: append_unique`.

- **Expected:** generated `.pre-commit-config.yaml` contains both local `src/.pre-commit-config.yaml` content AND upstream merged content.
- **Actual:** only local content appears.

## Why Three Phases

An earlier session wrote a single plan covering only Phase 5 (merging the two-pass consumer ops into one sequential pass). Analysis showed that fix alone is **insufficient** to resolve the reported bug:

Tracing the scenario with only the Phase 5 fix:
1. Phase 1 still discovers `conventional-commits` up front (regardless of its position in the sequence).
2. Phase 4 still builds the composite and collects the deferred auto-merge.
3. Phase 5 still executes the deferred merge before consumer ops run.
4. The sequential consumer pass hits `include ["src/**"]` and destroys the merged `.pre-commit-config.yaml` at root.

The real fix requires `repo:` to resolve **inline at its declaration point**, so its sub-composite (and the auto-merges it triggers) merges into the FS *after* the preceding include/template/rename have transformed it. That's a bigger architectural change than Phase 5's refactor.

Both changes need spec-level semantic decisions we don't currently have. Hence the three-phase structure:

- **Phase 0** — Allium spec work. Use `allium:elicit` to discover the sequential execution semantics, then encode them. Produces `spec/common-repo.allium` updates, a new `spec/detailed/operators.allium`, and `spec/detailed/auto-merge-composition.allium` updates.
- **Phase A** — Phase 5 single-pass refactor. Small, independently testable code change. Prerequisite for Phase B because consumer ops still need sequential execution even after repo-in-sequence is done.
- **Phase B** — Repo-in-sequence resolution. Architectural change to resolve `repo:` operations at their position in the sequence, not in Phase 1. Fully fixes the reported bug.

Each phase is a standalone working change with its own plan, tests, and review cycle.

## Open Semantic Questions (for Phase 0 elicit)

These are the undefined behaviors we need to nail down before writing code. The prompt's exploration agents flagged them explicitly:

1. **Sequential `repo:` semantics.** When `repo:` appears mid-sequence inside a `self:` or source block, what happens at that point?
   - Resolve the repo into a sub-composite, then merge it into the current FS?
   - Run the repo's own pipeline (its own Phase 1-6) against the current FS state?
   - What does "current FS state" even mean — the local files modified by preceding ops, or starting empty?
   - How do the sub-composite's deferred merges execute — immediately at that point, or deferred to the parent's Phase 5?

2. **Include / exclude interleaving.** What's the semantic when both match a file?
   - Sequential is the prompt's desired model: each op transforms the FS as it stands. But needs explicit rule.
   - Example: `include ["src/**"]` → `exclude ["*.md"]` → `include ["src/readme.md"]` — does the second include re-add readme.md from the original source, or only from what's currently in FS? (The second include can only re-add what currently exists; it can't resurrect removed files. Needs explicit spec rule.)

3. **Rename chaining.** Multiple rename ops, or multiple mappings within one rename op:
   - Do mappings within a single `rename:` op chain? (Findings note: "Mappings don't chain within one rename op." — needs codification.)
   - Do sequential `rename:` ops chain? (Yes by the sequential model, but still needs explicit rule.)

4. **CompositePrecedence presence guarantee.** The current spec (`spec/common-repo.allium:770-785`) constrains composite file *content* but not *presence*. Today, composite files can be silently dropped by filters. Is this acceptable? Should there be a guarantee like "a composite file contributed by an upstream repo via auto-merge cannot be silently removed by a consumer filter"? Or is silent drop the intended behavior (filters are authoritative)?

5. **Merge source removal.** When a merge op references a source file that was removed by a preceding exclude:
   - Error (current behavior of `read_file_as_string` — "File not found in filesystem")?
   - Silent skip?
   - Warning?
   - Decision should likely mirror the existing `MissingSourceAutoMerge` rule (warn + skip for auto-merges; error for explicit merges).

6. **`MergeLocalFiles` step 6 interleaving.** The current spec (step 6 of MergeLocalFiles) says "apply consumer filter operations" without specifying that merges and filters interleave. Needs explicit sequential rule.

7. **Scope of sequential execution.** Does "sequential" apply to:
   - All three contexts (self, source, with: clauses)? (Prompt says yes.)
   - Cross-repo ordering too? (No — cross-repo is depth-first post-order from Phase 3, already defined.)

## Key Files (Read These as Needed)

### Exploration artifacts (already written; don't re-do)
- `context/prompt.md` — original task prompt with the bug scenario, exploration findings, and Next Steps
- `context/operations/findings-synthesis.md` — synthesis of five parallel exploration agents' analysis

### Allium spec files (Phase 0 modifies these)
- `spec/common-repo.allium` — main Allium spec (missing sequential execution rules)
- `spec/detailed/auto-merge-composition.allium` — auto-merge spec
- `spec/detailed/merge-operations.allium` — merge operations
- `spec/detailed/operators.allium` — **does not exist yet**; referenced but missing; Phase 0 creates it

### Code files (Phases A and B modify these)
- `src/phases/local_merge.rs` — Phase 5; the two-pass split at lines 49-75, `apply_consumer_merges()` at 211-236, `apply_consumer_filters()` at 242-262
- `src/phases/orchestrator.rs` — pipeline orchestration; needs changes for repo-in-sequence in Phase B
- `src/phases/processing.rs` — Phase 2; already sequential; `apply_operation()` at 212-268
- `src/phases/discovery.rs` — Phase 1; repo discovery; Phase B may modify how/when this runs
- `src/phases/composite.rs` — Phase 4; composite construction and auto-merge collection
- `src/operators.rs` — operator implementations (include/exclude/rename/template/template_vars/tools as `pub(crate) mod` submodules)
- `src/config.rs:987` — `Operation` enum (untagged)
- `src/filesystem.rs` — `MemoryFS` type

### Test patterns
- `tests/cli_e2e_defer.rs:12-56` — `init_test_git_repo()` helper for E2E tests with real local git upstreams
- Use `cargo_bin_cmd!` macro, NOT deprecated `Command::cargo_bin`

## Plan Files

- `context/phase-0-spec-discovery.json` — Phase 0 plan (active; current task points here)
- `context/sequential-operations.json` — Phase A plan (Phase 5 single-pass; blocked on Phase 0)
- `context/phase-b-repo-in-sequence.json` — Phase B plan skeleton (blocked on Phase 0; detailed tasks written after Phase 0 produces spec)
- `docs/superpowers/plans/2026-04-12-sequential-operations.md` — Phase A reference plan with full code (may need revision after Phase 0)

## Branch State

- Working branch: `feat/sequential-operations-*` (created at session start; check `git branch --show-current`)
- Baseline tests pass (verified before planning)

## Fresh Session Startup

1. Run `git status` and `git branch --show-current` — verify on the feature branch, clean state (modulo context files).
2. Read this document fully.
3. Read `context/current-task.json` to find the active plan.
4. Read the referenced plan JSON.
5. Find first task with `status=pending` and `blocked_by=null`.
6. Read `context/prompt.md` and `context/operations/findings-synthesis.md` if the task needs full background (most Phase 0 tasks do).
7. Execute.

## Useful Commands

```bash
./script/test              # run test suite (cargo-nextest)
./script/ci                # CI checks (fmt, clippy, pre-commit, prose)
./script/cibuild           # full CI (update + checks + tests)
cargo xtask check-prose .  # prose linter (AI writing pattern detector)
```

## Allium Skills

- **`allium:elicit`** — structured discovery session to build/extend specs through conversation. Use this for Phase 0. It will ask the user to resolve the open semantic questions above.
- **`allium:tend`** — refine/fix/restructure existing specs. Use as cleanup after elicit if syntax or structure issues remain.
- **`allium:weed`** — find spec/code divergence. May be useful to verify Phase A and Phase B implementations against the Phase 0 spec.
- **`allium:propagate`** — generate tests from spec obligations. Candidate for Phase A and Phase B test generation once spec is finalized.
- **`allium:distill`** — extract spec from existing code. Not needed here (we're writing new semantics, not reverse-engineering).
