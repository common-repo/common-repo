# Architecture

**Analysis Date:** 2026-03-16

## Pattern Overview

**Overall:** Multi-Phase Pipeline Architecture with In-Memory Filesystem Staging

**Key Characteristics:**
- Six distinct, sequential phases that orchestrate the complete inheritance and merge operation
- In-memory filesystem abstraction (`MemoryFS`) for staging all changes before disk I/O
- Trait-based abstraction for Git operations and caching to enable testing and swapping implementations
- Hierarchical repository discovery supporting recursive inheritance and cycle detection
- Format-specific merge operators (YAML, JSON, TOML, INI, Markdown) for intelligent content merging
- Command-oriented CLI dispatching via clap with a thin binary wrapper around core library

## Layers

**CLI Layer:**
- Purpose: Parse command-line arguments and dispatch to appropriate command implementation
- Location: `src/cli.rs`, `src/main.rs`
- Contains: Argument structs, command routing, top-level error handling
- Depends on: Command modules, library exports
- Used by: User interaction with the binary

**Command Layer:**
- Purpose: Implement each subcommand (apply, validate, check, diff, ls, tree, etc.) with their specific logic
- Location: `src/commands/*.rs`
- Contains: Per-command `Args` structs (from clap), command-specific orchestration
- Depends on: Library core modules (config, phases, filesystem, etc.)
- Used by: CLI dispatcher

**Core Abstraction Layer:**
- Purpose: Define traits and interfaces for swappable implementations (Git, Cache, Filesystem)
- Location: `src/repository.rs` (GitOperations, CacheOperations traits), `src/filesystem.rs` (MemoryFS)
- Contains: Public trait definitions, concrete implementations (DefaultGitOperations, DefaultCacheOperations)
- Depends on: Error handling, path utilities
- Used by: Phase implementations, repository discovery

**Configuration Layer:**
- Purpose: Define schema structures and parsing logic for `.common-repo.yaml` files
- Location: `src/config.rs`
- Contains: Operation enum (Repo, Include, Exclude, Rename, Template, MergeYaml/Json/Toml/Ini/Markdown, etc.), parsing logic with backward compatibility
- Depends on: Serde (YAML parsing), error handling
- Used by: CLI commands, discovery phase, processing phase

**Operator Layer:**
- Purpose: Implement individual operations that transform filesystems (include, exclude, rename, template, tools validation)
- Location: `src/operators.rs` (dispatching), `src/path.rs` (regex-based renaming)
- Contains: Per-operator modules (include, exclude, rename, repo, template, template_vars, tools)
- Depends on: MemoryFS, config types, filesystem operations
- Used by: Processing phase

**Merge Layer:**
- Purpose: Provide format-specific merge operations for combining files from multiple repositories
- Location: `src/merge/*.rs` (yaml, json, toml, ini, markdown)
- Contains: Format parsers, path navigation (PathSegment), merge logic per format
- Depends on: Format-specific libraries (serde_yaml, serde_json, toml, etc.)
- Used by: Composite filesystem phase, processing phase

**Phase Orchestration Layer:**
- Purpose: Execute the six-phase pipeline and coordinate dependencies between phases
- Location: `src/phases/orchestrator.rs`, `src/phases/*.rs` (discovery, processing, ordering, composite, local_merge, write)
- Contains: Phase 1-6 implementations, RepoTree/RepoNode structures, OperationOrder tracking
- Depends on: All lower layers (config, operators, merge, MemoryFS, repository)
- Used by: Apply and other commands that need full pipeline execution

**Supporting Utilities:**
- Purpose: Cross-cutting concerns and common functions
- Location: `src/error.rs`, `src/cache.rs`, `src/git.rs`, `src/output.rs`, `src/version.rs`, `src/suggestions.rs`, `src/defaults.rs`, `src/path.rs`
- Contains: Error types, Git command wrappers, in-process caching, output formatting, helper functions
- Depends on: Standard library, external crates
- Used by: All other layers

## Data Flow

**Apply Command (Complete Pipeline):**

1. Parse `.common-repo.yaml` config file (CLI → Config Layer)
2. Phase 1 - Discovery and Cloning:
   - Recursively discover all inherited repositories from config
   - Build RepoTree with inheritance hierarchy
   - Clone each repository in parallel (with caching via RepositoryManager)
3. Phase 2 - Processing Individual Repos:
   - For each repository in the tree, apply its operations (include, exclude, rename, template)
   - Store processed filesystem as IntermediateFS with template variables
4. Phase 3 - Determining Operation Order:
   - Traverse RepoTree to determine deterministic merge order (depth-first)
   - Generate OperationOrder list with unique repository keys
5. Phase 4 - Composite Filesystem Construction:
   - Merge all intermediate filesystems in correct order
   - Apply merge operations (MergeYaml, MergeJson, etc.) if specified
6. Phase 5 - Local File Merging:
   - Load local files from working directory
   - Merge local files with composite filesystem (local takes precedence)
   - Apply any local-only operations
7. Phase 6 - Disk Output:
   - Write final MemoryFS to target output directory

**Validate Command:**
- Parse config
- Phase 1 only: Discover and clone repositories, check for cycles
- Report validity

**Tree Command:**
- Phase 1 only: Discover repositories
- Display inheritance tree structure

**Diff Command:**
- Run full pipeline (Phases 1-6) to get final_fs
- Compare final_fs with current working directory files
- Report differences

**State Management:**

- **MemoryFS**: Passed through entire pipeline, accumulated and transformed at each phase
- **RepoTree**: Built during Phase 1, used for dependency tracking and cycle detection
- **OperationOrder**: Generated in Phase 3, consumed in Phase 4
- **IntermediateFS**: Created per-repository in Phase 2, consumed in Phase 4
- **In-Process Cache (RepoCache)**: Maintains processed MemoryFS per (url, ref) pair to avoid re-processing same repository with same operations

## Key Abstractions

**RepoTree/RepoNode:**
- Purpose: Represent inheritance hierarchy with repository metadata and operations
- Examples: `src/phases/mod.rs` defines RepoNode and RepoTree structures
- Pattern: Tree structure with node_key() method for deduplication and fingerprinting operations via hash

**MemoryFS:**
- Purpose: Virtual filesystem used throughout pipeline for staging changes without touching disk
- Examples: `src/filesystem.rs` implements HashMap-based file storage with metadata
- Pattern: Provides methods for add_file, get_file, list_files_glob, merge_from, remove_file, rename, copy

**Operation Enum:**
- Purpose: Discriminated union representing all possible actions in configuration
- Examples: `src/config.rs` defines Operation with variants (Repo, Include, Exclude, Rename, Template, MergeYaml, etc.)
- Pattern: Each variant holds its specific config struct; processed by dedicated operator functions

**Merge Strategy (Format-Specific):**
- Purpose: Provide pluggable merge logic for different file formats
- Examples: `src/merge/yaml.rs`, `src/merge/json.rs` implement format-specific merge
- Pattern: Each format has apply_<format>_merge_operation function taking file content, operation, and template variables

**RepositoryManager:**
- Purpose: Abstract Git and cache operations for testability
- Examples: `src/repository.rs` defines GitOperations and CacheOperations traits
- Pattern: Trait-based design allows swapping real implementations (DefaultGitOperations) with mocks in tests

## Entry Points

**Binary Entry Point:**
- Location: `src/main.rs`
- Triggers: User invokes `common-repo` binary
- Responsibilities: Parse CLI arguments via clap, dispatch to command implementation, handle top-level errors

**Apply Command Entry:**
- Location: `src/commands/apply.rs::execute()`
- Triggers: `common-repo apply [OPTIONS]`
- Responsibilities: Set up cache, config, repository manager; invoke phases::orchestrator::execute_pull; handle output and errors

**Validate Command Entry:**
- Location: `src/commands/validate.rs::execute()`
- Triggers: `common-repo validate [OPTIONS]`
- Responsibilities: Parse config, run Phase 1 only, check for cycles and configuration errors

**Phase 1 Discovery Entry:**
- Location: `src/phases/discovery.rs::execute()`
- Triggers: Called by orchestrator at start of pipeline
- Responsibilities: Recursively discover repositories, build RepoTree, detect cycles, parallel clone

**Phase 2 Processing Entry:**
- Location: `src/phases/processing.rs::execute()`
- Triggers: Called after Phase 1
- Responsibilities: Apply per-repository operations, create intermediate filesystems

**Phase 4 Composition Entry:**
- Location: `src/phases/composite.rs::execute()`
- Triggers: Called after Phase 3
- Responsibilities: Merge intermediate filesystems in correct order, apply merge operations

## Error Handling

**Strategy:** Centralized error enum with context-rich variants

**Patterns:**

- **Error Type**: `src/error.rs` defines unified Error enum with variants for each failure mode (ConfigParse, GitClone, OperatorError, MergeConflict, etc.)
- **Propagation**: Result<T> alias used throughout for ? operator
- **Context**: Error variants include contextual info (url, ref, command output, cycle path, etc.) for debugging
- **User Messaging**: Error variants implement Display with formatted messages and optional hints
- **Early Validation**: Config parsing errors caught before expensive operations; cycle detection in Phase 1

## Cross-Cutting Concerns

**Logging:**
- Framework: `log` crate with level filtering
- Approach: Initialize in commands, configured by --log-level and --verbose flags; used in phase implementations for progress tracking

**Validation:**
- Configuration: Parsed and validated during Config Layer; schema validation happens in config::parse
- Tool Requirements: `tools` operator validates that required CLI tools are installed and meet version constraints
- Cycle Detection: Implemented in discovery phase before cloning to fail fast

**Authentication:**
- Approach: Git authentication handled by system's git command (uses SSH keys, credentials in .git/config)
- No explicit auth layer in code; delegates to `git clone` command

**Caching:**
- Two-layer strategy: Disk cache (repositories on filesystem) and in-process cache (MemoryFS after operations)
- Disk cache location: System cache directory (~/.cache/common-repo on Linux, ~/Library/Caches/common-repo on macOS)
- In-process cache: Arc<Mutex<HashMap>> in RepoCache, keyed by (url, ref)

---

*Architecture analysis: 2026-03-16*
