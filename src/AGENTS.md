# Library and CLI core

- `lib.rs`: public module/re-export surface; `main.rs` and `cli.rs`: binary wrapper and dispatch.
- `config.rs`: `.common-repo.yaml` schema, parser, and backward-compatible normalization.
- `filesystem.rs`: `MemoryFS`; `operators.rs`: include/exclude/repo/template/tools operations.
- `repository.rs`, `git.rs`, `cache.rs`: local/Git loading and ref caching.
- `merge/`: format engines; `phases/`: discovery through disk output.
- `path.rs`, `output.rs`, `error.rs`, `defaults.rs`, `suggestions.rs`: shared support.
- `version.rs`: semver/version checks; root `schema.yaml`: config schema for tooling.

- Preserve `lib.rs` re-exports when changing module boundaries.
- Keep CLI argument and execution code in `commands/`, outside the library pipeline.
