# File Filtering

This guide explains how to use `include` and `exclude` patterns to control which files are inherited from upstream repositories.

## Basic Syntax

Use `include` and `exclude` in the `with` clause of a `repo` operation:

```yaml
- repo:
    url: https://github.com/your-org/configs
    ref: v1.0.0
    with:
      - include: ["pattern1", "pattern2"]
      - exclude: ["pattern3"]
```

Or at the top level for local files:

```yaml
- include: ["src/**", "Cargo.toml"]
- exclude: ["**/*.bak"]
```

## Pattern Syntax

Patterns use glob syntax:

| Pattern | Matches |
|---------|---------|
| `*` | Any filename in current directory |
| `**` | Any path (recursive) |
| `*.rs` | All `.rs` files in current directory |
| `**/*.rs` | All `.rs` files recursively |
| `src/**` | Everything under `src/` |
| `.*` | Hidden files at root |
| `.*/**` | Everything in hidden directories |

## Order of Operations

Operations execute in the order they appear in the config file. For example:

1. `include` adds files to the output
2. `exclude` removes files from the output

When listed in this order, you can include a broad pattern and then exclude specific files:

```yaml
- repo:
    url: https://github.com/your-org/configs
    ref: v1.0.0
    with:
      - include: [".github/**"]
      - exclude: [".github/CODEOWNERS"]
```

## Common Patterns

### Include Only CI Files

Pull GitHub Actions workflows and nothing else:

```yaml
- repo:
    url: https://github.com/your-org/ci-templates
    ref: v1.0.0
    with:
      - include: [".github/workflows/*.yml"]
```

### Exclude Tests and Examples

Pull everything except test and example files:

```yaml
- repo:
    url: https://github.com/your-org/library
    ref: v2.0.0
    with:
      - include: ["**/*"]
      - exclude: ["tests/**", "examples/**", "**/*_test.rs"]
```

### Include Hidden Files from Upstream Repos

Dotfiles and hidden directories require explicit patterns:

```yaml
- repo:
    url: https://github.com/your-org/dotfiles
    ref: v1.0.0
    with:
      - include:
          - ".*"        # .gitignore, .editorconfig, etc.
          - ".*/**"     # .github/*, .vscode/*, etc.
```

> **Note:** This applies to files from upstream repositories. Local project
> dotfiles (e.g., `.editorconfig`, `.pre-commit-config.yaml`) are loaded
> automatically during the local file merge phase.

### Exclude Generated Files

Skip files that shouldn't be version-controlled:

```yaml
- repo:
    url: https://github.com/your-org/project
    ref: v1.0.0
    with:
      - include: ["**/*"]
      - exclude:
          - ".git/**"
          - "target/**"
          - "node_modules/**"
          - "**/*.generated.*"
```

### Multiple File Types

Include specific file types only:

```yaml
- repo:
    url: https://github.com/your-org/configs
    ref: v1.0.0
    with:
      - include:
          - "**/*.yml"
          - "**/*.yaml"
          - "**/*.toml"
          - "**/*.json"
```

## Combining with Other Operations

Operations in the `with` clause execute in declaration order (YAML order). Filtering does not have a fixed position relative to other operations:

```yaml
- repo:
    url: https://github.com/your-org/templates
    ref: v1.0.0
    with:
      # These run in the order listed
      - include: ["templates/**"]
      - exclude: ["templates/internal/**"]
      - rename:
          - "^templates/(.*)": "$1"
```

## Viewing Filtered Results

Check which files match your patterns:

```bash
# List all files that would be created
common-repo ls

# Filter the listing by pattern
common-repo ls --pattern "*.yml"

# Long format shows sizes
common-repo ls -l
```

## Troubleshooting

**Files not appearing?** Check that your include pattern matches. Use `common-repo ls` to see what's included.

**Too many files?** Add exclude patterns to filter out unwanted files.

**Hidden files missing from upstream?** Remember to explicitly include `.*` and `.*/**` patterns in the upstream repo's `with:` block. Local project dotfiles are loaded automatically.
