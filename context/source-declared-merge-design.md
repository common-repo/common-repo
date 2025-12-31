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

**Primary: Option 1 (Manifest File)** with elements of Option 4 (defaults).

### Rationale

1. **Separation of concerns**: Merge rules are infrastructure config, not content
2. **Discoverability**: Single file to check for all merge behavior
3. **Validation**: Easy to validate manifest schema
4. **No file pollution**: Original files remain clean
5. **Familiar pattern**: Similar to `.gitignore`, `package.json` scripts, etc.
6. **Extensible**: Can add new features (defaults, conditions) without changing file format

### Proposed Schema

```yaml
# .common-repo-source.yaml
version: 1

# Optional: default options per operator
defaults:
  markdown:
    append: true
    create-section: true
    position: end
  yaml:
    array_mode: append_unique
  json:
    append: true

# Required: explicit merge rules
merge:
  - files: "CLAUDE.md"
    operator: markdown
    section: "## Inherited Rules"
    # inherits append: true from defaults

  - files: "config/**/*.yaml"
    operator: yaml
    path: metadata.labels
    # inherits array_mode from defaults

  - files: ["Cargo.toml", "pyproject.toml"]
    operator: toml
    path: dependencies
    array_mode: append

# Files not listed here: normal copy/overwrite behavior
```

### Implementation Phases

1. **Phase 1**: Basic manifest parsing and single-file rules
2. **Phase 2**: Glob pattern support for `files:`
3. **Phase 3**: Defaults system
4. **Phase 4**: Consumer override mechanism

### Open Questions for Implementation

1. Should `.common-repo-source.yaml` be processed before or after consumer `with:` clauses?
   - **Recommendation**: Before. Consumer `with:` overrides source declarations.

2. Should we support the `path:` option in the source config to use a subdirectory?
   - **Recommendation**: Yes, source manifest applies relative to `path:` if specified.

3. How to handle version compatibility?
   - **Recommendation**: `version: 1` field allows future schema changes.

4. Should merge rules be additive or exclusive?
   - **Recommendation**: Additive. Files matching merge rules are merged; others copy normally.

## Next Steps

1. Create implementation sub-plan with detailed tasks
2. Start with manifest parsing (new module: `src/source_manifest.rs`)
3. Integrate with existing repo processing flow
4. Add consumer override support
5. Update documentation and schema.yaml
