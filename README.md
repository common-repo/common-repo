# common-repo

[![CI](https://github.com/common-repo/common-repo/actions/workflows/ci.yml/badge.svg)](https://github.com/common-repo/common-repo/actions/workflows/ci.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE.md)
[![Docs](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://common-repo.github.io/common-repo/)

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

## Documentation

Full documentation is available at [common-repo.github.io/common-repo](https://common-repo.github.io/common-repo/).

| Guide | Description |
|-------|-------------|
| [Getting Started](docs/src/getting-started.md) | Installation and first steps |
| [Configuration](docs/src/configuration.md) | All operators and options |
| [CLI Reference](docs/src/cli.md) | Command documentation |
| [Recipes](docs/src/recipes.md) | Common patterns |
| [Troubleshooting](docs/src/troubleshooting.md) | Common issues and solutions |
| [Authoring Source Repos](docs/src/authoring-source-repos.md) | Create your own source repos |

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

**YAML** - Add jobs to CI workflows ([docs](docs/src/configuration.md#yaml---merge-yaml-files)):
```yaml
- yaml:
    source: ci-jobs.yml
    dest: .github/workflows/ci.yml
    path: jobs
    append: true
```

**JSON** - Add scripts to package.json ([docs](docs/src/configuration.md#json---merge-json-files)):
```yaml
- json:
    source: scripts.json
    dest: package.json
    path: scripts
```

**TOML** - Add dependencies to Cargo.toml ([docs](docs/src/configuration.md#toml---merge-toml-files)):
```yaml
- toml:
    source: common-deps.toml
    dest: Cargo.toml
    path: dependencies
```

**INI** - Add editor settings ([docs](docs/src/configuration.md#ini---merge-ini-files)):
```yaml
- ini:
    source: editor-rules.ini
    dest: .editorconfig
    section: "*"
```

**Markdown** - Add sections to README ([docs](docs/src/configuration.md#markdown---merge-markdown-files)):
```yaml
- markdown:
    source: contributing-section.md
    dest: README.md
    section: "## Contributing"
    create-section: true
```

### Checking for updates

```bash
common-repo check --updates   # Check for newer versions
common-repo update            # Update refs
```

## Commands

```
init         Create a new .common-repo.yaml
add          Add a repository to existing config
apply        Apply configuration (runs 6-phase pipeline)
diff         Preview changes
ls           List files that would be created
tree         Show inheritance tree
check        Validate configuration, check for updates
update       Update refs to newer versions
validate     Validate configuration syntax
info         Show configuration overview
cache        Manage repository cache
completions  Generate shell completion scripts
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, testing, and guidelines.

## License

This project is licensed under the [GNU Affero General Public License v3.0](LICENSE.md).
