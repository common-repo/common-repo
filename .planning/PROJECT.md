# Upstream Terminology Rename

## What This Is

A refactoring of the common-repo codebase to replace "source repo" / "source repository" terminology with "upstream repo" / "upstream repository". This aligns the code with terminology that devops and automation engineers already know from git, package managers, and infrastructure pipelines. Documentation is being updated separately; this project covers the code changes.

## Core Value

Consistent, intuitive terminology throughout the codebase — "upstream" instead of "source" for the repository that provides shared files to consumers.

## Requirements

### Validated

<!-- Shipped and confirmed valuable. -->

- ✓ Existing common-repo CLI functionality — existing
- ✓ Provider/consumer repository model — existing
- ✓ Merge operators (yaml, json, toml) with `source:` field for fragment paths — existing

### Active

<!-- Current scope. Building toward these. -->

- [ ] Rename "source repo" → "upstream repo" in Rust struct names, variable names, and comments
- [ ] Update CLI help text and user-facing output to use "upstream" terminology
- [ ] Update error messages to use "upstream" terminology
- [ ] Rename config field names that reference "source repo" (hard rename, no backwards compat)
- [ ] Preserve `source:` field in merge operators (refers to fragment file path, not repository)
- [ ] Update "source-declared" → "upstream-declared" in operations terminology
- [ ] Update "source filtering" → "upstream filtering"
- [ ] Update "source authors" → "upstream authors"
- [ ] Update "provider" → "upstream" where it refers to the source repository role

### Out of Scope

- Merge operator `source:` field — refers to fragment file path, not the repository
- Documentation changes — handled separately before this work
- Backwards compatibility shims — hard rename, no deprecation path needed
- Renaming the provider/consumer model entirely — only "source" → "upstream" where it means the repository

## Context

- The codebase uses "source repo" and variations throughout to refer to the repository that provides shared files
- The `source:` field in merge operators (yaml, json, toml) is unrelated — it refers to the fragment file path within an operation, not the repository
- The provider/consumer relationship maps to upstream/consumer (or upstream/downstream)
- This is a brownfield Rust project with established conventions, tests, and CI

## Constraints

- **Backwards compatibility**: No backwards compat needed for config field renames (per user decision)
- **Merge operators**: The `source:` field in merge operators must NOT be renamed
- **Tests**: All existing tests must continue to pass after the rename
- **CI**: All CI checks (fmt, clippy, pre-commit, prose) must pass

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Hard rename (no deprecation) | Simplicity — no need to maintain backwards compat shims | — Pending |
| Merge operator `source:` stays | Refers to fragment file path, not repository | — Pending |
| "provider" → "upstream" where applicable | Aligns with the broader rename goal | — Pending |

---
*Last updated: 2026-03-16 after initialization*
