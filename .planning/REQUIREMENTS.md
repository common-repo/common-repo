# Requirements: Upstream Terminology Rename

**Defined:** 2026-03-16
**Core Value:** Consistent, intuitive "upstream" terminology replacing "source repo" throughout the codebase

## v1 Requirements

Requirements for the complete rename. Each maps to roadmap phases.

### Code Rename

- [ ] **CODE-01**: Rename struct fields, variable names, and function names that use "source_repo" or similar to "upstream_repo"
- [ ] **CODE-02**: Update all code comments referencing "source repo" to use "upstream repo"
- [ ] **CODE-03**: Rename "source-declared" operations terminology to "upstream-declared" in code
- [ ] **CODE-04**: Rename "source filtering" to "upstream filtering" in code
- [ ] **CODE-05**: Rename "source authors" to "upstream authors" in code
- [ ] **CODE-06**: Rename "source_ops" / "source ops" references to "upstream_ops" / "upstream ops"
- [ ] **CODE-07**: Preserve `source:` field in merge operators (yaml, json, toml, ini, markdown) — this refers to fragment file path, not the repository

### CLI Output

- [ ] **CLI-01**: Update CLI help text to use "upstream" instead of "source repo"
- [ ] **CLI-02**: Update user-facing output messages to use "upstream" terminology
- [ ] **CLI-03**: Update error messages to use "upstream" terminology

### Config

- [ ] **CONF-01**: Rename any config struct fields from "source" to "upstream" where they refer to the repository
- [ ] **CONF-02**: Hard rename with no backwards compatibility (no deprecation warnings)

### Tests

- [ ] **TEST-01**: Update test file names that reference "source" (e.g., `cli_e2e_source_ops.rs`)
- [ ] **TEST-02**: Update test assertions and string literals to match new terminology
- [ ] **TEST-03**: All existing tests pass after rename
- [ ] **TEST-04**: CI checks pass (fmt, clippy, pre-commit, prose)

## v2 Requirements

### Follow-up

- **FUP-01**: Update any external documentation references that link to source-related CLI output
- **FUP-02**: Consider renaming "provider/consumer" to "upstream/downstream" more broadly

## Out of Scope

| Feature | Reason |
|---------|--------|
| Merge operator `source:` field | Refers to fragment file path, not repository |
| Documentation updates | Handled separately before this work |
| Backwards compatibility shims | User decided on hard rename |
| Provider/consumer model rename | Only "source" → "upstream" where it means the repository |
| Schema/YAML config file field names | Only Rust code and CLI output |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| CODE-01 | — | Pending |
| CODE-02 | — | Pending |
| CODE-03 | — | Pending |
| CODE-04 | — | Pending |
| CODE-05 | — | Pending |
| CODE-06 | — | Pending |
| CODE-07 | — | Pending |
| CLI-01 | — | Pending |
| CLI-02 | — | Pending |
| CLI-03 | — | Pending |
| CONF-01 | — | Pending |
| CONF-02 | — | Pending |
| TEST-01 | — | Pending |
| TEST-02 | — | Pending |
| TEST-03 | — | Pending |
| TEST-04 | — | Pending |

**Coverage:**
- v1 requirements: 16 total
- Mapped to phases: 0
- Unmapped: 16

---
*Requirements defined: 2026-03-16*
*Last updated: 2026-03-16 after initial definition*
