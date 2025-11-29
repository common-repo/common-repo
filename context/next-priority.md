# Next Priority: `common-repo diff` Command

## Summary

Implement the `common-repo diff` command to preview changes without applying them. This is the last remaining CLI command from the implementation plan.

## Rationale

- All other CLI commands are complete: `apply`, `check`, `update`, `validate`, `init`, `cache`, `info`, `tree`, `ls`
- The `diff` command enables safe preview workflows before running `apply`
- Essential for CI/CD pipelines and cautious users who want to review changes first

## Acceptance Criteria

1. Command exists and is accessible via `common-repo diff`
2. Shows files that would be added, modified, or deleted
3. Supports `--config` flag for custom config path
4. Supports `--working-dir` flag for specifying comparison target
5. Output is human-readable with clear indicators for change types
6. Returns appropriate exit codes (0 for no changes, 1 for changes exist)
7. Has comprehensive E2E test coverage in `tests/cli_e2e_diff.rs`
8. All existing tests continue to pass

## Implementation References

- Plan: [Layer 4 - CLI & Orchestration](implementation-plan.md#layer-4-cli--orchestration-depends-on-all-layers)
- Design: [CLI Design](design.md#cli-design)
- Similar command: `src/commands/ls.rs` (file listing with pattern filtering)
- Similar command: `src/commands/check.rs` (configuration validation)

## Suggested Approach

1. Create `src/commands/diff.rs` following the pattern of existing commands
2. Execute phases 1-4 to build the composite filesystem
3. Compare composite filesystem against working directory
4. Categorize changes: added, modified, deleted
5. Format output with clear visual indicators
6. Add command to `src/cli.rs` and `src/commands/mod.rs`
7. Write E2E tests in `tests/cli_e2e_diff.rs`
