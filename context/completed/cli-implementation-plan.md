# CLI Implementation Plan

## Current Implementation State

Based on the analysis of the codebase, the **entire 6-phase pipeline is implemented**:

✅ **Phase 1: Discovery & Cloning** - `phases::discover_repos()`, `phases::clone_parallel()`
✅ **Phase 2: Processing Repos** - `phases::IntermediateResult`, process individual repos
✅ **Phase 3: Operation Order** - `phases::OperationOrder::execute()`
✅ **Phase 4: Composite Filesystem** - `phases::Phase4::execute()`
✅ **Phase 5: Local Merging** - `phases::Phase5::execute()`
✅ **Phase 6: Writing to Disk** - `phases::Phase6::execute()`

### Core Library Components Status

| Module | Status | Functionality |
|--------|--------|---------------|
| `config.rs` | ✅ Complete | YAML parsing, all operation types |
| `filesystem.rs` | ✅ Complete | In-memory FS, glob patterns, merging |
| `git.rs` | ✅ Complete | Clone, cache load/save, list tags, semver parsing |
| `cache.rs` | ✅ Complete | In-process caching (thread-safe) |
| `repository.rs` | ✅ Complete | RepositoryManager, fetch with cache |
| `operators.rs` | ✅ Complete | Include, Exclude, Rename, Repo, Template |
| `phases.rs` | ✅ Complete | All 6 phases + merge operations |
| `path.rs` | ✅ Complete | Glob matching, regex rename |
| `error.rs` | ✅ Complete | Error types |

---

## CLI Commands: Implementation Readiness

### Tier 1: Ready to Implement (Core library complete)

These commands can be implemented immediately with the existing library code.

#### 1. `common-repo apply` ⭐ **HIGHEST PRIORITY**

**Status**: 🟢 **READY** - All backend code exists

**What's needed**:
- CLI argument parsing (clap)
- Wire up `phases::Phase1` through `phases::Phase6`
- Progress output/logging
- Error handling and user-friendly messages

**Estimated effort**: 4-6 hours

**Implementation checklist**:
- [ ] Add `clap` dependency for CLI parsing
- [ ] Create `src/cli.rs` with command structure
- [ ] Create `src/commands/apply.rs`
- [ ] Wire up phases sequentially
- [ ] Add progress indicators (use `indicatif` crate)
- [ ] Add `--dry-run` flag (skip Phase 6)
- [ ] Add `--verbose` flag for detailed output
- [ ] Add `--no-cache` flag (use `fetch_repository_fresh`)
- [ ] Error handling with user-friendly messages
- [ ] Tests for CLI integration

**Dependencies**: None (all backend ready)

---

#### 2. `common-repo validate`

**Status**: 🟢 **READY** - Config parsing complete

**What's needed**:
- CLI argument parsing
- Call `config::parse()` or `config::from_file()`
- Add additional validations:
  - Check for circular dependencies (use `phases::discover_repos()`)
  - Validate regex patterns in rename operations
  - Validate glob patterns
- Format validation results

**Estimated effort**: 2-3 hours

**Implementation checklist**:
- [ ] Create `src/commands/validate.rs`
- [ ] Parse config file with `config::from_file()`
- [ ] Check for circular dependencies
- [ ] Validate regex patterns (compile them)
- [ ] Validate glob patterns
- [ ] Format output (✓ or ✗ with details)
- [ ] Add `--strict` flag
- [ ] Add `--json` output format
- [ ] Tests

**Dependencies**: None

---

#### 3. `common-repo init` (minimal version)

**Status**: 🟡 **PARTIALLY READY** - Can create minimal config

**What's needed**:
- Generate minimal `.common-repo.yaml` template
- Interactive mode needs external dependencies
- Template system not yet implemented

**Estimated effort**: 2-3 hours (minimal), 8-10 hours (with interactive)

**Implementation checklist** (minimal version):
- [ ] Create `src/commands/init.rs`
- [ ] Create minimal template content
- [ ] Check if `.common-repo.yaml` exists
- [ ] Write template to file
- [ ] Add `--force` flag to overwrite
- [ ] Add `--empty` flag for empty config
- [ ] Tests

**Implementation checklist** (interactive - defer to later):
- [ ] Add `dialoguer` crate for interactive prompts
- [ ] Create template registry
- [ ] Implement interactive wizard
- [ ] Add `--template` flag

**Dependencies**: None for minimal version

---

### Tier 2: Requires Moderate Additional Work

These commands need some additional logic but have most functionality ready.

#### 4. `common-repo tree`

**Status**: 🟡 **PARTIALLY READY** - Tree structure exists

**What's needed**:
- Visualization of `RepoTree` structure
- Tree formatting library

**Estimated effort**: 3-4 hours

**Implementation checklist**:
- [ ] Create `src/commands/tree.rs`
- [ ] Add `ptree` or `termtree` crate for tree visualization
- [ ] Parse config
- [ ] Call `phases::discover_repos()`
- [ ] Traverse `RepoTree` structure
- [ ] Format as tree with URLs and refs
- [ ] Add `--depth` flag
- [ ] Add colors for different levels
- [ ] Tests

**Dependencies**: Needs tree visualization library

**Blocked by**: None

---

#### 5. `common-repo info`

**Status**: 🟡 **PARTIALLY READY** - Tag listing works

**What's needed**:
- Aggregate information display
- Parse operations from config

**Estimated effort**: 3-4 hours

**Implementation checklist**:
- [ ] Create `src/commands/info.rs`
- [ ] Info about current config:
  - [ ] Parse config with `config::from_file()`
  - [ ] Count operations by type
  - [ ] Show inherited repos
  - [ ] Show cache status (use `repository::is_cached()`)
- [ ] Info about specific repo:
  - [ ] List tags with `repository::list_repository_tags()`
  - [ ] Show latest version (semver sort)
  - [ ] Fetch and parse config from repo
  - [ ] Show operations defined
- [ ] Add `--tags` flag
- [ ] Add `--operations` flag
- [ ] Add `--json` output
- [ ] Tests

**Dependencies**: None

**Blocked by**: None

---

#### 6. `common-repo ls`

**Status**: 🟡 **PARTIALLY READY** - MemoryFS has file listing

**What's needed**:
- Run phases up to Phase 4 (composite filesystem)
- Display file list with sources

**Estimated effort**: 3-4 hours

**Implementation checklist**:
- [ ] Create `src/commands/ls.rs`
- [ ] Parse config
- [ ] Run Phase 1-4 (stop before writing to disk)
- [ ] List files from composite filesystem
- [ ] Show source repo for each file (track in Phase 4)
- [ ] Add `--tree` flag
- [ ] Add `--source` flag to show sources
- [ ] Add `--conflicts` flag (files from multiple sources)
- [ ] Tests

**Dependencies**: Need to enhance Phase 4 to track file sources

**Blocked by**: File source tracking enhancement

---

#### 7. `common-repo cache list`

**Status**: 🟡 **PARTIALLY READY** - Cache structure known

**What's needed**:
- Walk cache directory
- Parse cache directory names
- Calculate sizes

**Estimated effort**: 2-3 hours

**Implementation checklist**:
- [ ] Create `src/commands/cache.rs`
- [ ] Get cache root (default: `~/.common-repo/cache/`)
- [ ] Walk cache directory with `walkdir`
- [ ] Parse directory names (reverse `url_to_cache_path()`)
- [ ] Calculate directory sizes
- [ ] Get last accessed time (file metadata)
- [ ] Format output (table format)
- [ ] Add `--verbose` flag
- [ ] Add `--json` output
- [ ] Tests

**Dependencies**: None

**Blocked by**: None

---

#### 8. `common-repo cache clean`

**Status**: 🟡 **PARTIALLY READY** - Cache paths known

**What's needed**:
- Identify unused/stale caches
- Delete cache directories

**Estimated effort**: 2-3 hours

**Implementation checklist**:
- [ ] Create cache clean function in `src/commands/cache.rs`
- [ ] Enumerate cache directories
- [ ] Filter by age (--older-than)
- [ ] Identify unused (not in any config)
- [ ] Delete directories
- [ ] Add `--dry-run` flag
- [ ] Add `--all` flag
- [ ] Add `--unused` flag
- [ ] Add confirmation prompt (skip with `--yes`)
- [ ] Tests

**Dependencies**: None

**Blocked by**: None

---

#### 9. `common-repo check`

**Status**: 🟡 **PARTIALLY READY** - Validation ready, updates need work

**What's needed**:
- Semver comparison logic
- Check for available updates

**Estimated effort**: 4-5 hours

**Implementation checklist**:
- [ ] Create `src/commands/check.rs`
- [ ] Validation part:
  - [ ] Use `validate` command logic
- [ ] Update checking:
  - [ ] Parse config
  - [ ] For each repo operation:
    - [ ] List remote tags
    - [ ] Parse current ref as semver
    - [ ] Parse remote tags as semver
    - [ ] Compare versions
    - [ ] Identify newer versions (patch, minor, major)
  - [ ] Format update report
- [ ] Add `--updates-only` flag
- [ ] Add `--validate-only` flag
- [ ] Add `--json` output
- [ ] Tests

**Dependencies**: Need semver comparison utility

**Blocked by**: Semver comparison helper

---

### Tier 3: Requires Significant Additional Work

These commands need substantial new functionality.

#### 10. `common-repo diff`

**Status**: 🔴 **NOT READY** - Needs diff generation

**What's needed**:
- Diff generation between current state and composite filesystem
- Text diff algorithm

**Estimated effort**: 6-8 hours

**Implementation checklist**:
- [ ] Create `src/commands/diff.rs`
- [ ] Add `similar` or `diff` crate for text diffing
- [ ] Parse config
- [ ] Run phases 1-5 (get composite FS)
- [ ] Load current filesystem state
- [ ] Compare file by file:
  - [ ] Detect added files
  - [ ] Detect removed files
  - [ ] Detect modified files
  - [ ] Generate text diffs for modified files
- [ ] Format output (git-style diff)
- [ ] Add `--stat` flag (summary only)
- [ ] Add `--name-only` flag
- [ ] Add color output
- [ ] Tests

**Dependencies**: Diff crate (`similar` or `diff`)

**Blocked by**: None (but lower priority)

---

#### 11. `common-repo update`

**Status**: 🔴 **NOT READY** - Needs config rewriting

**What's needed**:
- Semver version selection
- YAML config file rewriting (preserve comments, formatting)
- Update logic

**Estimated effort**: 8-10 hours

**Implementation checklist**:
- [ ] Create `src/commands/update.rs`
- [ ] Semver version selection:
  - [ ] List available tags
  - [ ] Parse and sort semver versions
  - [ ] Select target version based on flags (--patch, --minor, --major)
- [ ] Config file rewriting:
  - [ ] Parse config with source location tracking
  - [ ] Update ref values
  - [ ] Preserve YAML formatting and comments
  - [ ] Write back to file
- [ ] Add `--dry-run` flag
- [ ] Add `--yes` flag (skip confirmation)
- [ ] Add `--apply` flag (run apply after)
- [ ] Handle multiple repos (specific or all)
- [ ] Tests

**Dependencies**:
- Semver comparison utility
- YAML manipulation with formatting preservation (tricky!)

**Blocked by**: Semver comparison, YAML rewriting strategy

---

#### 12. `common-repo cache update`

**Status**: 🔴 **NOT READY** - Needs cache invalidation strategy

**What's needed**:
- Force re-fetch cached repos
- Update cache entries

**Estimated effort**: 2-3 hours

**Implementation checklist**:
- [ ] Create cache update function in `src/commands/cache.rs`
- [ ] Enumerate cached repos
- [ ] Use `fetch_repository_fresh()` for each
- [ ] Progress indication
- [ ] Tests

**Dependencies**: None (uses `fetch_repository_fresh`)

**Blocked by**: Lower priority, not critical for MVP

---

## Implementation Phases

### Phase 1: MVP (Core Functionality) - 2-3 days

**Goal**: Make the tool usable for basic workflows

Priority order:
1. **`common-repo apply`** ⭐ - Core functionality (4-6 hours)
2. **`common-repo validate`** - Safety/debugging (2-3 hours)
3. **`common-repo init`** (minimal) - Onboarding (2-3 hours)
4. **`common-repo cache list`** - Observability (2-3 hours)
5. **`common-repo cache clean`** - Maintenance (2-3 hours)

**Total estimated**: 12-18 hours

**Outputs**:
- Users can apply configurations
- Users can validate configurations
- Users can initialize new configs
- Users can manage cache

---

### Phase 2: Enhanced Workflows - 2-3 days

**Goal**: Add inspection and planning capabilities

Priority order:
6. **`common-repo tree`** - Understand inheritance (3-4 hours)
7. **`common-repo info`** - Inspect repos (3-4 hours)
8. **`common-repo ls`** - Preview files (3-4 hours)
9. **`common-repo check`** - Check for updates (4-5 hours)

**Total estimated**: 13-17 hours

**Outputs**:
- Users can visualize inheritance
- Users can inspect configurations
- Users can preview changes
- Users can check for updates

---

### Phase 3: Advanced Features - 3-4 days

**Goal**: Full-featured CLI

Priority order:
10. **`common-repo diff`** - Preview changes (6-8 hours)
11. **`common-repo update`** - Automated updates (8-10 hours)
12. **`common-repo init --interactive`** - Better onboarding (6-8 hours)
13. **`common-repo cache update`** - Advanced cache mgmt (2-3 hours)

**Total estimated**: 22-29 hours

**Outputs**:
- Full-featured tool
- Excellent user experience
- Production-ready

---

## Technical Dependencies

### Required Crates

Add to `Cargo.toml`:

```toml
[dependencies]
# Existing...
clap = { version = "4.5", features = ["derive", "cargo", "env"] }
anyhow = "1.0"  # Already present
indicatif = "0.17"  # Progress bars
console = "0.15"  # Terminal colors and styling

[dependencies]  # Phase 2+
ptree = "0.4"  # Tree visualization (or termtree)
walkdir = "2.5"  # Already present

[dependencies]  # Phase 3+
similar = "2.4"  # Text diffing
dialoguer = "0.11"  # Interactive prompts
```

### Code Structure

Proposed structure for CLI code:

```
src/
├── main.rs           # Binary entry point (NEW)
├── lib.rs            # Library (existing)
├── cli.rs            # CLI definition (NEW)
├── commands/         # Command implementations (NEW)
│   ├── mod.rs
│   ├── apply.rs      # Phase 1
│   ├── validate.rs   # Phase 1
│   ├── init.rs       # Phase 1
│   ├── cache.rs      # Phase 1 (list, clean)
│   ├── tree.rs       # Phase 2
│   ├── info.rs       # Phase 2
│   ├── ls.rs         # Phase 2
│   ├── check.rs      # Phase 2
│   ├── diff.rs       # Phase 3
│   └── update.rs     # Phase 3
├── util/             # CLI utilities (NEW)
│   ├── mod.rs
│   ├── output.rs     # Formatting helpers
│   ├── progress.rs   # Progress indicators
│   └── semver.rs     # Semver comparison
└── [existing modules...]
```

---

## Testing Strategy

### Unit Tests
- Each command module should have unit tests
- Mock the library layer (RepositoryManager, phases)
- Test CLI argument parsing

### Integration Tests
- Full command tests
- Use `assert_cmd` crate
- Test with real config files
- Test error cases

### Add to `Cargo.toml`:
```toml
[dev-dependencies]
assert_cmd = "2.0"  # CLI testing
predicates = "3.1"  # Assertions for CLI tests
tempfile = "3.0"    # Already present
```

---

## Recommended Implementation Order

### Week 1: Core MVP
1. Set up CLI infrastructure (main.rs, cli.rs, clap)
2. Implement `apply` command
3. Implement `validate` command
4. Implement `init` (minimal)
5. Implement `cache list` and `cache clean`

### Week 2: Enhanced Workflows
6. Implement `tree` command
7. Implement `info` command
8. Implement `ls` command
9. Implement `check` command

### Week 3: Advanced Features
10. Implement `diff` command
11. Implement `update` command
12. Polish and documentation
13. Full testing

---

## Implementation Status

**Note**: This plan was created during initial CLI development. All commands are now implemented.

### Completed Commands
- apply, validate, init, cache (list/clean), tree, info, ls, check, update, diff

---

**Last updated**: 2025-11-30
**Current status**: All CLI commands complete
