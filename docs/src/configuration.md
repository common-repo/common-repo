# Configuration Reference

This document covers all operators and options available in `.common-repo.yaml`.

<!-- BEGIN mktoc {"min_depth": 2, "max_depth": 3} -->

- [Configuration File](#configuration-file)
  - [Operator Quick Reference](#operator-quick-reference)
- [Core Operators](#core-operators)
  - [`repo` - Inherit from a Repository](#repo---inherit-from-a-repository)
  - [`include` - Add Files](#include---add-files)
  - [`exclude` - Remove Files](#exclude---remove-files)
  - [`rename` - Transform Paths](#rename---transform-paths)
  - [`template` - Mark Template Files](#template---mark-template-files)
  - [`template-vars` - Define Variables](#template-vars---define-variables)
  - [`tools` - Validate Required Tools](#tools---validate-required-tools)
- [Merge Operators](#merge-operators)
  - [Path Syntax](#path-syntax)
  - [`yaml` - Merge YAML Files](#yaml---merge-yaml-files)
  - [`json` - Merge JSON Files](#json---merge-json-files)
  - [`toml` - Merge TOML Files](#toml---merge-toml-files)
  - [`ini` - Merge INI Files](#ini---merge-ini-files)
  - [`markdown` - Merge Markdown Files](#markdown---merge-markdown-files)
- [Operation Order](#operation-order)
  - [Example Order](#example-order)
- [Complete Example](#complete-example)
<!-- END mktoc -->

## Configuration File

The `.common-repo.yaml` file is a list of operations executed in order. Each operation is a YAML map with a single key indicating the operator type.

### Operator Quick Reference

| Operator | Description |
|----------|-------------|
| [`repo`](#repo---inherit-from-a-repository) | Inherit files from a remote Git repository |
| [`include`](#include---add-files) | Add files from the current repository |
| [`exclude`](#exclude---remove-files) | Remove files from the in-memory filesystem |
| [`rename`](#rename---transform-paths) | Transform file paths using regex patterns |
| [`template`](#template---mark-template-files) | Mark files for variable substitution |
| [`template-vars`](#template-vars---define-variables) | Define variables for template substitution |
| [`tools`](#tools---validate-required-tools) | Validate that required tools are installed |
| [`yaml`](#yaml---merge-yaml-files) | Merge YAML configuration fragments |
| [`json`](#json---merge-json-files) | Merge JSON configuration fragments |
| [`toml`](#toml---merge-toml-files) | Merge TOML configuration fragments |
| [`ini`](#ini---merge-ini-files) | Merge INI configuration fragments |
| [`markdown`](#markdown---merge-markdown-files) | Merge markdown document fragments |

Example configuration:

```yaml
# .common-repo.yaml
- repo: { ... }
- include: [ ... ]
- exclude: [ ... ]
- rename: [ ... ]
- template: [ ... ]
- template-vars: { ... }
- tools: [ ... ]
- yaml: { ... }
- json: { ... }
- toml: { ... }
- ini: { ... }
- markdown: { ... }
```

## Core Operators

### `repo` - Inherit from a Repository

Pull files from a remote Git repository.

```yaml
- repo:
    url: https://github.com/owner/repo
    ref: v1.0.0
```

#### Options

| Option | Required | Description |
|--------|----------|-------------|
| `url` | Yes | Git repository URL |
| `ref` | Yes | Git reference (tag, branch, or commit SHA) |
| `path` | No | Sub-directory to use as root |
| `with` | No | Inline operations to apply |

#### Examples

**Basic inheritance:**
```yaml
- repo:
    url: https://github.com/common-repo/rust-cli
    ref: v1.2.0
```

**Inherit a sub-directory:**
```yaml
# Only pull files from the 'templates/rust' directory
- repo:
    url: https://github.com/common-repo/templates
    ref: main
    path: templates/rust
```

**Inline filtering with `with`:**
```yaml
- repo:
    url: https://github.com/common-repo/configs
    ref: v2.0.0
    with:
      - include: [".github/**", ".pre-commit-config.yaml"]
      - exclude: [".github/CODEOWNERS"]
      - rename:
          - ".github/workflows/ci-template.yml": ".github/workflows/ci.yml"
```

### `include` - Add Files

Add files from the current repository to the output based on glob patterns.

```yaml
- include:
    - "**/*"           # All files
    - ".*"             # Hidden files at root
    - ".*/**/*"        # All files in hidden directories
```

#### Patterns

- `**/*` - All files recursively
- `*.rs` - All Rust files in current directory
- `src/**/*.rs` - All Rust files under src/
- `.*` - Hidden files (dotfiles) at root
- `.*/**/*` - All files in hidden directories

### `exclude` - Remove Files

Remove files from the in-memory filesystem based on glob patterns.

```yaml
- exclude:
    - ".git/**"
    - "target/**"
    - "**/*.bak"
    - "node_modules/**"
```

### `rename` - Transform Paths

Transform file paths using regex patterns with capture group placeholders.

```yaml
- rename:
    - "old-name/(.*)": "new-name/%[1]s"
    - "^templates/(.*)": "%[1]s"
    - "(.+)\\.template$": "%[1]s"
```

#### Placeholders

- `%[1]s` - First capture group
- `%[2]s` - Second capture group
- etc.

#### Examples

**Strip a directory prefix:**
```yaml
- rename:
    - "^files/(.*)": "%[1]s"
```
Result: `files/config.yaml` becomes `config.yaml`

**Move files to a subdirectory:**
```yaml
- rename:
    - "^(.+\\.md)$": "docs/%[1]s"
```
Result: `README.md` becomes `docs/README.md`

**Rename file extensions:**
```yaml
- rename:
    - "(.+)\\.template$": "%[1]s"
```
Result: `config.yaml.template` becomes `config.yaml`

### `template` - Mark Template Files

Mark files for variable substitution. Templates are processed after all files are collected.

```yaml
- template:
    - "**/*.template"
    - ".github/workflows/*.yml"
    - "Cargo.toml"
```

Template files can contain `${VARIABLE}` placeholders that get replaced with values from `template-vars` or environment variables.

### `template-vars` - Define Variables

Define variables for template substitution.

```yaml
- template-vars:
    project_name: my-project
    author: Jane Doe
    rust_version: "1.75"
```

#### Environment Variable Defaults

Use shell-like syntax for environment variable defaults:

```yaml
- template-vars:
    project: ${PROJECT_NAME:-default-project}
    ci_enabled: ${CI:-false}
```

- `${VAR}` - Use environment variable VAR
- `${VAR:-default}` - Use VAR if set, otherwise use "default"

#### Variable Cascading

Variables cascade through the inheritance tree. Child repos can override ancestor variables:

```yaml
# In parent repo
- template-vars:
    log_level: info

# In child repo (overrides parent)
- template-vars:
    log_level: debug
```

### `tools` - Validate Required Tools

Check that required tools are installed with correct versions.

```yaml
- tools:
    - rustc: ">=1.70"
    - cargo: "*"
    - pre-commit: "^3.0"
    - node: "~18.0"
```

#### Version Constraints

| Syntax | Meaning |
|--------|---------|
| `*` | Any version |
| `1.70` | Exactly 1.70 |
| `>=1.70` | 1.70 or higher |
| `^1.70` | Compatible with 1.70 (>=1.70.0, <2.0.0) |
| `~1.70` | Approximately 1.70 (>=1.70.0, <1.71.0) |

This operator validates but does not install tools. Warnings are issued for missing or incompatible versions.

## Merge Operators

Merge operators intelligently combine configuration fragments into destination files.

### Source-Declared Merge (defer/auto-merge)

All merge operators support two additional options for source-declared merge behavior:

| Option | Type | Description |
|--------|------|-------------|
| `auto-merge` | string | Shorthand: sets source=dest to this value and implies defer=true |
| `defer` | bool | When true, this operation only runs when repo is used as a source |

**`auto-merge`** is the preferred syntax when the source and destination filenames are the same:

```yaml
# In source repo's .common-repo.yaml
- markdown:
    auto-merge: CLAUDE.md     # source=dest=CLAUDE.md, defer=true
    section: "## Rules"
    append: true
```

**`defer: true`** is used when source and destination paths differ:

```yaml
- yaml:
    source: config/base.yaml
    dest: config.yaml
    path: settings
    defer: true               # Only applies when repo is inherited
```

See [Source-Declared Merge Behavior](authoring-source-repos.md#source-declared-merge-behavior) for detailed usage.

### Path Syntax

The `path` option in merge operators supports multiple notations for navigating nested structures:

| Syntax | Example | Description |
|--------|---------|-------------|
| Dot notation | `foo.bar.baz` | Access nested keys |
| Bracket notation | `foo["bar"]` or `foo['bar']` | Access keys with special characters |
| Array indices | `items[0]` or `items[1].name` | Access array elements by index |
| Escaped dots | `foo\.bar` | Literal dot in key name |
| Mixed | `servers[0].config["special.key"]` | Combine notations |

**Examples:**

```yaml
# Navigate to nested object
- yaml:
    source: labels.yml
    dest: config.yml
    path: metadata.labels

# Access array element
- json:
    source: script.json
    dest: package.json
    path: scripts[0]

# Key with special characters
- toml:
    source: deps.toml
    dest: Cargo.toml
    path: dependencies["my-crate"]

# Escaped dot in key name
- yaml:
    source: fragment.yml
    dest: config.yml
    path: config\.v2.settings
```

### `yaml` - Merge YAML Files

```yaml
- yaml:
    source: fragment.yml
    dest: config.yml
```

#### Options

| Option | Required | Default | Description |
|--------|----------|---------|-------------|
| `source` | Yes* | - | Source fragment file |
| `dest` | Yes* | - | Destination file |
| `auto-merge` | No | - | Shorthand: sets source=dest, implies defer=true |
| `defer` | No | false | Only apply when repo is used as a source |
| `path` | No | root | Path to merge at (see [Path Syntax](#path-syntax)) |
| `array_mode` | No | replace | Array handling: `replace`, `append`, or `append_unique` |
| `append` | No | false | Deprecated: use `array_mode: append` instead |

*Either `source`+`dest` or `auto-merge` is required

#### Array Merge Modes

| Mode | Description |
|------|-------------|
| `replace` | Replace destination array with source array (default) |
| `append` | Append source items to the end of destination array |
| `append_unique` | Append only items not already in destination array |

#### Examples

**Merge at root:**
```yaml
- yaml:
    source: extra-config.yml
    dest: config.yml
```

**Merge at a specific path:**
```yaml
# Merge labels into metadata.labels
- yaml:
    source: labels.yml
    dest: kubernetes.yml
    path: metadata.labels
```

**Append to a list:**
```yaml
# Add items to an existing list
- yaml:
    source: extra-items.yml
    dest: config.yml
    path: items
    append: true
```

**Add jobs to CI workflow:**
```yaml
# Merge shared CI jobs into existing workflow
- yaml:
    source: ci-jobs.yml
    dest: .github/workflows/ci.yml
    path: jobs
    append: true
```

### `json` - Merge JSON Files

```yaml
- json:
    source: fragment.json
    dest: package.json
    path: dependencies
```

#### Options

| Option | Required | Default | Description |
|--------|----------|---------|-------------|
| `source` | Yes* | - | Source fragment file |
| `dest` | Yes* | - | Destination file |
| `auto-merge` | No | - | Shorthand: sets source=dest, implies defer=true |
| `defer` | No | false | Only apply when repo is used as a source |
| `path` | No | root | Dot-notation path to merge at |
| `append` | No | false | Append to arrays instead of replace |
| `position` | No | - | Where to append: `start` or `end` (only used when `append: true`) |

*Either `source`+`dest` or `auto-merge` is required

#### Examples

**Add dependencies to package.json:**
```yaml
- json:
    source: extra-deps.json
    dest: package.json
    path: dependencies
```

**Append scripts:**
```yaml
- json:
    source: scripts.json
    dest: package.json
    path: scripts
    append: true
    position: start
```

**Merge TypeScript compiler options:**
```yaml
- json:
    source: strict-options.json
    dest: tsconfig.json
    path: compilerOptions
```

### `toml` - Merge TOML Files

```yaml
- toml:
    source: fragment.toml
    dest: Cargo.toml
    path: dependencies
```

#### Options

| Option | Required | Default | Description |
|--------|----------|---------|-------------|
| `source` | Yes* | - | Source fragment file |
| `dest` | Yes* | - | Destination file |
| `auto-merge` | No | - | Shorthand: sets source=dest, implies defer=true |
| `defer` | No | false | Only apply when repo is used as a source |
| `path` | No | root | Path to merge at (see [Path Syntax](#path-syntax)) |
| `array_mode` | No | replace | Array handling: `replace`, `append`, or `append_unique` |
| `append` | No | false | Deprecated: use `array_mode: append` instead |
| `preserve-comments` | No | false | Keep comments in output |

*Either `source`+`dest` or `auto-merge` is required

See [Array Merge Modes](#array-merge-modes) for details on array handling options.

#### Examples

**Add Cargo dependencies:**
```yaml
- toml:
    source: common-deps.toml
    dest: Cargo.toml
    path: dependencies
```

**Add dev-dependencies with comments preserved:**
```yaml
- toml:
    source: test-deps.toml
    dest: Cargo.toml
    path: dev-dependencies
    preserve-comments: true
```

**Merge pyproject.toml settings:**
```yaml
- toml:
    source: lint-settings.toml
    dest: pyproject.toml
    path: tool.ruff
```

### `ini` - Merge INI Files

```yaml
- ini:
    source: fragment.ini
    dest: config.ini
    section: database
```

#### Options

| Option | Required | Default | Description |
|--------|----------|---------|-------------|
| `source` | Yes* | - | Source fragment file |
| `dest` | Yes* | - | Destination file |
| `auto-merge` | No | - | Shorthand: sets source=dest, implies defer=true |
| `defer` | No | false | Only apply when repo is used as a source |
| `section` | No | - | INI section to merge into |
| `append` | No | false | Append values instead of replace |
| `allow-duplicates` | No | false | Allow duplicate keys |

*Either `source`+`dest` or `auto-merge` is required

#### Examples

**Merge database settings:**
```yaml
- ini:
    source: db-settings.ini
    dest: config.ini
    section: database
```

**Add editor configuration rules:**
```yaml
# Merge settings into the [*] section for all files
- ini:
    source: editor-rules.ini
    dest: .editorconfig
    section: "*"
```

**Configure git settings:**
```yaml
- ini:
    source: git-aliases.ini
    dest: .gitconfig
    section: alias
```

### `markdown` - Merge Markdown Files

```yaml
- markdown:
    source: installation.md
    dest: README.md
    section: "## Installation"
```

#### Options

| Option | Required | Default | Description |
|--------|----------|---------|-------------|
| `source` | Yes* | - | Source fragment file |
| `dest` | Yes* | - | Destination file |
| `auto-merge` | No | - | Shorthand: sets source=dest, implies defer=true |
| `defer` | No | false | Only apply when repo is used as a source |
| `section` | No | - | Heading to merge under |
| `level` | No | 2 | Heading level (1-6) |
| `append` | No | false | Append to section |
| `position` | No | - | Where to insert: `start` or `end` (only used when `append: true`) |
| `create-section` | No | false | Create section if missing |

*Either `source`+`dest` or `auto-merge` is required

#### Examples

**Add installation instructions:**
```yaml
- markdown:
    source: install-instructions.md
    dest: README.md
    section: "## Installation"
    append: true
    position: end
    create-section: true
```

**Add contributing section:**
```yaml
# Insert a standard Contributing section in the README
- markdown:
    source: contributing-section.md
    dest: README.md
    section: "## Contributing"
    create-section: true
```

**Merge CLAUDE.md rules (source-declared):**
```yaml
# In source repo - will only apply when inherited
- markdown:
    auto-merge: CLAUDE.md
    section: "## Rules"
    append: true
```

## Operation Order

Operations execute in the order they appear in the configuration file. For inheritance:

1. Ancestor repos are processed before parent repos
2. Parent repos are processed before the local repo
3. Siblings are processed in declaration order

This means later operations can override earlier ones, and child repos can customize what they inherit from ancestors.

### Example Order

```yaml
# local .common-repo.yaml
- repo: {url: A, ref: v1}  # A is processed first (including A's ancestors)
- repo: {url: B, ref: v2}  # B is processed second (including B's ancestors)
- include: ["local/**"]    # Local operations are processed last
```

If A inherits from C, and B inherits from D:
```
Processing order: C -> A -> D -> B -> local
```

## Complete Example

Here's a complete configuration showing multiple operators:

```yaml
# .common-repo.yaml

# Inherit base Rust CLI configuration
- repo:
    url: https://github.com/common-repo/rust-cli
    ref: v2.0.0
    with:
      - include: ["**/*"]
      - exclude: [".git/**", "target/**"]

# Inherit pre-commit configuration
- repo:
    url: https://github.com/common-repo/pre-commit-rust
    ref: v1.5.0
    with:
      - include: [".pre-commit-config.yaml"]

# Include local files
- include:
    - src/**
    - Cargo.toml
    - README.md

# Exclude generated files
- exclude:
    - "**/*.generated.rs"

# Rename template files
- rename:
    - "(.+)\\.template$": "%[1]s"

# Define template variables
- template-vars:
    project_name: ${PROJECT_NAME:-my-project}
    author: ${AUTHOR:-Your Name}
    rust_edition: "2021"

# Mark files as templates
- template:
    - Cargo.toml
    - README.md

# Require tools
- tools:
    - rustc: ">=1.70"
    - cargo: "*"
    - pre-commit: ">=3.0"

# Merge additional dependencies into Cargo.toml
- toml:
    source: extra-deps.toml
    dest: Cargo.toml
    path: dependencies

# Add CI workflow
- yaml:
    source: ci-jobs.yml
    dest: .github/workflows/ci.yml
    path: jobs
    append: true
```
