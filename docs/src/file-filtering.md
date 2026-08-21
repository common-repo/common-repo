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

Patterns use glob syntax, matched against each file's full relative path (for example, `src/main.rs` or `.github/workflows/ci.yml`). The matcher differs from shell globbing in two ways:

- `*` matches any sequence of characters, including `/`. The pattern `*.rs` matches `src/main.rs`, not only top-level `.rs` files.
- Patterns match dotfiles. `*` and `**` match names that start with `.`, so `**/*` matches `.gitignore` and `.github/workflows/ci.yml` without any extra patterns.

| Pattern | Matches |
|---------|---------|
| `**/*` | Every file at any depth, dotfiles included |
| `*` | Same as `**/*`, because `*` crosses `/` |
| `**` | Same as `**/*` |
| `*.rs` | All `.rs` files at any depth |
| `**/*.rs` | All `.rs` files at any depth (`**` matches zero or more directories, so top-level `.rs` files match too) |
| `src/**` | Everything under `src/`, dotfiles included |
| `.github/**` | Everything under `.github/` |

Prefer `**/*` when you want every file.

`*/**` is not an "every file" pattern. The `/` between `*` and `**` is literal, so the pattern matches only files at least one directory deep — a top-level file such as `README.md` does not match.

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

Broad patterns already match dotfiles. `**/*` matches top-level dotfiles such as `.gitignore` and files inside hidden directories such as `.github/workflows/ci.yml`, so separate `.*` or `.*/**` patterns add nothing:

```yaml
- repo:
    url: https://github.com/your-org/dotfiles
    ref: v1.0.0
    with:
      - include: ["**/*"]   # matches dotfiles and hidden directories too
```

To pull only hidden files, use a pattern with a literal leading dot. `.github/**` matches everything under `.github/`, and `.*` matches every path whose first character is a dot, including files inside those hidden directories.

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

**Hidden files missing from upstream?** Check the `include` patterns applied to the upstream repo. Broad patterns such as `**/*` already match dotfiles, so no separate dotfile pattern is needed; a narrow pattern such as `src/**` matches dotfiles only when they are under `src/`. Local project dotfiles are loaded automatically.
