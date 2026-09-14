# Developer utility

- Workspace member invoked as `cargo xtask`; `.cargo/config.toml` defines the alias.
- `src/main.rs` — CLI argument definitions and subcommand dispatch.
- `src/check_prose.rs` — prose checks used by `script/ci`; patterns are documented in `context/ai-writing-patterns.md` at the repository root.
- Run `cargo xtask check-prose .` from the repository root; individual paths and JSON output are supported.
