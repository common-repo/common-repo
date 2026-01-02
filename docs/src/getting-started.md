# Getting Started with common-repo

This guide walks you through installing common-repo and applying your first configuration.

## Installation

### Quick Install (Recommended)

Install the latest release with a single command:

```bash
curl -fsSL https://raw.githubusercontent.com/common-repo/common-repo/main/install.sh | sh
```

This automatically detects your platform and installs the appropriate binary to `~/.local/bin`. The installer also creates a short alias `cr` so you can use either `common-repo` or `cr` to run commands.

**Installation options:**

```bash
# Install a specific version
VERSION=v0.20.0 curl -fsSL https://raw.githubusercontent.com/common-repo/common-repo/main/install.sh | sh

# Install to a custom directory
INSTALL_DIR=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/common-repo/common-repo/main/install.sh | sh

# Also install prek (fast pre-commit hooks)
INSTALL_PREK=1 curl -fsSL https://raw.githubusercontent.com/common-repo/common-repo/main/install.sh | sh

# Install with sudo (for system-wide installation)
curl -fsSL https://raw.githubusercontent.com/common-repo/common-repo/main/install.sh | sudo sh

# Skip creating the 'cr' alias
SKIP_ALIAS=1 curl -fsSL https://raw.githubusercontent.com/common-repo/common-repo/main/install.sh | sh

# Use GitHub token to avoid API rate limits (useful in CI)
GITHUB_TOKEN=ghp_xxx curl -fsSL https://raw.githubusercontent.com/common-repo/common-repo/main/install.sh | sh
```

### From Pre-built Binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/common-repo/common-repo/releases).

Available platforms:
- Linux x86_64 (glibc): `common-repo-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz`
- Linux x86_64 (musl): `common-repo-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz`
- Linux ARM64: `common-repo-vX.Y.Z-aarch64-unknown-linux-gnu.tar.gz`
- macOS ARM64 (Apple Silicon): `common-repo-vX.Y.Z-aarch64-apple-darwin.tar.gz`
- Windows x86_64: `common-repo-vX.Y.Z-x86_64-pc-windows-msvc.zip`

### From Source

```bash
# Clone the repository
git clone https://github.com/common-repo/common-repo.git
cd common-repo

# Build and install
cargo install --path .
```

Or install directly from GitHub:

```bash
cargo install --git https://github.com/common-repo/common-repo
```

### Verify Installation

```bash
common-repo --version
# Or use the short alias
cr --version
```

## Your First Configuration

### 1. Initialize a Configuration File

Navigate to your project directory and run the interactive wizard:

```bash
cd your-project
common-repo init
```

The wizard will:
- Prompt you to enter repository URLs (supports GitHub shorthand like `org/repo`)
- Auto-detect the latest semver tag for each repository
- Optionally set up pre-commit hooks
- Generate a ready-to-use `.common-repo.yaml`

**Alternative: Initialize from an existing repo**

```bash
# Full URL
common-repo init https://github.com/your-org/shared-configs

# GitHub shorthand
common-repo init your-org/shared-configs
```

### 2. Define What to Inherit

Edit `.common-repo.yaml` to inherit from a repository. Here's a simple example that pulls pre-commit configuration:

```yaml
# .common-repo.yaml

# Inherit files from a remote repository
- repo:
    url: https://github.com/your-org/shared-configs
    ref: v1.0.0
    with:
      - include: ["**/*"]
      - exclude: [".git/**", "README.md"]
```

### 3. Preview Changes

Before applying, see what files would be created:

```bash
# List files that would be created
common-repo ls

# See a diff of changes
common-repo diff
```

### 4. Apply the Configuration

```bash
# Dry run first (recommended)
common-repo apply --dry-run

# Apply for real
common-repo apply
```

## Understanding the Output

After running `common-repo apply`, you'll see:

- Files pulled from inherited repositories
- Any merge operations applied (YAML, JSON, etc.)
- Template variables substituted

The tool caches cloned repositories at `~/.common-repo/cache/` for faster subsequent runs.

## Common Workflows

### Check for Updates

```bash
# See if inherited repos have newer versions
common-repo check --updates
```

### Update to Latest Compatible Versions

```bash
# Update refs to latest compatible versions (minor/patch only)
common-repo update

# Include breaking changes (major versions)
common-repo update --latest
```

### View Inheritance Tree

```bash
# See how repos inherit from each other
common-repo tree
```

## Example: Inheriting CI/CD Configuration

Here's a practical example that inherits GitHub Actions workflows:

```yaml
# .common-repo.yaml

# Pull CI workflows from your org's standard configs
- repo:
    url: https://github.com/your-org/ci-templates
    ref: v2.1.0
    with:
      - include: [".github/**"]

# Customize the project name in templates
- template-vars:
    project_name: my-awesome-project
    rust_version: "1.75"

# Mark workflow files as templates for variable substitution
- template:
    - ".github/workflows/*.yml"
```

## Next Steps

- [Configuration Reference](configuration.md) - All operators and options
- [CLI Reference](cli.md) - Complete command documentation
- [Recipes](recipes.md) - Common patterns and examples
