# Test Coverage Improvement Plan

## Current State

Run `cargo tarpaulin --features integration-tests` to get current coverage metrics.

## Goal

**Target: 90%+ overall line coverage**

## Priority Areas for Improvement

### Priority 1: Critical Command Modules

#### 1.1 `src/commands/check.rs`
**Status**: ✅ **COMPLETE** - Exceeds target
**Tests**: 15 E2E tests in `tests/cli_e2e_check.rs`

**What Was Covered:**
- Config validation and error handling
- Repository counting and operation display
- Help flags and environment variables
- Basic --updates functionality

**Remaining Work:**
- Update display paths that only execute when updates are actually found
- Requires semantic version tags on test fixtures for reliable testing

---

#### 1.2 `src/commands/update.rs`
**Status**: ✅ **COMPLETE** - Exceeds target
**Tests**: 18 E2E tests in `tests/cli_e2e_update.rs`

**What Was Covered:**
- Update display when updates are found
- --yes flag to bypass interactive prompts
- Actual config file modification
- Breaking change detection with --latest flag
- Compatible update filtering with --compatible flag
- Dry-run mode verification

**Remaining Work:**
- Interactive stdin prompt paths (requires stdin mocking)
- Filesystem error handling (requires error injection)
- Edge cases difficult to trigger in integration tests

---

### Priority 2: Complex Modules

#### 2.1 `src/phases.rs`
**Status**: 🟡 **IN PROGRESS** - Largest remaining gap
**Target**: 90%+

**Key Areas Needing Coverage:**

**Phase 1 (Discovery & Cloning):**
- Error handling in discovery
- Cycle detection edge cases
- Config parsing errors
- Network fallback logic

**Phase 2 (Processing):**
- Cache key generation
- Operation fingerprinting
- Serialization edge cases

**Phase 5 (Merge Operations):**
Major gap - extensive merge operator logic:
- YAML merge operations (nested objects, arrays, overwrite modes)
- JSON merge operations (deep merging, type conflicts)
- TOML merge operations (section merging, array handling)
- INI merge operations (section creation, key-value updates)
- Markdown merge operations (header levels, content insertion modes)
- Merge configuration parsing
- File path handling
- Error paths and edge cases

**Phase 6 (Write to Disk):**
- Permission handling edge cases
- Error paths

**Testing Strategy:**
- Comprehensive tests for each merge operator type
- Error handling in each phase
- Edge cases in cycle detection
- Network failure fallback scenarios
- Cache key generation with complex operations

---

#### 2.2 `src/version.rs`
**Status**: ✅ **COMPLETE** - Exceeds target

**What Was Covered:**
- Version extraction from git ref formats
- Semver comparison (major, minor, patch)
- Non-semver ref handling
- Version filtering
- Repository dependency collection
- Error cases (invalid versions, missing tags)

---

#### 2.3 `src/git.rs`
**Status**: ✅ **COMPLETE** - Exceeds target

**What Was Covered:**
- Directory removal errors
- Git clone command failures
- Authentication error handling
- Cache loading
- Path filtering
- Tag listing with various formats
- Tag parsing and error handling

---

### Priority 3: Additional Modules

#### 3.1 `src/cli.rs`
**Status**: ✅ **COMPLETE** - Perfect coverage

---

#### 3.2 `src/config.rs`
**Status**: ✅ **COMPLETE** - Exceeds target

**What Was Covered:**
- Malformed YAML configurations
- File I/O errors
- Edge cases in operation parsing
- Validation failures

---

#### 3.3 `src/operators.rs`
**Status**: ✅ **COMPLETE** - Near perfect

**Progress:**
- Added 2 tests for tool validation error paths
- All error paths covered except one untestable line (hardcoded regex pattern)

**Tests Added:**
- `test_check_tool_nonzero_exit`: Tool exit code handling
- `test_check_tool_actual_version_mismatch`: Version requirement mismatch

---

#### 3.4 `src/commands/apply.rs`
**Status**: ✅ **COMPLETE** - Exceeds target

---

#### 3.5 `src/repository.rs`
**Status**: ✅ **COMPLETE** - Perfect coverage

---

#### 3.6 `src/cache.rs`
**Status**: ✅ **COMPLETE** - Exceeds target

---

## Implementation Progress

### ✅ Phase 1: Critical Command Modules (COMPLETE)
- `check` command: 15 E2E tests
- `update` command: 18 E2E tests
- Both commands exceed target coverage

### ✅ Phase 2: Core Library Modules (COMPLETE)
- `operators.rs`: Near-perfect coverage
- `config.rs`: Exceeds target
- `git.rs`: Exceeds target
- `version.rs`: Exceeds target
- `cache.rs`: Exceeds target
- `repository.rs`: Perfect coverage
- `cli.rs`: Perfect coverage

### 🟡 Phase 3: Complex Pipeline (IN PROGRESS)
- `phases.rs`: Largest remaining gap
- Focus on merge operator coverage (YAML, JSON, TOML, INI, Markdown)

---

## Success Metrics

### Target Coverage Goals
- **Overall Project**: 90%+
- **All Modules**: 90%+

### Quality Metrics
- All tests must pass CI
- No decrease in existing coverage
- All tests must be maintainable and well-documented
- Comprehensive error path coverage

---

## Testing Best Practices

1. **Use descriptive test names**: `test_module_function_scenario_expected_behavior()`
2. **Follow AAA pattern**: Arrange, Act, Assert
3. **Use fixtures and helpers** to reduce duplication
4. **Test both happy and error paths**
5. **Mock external dependencies** (git, network) for unit tests
6. **Use integration tests** for end-to-end validation
7. **Document test intent** with clear comments
8. **Keep tests focused** - one behavior per test
9. **Maintain test isolation** - no shared mutable state

---

## Notes

- Integration tests are gated behind `--features integration-tests`
- Use `cargo tarpaulin --features integration-tests` to measure coverage
- Priority: error paths, edge cases, and merge operators in phases.rs

---

## Tracking Progress

Run `cargo tarpaulin --features integration-tests` to check current coverage.

| Module | Target | Status |
|--------|--------|--------|
| commands/check.rs | 90%+ | ✅ Exceeds Target |
| commands/update.rs | 90%+ | 🟡 Close to Target |
| operators.rs | 90%+ | ✅ Near Perfect |
| config.rs | 90%+ | ✅ Exceeds Target |
| version.rs | 90%+ | ✅ Exceeds Target |
| git.rs | 90%+ | 🟡 Close to Target |
| cli.rs | 90%+ | ✅ Perfect |
| repository.rs | 90%+ | ✅ Perfect |
| cache.rs | 90%+ | ✅ Exceeds Target |
| phases.rs | 90%+ | 🔴 **NEEDS WORK** - Largest gap |
| **Overall** | **90%+** | **🟡 IN PROGRESS** |

**Key**: ✅ Complete | 🟡 Close/In Progress | 🔴 Needs Work

---

Last Updated: 2025-11-20 (Target raised to 90%)
