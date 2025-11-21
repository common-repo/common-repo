# Test Coverage Improvement Plan

## Current State

**Overall Coverage**: 65.19% (1028/1577 lines covered)

Generated: 2025-11-13
Tool: cargo-tarpaulin with `--features integration-tests`

## Coverage by Module

### ✅ Excellent Coverage (90-100%)
- `src/filesystem.rs`: **100%** (61/61)
- `src/path.rs`: **100%** (39/39)
- `src/main.rs`: **100%** (3/3)

### ✅ Good Coverage (80-90%)
- `src/config.rs`: **87.7%** (114/130)
- `src/operators.rs`: **88.0%** (125/142)
- `src/repository.rs`: **86.7%** (39/45)
- `src/commands/apply.rs`: **85.1%** (40/47)
- `src/cache.rs`: **81.4%** (35/43)

### ⚠️ Moderate Coverage (50-80%)
- `src/git.rs`: **73.0%** (84/115) - 31 uncovered lines
- `src/phases.rs`: **60.6%** (444/733) - **289 uncovered lines**
- `src/version.rs`: **55.4%** (41/74) - 33 uncovered lines
- `src/cli.rs`: **60.0%** (3/5) - 2 uncovered lines

### ❌ Low Coverage (<50%)
- `src/commands/check.rs`: **0%** (0/60) - **60 uncovered lines**
- `src/commands/update.rs`: **0%** (0/80) - **80 uncovered lines**

## Priority Areas for Improvement

### Priority 1: Critical Command Modules (0% Coverage)

#### 1.1 `src/commands/check.rs` (60 uncovered lines)
**Current**: 56.67% coverage (34/60 lines) - **IN PROGRESS** ✅
**Target**: 80%+
**Effort**: Medium
**Impact**: High - Core functionality

**Progress:**
- ✅ Created `tests/cli_e2e_check.rs` with 15 comprehensive tests
- ✅ Coverage improved from 0% → 56.67%
- ✅ All basic functionality paths covered (config validation, error handling, basic --updates)

**Remaining Uncovered Areas (lines 93-95, 106-141):**
- Lines 93-95: Update flag detection logic (breaking_changes, compatible_updates)
- Lines 106-113: Update summary display ("X repositories checked, Y have updates")
- Lines 116-141: Update detail display (repo URL, version info, breaking change warnings)

**Why Not Higher Coverage?**
These lines only execute when `--updates` finds actual version updates. Integration tests hit real repositories where current state is unpredictable.

**To Reach 80%+ Coverage:**
Need to create semantic version tags (v0.0.1, v0.1.0, v0.2.0) on commits that contain appropriate test fixtures in testdata directories. This requires:
1. Creating testdata/.common-repo.yaml fixtures
2. Committing and tagging with semantic versions
3. Creating test configs that reference old tags (e.g., v0.0.1)
4. Tests will then reliably find updates (v0.1.0, v0.2.0)

**Note:** This should be done as a separate task after the repository has stable test fixtures in place.

**Tests Implemented:** 15 tests (target was 8-10) ✅

---

#### 1.2 `src/commands/update.rs` (80 uncovered lines)
**Current**: 83.75% coverage (67/80 lines) - **COMPLETED** ✅
**Target**: 80%+
**Effort**: High
**Impact**: High - Core functionality

**Progress:**
- ✅ Created 18 comprehensive E2E tests in `tests/cli_e2e_update.rs`
- ✅ Coverage improved from 36.25% → 83.75% (+47.50%)
- ✅ All major functionality paths covered (update display, --yes flag, file modification, dry-run)
- ✅ Tests verify actual config file updates with --yes flag
- ✅ Breaking change detection with --latest flag tested
- ✅ Compatible update filtering with --compatible flag tested

**Remaining Uncovered Areas (lines 147, 163-170, 197-199, 211, 227):**
- Line 147: Breaking change warning display (edge case)
- Lines 163-170: Interactive confirmation prompt (stdin.read_line - requires mocking)
- Lines 197-199: File write error handling (error path)
- Line 211: "No repositories were updated" message (edge case)
- Line 227: End of update_repo_ref function (unreachable)

**Why Not Higher Coverage?**
Remaining lines are mostly:
1. Interactive stdin prompt (lines 163-170) - requires stdin mocking which is complex in integration tests
2. Error handling paths (197-199) - would need to simulate filesystem errors
3. Edge cases that are hard to trigger reliably

**Tests Implemented:** 18 tests (exceeded target of 12-15) ✅

**Key Tests Added:**
- `test_update_shows_available_updates` - Tests update display when updates exist
- `test_update_with_yes_flag` - Tests --yes flag to bypass interactive prompt
- `test_update_modifies_config_file` - Tests actual file modification
- `test_update_shows_breaking_changes_with_latest` - Tests --latest flag
- `test_update_compatible_only_filtering` - Tests --compatible filtering

---

### Priority 2: Complex Modules with Large Uncovered Sections

#### 2.1 `src/phases.rs` (289 uncovered lines)
**Current**: 60.6% coverage
**Target**: 80%+
**Effort**: High
**Impact**: High - Core pipeline logic

**Uncovered Areas by Phase:**

**Phase 1 (Discovery & Cloning):**
- Lines 175, 217, 240, 253: Error handling in discovery
- Lines 271, 273, 275, 277-278: Cycle detection edge cases
- Lines 332-333: Config parsing errors
- Lines 388-389, 391, 393-395, 400: Network fallback logic

**Phase 2 (Processing):**
- Lines 589, 603, 608-609: Cache key generation
- Lines 640, 643, 645, 650, 652, 654-655: Operation fingerprinting
- Lines 657-658, 660-661, 663-664, 666-667: Serialization edge cases

**Phase 5 (Merge Operations):**
This is the largest gap - extensive merge operator logic not covered:
- Lines 1382-1383, 1425: Merge configuration parsing
- Lines 1661, 1670: File path handling
- Lines 1706, 1713, 1726-1727, 1729-1731: YAML merge operations
- Lines 1740-1741, 1745, 1751, 1789: JSON merge operations
- Lines 1861-2015: TOML merge operations (large block)
- Lines 2017-2088: INI merge operations (large block)
- Lines 2107-2163: Markdown merge operations (large block)
- Lines 2197-2262: Additional merge logic
- Lines 2273-2274, 2282-2283: Error paths
- Lines 2303-2304, 2311, 2361: Edge case handling
- Lines 2365-2379: Merge operator application
- Lines 2394-2395, 2397, 2401, 2410: Output handling

**Phase 6 (Write to Disk):**
- Lines 2438-2452: Permission handling edge cases
- Lines 2508, 2532, 2538, 2549-2553: Error paths

**Testing Strategy:**
- Add comprehensive tests for each merge operator type:
  - YAML merge: nested objects, arrays, overwrite modes
  - JSON merge: deep merging, type conflicts
  - TOML merge: section merging, array handling
  - INI merge: section creation, key-value updates
  - Markdown merge: header levels, content insertion modes
- Test error handling in each phase
- Test edge cases in cycle detection
- Test network failure fallback scenarios
- Test cache key generation with complex operations

**Estimated Tests Needed:** 25-30 tests

---

#### 2.2 `src/version.rs` (33 uncovered lines)
**Current**: 55.4% coverage
**Target**: 85%+
**Effort**: Low-Medium
**Impact**: Medium - Update functionality

**Uncovered Areas:**
- Lines 66-67, 70, 72-74: Version extraction from refs
- Lines 77, 81-82, 85, 88, 91: Version comparison logic
- Lines 93-95, 97-98, 100-101: Error handling
- Lines 104, 107-108, 110, 113, 116: Tag filtering
- Lines 125-131: Repository collection logic
- Line 150: Edge case handling

**Testing Strategy:**
- Test version extraction from various git ref formats
- Test semver comparison (major, minor, patch)
- Test non-semver ref handling
- Test version filtering with different patterns
- Test repository dependency collection
- Test error cases (invalid versions, missing tags)

**Estimated Tests Needed:** 8-10 tests

---

#### 2.3 `src/git.rs` (31 uncovered lines)
**Current**: 73.0% coverage
**Target**: 85%+
**Effort**: Medium
**Impact**: High - Core git operations

**Uncovered Areas:**
- Line 53: Directory removal errors
- Lines 67-69, 73: Git clone command failures
- Lines 76-78, 80, 90, 93-96: Authentication error handling
- Line 152: Cache loading errors
- Line 230: Path filtering errors
- Lines 311-313, 317-321: Tag listing command failures
- Lines 331-336, 338: Tag parsing and error handling

**Testing Strategy:**
- Mock git command failures
- Test authentication error messages
- Test network timeout scenarios
- Test invalid repository URLs
- Test tag listing with various formats
- Test path filtering edge cases
- Use temporary directories to simulate real git operations

**Estimated Tests Needed:** 10-12 tests

---

### Priority 3: Minor Gaps

#### 3.1 `src/cli.rs` (2 uncovered lines)
**Current**: 60.0% coverage
**Target**: 100%
**Effort**: Minimal
**Impact**: Low

**Uncovered**: Lines 69-70
**Testing Strategy:** Add CLI integration test for error paths

**Estimated Tests Needed:** 1-2 tests

---

#### 3.2 `src/config.rs` (16 uncovered lines)
**Current**: 87.7% coverage
**Target**: 95%+
**Effort**: Low
**Impact**: Medium

**Uncovered Areas:**
- Lines 295-296, 308, 312: Error handling
- Lines 322-323, 332: Edge cases in parsing
- Lines 357-358, 365-366: File reading errors
- Lines 404-405, 418: Validation errors
- Lines 448, 463: Additional edge cases

**Testing Strategy:**
- Test malformed YAML configurations
- Test file I/O errors
- Test edge cases in operation parsing
- Test validation failures

**Estimated Tests Needed:** 5-6 tests

---

#### 3.3 `src/operators.rs` (17 uncovered lines)
**Current**: 88.0% coverage
**Target**: 95%+
**Effort**: Low
**Impact**: Medium

**Uncovered Areas:**
- Lines 1267-1269, 1302: Template processing errors
- Lines 1553-1555, 1565-1566: Operator application errors
- Lines 1571-1573, 1580-1584: Edge cases

**Testing Strategy:**
- Test template variable edge cases
- Test operator errors
- Test complex template scenarios

**Estimated Tests Needed:** 4-5 tests

---

#### 3.4 `src/commands/apply.rs` (7 uncovered lines)
**Current**: 85.1% coverage
**Target**: 95%+
**Effort**: Low
**Impact**: Low

**Uncovered**: Lines 145, 162, 169-172, 174
**Testing Strategy:** Add error path tests for apply command

**Estimated Tests Needed:** 2-3 tests

---

#### 3.5 `src/repository.rs` (6 uncovered lines)
**Current**: 86.7% coverage
**Target**: 95%+
**Effort**: Minimal
**Impact**: Low

**Uncovered**: Lines 116-117, 124-125, 132-133
**Testing Strategy:** Add edge case tests for repository operations

**Estimated Tests Needed:** 2-3 tests

---

#### 3.6 `src/cache.rs` (8 uncovered lines)
**Current**: 81.4% coverage
**Target**: 95%+
**Effort**: Minimal
**Impact**: Low

**Uncovered**: Lines 108, 121, 134, 144, 152, 160, 169, 177
**Testing Strategy:** Test error handling in cache operations (lock poisoning, etc.)

**Estimated Tests Needed:** 2-3 tests

---

## Implementation Roadmap

### Phase 1: Critical Command Coverage (Week 1-2)
**Target**: Bring command modules from 0% to 80%+

1. ✅ **Day 1-3**: Implement `check` command tests (8-10 tests)
   - Set up test fixtures for version checking
   - Test CLI integration
   - Test error scenarios

2. ✅ **Day 4-7**: Implement `update` command tests (12-15 tests)
   - Set up interactive test framework
   - Test dry-run mode
   - Test actual update operations
   - Test error handling

**Expected Coverage After Phase 1**: ~70% overall

---

### Phase 2: Merge Operators Coverage (Week 3-4)
**Target**: Improve `phases.rs` from 60.6% to 80%+

1. ✅ **Day 1-2**: YAML and JSON merge operator tests (8-10 tests)
2. ✅ **Day 3-4**: TOML merge operator tests (6-8 tests)
3. ✅ **Day 5-6**: INI and Markdown merge operator tests (8-10 tests)
4. ✅ **Day 7**: Phase error handling tests (3-4 tests)

**Expected Coverage After Phase 2**: ~75% overall

---

### Phase 3: Git and Version Coverage (Week 5)
**Target**: Improve `git.rs` to 85%+ and `version.rs` to 85%+

1. ✅ **Day 1-2**: Git error handling tests (10-12 tests)
2. ✅ **Day 3-4**: Version comparison and filtering tests (8-10 tests)

**Expected Coverage After Phase 3**: ~80% overall

---

### Phase 4: Minor Gaps Cleanup (Week 6)
**Target**: Improve remaining modules to 95%+

1. ✅ **Day 1**: Config edge cases (5-6 tests)
2. ✅ **Day 2**: Operator edge cases (4-5 tests)
3. ✅ **Day 3**: Cache, repository, CLI edge cases (6-8 tests)

**Expected Coverage After Phase 4**: ~85% overall

---

## Success Metrics

### Target Coverage Goals
- **Overall Project**: 85%+ (currently 65.19%)
- **Critical Modules**: 90%+ (commands, phases, git)
- **Core Library**: 95%+ (config, operators, filesystem)

### Test Suite Goals
- **Current**: 228 tests (3.4s runtime with integration tests)
- **Target**: ~320 tests
- **Estimated Runtime**: 5-6s (with integration tests)

### Quality Metrics
- All new tests must pass CI
- No decrease in existing coverage
- Test runtime increase <50%
- All tests must be maintainable and well-documented

---

## Testing Best Practices

1. **Use descriptive test names**: `test_module_function_scenario_expected_behavior()`
2. **Follow AAA pattern**: Arrange, Act, Assert
3. **Use fixtures and helpers** to reduce duplication
4. **Test both happy and error paths**
5. **Use property-based testing** for complex scenarios where applicable
6. **Mock external dependencies** (git, network) for unit tests
7. **Use integration tests** for end-to-end validation
8. **Document test intent** with clear comments
9. **Keep tests focused** - one behavior per test
10. **Maintain test isolation** - no shared mutable state

---

## Notes

- Integration tests are gated behind `--features integration-tests`
- Use `cargo tarpaulin --features integration-tests` to measure coverage
- Current test suite split: 205 unit tests, 23 integration tests
- Priority should be on testing error paths and edge cases
- Merge operators need the most attention (largest gap)
- Command modules need tests from scratch

---

## Tracking Progress

Update this document as coverage improves:

| Module | Current | Target | Status |
|--------|---------|--------|--------|
| commands/check.rs | 90.0% (54/60) | 80% | 🟢 **COMPLETE** (+15 tests) |
| commands/update.rs | 83.75% (67/80) | 80% | 🟢 **COMPLETE** (+18 tests) |
| phases.rs | 75.72% (680/898) | 80% | 🟡 In Progress (218 lines remaining) |
| version.rs | 94.59% (70/74) | 85% | 🟢 Exceeds Target |
| git.rs | 87.18% (102/117) | 85% | 🟢 Exceeds Target |
| config.rs | 97.14% (136/140) | 95% | 🟢 Exceeds Target |
| operators.rs | 94.23% (147/156) | 95% | 🟡 Near Target (9 lines) |
| cli.rs | 100% (5/5) | 95% | 🟢 Perfect |
| repository.rs | 100% (45/45) | 95% | 🟢 Perfect |
| **Overall** | **84.62%** (1496/1768) | **85%** | **🟡 Nearly There!** (-0.38%) |

---

Last Updated: 2025-11-20 (Phase 1 commands complete: check.rs 90%, update.rs 83.75%, overall 84.62%)
