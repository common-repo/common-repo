# common-repo

[![CI](https://github.com/common-repo/common-repo/actions/workflows/ci.yml/badge.svg)](https://github.com/common-repo/common-repo/actions/workflows/ci.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE.md)
[![crates.io](https://img.shields.io/crates/v/common-repo.svg)](https://crates.io/crates/common-repo)
[![docs.rs](https://docs.rs/common-repo/badge.svg)](https://docs.rs/common-repo)

Manage repository configuration files as dependencies. Define inheritance in `.common-repo.yaml`, pull files from multiple Git repositories, and merge them with version pinning.

## Install

### Shell installer (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/common-repo/common-repo/main/install.sh | sh
```

The installer creates both `common-repo` and a short alias `cr`. To skip the alias: `SKIP_ALIAS=1 sh -s < install.sh`

### cargo-binstall (pre-built binary)

```bash
cargo binstall common-repo
```

### From crates.io

```bash
cargo install common-repo
```

### From source (latest development)

```bash
cargo install --git https://github.com/common-repo/common-repo
```

### GitHub Releases

Download the latest release for your platform from [GitHub Releases](https://github.com/common-repo/common-repo/releases) and add it to your PATH.

### Platform notes

- **Linux/macOS**: Shell installer auto-detects architecture (x86_64, aarch64)
- **Windows**: Use `cargo install` or download from GitHub Releases
- **Nix**: `nix run github:common-repo/common-repo` (flake available)

### Minimum Supported Rust Version (MSRV)

This project supports Rust 1.85.0 and later. MSRV updates are considered non-breaking changes.

## Quick Start

```bash
# Initialize interactively - add repos, auto-detect versions
common-repo init

# Or initialize from an existing repo
common-repo init https://github.com/your-org/shared-configs
common-repo init your-org/shared-configs  # GitHub shorthand

# Add more repos to an existing config
common-repo add your-org/another-repo
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
add        Add a repository to existing config
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
| [Troubleshooting](docs/troubleshooting.md) | Common issues and solutions |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, testing, and guidelines.

## License

This project is licensed under the [GNU Affero General Public License v3.0](LICENSE.md).
