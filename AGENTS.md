# common-repo

Rust CLI and library for composing versioned repository configuration.

## Layout

- `src/`: library modules, CLI parsing, command dispatch, and the composition pipeline.
- `tests/`: integration and CLI end-to-end tests; fixture and snapshot trees are test support.
- `docs/`: mdBook source (`book.toml` + `src/SUMMARY.md`).
- `context/`: active design, task, and operational notes; `context/completed/` is archive material.
- `script/`: setup, dependency refresh, tests, and local/CI checks.
- `xtask/`: the `cargo xtask` developer utility, currently prose-style checking.
- `benches/`: Criterion benchmarks for parsing, filesystem operations, and operators.
- `spec/`: Allium specifications for behavior and detailed merge/operator semantics.
- `.github/`: CI, release, documentation, and action validation workflows.
- `action.yml`: public composite sync action; `install.sh`: binary installer.
- `.common-repo.yaml`, `.common-repo/`: inherited maintenance config and local workflow merge fragments.

## Key libraries

- `serde`/`serde_yaml`/`serde_json`, `toml`/`taplo`, `rust-ini`, `pulldown-cmark`, `xot` — structured-format parsing and merging.
- `clap`/`clap_complete`, `dialoguer`, `indicatif` — CLI, completions, prompts, progress.
- `glob`/`regex`/`walkdir` — file selection; `semver` — version selection; `rayon` — parallel work.
- `assert_cmd`/`assert_fs`, `insta`, `proptest`, `datatest-stable` — CLI/filesystem, snapshot, property, and fixture tests.
- `MemoryFS` in `src/filesystem.rs` is the staging abstraction for operators and merge phases.

## Development

- Run `prek install` on new checkouts/worktrees.
- Tests: `./script/test` (nextest, falling back to `cargo test`); `QUICK=1` or `SKIP_UPDATE=1` skips updates.
- Checks: `./script/ci`; full CI sequence: `./script/cibuild`.
- Integration behavior uses the `integration-tests` feature where declared.

- Implementation changes follow [TDD.md](TDD.md); CLI tests use `cargo_bin_cmd!`, never deprecated `Command::cargo_bin`.
- Preserve CLI flags, public APIs, and accepted config formats unless a breaking change is explicitly requested.
- Before publishing a branch, rebase on current main and resolve conflicts.

## Maintaining this index

- Update the affected `AGENTS.md` files in the same change when paths, responsibilities, commands, dependencies, or conventions change.
- Keep indexes brief: record semantic entry points and non-obvious constraints; link to existing documentation instead of duplicating it.
- Add a directory index only when it provides useful navigation beyond its parent; omit generated, vendored, and fixture trees.
- Every `AGENTS.md` must have a sibling `CLAUDE.md` containing only `@AGENTS.md`.
