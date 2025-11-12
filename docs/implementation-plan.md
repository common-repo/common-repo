# common-repo Implementation Plan

## Overview

This document outlines the implementation order for common-repo v2, organized by dependency hierarchy. Components are grouped into layers, where each layer depends only on layers above it.

## Implementation Layers

### Layer 0: Foundation (No dependencies)

These are the fundamental building blocks that everything else depends on.

#### 0.1 Configuration Schema & Parsing
**Purpose**: Define data structures and parse `.common-repo.yaml` files

**Components**:
- `config::Schema` - Rust structs representing the YAML schema
- `config::Operation` - Enum for all operator types (repo, include, exclude, rename, etc.)
- `config::parse()` - Parse YAML into Schema using serde_yaml
- Error types for invalid configurations

**Why first**: Every other component needs to work with parsed configuration

**Dependencies**:
- External: `serde`, `serde_yaml`
- Internal: None

**Testing**: Unit tests with valid/invalid YAML fixtures

---

#### 0.2 In-Memory Filesystem
**Purpose**: Represent filesystems in memory for fast manipulation

**Components**:
- `filesystem::MemoryFS` - Core in-memory filesystem structure
  - Store files as `HashMap<PathBuf, Vec<u8>>`
  - Support adding, removing, renaming files
  - Support listing files with glob patterns
  - Support metadata (permissions, timestamps)
- `filesystem::File` - Represents a file with content and metadata

**Why first**: Required by all phases that manipulate files

**Dependencies**:
- External: `glob`, `regex`
- Internal: None

**Testing**: Unit tests for all filesystem operations

---

#### 0.3 Error Handling
**Purpose**: Consistent error types throughout the application

**Components**:
- `error::Error` - Main error enum using thiserror
  - ConfigParseError
  - GitCloneError
  - CacheError
  - OperatorError
  - CycleDetectedError
  - MergeConflictWarning
  - etc.
- `error::Result<T>` - Type alias for Result<T, Error>

**Why first**: All other components need to return proper errors

**Dependencies**:
- External: `thiserror`, `anyhow`
- Internal: None

**Testing**: Error creation and formatting tests

---

### Layer 1: Core Utilities (Depends on Layer 0)

#### 1.1 Git Operations
**Purpose**: Clone repositories and interact with git

**Components**:
- `git::clone_shallow()` - Shallow clone a repo at specific ref
- `git::load_from_cache()` - Load cached repo into MemoryFS
- `git::save_to_cache()` - Save repo to cache directory
- `git::url_to_cache_path()` - Convert url+ref to cache path
- `git::list_tags()` - List all tags from a remote repository
- `git::parse_semver_tag()` - Parse a tag string into a semantic version

**Cache Strategy**: Refs are assumed immutable - no TTL needed

**Dependencies**:
- External: `git2` or shell out to `git` command, `semver`
- Internal: Layer 0 (MemoryFS, Error)

**Testing**: Integration tests with mock/test repos

---

#### 1.2 Repository Manager
**Purpose**: High-level orchestration of git clone/cache/load operations

**Components**:
- `repository::RepositoryManager` - Main interface for fetching repositories
- `repository::GitOperations` trait - Abstraction for git operations (mockable)
- `repository::CacheOperations` trait - Abstraction for cache operations (mockable)
- `RepositoryManager::fetch_repository()` - Smart fetch (uses cache if available)
- `RepositoryManager::fetch_repository_fresh()` - Force fresh clone (bypass cache)
- `RepositoryManager::is_cached()` - Check if repository is already cached
- `RepositoryManager::list_repository_tags()` - List available tags from remote

**Why here**: Provides clean abstraction for operators to fetch repositories without worrying about caching details. Enables easy testing with mocks.

**Dependencies**:
- External: None (orchestrates existing components)
- Internal: Layer 1.1 (Git Operations), Layer 1.3 (Repository Cache)

**Testing**: Unit tests with mock git/cache operations, full coverage of cache hit/miss scenarios

---

#### 1.3 Path Operations
**Purpose**: Handle path transformations and glob matching

**Components**:
- `path::glob_match()` - Match paths against glob patterns
- `path::regex_rename()` - Apply regex rename with capture groups
- `path::encode_url_path()` - Encode URL for filesystem paths

**Dependencies**:
- External: `glob`, `regex`
- Internal: Layer 0 (Error)

**Testing**: Unit tests with various patterns

---

#### 1.3 Repository Cache
**Purpose**: In-process caching of processed repositories

**Components**:
- `cache::RepoCache` - HashMap<(url, ref), IntermediateFS>
- `cache::get_or_process()` - Return cached or process new
- Thread-safe with Arc<Mutex<>> or similar

**Why here**: Prevents duplicate processing of same url+ref combo in single run

**Dependencies**:
- Internal: Layer 0 (MemoryFS, Error)

**Testing**: Unit tests for cache hits/misses

---

### Layer 2: Operators (Depends on Layers 0-1)

These implement the individual operations from the schema.

#### 2.1 Repo Operator
**Purpose**: Pull files from inherited repositories

**Components**:
- `operators::repo::apply()` - Process repo operator
- `operators::repo::apply_with_clause()` - Apply inline `with:` operations
  - The `with:` operations are syntactic sugar applied to this repo's intermediate filesystem
  - They run after the repo's own operations but before merging

**Dependencies**:
- Internal: Layer 0 (Config, MemoryFS, Error), Layer 1 (Git, Cache)

**Testing**: Integration tests with nested repos

---

#### 2.2 Basic File Operators
**Purpose**: Include, exclude, rename operations

**Components**:
- `operators::include::apply()` - Add files matching patterns to MemoryFS
- `operators::exclude::apply()` - Remove files matching patterns from MemoryFS
- `operators::rename::apply()` - Rename files using regex patterns

**Dependencies**:
- Internal: Layer 0 (MemoryFS, Config, Error), Layer 1 (Path)

**Testing**: Unit tests with sample filesystems

---

#### 2.3 Template Operators
**Purpose**: Mark and process templates with variable substitution

**Components**:
- `operators::template::mark()` - Mark files as templates
- `operators::template::process()` - Process templates with variables
  - Simple `${VAR}` substitution initially
  - Environment variables resolved at runtime
- `operators::template_vars::collect()` - Build unified variable context
  - Later definitions override earlier ones
  - Environment variables provide values

**Dependencies**:
- External: Simple regex-based substitution initially
- Internal: Layer 0 (MemoryFS, Config, Error)

**Testing**: Unit tests with template fixtures

---

#### 2.4 Merge Operators
**Purpose**: YAML, JSON, TOML, INI, Markdown fragment merging

**Components**:
- `operators::merge::yaml::apply()` - Merge YAML fragments
- `operators::merge::json::apply()` - Merge JSON fragments
- `operators::merge::toml::apply()` - Merge TOML fragments
- `operators::merge::ini::apply()` - Merge INI fragments
- `operators::merge::markdown::apply()` - Merge Markdown fragments
- `operators::merge::conflict_warning()` - Emit warnings on overwrites

**Merge Strategy**: Last-write-wins with warnings on conflicts

**Dependencies**:
- External: `serde_yaml`, `serde_json`, `toml`, `ini`, markdown parser
- Internal: Layer 0 (MemoryFS, Config, Error)

**Note**: These are complex and can be implemented incrementally. Start with YAML and JSON.

**Testing**: Unit tests with merge scenarios for each format

---

#### 2.5 Tool Validation Operator
**Purpose**: Validate required tools exist with correct versions

**Components**:
- `operators::tools::validate()` - Check tool availability and versions
- `operators::tools::parse_version_constraint()` - Parse semver constraints
- `operators::tools::check_tool()` - Check individual tool

**Dependencies**:
- External: `semver`
- Internal: Layer 0 (Config, Error)

**Note**: This is validation-only, warns on failures

**Testing**: Integration tests (may need to mock system commands)

---

### Layer 3: Phases (Depends on Layers 0-2)

These implement the 7 phases from the design doc.

#### 3.1 Phase 1: Discovery and Cloning
**Purpose**: Fetch all repos in parallel using breadth-first traversal

**Components**:
- `phase1::discover_repos()` - Recursively discover all inherited repos
- `phase1::clone_parallel()` - Clone repos in parallel (breadth-first)
  - Clone all repos at depth N before moving to depth N+1
  - Maximizes parallelism and minimizes total time
- `phase1::detect_cycles()` - Detect circular dependencies
- `phase1::RepoTree` - Data structure for dependency tree
- `phase1::handle_network_failure()` - Fall back to cache if network fails

**Network Failure Behavior**:
- If clone fails but cache exists, continue with cached version and warn
- If clone fails and no cache exists, abort with error

**Dependencies**:
- Internal: Layer 0 (Config, MemoryFS, Error), Layer 1 (Git, Cache)
- External: `tokio` or `rayon` for parallelism

**Testing**: Integration tests with mock repo trees

---

#### 3.2 Phase 2: Processing Individual Repos
**Purpose**: Transform each repo into intermediate filesystem

**Components**:
- `phase2::process_repo()` - Apply operations to produce intermediate FS
  - Uses Layer 1.3 RepoCache to avoid duplicate processing of identical repo + `with:` combinations
- `phase2::IntermediateFS` - Wrapper around MemoryFS with metadata

**Dependencies**:
- Internal: Layer 0 (Config, MemoryFS, Error), Layer 2 (All operators)

**Testing**: Integration tests with sample repos

---

#### 3.3 Phase 3: Determining Operation Order
**Purpose**: Build deterministic operation order using depth-first traversal

**Components**:
- `phase3::build_operation_order()` - Depth-first traversal to determine order
- `phase3::OperationOrder` - List of repos in application order

**Important**: While cloning uses breadth-first for speed, operation ordering uses depth-first for correctness (ancestors before parents before local)

**Dependencies**:
- Internal: Layer 0 (Config, Error), Layer 3.1 (RepoTree)

**Testing**: Unit tests with various inheritance patterns

---

#### 3.4 Phase 4: Composite Filesystem Construction
**Purpose**: Merge all intermediate filesystems

**Components**:
- `phase4::build_composite()` - Merge intermediate FSs in order
- `phase4::apply_merge_operators()` - Apply merge operations
  - Emit warnings on merge conflicts (last-write-wins)
- `phase4::process_templates()` - Process all templates with unified context

**Dependencies**:
- Internal: Layer 0 (MemoryFS, Error), Layer 2 (Template, Merge operators), Layer 3.2 (IntermediateFS)

**Testing**: Integration tests with multiple repos

---

#### 3.5 Phase 5: Local File Merging
**Purpose**: Merge composite FS with local files

**Components**:
- `phase5::merge_with_local()` - Apply local merge operations
- `phase5::load_local_fs()` - Load local files into MemoryFS

**Dependencies**:
- Internal: Layer 0 (MemoryFS, Config, Error), Layer 2 (Merge operators)

**Testing**: Integration tests with local files

---

#### 3.6 Phase 6: Writing to Disk
**Purpose**: Write final filesystem to disk

**Components**:
- `phase6::write_to_disk()` - Write MemoryFS to host filesystem
- `phase6::preserve_permissions()` - Maintain file permissions

**Dependencies**:
- Internal: Layer 0 (MemoryFS, Error)
- External: `std::fs`

**Testing**: Integration tests with temp directories

---


### Layer 3.5: Version Detection (Depends on Layers 0-1)

**Note**: This is a core feature, not deferred to future phases.

#### 3.5.1 Version Checking
**Purpose**: Detect when inherited repos have newer versions available

**Components**:
- `version::check_updates()` - Check all inherited repos for newer versions
- `version::compare_refs()` - Compare current ref against available tags
- `version::UpdateInfo` - Data structure for update information
  - Current ref
  - Latest available version
  - Breaking changes available (major version bump)
  - Compatible updates available (minor/patch)
- `version::filter_semver_tags()` - Filter git tags to semantic versions only

**Dependencies**:
- External: `semver`
- Internal: Layer 0 (Config, Error), Layer 1 (Git)

**Integration with Pull Flow**:
1. During `common-repo pull`, after Phase 1 discovers all repos:
   - Check each inherited repo's current ref against available tags
   - If ref is a semantic version (e.g., `v1.2.3`), query remote for newer tags
   - Compare versions and categorize updates:
     - **Patch updates** (1.2.3 → 1.2.4): Bug fixes, safe to update
     - **Minor updates** (1.2.3 → 1.3.0): New features, backward compatible
     - **Major updates** (1.2.3 → 2.0.0): Breaking changes, requires review
   - Display summary to user (e.g., "2 repos have updates available")
2. Continue with normal pull operation using current refs
3. User can run `common-repo update` separately to see detailed update information

**Testing**: Integration tests with mock repos at different versions

---

### Layer 4: CLI & Orchestration (Depends on all layers)

#### 4.1 CLI Interface
**Purpose**: Command-line interface

**Components**:
- `cli::parse_args()` - Parse command-line arguments
- `cli::commands::pull()` - Main pull command
  - Execute all phases
  - After Phase 1: Check for version updates and warn/inform user
  - Display which repos are outdated
- `cli::commands::check()` - Validate configuration without pulling
- `cli::commands::update()` - Check for updates only (no pull)
  - List all inherited repos with their current and available versions
  - Show breaking vs compatible updates
  - Optionally: suggest updated `.common-repo.yaml` with new refs

**Dependencies**:
- External: `clap`
- Internal: All layers including Layer 3.5 (Version)

**Testing**: Integration tests, end-to-end tests

---

#### 4.2 Main Orchestrator
**Purpose**: Coordinate all phases

**Components**:
- `main.rs` - Entry point
- `orchestrator::run()` - Execute all phases in order
- `orchestrator::logging()` - Setup logging/progress

**Dependencies**:
- External: `env_logger`, `log`
- Internal: All layers

**Testing**: End-to-end integration tests

---

## Implementation Strategy

### Phase 1: MVP (Minimum Viable Product)
**Goal**: Get basic functionality working end-to-end

**Include**:
- Layer 0: All foundation components
- Layer 1: Git operations, basic path operations, repo cache
- Layer 2.1: Repo operator (without `with:` clause)
- Layer 2.2: Basic file operators (include, exclude, rename)
- Layer 3: All phases (simplified - no merge operators, no templates)
- Layer 4: Basic CLI with pull command

**Exclude** (defer to later):
- Template operators
- Merge operators
- Tool validation
- Advanced error handling
- `with:` clause in repo operator
- Performance optimizations

**Milestone**: Can pull a simple inherited repo with include/exclude/rename and write to disk

---

### Phase 2: Operators & Version Detection
**Goal**: Add template and merge capabilities, plus version checking

**Add**:
- Layer 2.1: Full repo operator with `with:` clause support
- Layer 2.3: Template operators (simple `${VAR}` substitution)
- Layer 2.4: YAML merge operator
- Layer 2.4: JSON merge operator
- Layer 3.5: Version detection and update checking (core feature)
- `common-repo update` command to check for outdated refs

**Milestone**: Can handle templates, merge YAML/JSON configs, and detect outdated dependencies

---

### Phase 3: Full Feature Set
**Goal**: Complete all operators

**Add**:
- Layer 2.4: TOML merge operator
- Layer 2.4: INI merge operator
- Layer 2.4: Markdown merge operator
- Layer 2.5: Tool validation operator
- Advanced template engine (if needed beyond simple substitution)
- `common-repo check` command for config validation

**Milestone**: Feature complete per design doc

---

### Phase 4: Performance & Polish
**Goal**: Optimize and productionize

**Add**:
- Parallel cloning optimization
- Progress indicators
- Better error messages
- Comprehensive logging
- Documentation

**Milestone**: Production ready

---

## Testing Strategy

### Unit Tests
- Every operator in isolation
- Path operations
- Configuration parsing
- Error handling
- In-process cache behavior

### Integration Tests
- Each phase with mock data
- Operator combinations
- Cache behavior
- Git operations (with test repos)
- Version detection scenarios

### End-to-End Tests
- Full scenarios with real repos
- Performance benchmarks
- Error scenarios
- Cache scenarios
- Network failure handling

---

## Open Questions to Resolve During Implementation

1. **Parallel execution library**: Use `tokio` (async) or `rayon` (parallel iterators)?
   - Recommendation: Start with `rayon` for simplicity, consider `tokio` if we need async git operations

2. **Git library**: Use `git2` (libgit2 bindings) or shell out to `git` command?
   - Recommendation: Shell out to `git` initially (simpler), migrate to `git2` if needed

3. **Template engine**: Which template substitution approach?
   - Decision: Start with simple regex-based `${VAR}` substitution
   - Defaults handled via template-vars definitions, not inline syntax

4. **Error handling strategy**: `anyhow` vs custom error types?
   - Recommendation: Use `thiserror` for library code, `anyhow` for CLI/application code

5. **Progress indication**: How to show progress without slowing things down?
   - Recommendation: Basic progress in Phase 2/3, full progress bars in Phase 4

---

## Dependencies Summary

**Essential Crates** (needed for MVP):
- `serde`, `serde_yaml` - Configuration parsing
- `glob` - Glob pattern matching
- `regex` - Rename operations
- `thiserror`, `anyhow` - Error handling
- `clap` - CLI argument parsing

**Phase 2 Crates**:
- `serde_json` - JSON merge
- `semver` - Version detection and comparison

**Phase 3 Crates**:
- `toml` - TOML merge
- `ini` - INI merge
- Markdown parser TBD

**Phase 4 Crates**:
- `rayon` or `tokio` - Parallelism
- `indicatif` - Progress bars
- `env_logger`, `log` - Logging

**Optional** (evaluate during implementation):
- `git2` - Alternative to shelling out
- `walkdir` - Filesystem traversal
- `tempfile` - Temporary directories for tests
