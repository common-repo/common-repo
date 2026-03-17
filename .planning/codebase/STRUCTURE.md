# Codebase Structure

**Analysis Date:** 2026-03-16

## Directory Layout

```
common-repo/
├── src/                       # Core library and binary source
│   ├── main.rs               # Binary entry point
│   ├── lib.rs                # Library public API and module declarations
│   ├── cli.rs                # CLI argument parsing and command dispatch
│   ├── commands/              # Command implementations
│   │   ├── mod.rs            # Module exports
│   │   ├── apply.rs          # `apply` command - full pipeline execution
│   │   ├── validate.rs       # `validate` command - validate config and check for cycles
│   │   ├── check.rs          # `check` command - check validity and updates
│   │   ├── diff.rs           # `diff` command - show differences from config result
│   │   ├── ls.rs             # `ls` command - list files that would be created
│   │   ├── tree.rs           # `tree` command - display inheritance tree
│   │   ├── init.rs           # `init` command - initialize new config file
│   │   ├── add.rs            # `add` command - add repository to config
│   │   ├── update.rs         # `update` command - update repository refs
│   │   ├── info.rs           # `info` command - show repo information
│   │   ├── cache.rs          # `cache` command - manage repository cache
│   │   └── completions.rs    # `completions` command - generate shell completions
│   ├── config.rs             # Configuration schema and YAML parsing
│   ├── filesystem.rs         # In-memory filesystem abstraction (MemoryFS)
│   ├── operators.rs          # Operation implementations (include, exclude, rename, template, tools)
│   ├── merge/                 # Format-specific merge operations
│   │   ├── mod.rs            # Merge module exports and PathSegment definition
│   │   ├── yaml.rs           # YAML merge with anchor/alias support
│   │   ├── json.rs           # JSON merge with path navigation
│   │   ├── toml.rs           # TOML merge
│   │   ├── ini.rs            # INI format merge
│   │   └── markdown.rs       # Markdown section merge
│   ├── phases/               # Six-phase pipeline implementations
│   │   ├── mod.rs            # Phase module definitions, RepoTree/RepoNode structures
│   │   ├── orchestrator.rs   # Orchestrator for full pipeline execution
│   │   ├── discovery.rs      # Phase 1 - discover and clone repos
│   │   ├── processing.rs     # Phase 2 - apply per-repo operations
│   │   ├── ordering.rs       # Phase 3 - determine merge order
│   │   ├── composite.rs      # Phase 4 - merge intermediate filesystems
│   │   ├── local_merge.rs    # Phase 5 - merge with local files
│   │   └── write.rs          # Phase 6 - write final filesystem to disk
│   ├── repository.rs         # RepositoryManager with GitOperations/CacheOperations traits
│   ├── git.rs                # Git command wrappers (clone_shallow, list_tags)
│   ├── cache.rs              # In-process cache (RepoCache) for processed filesystems
│   ├── error.rs              # Unified error type with context-rich variants
│   ├── output.rs             # Output formatting utilities
│   ├── path.rs               # Path manipulation (regex rename, validation)
│   ├── suggestions.rs        # User-friendly error suggestions
│   ├── version.rs            # Version information
│   ├── defaults.rs           # Constants (default config filename, etc.)
│   └── path_proptest.rs      # Property-based tests for path handling
├── tests/                     # Integration and E2E test suites
│   ├── cli_e2e_*.rs          # E2E tests for each command
│   ├── integration_*.rs      # Integration tests for specific features
│   ├── common/               # Shared test utilities
│   ├── testdata/             # Test fixture repositories
│   └── schema_parsing_test.rs # Configuration parsing tests
├── benches/                   # Benchmark suites
│   ├── config_parsing.rs     # Benchmark configuration parsing performance
│   ├── filesystem_ops.rs     # Benchmark filesystem operations
│   └── operators.rs          # Benchmark operator performance
├── examples/                  # Usage examples
│   └── repository_usage.rs   # Example of using library API
├── xtask/                     # Cargo xtask for development tasks
├── script/                    # Shell scripts for development workflow
│   ├── setup                 # First-time setup
│   ├── bootstrap             # Install dependencies
│   ├── test                  # Run test suite
│   ├── ci                    # Run CI checks (no tests)
│   └── cibuild               # Full CI validation (checks + tests)
├── docs/                      # User documentation
│   ├── book.toml             # mdBook configuration
│   └── src/                  # Documentation source files in Markdown
├── context/                   # Task tracking and design documents
│   ├── current-task.json     # Current task being worked on
│   ├── design.md             # Design decisions and rationale
│   ├── purpose.md            # Project purpose and vision
│   └── completed/            # Archived completed task plans
├── .planning/                 # GSD planning documents
│   └── codebase/             # Architecture and codebase analysis
├── Cargo.toml                # Rust manifest with dependencies
├── Cargo.lock                # Locked dependency versions
├── rust-toolchain.toml       # Rust version specification
├── .pre-commit-config.yaml   # Pre-commit hook configuration
├── deny.toml                 # Cargo deny security audit config
├── schema.yaml               # Schema for `.common-repo.yaml` files (documentation)
├── action.yml                # GitHub Action configuration
├── README.md                 # Project overview
├── CHANGELOG.md              # Release notes and version history
└── LICENSE.md                # MIT license
```

## Directory Purposes

**src/:**
- Purpose: Main source code for the library and binary
- Contains: Rust modules organized by logical layer (CLI, commands, core abstractions, phases, merges, utilities)
- Key files: `lib.rs` (public API), `main.rs` (binary entry), `cli.rs` (argument parsing)

**src/commands/:**
- Purpose: Implementations of each CLI subcommand
- Contains: Per-command Args structs and execute() functions
- Key files: `apply.rs` (main pipeline), `validate.rs` (config validation), `diff.rs` (change detection)

**src/phases/:**
- Purpose: Implementation of the six-phase pipeline
- Contains: Each phase module plus orchestrator that coordinates them
- Key files: `orchestrator.rs` (coordinates all phases), `discovery.rs` (phase 1), `processing.rs` (phase 2)

**src/merge/:**
- Purpose: Format-specific merge operation implementations
- Contains: Separate modules per supported format (YAML, JSON, TOML, INI, Markdown)
- Key files: `mod.rs` (PathSegment types and path parsing), `yaml.rs` (YAML merge)

**tests/:**
- Purpose: Integration and end-to-end test coverage
- Contains: Test files organized by command (cli_e2e_*.rs), features (integration_*.rs), and fixtures
- Key files: `cli_e2e_apply.rs` (apply command tests), common test utilities

**benches/:**
- Purpose: Performance benchmarks for critical operations
- Contains: Benchmark suites for parsing, filesystem ops, and operators
- Key files: Run with `cargo bench`

**script/:**
- Purpose: Development workflow scripts following "Scripts to Rule Them All" pattern
- Contains: Shell scripts for setup, testing, CI checks
- Key files: `test` (runs test suite), `ci` (runs checks without tests), `cibuild` (full CI)

**docs/:**
- Purpose: User-facing documentation served at [common-repo.github.io/common-repo](https://common-repo.github.io/common-repo/)
- Contains: mdBook source files in Markdown format
- Key files: `book.toml` (mdBook config), `src/` directory with `.md` files

**context/:**
- Purpose: Task tracking, design decisions, and implementation planning
- Contains: JSON task files, markdown design docs, progress tracking
- Key files: `current-task.json` (active task), `design.md` (architectural decisions)

## Key File Locations

**Entry Points:**
- `src/main.rs`: Binary entry point that parses CLI args and dispatches to command
- `src/commands/apply.rs`: Main apply command that orchestrates the full pipeline
- `src/cli.rs`: CLI argument structure and command routing

**Configuration:**
- `src/config.rs`: Configuration schema (Operation enum) and YAML parsing
- `schema.yaml`: Human-readable schema documentation for .common-repo.yaml files

**Core Logic:**
- `src/phases/orchestrator.rs`: Coordinates Phases 1-6 in sequence
- `src/filesystem.rs`: In-memory filesystem abstraction used throughout
- `src/operators.rs`: Operation implementations (include, exclude, rename, template)

**Testing:**
- `tests/cli_e2e_apply.rs`: E2E tests for apply command
- `tests/integration_merge_*.rs`: Integration tests for merge operations per format
- `tests/common/`: Shared test utilities and fixtures

**Merge Operations:**
- `src/merge/yaml.rs`: YAML-specific merge with anchor/alias support
- `src/merge/json.rs`: JSON merge with nested path navigation
- `src/merge/toml.rs`: TOML format merge
- `src/merge/ini.rs`: INI key-value section merge
- `src/merge/markdown.rs`: Markdown section-based merge

## Naming Conventions

**Files:**
- Source modules: `snake_case.rs` (e.g., `file_system.rs`, `repo_cache.rs`)
- Command implementations: `<command_name>.rs` (e.g., `apply.rs`, `validate.rs`)
- E2E tests: `cli_e2e_<command>.rs` (e.g., `cli_e2e_apply.rs`)
- Integration tests: `integration_<feature>.rs` or `<feature>_integration.rs`
- Merge format modules: `<format>.rs` (e.g., `yaml.rs`, `json.rs`)

**Directories:**
- Feature groupings: `snake_case/` (e.g., `commands/`, `phases/`, `merge/`)
- Test fixtures: `testdata/` (contains example repositories and config files)
- Nested by feature area

**Functions and Methods:**
- Functions: `snake_case` (e.g., `clone_shallow()`, `apply_include_operation()`)
- Pub function entry points: Short, descriptive names (e.g., `execute()`, `apply()`)
- Internal helpers: Prefixed with context when needed

**Types:**
- Structs and enums: `PascalCase` (e.g., `MemoryFS`, `RepoOp`, `Error`)
- Trait names: `PascalCase` ending in "-able" or descriptive (e.g., `GitOperations`, `CacheOperations`)
- Type aliases: `snake_case` (e.g., `type Result<T> = std::result::Result<T, Error>;`)

## Where to Add New Code

**New CLI Command:**
1. Create new file: `src/commands/<command>.rs`
2. Define `struct <Command>Args` derived from clap::Args
3. Implement `fn execute(args: <Command>Args) -> Result<()>`
4. Add variant to Commands enum in `src/cli.rs`
5. Add module to `src/commands/mod.rs`
6. Add E2E tests in `tests/cli_e2e_<command>.rs`

**New Operation/Operator:**
1. Add variant to `Operation` enum in `src/config.rs`
2. Create handler function in `src/operators.rs` module
3. Call handler from phase implementation that applies operations
4. Add tests in `tests/` appropriate to the operation type

**New Merge Format:**
1. Create `src/merge/<format>.rs` implementing merge logic
2. Export `apply_<format>_merge_operation` from `src/merge/mod.rs`
3. Add `Merge<Format>` variant to `Operation` enum in `src/config.rs`
4. Call merge handler from `src/phases/composite.rs::apply_merge_operations`
5. Add integration tests in `tests/integration_merge_<format>.rs` and `tests/cli_e2e_*.rs`

**New Phase (if pipeline changes):**
1. Create `src/phases/<name>.rs` with `pub fn execute(...)` function
2. Add module to `src/phases/mod.rs`
3. Add re-export as phase alias (e.g., `pub(crate) use <name> as phase<N>;`)
4. Call from `src/phases/orchestrator.rs::execute_pull()`
5. Add tests in phase module

**Utilities and Helpers:**
- Filesystem utilities: `src/filesystem.rs`
- Path manipulation: `src/path.rs`
- Error types: `src/error.rs`
- Git operations: `src/git.rs`
- Caching: `src/cache.rs`
- Output formatting: `src/output.rs`

**Tests:**
- Unit tests: Co-locate in module with `#[cfg(test)]` section
- E2E tests: `tests/cli_e2e_<command>.rs`
- Integration tests: `tests/integration_<feature>.rs`
- Test fixtures: `tests/testdata/`
- Shared utilities: `tests/common/`

## Special Directories

**src/commands/:**
- Purpose: CLI command implementations
- Generated: No
- Committed: Yes - all command implementations are source-controlled

**.planning/codebase/:**
- Purpose: GSD mapping documents (ARCHITECTURE.md, STRUCTURE.md, CONVENTIONS.md, TESTING.md, STACK.md, INTEGRATIONS.md, CONCERNS.md)
- Generated: No - manually created and maintained
- Committed: Yes - provides guidance for future work

**tests/testdata/:**
- Purpose: Example repositories and configuration files used in tests
- Generated: No - checked in as test fixtures
- Committed: Yes - essential for reproducible testing

**target/:**
- Purpose: Cargo build artifacts (compiled binaries, dependencies, etc.)
- Generated: Yes - produced by `cargo build`
- Committed: No - listed in `.gitignore`

**.git/config.local** (if using local git credentials):
- Purpose: Local git configuration for authentication
- Generated: Yes - created during development
- Committed: No - listed in `.gitignore`

---

*Structure analysis: 2026-03-16*
