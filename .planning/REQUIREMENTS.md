# Requirements: Upstream Terminology Rename

**Defined:** 2026-03-16
**Core Value:** Consistent, intuitive "upstream" terminology replacing "source repo" throughout the codebase

## v1 Requirements

Requirements for the complete rename. Each maps to roadmap phases.

### Code Rename

- [x] **CODE-01**: Rename struct fields, variable names, and function names that use "source_repo" or similar to "upstream_repo"
- [x] **CODE-02**: Update all code comments referencing "source repo" to use "upstream repo"
- [x] **CODE-03**: Rename "source-declared" operations terminology to "upstream-declared" in code
- [x] **CODE-04**: Rename "source filtering" to "upstream filtering" in code
- [x] **CODE-05**: Rename "source authors" to "upstream authors" in code
- [x] **CODE-06**: Rename "source_ops" / "source ops" references to "upstream_ops" / "upstream ops"
- [x] **CODE-07**: Preserve `source:` field in merge operators (yaml, json, toml, ini, markdown) — this refers to fragment file path, not the repository

### CLI Output

- [x] **CLI-01**: Update CLI help text to use "upstream" instead of "source repo"
- [x] **CLI-02**: Update user-facing output messages to use "upstream" terminology
- [x] **CLI-03**: Update error messages to use "upstream" terminology

### Config

- [x] **CONF-01**: Rename any config struct fields from "source" to "upstream" where they refer to the repository
- [x] **CONF-02**: Hard rename with no backwards compatibility (no deprecation warnings)

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
| CONF-01 | Phase 1 | Complete |
| CONF-02 | Phase 1 | Complete |
| CODE-01 | Phase 2 | Complete |
| CODE-03 | Phase 3 | Complete |
| CODE-04 | Phase 3 | Complete |
| CODE-05 | Phase 3 | Complete |
| CODE-06 | Phase 3 | Complete |
| CODE-07 | Phase 4 | Complete |
| CODE-02 | Phase 5 | Complete |
| CLI-01 | Phase 6 | Complete |
| CLI-02 | Phase 6 | Complete |
| CLI-03 | Phase 6 | Complete |
| TEST-01 | Phase 7 | Pending |
| TEST-02 | Phase 7 | Pending |
| TEST-03 | Phase 8 | Pending |
| TEST-04 | Phase 8 | Pending |

**Coverage:**
- v1 requirements: 16 total
- Mapped to phases: 16
- Unmapped: 0

---
*Requirements defined: 2026-03-16*
*Last updated: 2026-03-17 after roadmap creation*
