# Source-block include additive — context

Branch state, what's done vs to-do, and prior pitfalls. The design itself
lives at `context/source-block-include-additive-design.md`. The convention
for byte-exact apply fixtures lives at `context/expected-fixture-convention.md`.

## Branch

`fix/source-block-include-additive-1778182081` — branched from `main` at
`da611cc` (changelog).

## What's committed on the branch

- `455adcc` — `test(fixtures): byte-exact apply runner for <name>.expected/ tests`.
  Adds `tests/common/expected_fixture.rs` with `run_expected_fixtures` plus the
  self-test `tests/cli_e2e_expected_fixture.rs` and a minimal
  `expected-fixture-selftest/` fixture.

## What's uncommitted on the branch

Behaviour changes:
- `src/phases/orchestrator.rs` — Fix 1 (additive include in source-block) +
  empty-start composite. `execute_sequential_pipeline` now loads a read-only
  source FS from the working directory at function entry for both modes;
  `include` pulls additively from it. The composite starts empty for both
  modes; files enter only via `include` or `repo:` integration.
- `src/phases/processing.rs` — Fix 2 (additive include in self/cloned-repo).
  `apply_operation` now takes `&source_fs` so the cloned-repo pipeline path
  also has a read-only source FS to pull from.
- Same files include in-flight debug+trace logging at operator-dispatch
  granularity. The operator-level + yaml-merge-entry trace logs are already
  landed in PR #330 via the sibling branch
  `feat/observability-and-three-tier-fixture`.

Tests:
- `tests/cli_e2e_source_sequential.rs` — rewritten to assert additive include.
- `tests/cli_e2e_include_additive_simple.rs` — minimal regression: two
  sequential `include:` operators must add, not replace.
- `tests/testdata/include-additive-simple/` — fixture for the above using
  the `<name>.expected/` convention.
- `tests/testdata/inheritance-merge/upstream-intermediate/.common-repo.yaml`
  and `…upstream-overlay/.common-repo.yaml` — added `include: ['**']` to
  match the design's empty-start composite expectation.

Lost (untracked-stash mishap during PR split — irrecoverable from disk):
- `tests/cli_e2e_source_block_include.rs` — regression suite for the original
  source-block include bug. Had at minimum the following test names from an
  earlier inspection:
  `apply_at_overlay_inherits_base_files_through_subsequent_include`,
  `apply_at_consumer_propagates_full_chain_with_dotfiles_at_each_tier`,
  `upstream_pipeline_include_after_exclude_does_not_wipe_unrelated_files`.
- `tests/testdata/source-block-include-additive/` — fixture tree with
  `consumer/`, `consumer-of-exclude-then-include/`, `upstream-base/`,
  `upstream-overlay/`, `upstream-with-exclude-then-include/`. Some file
  contents observed earlier:
  - `consumer/.common-repo.yaml`: `- repo: { url: ../upstream-overlay }` then
    `- include: ['.consumer-config', 'consumer.txt']`.
  - `upstream-base/.common-repo.yaml`: `- include: ['**']`.
  - Upstream tiers shipped `.editorconfig`, `.gitignore`, `.overlay-rc`,
    `base.txt`, `overlay.txt`, etc.
  - `upstream-with-exclude-then-include/` shipped `drop.txt`, `keep.txt`,
    `other.txt` to exercise an exclude-then-include sequence.
  Reconstruct only if a regression suite at this level of detail proves
  needed; the inheritance-merge tests already cover the failing rule.
- `context/source-block-include-additive-{design,context}.md` — this file is
  the reconstruction; the original was lost. The companion design doc was
  rewritten from session memory and is reasonably faithful to the algorithm,
  but the inline citations to specific source line ranges did not survive.

## Production usage motivating the rule

`~/git/common-repo/upstream/.common-repo.yaml` (the meta-upstream) declares:

```yaml
- yaml: { auto-merge: .pre-commit-config.yaml, array_mode: append_unique }
- toml: { auto-merge: cog.toml }
```

…and is consumed by `common-repo/pre-commit`, `common-repo/conventional-commits`,
`common-repo/semantic-release`. Without the `if-exists: auto` rule, consumers'
local `.pre-commit-config.yaml` is overwritten by the upstream's, so the rule
is load-bearing for the org's auto-merge workflow.

## To do

The behaviour fix (Fix 1 + Fix 2 + empty-start composite) is implemented
on-disk but regresses ~18 existing e2e tests: 3 `cli_e2e_inheritance_merge`,
7 `cli_e2e_ls_filtering`, 2 `cli_e2e_sequential_ops`, 6 `cli_e2e_upstream_ops`.
On main all 18 pass; on this branch they fail because the rule that would make
them pass under the new additive semantics is not yet wired up.

1. Implement Phase 1 (tag pre-pass) and Phase 2 (inline merge at write time)
   per the design's algorithm section.
2. TDD against
   `cli_e2e_inheritance_merge::merge_at_intermediate_combines_base` first.
3. Walk remaining failures one at a time.
4. Land the Fix 1/Fix 2 + rule PR atop the already-merged observability +
   three-tier-merge PR.

## Prior pitfalls

- **Over-engineering with 3-way merge.** Earlier in the work I started
  imagining lockfiles and 3-way merge to handle idempotency. The production
  configs sidestep this with the `src/`-rename pattern (canonical content
  under `src/`, renamed to root after include) so the consumer's local file
  is never the same path as the upstream's pre-rename source. Implementing
  3-way merge first would have been wasted work.
- **Drifting from TDD into design questions.** When the algorithm felt
  ambiguous I tended to spawn open-ended questions instead of writing the
  next failing test. The corrective pattern: pick the smallest failing test,
  watch it fail, write minimal code to make it pass, repeat.
- **`git stash -u` doesn't always capture untracked dirs.** Verify with
  `git stash show -u --name-status` before destructive operations on the
  working tree. (Source of the lost docs/tests above.)
