# Source-Declared Merge Behavior: Design Document

## Problem Statement

Currently, when a consumer wants to merge files from a source repository (rather than overwrite), they must explicitly declare the merge operator in their `.common-repo.yaml`:

```yaml
# Consumer config (current approach)
- repo: org/my-claude-rules
  ref: v1.0.0
  with:
    - markdown:
        source: CLAUDE.md
        dest: CLAUDE.md
        section: "## Inherited Rules"
        append: true
```

This creates boilerplate in every consumer and forces merge logic knowledge into consumers. Ideally, the source repository should be able to declare: "When this file is applied, merge it rather than overwrite."

### Primary Use Case

An organization maintains a source repo with CLAUDE.md rules that should merge into consumer repos. The source author knows best how their content should integrate.

## Design Options

### Option 1: Manifest File (`.common-repo-source.yaml`)

A dedicated manifest file in the source repository root declares merge behavior for files.

**Example:**
```yaml
# .common-repo-source.yaml in source repo
version: 1
merge-rules:
  - pattern: "CLAUDE.md"
    operator: markdown
    options:
      section: "## Inherited Rules"
      append: true
  - pattern: "*.toml"
    operator: toml
    options:
      path: dependencies
      array_mode: append
```

**Pros:**
- Clean separation of concerns
- Can declare rules for many files in one place
- Explicit and discoverable
- Supports glob patterns for batch rules
- Easy to validate schema
- Doesn't pollute individual files

**Cons:**
- Extra file to maintain
- May be overlooked by source authors
- Indirection: need to look in two places (file + manifest)
- Pattern matching adds complexity

### Option 2: File Metadata (Frontmatter/Comments)

Embed merge instructions in the files themselves using format-appropriate metadata.

**Examples:**

Markdown (YAML frontmatter):
```markdown
---
common-repo:
  merge: markdown
  section: "## Inherited Rules"
  append: true
---
# CLAUDE.md content here
```

YAML (comment directive):
```yaml
# common-repo: merge=yaml path=metadata.labels array_mode=append
metadata:
  labels:
    app: myapp
```

TOML (comment directive):
```toml
# common-repo: merge=toml path=dependencies
[dependencies]
serde = "1.0"
```

**Pros:**
- Self-contained: merge intent travels with file
- No extra files to maintain
- Immediately visible when viewing file
- Natural for Markdown (frontmatter is standard)

**Cons:**
- Pollutes file content
- Format-specific parsing required
- Frontmatter not standard for all formats
- Comments may be stripped by tools
- More complex to implement per-format
- Consumers might not want frontmatter in their files

### Option 3: Naming Convention (`*.merge.*`)

Use special file extensions to signal merge intent.

**Examples:**
```
CLAUDE.merge.md      -> merges into CLAUDE.md
config.merge.toml    -> merges into config.toml
settings.merge.yaml  -> merges into settings.yaml
```

**Pros:**
- Zero configuration
- Obvious intent from filename
- Simple implementation
- Works uniformly across formats
- No file content modification

**Cons:**
- Rigid: can't customize merge options per-file
- Doesn't work for exact filenames (can't rename CLAUDE.md)
- Default merge options only (no path, section, etc.)
- Unconventional pattern
- Source file has different name than destination

### Option 4: Hybrid - Manifest with File Defaults

Combine manifest approach with sensible defaults based on file type.

**Example:**
```yaml
# .common-repo-source.yaml
version: 1
defaults:
  markdown:
    append: true
    create-section: true
  yaml:
    array_mode: append_unique

merge-rules:
  - pattern: "CLAUDE.md"
    section: "## Inherited Rules"
  # Uses markdown defaults: append=true

  - pattern: "config/**/*.yaml"
    path: metadata
  # Uses yaml defaults: array_mode=append_unique
```

**Pros:**
- Reduces repetition with defaults
- Still explicit about which files merge
- Flexible per-file customization
- Single source of truth

**Cons:**
- More complex schema
- Implicit behavior from defaults

### Option 5: Hybrid - Marker Files

Use sidecar files to declare merge behavior without modifying originals.

**Example:**
```
CLAUDE.md              # The actual content
CLAUDE.md.merge.yaml   # Merge configuration
```

Contents of `CLAUDE.md.merge.yaml`:
```yaml
operator: markdown
section: "## Inherited Rules"
append: true
```

**Pros:**
- Doesn't modify original files
- Per-file configuration
- File stays next to its config
- Easy to add/remove merge behavior

**Cons:**
- File proliferation
- Easy to forget sidecar
- Unusual pattern

### Option 6: Reuse Existing Operator Syntax (RECOMMENDED)

Instead of inventing a new schema, reuse the existing `with:` operator syntax in a source-side config file. Source repos declare operations using the same familiar syntax consumers already use.

**Example:**
```yaml
# Source repo: .common-repo-source.yaml
with:
  - markdown:
      source: CLAUDE.md
      dest: CLAUDE.md
      section: "## Inherited Rules"
      append: true
  - yaml:
      source: config/labels.yaml
      dest: config.yaml
      path: metadata.labels
      array_mode: append
  - toml:
      source: deps.toml
      dest: Cargo.toml
      path: dependencies
```

These operations auto-apply when a consumer references the source repo.

**Pros:**
- **Zero new syntax**: Uses existing, familiar operator format
- **Minimal implementation**: Reuses existing `Operation` enum and parsing
- **Full operator power**: All existing options available (path, section, array_mode, etc.)
- **Validated already**: Existing schema validation works
- **Consistent mental model**: Same syntax in source and consumer configs
- **Easy to test**: Source authors can test their config as if it were a consumer

**Cons:**
- Slightly more verbose than a pattern-based approach
- No glob patterns (must list each file explicitly)
- `source` field is redundant when source == dest

**Simplification variant** - omit redundant fields:
```yaml
# When source == dest, allow shorthand
with:
  - markdown:
      file: CLAUDE.md  # Instead of source: + dest:
      section: "## Inherited Rules"
      append: true
```

## Consumer Override Mechanism

Regardless of source declaration, consumers should be able to:

1. **Disable source-declared merge**: Force overwrite instead
2. **Override merge options**: Change section, path, append mode, etc.
3. **Ignore specific files**: Skip certain source-declared merges

**Proposed syntax:**
```yaml
# Consumer .common-repo.yaml
- repo: org/my-claude-rules
  ref: v1.0.0
  override:
    # Disable merge for specific file
    - file: "CLAUDE.md"
      merge: false

    # Override merge options
    - file: "config.yaml"
      merge:
        path: custom.location
        array_mode: replace
```

Alternative: use `with:` clause to override (existing pattern):
```yaml
- repo: org/my-claude-rules
  ref: v1.0.0
  with:
    # Explicit operation overrides source-declared
    - copy:
        source: CLAUDE.md
        dest: CLAUDE.md
    # This replaces any source-declared merge
```

## Conflict Resolution

When multiple source repos declare merge for the same destination:

1. **Order-dependent**: Apply in config order (last wins for conflicts)
2. **Error on conflict**: Fail if two sources declare incompatible merges
3. **Consumer resolution**: Require consumer to explicitly resolve

**Recommendation**: Order-dependent with warnings. This matches current behavior where operations apply sequentially.

## Recommendation

**Primary: Option 6 (Reuse Existing Operator Syntax)**

### Rationale

1. **Minimal new code**: Reuses existing `Operation` enum, parsing, and validation
2. **Zero learning curve**: Source authors already know the `with:` syntax
3. **Battle-tested**: Existing operator implementations handle edge cases
4. **Consistent**: Same syntax works in both source and consumer configs
5. **Full flexibility**: All operator options immediately available
6. **Easy testing**: Source authors can validate their config locally

### Proposed Schema

```yaml
# .common-repo-source.yaml
with:
  - markdown:
      source: CLAUDE.md
      dest: CLAUDE.md
      section: "## Inherited Rules"
      append: true

  - yaml:
      source: labels.yaml
      dest: config.yaml
      path: metadata.labels
      array_mode: append

  - toml:
      source: deps.toml
      dest: Cargo.toml
      path: dependencies

# Files not listed here: normal copy/overwrite behavior
```

### Processing Order

1. Load source repo files
2. Parse `.common-repo-source.yaml` if present
3. Apply source-declared operations (merge operators)
4. Apply consumer `with:` operations (override source declarations)
5. Copy remaining files normally

Consumer operations always take precedence over source declarations.

### Implementation Phases

1. **Phase 1**: Parse `.common-repo-source.yaml` using existing config parser
2. **Phase 2**: Apply source operations before consumer operations
3. **Phase 3**: Consumer override mechanism (explicit `with:` overrides)

### Implementation Simplifications

Since we reuse existing types:
- No new `SourceManifest` struct needed - just parse as `Vec<Operation>`
- No new validation logic - existing operator validation works
- No new error types - existing `ConfigError` covers it
- Glob patterns deferred - can add later if needed

### Open Questions

1. **File name**: `.common-repo-source.yaml` vs `.common-repo.yaml` with different semantics?
   - **Recommendation**: Use `.common-repo-source.yaml` to avoid confusion with consumer configs.

2. **Consumer override syntax**: Explicit `override:` field or implicit via `with:`?
   - **Recommendation**: Implicit. Any consumer `with:` operation for the same dest file overrides source declaration.

3. **What about `copy`, `rename`, `delete` operators in source config?**
   - **Recommendation**: Allow all operators, not just merge. Source can declare any file transformations.

## Next Steps

1. Update implementation plan to reflect simpler approach
2. Modify config parser to handle source-side config
3. Integrate source operations into apply workflow
4. Update documentation and schema.yaml
