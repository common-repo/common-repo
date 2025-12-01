# Getting Started with common-repo

This guide walks you through installing common-repo and applying your first configuration.

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/common-repo/common-repo.git
cd common-repo

# Build and install
cargo install --path .
```

### Verify Installation

```bash
common-repo --version
```

## Your First Configuration

### 1. Initialize a Configuration File

Navigate to your project directory and create a `.common-repo.yaml`:

```bash
cd your-project
common-repo init
```

This creates a minimal configuration file with examples.

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
