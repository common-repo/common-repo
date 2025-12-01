# CLI Reference

Complete reference for all common-repo commands.

## Global Options

These options are available for all commands:

| Option | Description |
|--------|-------------|
| `--color <WHEN>` | Colorize output: `always`, `never`, `auto` (default: auto) |
| `--log-level <LEVEL>` | Set log level: `error`, `warn`, `info`, `debug`, `trace` (default: info) |
| `-h, --help` | Print help information |
| `-V, --version` | Print version |

## Commands

### `apply` - Apply Configuration

Apply the `.common-repo.yaml` configuration to your repository. This runs the full 6-phase pipeline: discover repos, clone, process, merge, and write files.

```bash
common-repo apply [OPTIONS]
```

#### Options

| Option | Description |
|--------|-------------|
| `-c, --config <PATH>` | Path to config file (default: `.common-repo.yaml`) |
| `-o, --output <PATH>` | Output directory (default: current directory) |
| `--cache-root <PATH>` | Cache directory (default: `~/.common-repo/cache`) |
| `-n, --dry-run` | Show what would be done without making changes |
| `-f, --force` | Overwrite existing files without prompting |
| `-v, --verbose` | Show detailed progress information |
| `--no-cache` | Bypass cache and fetch fresh clones |
| `-q, --quiet` | Suppress output except errors |

#### Examples

```bash
# Apply configuration
common-repo apply

# Preview changes without applying
common-repo apply --dry-run

# Apply with verbose output
common-repo apply --verbose

# Apply to a different directory
common-repo apply --output ./output

# Force fresh clones (ignore cache)
common-repo apply --no-cache

# Use a different config file
common-repo apply --config my-config.yaml
```

### `check` - Validate and Check Updates

Check configuration validity and optionally check for repository updates.

```bash
common-repo check [OPTIONS]
```

#### Options

| Option | Description |
|--------|-------------|
| `-c, --config <FILE>` | Path to config file (default: `.common-repo.yaml`) |
| `--cache-root <DIR>` | Cache directory |
| `--updates` | Check for newer versions of inherited repositories |

#### Examples

```bash
# Validate configuration
common-repo check

# Check for available updates
common-repo check --updates
```

#### Output

When checking for updates, you'll see:
- Current ref for each inherited repo
- Available newer versions (if any)
- Whether updates are compatible (minor/patch) or breaking (major)

### `diff` - Preview Changes

Show differences between current files and what the configuration would produce.

```bash
common-repo diff [OPTIONS]
```

#### Options

| Option | Description |
|--------|-------------|
| `-c, --config <FILE>` | Path to config file (default: `.common-repo.yaml`) |
| `--cache-root <DIR>` | Cache directory |
| `--working-dir <DIR>` | Directory to compare against (default: current) |
| `--summary` | Show only a summary, not individual files |

#### Examples

```bash
# Show full diff
common-repo diff

# Show summary only
common-repo diff --summary

# Compare against a different directory
common-repo diff --working-dir ./other-project
```

### `init` - Initialize Configuration

Create a new `.common-repo.yaml` configuration file.

```bash
common-repo init [OPTIONS]
```

#### Options

| Option | Description |
|--------|-------------|
| `-i, --interactive` | Interactive setup wizard |
| `-t, --template <TEMPLATE>` | Start from a predefined template |
| `--minimal` | Create minimal configuration with examples (default) |
| `--empty` | Create empty configuration file |
| `-f, --force` | Overwrite existing configuration |

#### Examples

```bash
# Create minimal config with examples
common-repo init

# Create empty config
common-repo init --empty

# Overwrite existing config
common-repo init --force

# Use a template (if available)
common-repo init --template rust-cli
```

### `update` - Update Repository Refs

Update repository refs in your configuration to newer versions.

```bash
common-repo update [OPTIONS]
```

#### Options

| Option | Description |
|--------|-------------|
| `-c, --config <FILE>` | Path to config file (default: `.common-repo.yaml`) |
| `--cache-root <DIR>` | Cache directory |
| `--compatible` | Update to latest compatible versions only (default) |
| `--latest` | Update to latest versions, including breaking changes |
| `--yes` | Don't ask for confirmation |
| `--dry-run` | Show what would be updated without changing files |

#### Examples

```bash
# Update to latest compatible versions (minor/patch only)
common-repo update

# Preview updates without applying
common-repo update --dry-run

# Update to latest versions including breaking changes
common-repo update --latest

# Update without confirmation
common-repo update --yes
```

### `info` - Show Configuration Info

Display information about the current configuration.

```bash
common-repo info [OPTIONS]
```

#### Options

| Option | Description |
|--------|-------------|
| `-c, --config <FILE>` | Path to config file (default: `.common-repo.yaml`) |
| `--cache-root <DIR>` | Cache directory |

#### Examples

```bash
# Show configuration overview
common-repo info
```

#### Output

Shows:
- Inherited repositories and their refs
- Operations defined
- Template variables
- Required tools

### `ls` - List Files

List files that would be created or modified by the configuration.

```bash
common-repo ls [OPTIONS]
```

#### Options

| Option | Description |
|--------|-------------|
| `-c, --config <FILE>` | Path to config file (default: `.common-repo.yaml`) |
| `--cache-root <DIR>` | Cache directory |
| `--working-dir <DIR>` | Working directory for local file operations |
| `-p, --pattern <PATTERN>` | Filter by glob pattern (e.g., `*.rs`, `src/**`) |
| `-l, --long` | Long format with size and permissions |
| `-s, --sort <SORT>` | Sort by: `name`, `size`, `path` (default: name) |
| `--count` | Show only total file count |
| `-r, --reverse` | Reverse sort order |

#### Examples

```bash
# List all files
common-repo ls

# Long format
common-repo ls -l

# Filter by pattern
common-repo ls --pattern "*.yml"
common-repo ls -p "src/**/*.rs"

# Count files
common-repo ls --count

# Sort by size, largest first
common-repo ls -l --sort size --reverse
```

### `validate` - Validate Configuration

Validate a `.common-repo.yaml` configuration file for syntax and semantic errors.

```bash
common-repo validate [OPTIONS]
```

#### Options

| Option | Description |
|--------|-------------|
| `-c, --config <FILE>` | Path to config file (default: `.common-repo.yaml`) |
| `--cache-root <DIR>` | Cache directory |
| `--check-repos` | Also verify that referenced repositories are accessible |
| `--strict` | Fail on warnings (not just errors) |

#### Examples

```bash
# Validate syntax
common-repo validate

# Also check that repos are accessible
common-repo validate --check-repos

# Strict mode (fail on warnings)
common-repo validate --strict
```

### `cache` - Manage Cache

Manage the repository cache.

```bash
common-repo cache <SUBCOMMAND> [OPTIONS]
```

#### Subcommands

**`list`** - List cached repositories
```bash
common-repo cache list
```

**`clean`** - Clean cached repositories
```bash
common-repo cache clean
```

#### Options

| Option | Description |
|--------|-------------|
| `--cache-root <DIR>` | Cache directory |

#### Examples

```bash
# List all cached repos
common-repo cache list

# Clean the cache
common-repo cache clean
```

### `tree` - Show Inheritance Tree

Display the repository inheritance tree.

```bash
common-repo tree [OPTIONS]
```

#### Options

| Option | Description |
|--------|-------------|
| `-c, --config <FILE>` | Path to config file (default: `.common-repo.yaml`) |
| `--cache-root <DIR>` | Cache directory |
| `--depth <NUM>` | Maximum depth to display (omit for full tree) |

#### Examples

```bash
# Show full inheritance tree
common-repo tree

# Show only first two levels
common-repo tree --depth 2
```

#### Output

```
my-project
├── github.com/common-repo/rust-cli@v2.0.0
│   └── github.com/common-repo/base@v1.0.0
└── github.com/common-repo/pre-commit@v1.5.0
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `COMMON_REPO_CONFIG` | Default config file path |
| `COMMON_REPO_CACHE` | Default cache directory |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Configuration error |
| 3 | Network error |

## Common Workflows

### First-Time Setup

```bash
# Initialize config
common-repo init

# Edit .common-repo.yaml to add your repos

# Preview what would be created
common-repo ls
common-repo diff

# Apply
common-repo apply
```

### Regular Maintenance

```bash
# Check for updates
common-repo check --updates

# Review and apply updates
common-repo update --dry-run
common-repo update
```

### Debugging

```bash
# Verbose output
common-repo apply --verbose --log-level debug

# Validate configuration
common-repo validate --check-repos --strict

# View inheritance
common-repo tree
common-repo info
```
