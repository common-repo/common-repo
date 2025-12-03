# common-repo

Manage repository configuration files as dependencies. Define inheritance in `.common-repo.yaml`, pull files from multiple Git repositories, and merge them with version pinning.

## Install

```bash
# One-liner
curl -fsSL https://raw.githubusercontent.com/common-repo/common-repo/main/install.sh | sh

# From source
cargo install --git https://github.com/common-repo/common-repo
```

## Usage

Create `.common-repo.yaml` in your repository:

```yaml
- repo:
    url: https://github.com/your-org/shared-configs
    ref: v1.0.0
    with:
      - include: [".github/**", ".pre-commit-config.yaml"]
      - exclude: [".git/**"]

- repo:
    url: https://github.com/your-org/ci-templates
    ref: main
    with:
      - include: [".github/workflows/*.yml"]
      - rename:
          ".github/workflows/ci.yml": ".github/workflows/build.yml"
```

Then run:

```bash
common-repo ls      # List files that would be created
common-repo diff    # Preview changes
common-repo apply   # Apply configuration
```

### Merging files

Merge fragments into existing files (YAML, JSON, TOML, INI, Markdown):

```yaml
- yaml:
    source: ci-jobs.yml
    dest: .github/workflows/ci.yml
    path: jobs.test
    append: true
```

### Checking for updates

```bash
common-repo check --updates   # Check for newer versions
common-repo update            # Update refs
```

## Commands

```
init       Create a new .common-repo.yaml
apply      Apply configuration (runs 6-phase pipeline)
diff       Preview changes
ls         List files that would be created
tree       Show inheritance tree
check      Validate configuration, check for updates
update     Update refs to newer versions
validate   Validate configuration syntax
info       Show configuration overview
cache      Manage repository cache
```

## Documentation

| Guide | Description |
|-------|-------------|
| [Getting Started](docs/getting-started.md) | Installation and first steps |
| [Configuration](docs/configuration.md) | All operators and options |
| [CLI Reference](docs/cli.md) | Command documentation |
| [Recipes](docs/recipes.md) | Common patterns |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, testing, and guidelines.

## License

[Add your license here]
