# CLI commands

- `add.rs`, `init.rs`: create or extend `.common-repo.yaml`.
- `apply.rs`, `diff.rs`, `ls.rs`: write or preview what `apply` writes (`PullOutcome::local_output`), or list the source composite (`ls` is not what `apply` writes when `self:` is present).
- `check.rs`, `validate.rs`, `tree.rs`, `info.rs`: validate, inspect updates/tree/configuration.
- `update.rs`, `cache.rs`: refresh repository refs and manage cache.
- `completions.rs`: shell completion generation.

- Each module exposes clap args plus `execute`; dispatch stays in `src/cli.rs`.
- `diff` returns status 1 for detected changes (see `docs/src/cli.md`).
- `validate` and `tree` pass color settings to rendering.
