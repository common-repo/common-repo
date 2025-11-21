# common-repo CLI Design

## Overview

The `common-repo` CLI provides commands for managing repository configuration inheritance, caching, and updates. The design prioritizes:
- **Speed**: Most operations complete in milliseconds with caching
- **Clarity**: Clear command names and helpful output
- **Safety**: Dry-run and diff capabilities before making changes
- **Developer Experience**: Git-like interface familiar to developers

## Command Structure

```
common-repo <command> [options]
```

---

## Core Commands

### `common-repo apply`

**Purpose**: Apply the `.common-repo.yaml` configuration to the current repository.

This is the primary command - it executes the full 6-phase pipeline:
1. Discovery and cloning of inherited repos
2. Processing individual repos into intermediate filesystems
3. Determining operation order
4. Constructing composite filesystem
5. Merging with local files
6. Writing to disk

**Usage**:
```bash
common-repo apply [options]
```

**Options**:
- `-c, --config <path>` - Path to config file (default: `.common-repo.yaml`)
- `-f, --force` - Overwrite files without confirmation prompts
- `-n, --dry-run` - Show what would be done without making changes
- `--no-cache` - Bypass cache and fetch fresh clones
- `--parallel <n>` - Number of parallel clone operations (default: auto)
- `-v, --verbose` - Show detailed progress information
- `-q, --quiet` - Suppress all output except errors

**Examples**:
```bash
# Apply configuration
common-repo apply

# Preview what would happen
common-repo apply --dry-run

# Force apply without prompts, verbose output
common-repo apply --force --verbose

# Use different config file
common-repo apply --config .common-repo.prod.yaml
```

**Output**:
```
🔍 Discovering repositories...
  ✓ https://github.com/common-repo/rust-cli @ v1.2.0
  ✓ https://github.com/common-repo/semantic-versioning @ v2.0.1

📦 Cloning repositories... (2 parallel)
  ✓ rust-cli (cached)
  ↓ semantic-versioning (1.2 MB)

🔧 Processing operations...
  ✓ Phase 1: Discovery (2 repos, 0.03s)
  ✓ Phase 2: Processing (54 files)
  ✓ Phase 3: Operation order determined
  ✓ Phase 4: Composite filesystem (48 files)
  ✓ Phase 5: Local merging (3 conflicts resolved)
  ✓ Phase 6: Writing to disk

✅ Applied successfully in 0.18s
   48 files written
   3 files merged
   12 files unchanged
```

---

### `common-repo init`

**Purpose**: Initialize a new `.common-repo.yaml` configuration file.

**Usage**:
```bash
common-repo init [options]
```

**Options**:
- `-i, --interactive` - Interactive wizard to configure repositories
- `-t, --template <name>` - Start from a template (e.g., `rust-cli`, `python-django`)
- `--minimal` - Create minimal configuration with examples (default)
- `--empty` - Create empty configuration file
- `-f, --force` - Overwrite existing configuration

**Examples**:
```bash
# Create minimal config with examples
common-repo init

# Interactive setup
common-repo init --interactive

# Start from template
common-repo init --template rust-cli

# Empty config
common-repo init --empty
```

**Interactive Example**:
```
🎉 Welcome to common-repo!

Let's set up your repository configuration.

? What type of project is this?
  ❯ Rust CLI application
    Python web application
    Node.js/TypeScript project
    Go service
    Custom/Other

? Which common configurations do you want?
  ◉ Pre-commit hooks
  ◉ CI/CD workflows (GitHub Actions)
  ◯ Semantic versioning
  ◉ Linters and formatters

? Pin to stable versions or track latest?
  ❯ Stable (recommended)
    Latest (auto-update to newest)
    Custom

✅ Created .common-repo.yaml
   Added 3 repositories
   Run `common-repo apply` to fetch and apply configurations
```

---

### `common-repo check`

**Purpose**: Validate configuration and check for available updates.

**Usage**:
```bash
common-repo check [options]
```

**Options**:
- `-c, --config <path>` - Path to config file (default: `.common-repo.yaml`)
- `--updates-only` - Only check for updates, skip validation
- `--validate-only` - Only validate config, skip update check
- `--json` - Output in JSON format

**Examples**:
```bash
# Full check (validate + updates)
common-repo check

# Only check for updates
common-repo check --updates-only

# Only validate configuration
common-repo check --validate-only
```

**Output**:
```
🔍 Validating .common-repo.yaml...
  ✓ Syntax valid
  ✓ All repository URLs reachable
  ✓ All refs exist
  ✓ No circular dependencies
  ✓ All regex patterns valid

📦 Checking for updates...
  ✓ common-repo/rust-cli: v1.2.0 (latest)
  ⚠ common-repo/semantic-versioning: v2.0.1 → v2.1.0 available (minor)
  ⚠ common-repo/python-uv: v0.8.0 → v1.0.0 available (major ⚠️  breaking)

💡 Run `common-repo update` to upgrade dependencies
   Run `common-repo update --major` to include breaking changes
```

---

### `common-repo update`

**Purpose**: Update repository refs to newer versions.

**Usage**:
```bash
common-repo update [options] [repos...]
```

**Options**:
- `-c, --config <path>` - Path to config file (default: `.common-repo.yaml`)
- `--patch` - Only update patch versions (default)
- `--minor` - Update patch and minor versions
- `--major` - Update all versions including breaking changes
- `-n, --dry-run` - Show what would be updated without applying
- `-y, --yes` - Update without confirmation
- `--apply` - Run `apply` after updating

**Examples**:
```bash
# Update all to latest patch versions
common-repo update

# Update specific repo to latest minor version
common-repo update --minor common-repo/rust-cli

# Update all including breaking changes
common-repo update --major

# Preview updates
common-repo update --minor --dry-run

# Update and apply immediately
common-repo update --minor --apply
```

**Output**:
```
📦 Checking for updates...

Available updates:
  common-repo/semantic-versioning: v2.0.1 → v2.1.0 (minor)
  common-repo/pre-commit-hooks: v1.5.2 → v1.5.3 (patch)

? Update these dependencies? (Y/n) y

✅ Updated 2 dependencies

💡 Run `common-repo apply` to apply the updates
```

---

### `common-repo diff`

**Purpose**: Show what would change if configuration were applied.

**Usage**:
```bash
common-repo diff [options] [paths...]
```

**Options**:
- `-c, --config <path>` - Path to config file (default: `.common-repo.yaml`)
- `--cached` - Show diff against cached state
- `--stat` - Show summary statistics only
- `--name-only` - Show only file names
- `--color <when>` - Colorize output: always, never, auto (default: auto)

**Examples**:
```bash
# Show all changes
common-repo diff

# Show changes for specific files
common-repo diff .github/workflows/

# Summary only
common-repo diff --stat

# Just file names
common-repo diff --name-only
```

**Output**:
```
📊 Changes that would be applied:

Modified: .github/workflows/ci.yml
@@ -12,7 +12,10 @@
     - name: Run tests
       run: cargo test
+    - name: Run clippy
+      run: cargo clippy --all-targets

Added: .pre-commit-config.yaml (142 lines)
Removed: legacy-config.ini
Modified: README.md (3 insertions, 1 deletion)

Summary:
  3 files modified
  1 file added
  1 file removed
  4 total changes
```

---

## Cache Management Commands

### `common-repo cache list`

**Purpose**: List cached repositories.

**Usage**:
```bash
common-repo cache list [options]
```

**Options**:
- `--verbose` - Show detailed information
- `--json` - Output in JSON format

**Output**:
```
📦 Cached repositories (~/.common-repo/cache/):

common-repo/rust-cli
  v1.2.0  (102 MB)  last used: 2 hours ago
  v1.1.0  (98 MB)   last used: 3 days ago

common-repo/semantic-versioning
  v2.1.0  (45 MB)   last used: 1 hour ago
  v2.0.1  (44 MB)   last used: 2 days ago

Total: 289 MB (4 cached versions)
```

---

### `common-repo cache clean`

**Purpose**: Remove cached repositories.

**Usage**:
```bash
common-repo cache clean [options]
```

**Options**:
- `--all` - Remove all cached repositories
- `--unused` - Remove unused/stale cache entries
- `--older-than <days>` - Remove caches older than N days
- `-n, --dry-run` - Show what would be removed
- `-y, --yes` - Don't ask for confirmation

**Examples**:
```bash
# Remove unused caches
common-repo cache clean --unused

# Remove all caches older than 30 days
common-repo cache clean --older-than 30

# Remove all caches
common-repo cache clean --all

# Preview what would be removed
common-repo cache clean --unused --dry-run
```

---

### `common-repo cache update`

**Purpose**: Update cached repositories to latest refs.

**Usage**:
```bash
common-repo cache update [repos...]
```

**Examples**:
```bash
# Update all cached repos
common-repo cache update

# Update specific repo
common-repo cache update common-repo/rust-cli
```

---

## Inspection Commands

### `common-repo tree`

**Purpose**: Display the repository inheritance tree.

**Usage**:
```bash
common-repo tree [options]
```

**Options**:
- `-c, --config <path>` - Path to config file (default: `.common-repo.yaml`)
- `-d, --depth <n>` - Maximum depth to display
- `--no-cache` - Fetch fresh data without cache

**Output**:
```
📦 Repository inheritance tree:

. (local)
├── common-repo/rust-cli @ v1.2.0
│   ├── common-repo/ci-base @ v3.0.1
│   │   └── common-repo/github-actions @ v2.5.0
│   └── common-repo/rustfmt-config @ v1.0.3
└── common-repo/semantic-versioning @ v2.1.0
    └── common-repo/release-please @ v4.1.2

Depth: 3 levels
Total repos: 6 (including local)
```

---

### `common-repo info`

**Purpose**: Show information about a repository or the current configuration.

**Usage**:
```bash
common-repo info [repo-url] [options]
```

**Options**:
- `-c, --config <path>` - Path to config file (default: `.common-repo.yaml`)
- `--tags` - List available tags/versions
- `--operations` - Show operations defined in repo
- `--json` - Output in JSON format

**Examples**:
```bash
# Info about current config
common-repo info

# Info about specific repo
common-repo info https://github.com/common-repo/rust-cli

# List available versions
common-repo info common-repo/rust-cli --tags
```

**Output (current config)**:
```
📋 Configuration: .common-repo.yaml

Inherited repositories: 2
  • common-repo/rust-cli @ v1.2.0
  • common-repo/semantic-versioning @ v2.1.0

Operations: 8
  • 2 repo operations
  • 3 include operations
  • 2 exclude operations
  • 1 template operation

Cache status: 2/2 repositories cached
Last applied: 2 hours ago
```

**Output (specific repo)**:
```
📦 common-repo/rust-cli

URL: https://github.com/common-repo/rust-cli
Latest version: v1.2.0
Description: Standard Rust CLI project configuration

Available versions:
  v1.2.0  (latest)
  v1.1.0
  v1.0.5
  v1.0.0

Exports:
  • .github/workflows/ci.yml
  • .github/workflows/release.yml
  • Cargo.toml (fragment)
  • rustfmt.toml
  • clippy.toml

Dependencies:
  • common-repo/ci-base @ v3.0.1
  • common-repo/rustfmt-config @ v1.0.3
```

---

### `common-repo ls`

**Purpose**: List files that would be created/modified by the configuration.

**Usage**:
```bash
common-repo ls [options]
```

**Options**:
- `-c, --config <path>` - Path to config file (default: `.common-repo.yaml`)
- `--tree` - Display as tree structure
- `--source` - Show which repo each file comes from
- `--conflicts` - Only show files with merge conflicts

**Examples**:
```bash
# List all files
common-repo ls

# Show file sources
common-repo ls --source

# Tree view
common-repo ls --tree
```

**Output**:
```
📄 Files in composite filesystem (48 files):

.github/
  workflows/
    ci.yml          ← common-repo/rust-cli
    release.yml     ← common-repo/semantic-versioning
.pre-commit-config.yaml  ← common-repo/rust-cli
Cargo.toml          [merged from 2 sources]
README.md           [local + 1 merge]
rustfmt.toml        ← common-repo/rust-cli
clippy.toml         ← common-repo/rust-cli

[... 42 more files ...]
```

---

## Utility Commands

### `common-repo validate`

**Purpose**: Validate a `.common-repo.yaml` configuration file.

**Usage**:
```bash
common-repo validate [config-path] [options]
```

**Options**:
- `--strict` - Enable strict validation (fail on warnings)
- `--json` - Output in JSON format

**Examples**:
```bash
# Validate current config
common-repo validate

# Validate specific file
common-repo validate .common-repo.prod.yaml

# Strict mode
common-repo validate --strict
```

---

### `common-repo version`

**Purpose**: Show version information.

**Usage**:
```bash
common-repo version [options]
```

**Options**:
- `--check` - Check for updates to common-repo itself
- `--verbose` - Show detailed build information

**Output**:
```
common-repo 0.1.0
commit: 3ec9f9d
built: 2025-11-12
rustc: 1.70.0
```

---

### `common-repo help`

**Purpose**: Show help information.

**Usage**:
```bash
common-repo help [command]
```

**Examples**:
```bash
# General help
common-repo help

# Command-specific help
common-repo help apply
```

---

## Global Options

These options work with all commands:

- `-h, --help` - Show help for the command
- `-V, --version` - Show version information
- `--color <when>` - Colorize output: always, never, auto (default: auto)
- `--log-level <level>` - Set log level: error, warn, info, debug, trace (default: info)

---

## Environment Variables

- `COMMON_REPO_CONFIG` - Default config file path (default: `.common-repo.yaml`)
- `COMMON_REPO_CACHE` - Cache directory (default: `~/.common-repo/cache`)
- `COMMON_REPO_NO_COLOR` - Disable colored output (set to `1`)
- `COMMON_REPO_LOG_LEVEL` - Default log level
- `COMMON_REPO_PARALLEL` - Default parallel clone operations

---

## Exit Codes

- `0` - Success
- `1` - General error
- `2` - Configuration error (invalid YAML, missing refs, etc.)
- `3` - Network error (failed to clone, repo unreachable)
- `4` - Cache error
- `5` - Validation error
- `130` - Interrupted by user (Ctrl+C)

---

## Comparison with Similar Tools

### vs `cargo`
- Similar command structure: `cargo build` → `common-repo apply`
- Similar update workflow: `cargo update` → `common-repo update`
- Similar inspection: `cargo tree` → `common-repo tree`

### vs `git`
- Clear operation semantics like git (apply, diff, status-like info)
- Familiar subcommand structure

### vs `npm/yarn`
- Package management concepts: update, info, cache clean
- Semantic versioning awareness

---

## Implementation Priority

**Phase 1 (MVP)**:
1. `apply` - Core functionality
2. `init` - Basic initialization (minimal template)
3. `validate` - Config validation
4. `cache list` - Basic cache inspection
5. `cache clean` - Cache management

**Phase 2**:
6. `check` - Update checking
7. `update` - Automated updates
8. `diff` - Preview changes
9. `tree` - Inheritance visualization
10. `info` - Repository information

**Phase 3**:
11. `init --interactive` - Interactive wizard
12. `ls` - File listing
13. `cache update` - Advanced cache management
14. Additional options and polish

---

## Future Enhancements

### Templates Registry
```bash
# Browse available templates
common-repo templates list

# Search templates
common-repo templates search rust

# Use template
common-repo init --template @common-repo/rust-cli
```

### Watch Mode
```bash
# Auto-apply on config changes
common-repo watch
```

### Plugin Support
```bash
# Install plugin for custom merge strategies
common-repo plugin install pre-commit-merge

# List installed plugins
common-repo plugin list
```

### Workspace Support
```bash
# Apply to all repos in a workspace
common-repo apply --workspace

# Update all workspace repos
common-repo update --workspace
```
