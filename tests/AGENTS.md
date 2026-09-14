# Tests

- `cli_e2e_*.rs` — black-box commands, inheritance, sequencing, and merge behavior; use `assert_cmd::cargo::cargo_bin_cmd!("common-repo")`.
- `integration_merge_*.rs`, `*_merge_integration.rs` — format merge APIs; `schema_parsing_test.rs` — datatest-driven schema fixtures.
- `cli_snapshot_tests.rs`, `snapshots/` — insta CLI-output snapshots.
- `common/expected_fixture.rs` — byte-exact apply runner; `testdata/*/*.expected/` stores input config plus the complete expected output tree. `__FIXTURE__` resolves upstream paths; see [fixture conventions](../context/expected-fixture-convention.md).
- Reuse the expected-fixture runner for deterministic apply behavior; keep generated output out of hand-captured Rust assertions.
- `testdata/` and `snapshots/` are test data; do not add agent indexes inside them.
- Some cases require `--features integration-tests`; inspect each test's gate/ignore attribute when selecting a targeted run.
