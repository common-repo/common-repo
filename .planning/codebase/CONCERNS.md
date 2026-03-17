# Codebase Concerns

**Analysis Date:** 2026-03-16

## Tech Debt

**Deprecated Configuration Field:**
- Issue: `append` boolean field in merge operations remains for backward compatibility but is now superseded by `array_mode` enum
- Files: `src/config.rs:178-180`, `src/config.rs:332`
- Impact: Dual field handling creates maintenance burden; migration path not yet enforced
- Fix approach: (1) Deprecation warning when `append` is used, (2) Document migration guidance, (3) Remove in next major version

**Unimplemented Manifest Tracking:**
- Issue: TODO comment indicating desire to track managed files in a manifest, but feature not implemented
- Files: `src/commands/diff.rs:245`
- Impact: No way to identify which files are managed vs. user-created; manual tracking required
- Fix approach: Design and implement manifest file format (`.common-repo.manifest` or similar) to track which files were applied by common-repo

**Large Number of Clone Operations:**
- Issue: 1,748+ `clone()` calls throughout codebase, indicating potential performance and memory overhead
- Files: Distributed across `src/cli.rs`, `src/phases/`, `src/operators.rs`, `src/filesystem.rs`, `src/merge/` and others
- Impact: Unnecessary memory allocations, particularly in file processing loops and phase transitions
- Fix approach: Use references or Cow<T> for read-only operations; implement copy-on-write for filesystem operations

## Known Bugs

None explicitly documented in code comments beyond TODOs.

## Security Considerations

**Path Encoding Safety (Validated):**
- Risk: Cache directory names generated from repository URLs must not contain unsafe filesystem characters
- Files: `src/path.rs:130-150` (encode_url_path function)
- Current mitigation: encode_url_path() replaces problematic characters with `-` or `_`; property-tested with proptest in `src/path_proptest.rs`
- Recommendations: Current implementation is sound; proptest coverage ensures correctness across character space

**Command Execution via Git:**
- Risk: Git clone operations receive URL and ref from configuration files; malformed inputs could affect system behavior
- Files: `src/git.rs:48-98` (clone_shallow function)
- Current mitigation: Uses system `git` command with explicit arguments array (no shell injection risk); validates ref and URL in error messages
- Recommendations: Validate URL format before passing to git; consider blocking suspicious schemes (local file paths with `file://`)

**Configuration Parsing:**
- Risk: YAML configuration loaded from untrusted sources (.common-repo.yaml) is deserialized with serde
- Files: `src/config.rs` (parse function with backward compatibility fallback)
- Current mitigation: Schema validation in YamlMergeOp::validate(); error handling via Result<T> types
- Recommendations: (1) Add input size limits for YAML parsing (prevent DoS via large configs), (2) Validate all operation parameters before execution

**File Path Traversal:**
- Risk: Include/exclude/rename operations work with glob patterns and regex; potential for `../` escaping in paths
- Files: `src/operators.rs` (include, exclude, rename modules), `src/path.rs` (glob_match, regex_rename)
- Current mitigation: Filesystem operations use PathBuf canonicalization; glob and regex patterns processed as strings
- Recommendations: (1) Add explicit path traversal checks in include/exclude operations, (2) Ensure glob results are always within source directory bounds

**No Unsafe Code:**
- Risk: Codebase uses only safe Rust
- Current mitigation: No `unsafe` blocks found in source code
- Recommendations: Maintain this invariant; flag any future use of `unsafe` for security review

## Performance Bottlenecks

**Recursive Clone Operations in Phase 5:**
- Problem: MemoryFS cloned during local_merge phase (`.clone()` on entire filesystem)
- Files: `src/phases/local_merge.rs:47`
- Cause: Phase 5 clones composite_fs before applying local operations; large repositories will duplicate memory
- Improvement path: Implement copy-on-write filesystem or lazy loading; modify merge logic to work in-place with borrowing

**Unbounded File Enumeration in Local Directory:**
- Problem: load_local_fs() walks entire working directory into memory MemoryFS
- Files: `src/phases/local_merge.rs:67-100`
- Cause: No size limits on file enumeration or per-file size; large monorepos could exhaust memory
- Improvement path: (1) Add configurable max file count and max file size limits, (2) Implement streaming for large files, (3) Add progress indicators

**String Allocations in Template Processing:**
- Problem: String clones and allocations in template variable substitution
- Files: `src/merge/` (all merge implementations), template operator
- Cause: Multiple string transformations (YAML/JSON/TOML parsing, template rendering, merging)
- Improvement path: Use Cow<str> or string builders to minimize allocations; benchmark template processing with large files

**Glob Pattern Compilation:**
- Problem: Glob patterns compiled on each use without caching
- Files: `src/path.rs:57-59` (glob_match), `src/operators.rs` (include/exclude apply functions)
- Cause: Pattern::new() called for every glob_match invocation; no pattern caching layer
- Improvement path: Implement pattern cache with LRU eviction; pre-compile patterns during config parsing

**Semver Parsing on Every Tag:**
- Problem: Git tags parsed individually without caching results
- Files: `src/git.rs:373-395` (parse_semver_tag)
- Cause: Called for every tag returned from list_tags(); no caching of parsed versions
- Improvement path: Cache parsed semver results during version checking

## Fragile Areas

**Hash-Based Cache Keys:**
- Files: `src/phases/processing.rs:35-36` (DefaultHasher usage), `src/cache.rs` (CacheKey implementation)
- Why fragile: Hash-based cache invalidation relies on operation equality; changes to operation semantics could invalidate caches without detection
- Safe modification: (1) Document all Operation enum changes as requiring cache invalidation, (2) Consider version number in cache key, (3) Add test that verifies cache keys differ for semantically different operations
- Test coverage: Cache key generation tested but not semantics; add integration test for cache invalidation

**Template Variable Substitution:**
- Files: Operator implementations in `src/operators.rs`, merge modules in `src/merge/`
- Why fragile: Template vars collected across phases; variable name conflicts could silently override; no warning for undefined variables
- Safe modification: (1) Add validation pass to detect variable name conflicts, (2) Error on undefined template variables, (3) Document variable scoping rules
- Test coverage: Template operations have tests but not conflict scenarios

**Recursive Repository Processing (Phase 2):**
- Files: `src/phases/processing.rs` (recursive execution with in-process cache)
- Why fragile: Cycle detection handles direct cycles but not indirect ones; cache key computation is complex
- Safe modification: (1) Validate cycle detection against all known cyclic patterns, (2) Add explicit test for indirect cycles, (3) Document caching assumptions
- Test coverage: Basic cycle detection tested; indirect cycles not explicitly tested

**Merge Conflict Resolution:**
- Files: `src/merge/yaml.rs`, `src/merge/json.rs`, `src/merge/toml.rs`, `src/merge/ini.rs`, `src/merge/markdown.rs`
- Why fragile: Merge logic varies by file type; conflict handling strategies inconsistent (some silent, some error)
- Safe modification: (1) Unify conflict handling across merge implementations, (2) Document conflict semantics for each type, (3) Add tests covering common conflict scenarios
- Test coverage: Individual merge format tests exist; cross-format conflict scenarios lacking

## Scaling Limits

**In-Memory Repository Storage:**
- Current capacity: Entire filesystem stored in HashMap<PathBuf, File> with Vec<u8> contents
- Limit: Large monorepos (>10GB combined files) will exhaust available RAM; no streaming or paging
- Scaling path: (1) Implement file streaming for large files, (2) Use mmap for read-only access, (3) Add chunked processing for very large repos

**Configuration Complexity:**
- Current capacity: No documented limits on number of repos, operations, or nesting depth
- Limit: Deep repository inheritance chains could cause exponential processing time; large schemas cause slow parsing
- Scaling path: (1) Add limits to configuration validation (max repos, max nesting depth), (2) Profile schema parsing time, (3) Implement lazy loading of inherited repos

**Cache Directory Size:**
- Current capacity: Cache stores full shallow clones; no size management
- Limit: Multiple versions and branches of large repos will consume disk space without bounds
- Scaling path: (1) Implement cache eviction policy (LRU), (2) Add cache size limits with warnings, (3) Provide cache cleanup utilities

## Dependencies at Risk

**No Critical Supply Chain Risks Detected:**
- All dependencies are well-maintained (e.g., serde, clap, anyhow)
- Minimal unsafe code in dependencies (via dependency audit)
- Recommendation: Continue regular dependency updates via `cargo update`

**Libyaml-based YAML Parsing:**
- Risk: `unsafe-libyaml` in Cargo.lock (1943:0.9.34+deprecated) used by serde_yaml
- Impact: Small amount of unsafe code in transitive dependency; marked as deprecated version
- Migration plan: Monitor serde_yaml releases for switch to pure-Rust YAML parser; update when available

## Missing Critical Features

**No Dry-Run Validation:**
- Problem: No way to safely preview changes before applying; users must trust configuration
- Blocks: High-risk operations in production repos; no confidence in schema correctness
- Workaround: Use `diff` command to see changes, but doesn't validate merge conflicts

**No Version Pinning in Configuration:**
- Problem: `ref:` field accepts any git ref; no way to pin to exact commit hash or tag
- Blocks: Deterministic builds; repository definitions are not fully reproducible
- Workaround: Use specific tag names, but tag deletion/retagging is not prevented

**No Rollback Mechanism:**
- Problem: Applied changes are not reversible; no backup of original files before apply
- Blocks: Safe trial-and-error iteration on configuration; schema bugs cause permanent file loss
- Workaround: Manual git restore, but this is slow and error-prone

**No Concurrent Repository Cloning:**
- Problem: Despite rayon dependency, repositories are cloned sequentially
- Blocks: Slow initial apply with many repos; inefficient use of network bandwidth
- Workaround: None; must wait for sequential clone completion

## Test Coverage Gaps

**Phase Integration Tests:**
- What's not tested: End-to-end interaction of all 5 phases with real repository; only unit tests per phase
- Files: `tests/integration_test.rs` exists but tests individual operations, not full pipeline
- Risk: Phase boundaries and data flow between phases could have bugs not caught by unit tests
- Priority: High

**Merge Conflict Scenarios:**
- What's not tested: Conflicting changes from multiple inherited repos; YAML/JSON/TOML structure conflicts
- Files: `tests/cli_e2e_yaml_merge.rs`, `tests/integration_merge_yaml.rs` test merges but not conflict states
- Risk: Merge conflicts silently resolve or overwrite without warning
- Priority: High

**Error Path Recovery:**
- What's not tested: Graceful handling of network failures, disk full, permission errors during apply
- Files: Error types defined but not exercised in tests
- Risk: Partial state on disk if operation fails mid-way
- Priority: Medium

**Large Monorepo Scenarios:**
- What's not tested: Performance and correctness with repos containing >1000 files, >100MB total size
- Files: Test fixtures are small (< 10 files)
- Risk: Unexpected behavior or OOM in production with real-world repos
- Priority: Medium

**Template Variable Edge Cases:**
- What's not tested: Undefined variables, circular variable references, nested variable substitution
- Files: Template operator tests basic substitution only
- Risk: Silent failures or unexpected output with complex variable usage
- Priority: Medium

**Cycle Detection Edge Cases:**
- What's not tested: Indirect cycles (A→B→C→A), complex inheritance patterns, self-referential repos
- Files: `src/repository.rs` has cycle detection but tests only direct cycles
- Risk: Stack overflow or incorrect ordering with indirect cycles
- Priority: High

**Windows Path Handling:**
- What's not tested: Backslash path separators, UNC paths, reserved filenames (CON, PRN, etc.)
- Files: Path tests use forward slashes; no Windows-specific tests
- Risk: Cross-platform incompatibilities in production on Windows
- Priority: Medium

---

*Concerns audit: 2026-03-16*
