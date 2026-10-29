# Tests

- `cli_e2e_*.rs` — black-box commands, inheritance, sequencing, and merge behavior; use `assert_cmd::cargo::cargo_bin_cmd!("common-repo")`.
- `integration_merge_*.rs`, `*_merge_integration.rs` — format merge APIs; `schema_parsing_test.rs` — datatest-driven schema fixtures.
- `cli_snapshot_tests.rs`, `snapshots/` — insta CLI-output snapshots.
- `common/expected_fixture.rs` — byte-exact apply runner; `testdata/*/*.expected/` stores input config plus the complete expected output tree. An optional sibling `<name>.input/` seeds pre-existing working-directory files before apply. `__FIXTURE__` resolves upstream paths; see [fixture conventions](../context/expected-fixture-convention.md).
- Reuse the expected-fixture runner for deterministic apply behavior; keep generated output out of hand-captured Rust assertions. `run_expected_fixtures_with(root, Options { reapply: true, diff_after_apply: true })` also re-applies each case (fixed-point check) and requires `diff` to read the freshly applied tree as clean; `run_expected_fixtures` does neither.
- `testdata/auto-merge-stable/` (run by `cli_e2e_auto_merge_stable.rs`) pins byte stability of no-op merges across yaml, json, toml, ini, and xml, including the `base` → `mid(exclude)` → `consumer` chain from issue #376; its upstream files are written hand-style on purpose — do not reformat them to the writer's output.
- `testdata/` and `snapshots/` are test data; do not add agent indexes inside them.
- Some cases require `--features integration-tests`; inspect each test's gate/ignore attribute when selecting a targeted run.
