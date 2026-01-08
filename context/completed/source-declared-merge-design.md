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

### Primary Scenario

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

### Option 6: Reuse Operator Syntax with `defer` Flag (RECOMMENDED)

Source repos use the same `.common-repo.yaml` file with top-level operators marked as deferred. No new file needed - just a flag that says "apply this when I'm used as a source."

**Two syntax forms:**

1. **`auto-merge: <file>`** - shorthand when source=dest (most common case)
2. **`defer: true`** + explicit `source:`/`dest:` - when paths differ

**Example:**
```yaml
# Source repo: .common-repo.yaml
# Uses list format to preserve operator ordering
- include:
    - "**/*"  # files to export (optional, defaults to all)

# Shorthand: auto-merge sets source=dest and implies defer
- markdown:
    auto-merge: CLAUDE.md
    section: "## Inherited Rules"
    append: true

# Verbose form: when source != dest
- yaml:
    source: config/labels.yaml
    dest: config.yaml
    path: metadata.labels
    array_mode: append
    defer: true

# Shorthand works for all merge operators
- toml:
    auto-merge: Cargo.toml
    path: dependencies
```

When a consumer references this repo, the deferred operations auto-apply.

**Pros:**
- **Same file name**: No new `.common-repo-source.yaml` - just `.common-repo.yaml`
- **Concise syntax**: `auto-merge: CLAUDE.md` vs 3 separate fields
- **Type safe**: `auto-merge` is string, `defer` is bool - no overloading
- **Full operator power**: All existing options available
- **Clear intent**: Both forms explicitly mark source-side behavior
- **Reuses parsing**: Existing operator parsing, just add two fields

**Cons:**
- Need to add `defer` and `auto-merge` fields to operator structs

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

**Primary: Option 6 (Reuse Operator Syntax with `defer` Flag)**

### Rationale

1. **Same file**: Uses `.common-repo.yaml` - no new file to learn
2. **Minimal new syntax**: Just add `defer: true` to existing operators
3. **Well-tested**: Existing operator implementations handle edge cases
4. **Clear intent**: `defer` flag explicitly marks source-side behavior
5. **Full flexibility**: All operator options immediately available

### Proposed Schema

```yaml
# Source repo: .common-repo.yaml
# Uses list format to preserve operator ordering
- include:
    - "**/*"  # optional, defaults to all files

# Shorthand: auto-merge (source=dest, implies defer)
- markdown:
    auto-merge: CLAUDE.md
    section: "## Inherited Rules"
    append: true

# Verbose: when source != dest, use defer: true
- yaml:
    source: labels.yaml
    dest: config.yaml
    path: metadata.labels
    array_mode: append
    defer: true

# Files not listed here: normal copy behavior
```

### Processing Order

1. Load source repo files
2. Parse source's `.common-repo.yaml`, extract deferred operations
3. Apply deferred operations (merge operators from source)
4. Apply consumer `with:` operations (override source declarations)
5. Copy remaining files normally

Consumer operations always take precedence over source declarations.

### Implementation Tasks

1. **Add `defer` and `auto-merge` fields** to merge operator structs
2. **Validation**: `auto-merge` implies defer; error if both `auto-merge` and `source`/`dest` set
3. **Collect deferred ops** when loading source repo (parser already handles list format)
4. **Apply deferred ops** before consumer `with:` operations

### Open Questions

1. **Flag name**: `defer` vs `deferred` vs `auto-apply`?
   - **Recommendation**: `defer: true` - short, clear meaning

2. **Consumer override**: Explicit disable or implicit via `with:`?
   - **Recommendation**: Implicit. Consumer `with:` for same dest file overrides.

3. **Non-merge operators**: Allow `copy`, `rename`, `delete` with `defer`?
   - **Recommendation**: Yes, any operator can be deferred.

4. **`include:` field**: Required or optional?
   - **Recommendation**: Optional, defaults to all files.

## Next Steps

1. Update implementation plan for `defer` flag approach
2. Add `defer` field to operator structs in `src/config.rs`
3. Integrate deferred ops into apply workflow (parser already handles list format)
4. Update documentation and schema.yaml
