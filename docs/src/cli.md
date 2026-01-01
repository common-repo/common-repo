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

### `add` - Add Repository

Add a repository to an existing `.common-repo.yaml` configuration file. Automatically detects the latest semver tag.

```bash
common-repo add [OPTIONS] <URI>
```

#### Arguments

| Argument | Description |
|----------|-------------|
| `<URI>` | Repository URL to add (e.g., `https://github.com/org/repo` or `org/repo` GitHub shorthand) |

#### Options

| Option | Description |
|--------|-------------|
| `-q, --quiet` | Non-interactive mode: create minimal config without prompting if none exists |

#### Examples

```bash
# Add a repo to existing config
common-repo add your-org/shared-configs

# Add using full URL
common-repo add https://github.com/your-org/shared-configs

# Create new config with --quiet (non-interactive)
common-repo add --quiet your-org/shared-configs
```

#### Behavior

- If `.common-repo.yaml` exists: appends the new repository before the `include` section
- If no config exists: prompts for confirmation to create a minimal config (use `--quiet` to skip prompt)
- Automatically fetches and uses the latest semver tag, or falls back to `main` if no tags found
- Warns when adding repositories with only 0.x.x versions (unstable API)

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

### `completions` - Generate Shell Completions

Generate shell completion scripts for tab-completion support.

```bash
common-repo completions <SHELL>
```

#### Arguments

| Argument | Description |
|----------|-------------|
| `<SHELL>` | Shell to generate completions for: `bash`, `zsh`, `fish`, `powershell`, `elvish` |

#### Examples

```bash
# Generate bash completions
common-repo completions bash > ~/.local/share/bash-completion/completions/common-repo

# Generate zsh completions
common-repo completions zsh > ~/.zfunc/_common-repo

# Generate fish completions
common-repo completions fish > ~/.config/fish/completions/common-repo.fish

# Generate PowerShell completions
common-repo completions powershell >> $PROFILE
```

#### Installation

**Bash**

```bash
# Option 1: User-level installation
mkdir -p ~/.local/share/bash-completion/completions
common-repo completions bash > ~/.local/share/bash-completion/completions/common-repo

# Option 2: Source directly in .bashrc
echo 'eval "$(common-repo completions bash)"' >> ~/.bashrc
```

**Zsh**

```bash
# Add to fpath (recommended)
mkdir -p ~/.zfunc
common-repo completions zsh > ~/.zfunc/_common-repo

# Add to .zshrc (before compinit)
echo 'fpath=(~/.zfunc $fpath)' >> ~/.zshrc
echo 'autoload -Uz compinit && compinit' >> ~/.zshrc
```

**Fish**

```bash
common-repo completions fish > ~/.config/fish/completions/common-repo.fish
```

**PowerShell**

```powershell
# Add to your PowerShell profile
common-repo completions powershell >> $PROFILE
```

**Elvish**

```bash
common-repo completions elvish >> ~/.elvish/rc.elv
```

**Note on `cr` alias:** If you installed via the shell installer, the `cr` alias is available. Completions generated for `common-repo` will work when you type `common-repo`. For the `cr` alias, you can create a symlink or alias in your shell configuration, or generate completions separately using `cr completions <shell>`.

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

Create a new `.common-repo.yaml` configuration file. By default, launches an interactive wizard that guides you through adding repositories with automatic version detection.

```bash
common-repo init [OPTIONS] [URI]
```

#### Arguments

| Argument | Description |
|----------|-------------|
| `[URI]` | Repository URL to initialize from (e.g., `https://github.com/org/repo` or `org/repo` GitHub shorthand) |

#### Options

| Option | Description |
|--------|-------------|
| `-i, --interactive` | Interactive setup wizard (default when no URI provided) |
| `-f, --force` | Overwrite existing configuration |

#### Examples

```bash
# Launch interactive wizard (default)
common-repo init

# Initialize from a specific repository
common-repo init https://github.com/your-org/shared-configs

# Use GitHub shorthand
common-repo init your-org/shared-configs

# Overwrite existing config
common-repo init --force

# Explicitly use interactive mode
common-repo init --interactive
```

#### Interactive Wizard

When run without arguments, `init` launches an interactive wizard that:

1. Prompts for repository URLs (supports GitHub shorthand like `org/repo`)
2. Auto-detects the latest semver tag for each repository
3. Falls back to `main` branch if no semver tags are found
4. Optionally sets up pre-commit hooks (detects `prek` or `pre-commit` CLI)
5. Generates a ready-to-use `.common-repo.yaml`

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
# Initialize config interactively
common-repo init

# Or add repos one at a time
common-repo add your-org/shared-configs
common-repo add your-org/ci-templates

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

### Shell Completions

Enable tab-completion for faster command entry:

```bash
# Bash
common-repo completions bash > ~/.local/share/bash-completion/completions/common-repo

# Zsh
common-repo completions zsh > ~/.zfunc/_common-repo

# Fish
common-repo completions fish > ~/.config/fish/completions/common-repo.fish
```

See [`completions`](#completions---generate-shell-completions) for detailed installation instructions.

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
