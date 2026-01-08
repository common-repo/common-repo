# common-repo Implementation Design

## Overview

This document describes the implementation architecture for common-repo v2. The design prioritizes speed and determinism to ensure the tool is fast enough that developers will actually use it.

## Core Concepts

### Terminology

- **Local/Consumer Repo**: The repository where `common-repo` is being run, containing a `.common-repo.yaml` that defines what to inherit
- **Inherited Repo**: A repository referenced by a `repo:` operation in a `.common-repo.yaml`
- **Parent Repo**: A directly inherited repo (referenced in the local repo's config)
- **Ancestor Repo**: An indirectly inherited repo (referenced by a parent or another ancestor)
- **Fragment**: A partial file that gets merged into a complete destination file
- **Intermediate Filesystem**: The in-memory result after processing a single repo's operations
- **Composite Filesystem**: The final in-memory filesystem built by merging all intermediate filesystems according to operation order

### The `.common-repo.yaml` Duality

A `.common-repo.yaml` serves two purposes:

1. **Consumption**: Defines which repos to inherit from via `repo:` operations
2. **Production**: Defines which files to export and how (via `include:`, `exclude:`, `rename:`, `template:`, merge operators, etc.)

A repo can be pure consumer (no exports), pure producer (no inheritance), or both.

**Important**: If a `.common-repo.yaml` doesn't define any `include:` or merge operators, it won't contribute any of its own files to the output - but it can still define `repo:` operations that bring in ancestor files.

## Execution Model

### High-Level Flow

```
1. Parse local `.common-repo.yaml`
2. Discover all inherited repos (parents and ancestors) - recursive traversal
3. Clone all inherited repos in parallel (breadth-first) into isolated in-memory filesystems
4. For each repo, process its operations to produce an intermediate filesystem
5. Determine the deterministic operation order based on topology
6. Apply all operations to the composite filesystem in order
7. Merge composite filesystem with local files where necessary
8. Write final filesystem to disk
9. Update cache with all fetched repos
```

### Phase 1: Discovery and Cloning

**Goal**: Fetch all required repos as quickly as possible.

1. Parse the local `.common-repo.yaml`
2. Extract all `repo:` operations (these are the parents)
3. For each parent:
   - Check cache first: `~/.common-repo/cache/{url}/{ref}/`
   - If cached: load from disk into memory
   - If not cached: shallow clone (`git clone --depth=1`) at the specified ref into memory
4. Parse each parent's `.common-repo.yaml` to discover ancestors
5. Repeat recursively until all repos are discovered
6. Clone all discovered repos in parallel (breadth-first traversal)
7. If any clone fails and no cache exists: abort with error
8. Detect cycles in the inheritance tree and abort if found

**Cache Structure**: `~/.common-repo/cache/{hash-of-url}-{ref}/`
- URL is hashed (64-bit hex) to produce filesystem-safe directory names
- Ref is the git ref string with `/` replaced by `-` (e.g., `main`, `feature-some-branch`)
- Example: `~/.common-repo/cache/3f2c9ab4d1e6a8b0-main/`
- (Future enhancement: optionally expose human-readable directory names if needed)

**Important**: Multiple branches of the inheritance tree can reference the same repo. This is allowed and not considered a cycle.

**Duplicate repo handling**: If the same repo (url+ref combination) appears multiple times in the inheritance tree, it will produce a deterministic intermediate filesystem output for that ref. The implementation may either:
- Process it each time it appears in the tree, or
- Use an in-process cache to only process each url+ref combination once

Both approaches produce the same deterministic result.

### Phase 2: Processing Individual Repos

**Goal**: Transform each repo into its intermediate filesystem representation.

For each inherited repo (and the local repo):

1. Start with an empty in-memory filesystem for this repo
2. Process operations defined in its `.common-repo.yaml` in order:
   - `repo:` operations: These define inheritance but don't directly affect this repo's intermediate filesystem (they're handled in Phase 3)
   - `include:` Add files from this repo based on glob patterns
   - `exclude:` Remove files from the intermediate filesystem
   - `rename:` Transform file paths using regex with `%[N]s` placeholders
   - `template:` Mark files for template processing (deferred until Phase 4)
   - `yaml:`, `json:`, `toml:`, `ini:`, `markdown:`: Define merge operations (deferred until Phase 4)
   - `template-vars:` Define variables for template rendering
   - `tools:` Validate tool availability and versions (warn if missing/wrong)
3. Result: An intermediate filesystem representing what this repo exports

**Note**: At this phase, we're just preparing each repo's contribution. We haven't merged anything yet.

### Phase 3: Determining Operation Order

**Goal**: Establish a deterministic order for applying operations.

The order is determined by the sequence of `repo:` operations as they appear in `.common-repo.yaml` files:

1. Start with the local repo's `.common-repo.yaml`
2. Process `repo:` operations in the order they appear
3. For each inherited repo, recursively process its `repo:` operations depth-first
4. The resulting order is deterministic and defines the merge sequence

**Example**:
```yaml
# local .common-repo.yaml
- repo: {url: A, ref: v1}
- repo: {url: B, ref: v2}
```

If A inherits from C and D, and B inherits from E:
```
Operation order: C, D, A, E, B, local
```

This ensures:
- Ancestors are processed before parents
- Parents are processed before the local repo
- Siblings are processed in declaration order

### Phase 4: Composite Filesystem Construction

**Goal**: Build the final filesystem by merging intermediate filesystems.

1. Start with an empty composite filesystem
2. Apply intermediate filesystems in the determined order
3. For each intermediate filesystem:
   - Apply file additions (from `include:` operations)
   - Apply file removals (from `exclude:` operations)
   - Files added later overwrite files added earlier (deterministic overwriting)
4. Process merge operations (`yaml:`, `json:`, `toml:`, `ini:`, `markdown:`):
   - These intelligently merge fragments or entire files
   - Support path-based merging (e.g., merge into `dependencies` key)
   - Support append modes and positioning
   - Merge operators defined in inherited repos merge with their own files and their ancestors
5. Build unified template variable context:
   - Collect all `template-vars:` from all repos
   - Later definitions override earlier ones (allows consumers to override ancestor vars)
   - Environment variables can provide defaults: `${PROJECT_NAME:-default}`
6. Process templates using the unified context

**Result**: A complete in-memory filesystem ready to write to disk.

### Phase 5: Local File Merging

**Goal**: Merge composite filesystem with existing local files.

**Default behavior**: The host filesystem is overwritten with the contents of the composite filesystem.

**Customizing merge behavior**: The local repo can define merge operators in its `.common-repo.yaml` if it wants different behavior:

1. If the local repo defines merge operators (`yaml:`, `json:`, etc.):
   - Apply intelligent merging between composite version and local version
   - Use the same merge logic as Phase 4
2. If no merge operators are defined:
   - Composite filesystem overwrites local files (default)

**Note**: Merge operators defined in inherited repos apply to merging their own files with their ancestors during composite filesystem construction (Phase 4), not to the final local merge.

### Phase 6: Writing to Disk

**Goal**: Update the local repository with the final filesystem.

1. Write all files from the composite filesystem to the local repo
2. Preserve file permissions where applicable
3. Create directories as needed

### Caching Strategy

**Goal**: Speed up future runs.

Caching happens automatically during Phase 1 (Discovery and Cloning):

1. For each repo that needs to be fetched (not already cached):
   - Clone shallow to `~/.common-repo/cache/{url}/{ref}/`
2. Future runs with the same ref will use the cache (instant loading vs. network clone)
3. Cache is managed transparently by the RepositoryManager

## Operator Implementation Details

### Core Operators

#### `repo:`
```yaml
- repo:
    url: https://github.com/common-repo/rust-cli
    ref: v1.2.3
    path: src  # Optional: only load files under this sub-directory
    with:
      - include: [.*]
      - exclude: [.gitignore]
      - rename: [{".*\\.md": "docs/%[1]s"}]
```

- **url**: Repository URL (GitHub, GitLab, any git URL)
- **ref**: Git reference (tag, branch, commit SHA) - pinned for determinism
- **path**: Optional sub-directory path within the repository to use as the effective root (enables multiple configurations per repository)
- **with**: Optional inline operations to apply to this repo's files before merging

The `with:` operations are syntactic sugar - they're applied to this specific repo's intermediate filesystem before merging into the composite filesystem.

**Path filtering example**:
```yaml
# Load only Python UV config from a monorepo
- repo:
    url: https://github.com/common-repo/python
    ref: main
    path: uv

# Load only Django config from the same monorepo
- repo:
    url: https://github.com/common-repo/python
    ref: main
    path: django
```

This enables powerful reuse patterns where a single repository can contain multiple specialized configurations.

#### `include:`
```yaml
- include:
    - "**/*"
    - .*/**/*
    - .gitignore
```

Adds files from the current repo to its intermediate filesystem based on glob patterns.

#### `exclude:`
```yaml
- exclude:
    - .github/workflows/template_*
    - "**/*.md"
```

Removes files from the intermediate filesystem based on glob patterns.

#### `rename:`
```yaml
- rename:
    - "badname/(.*)": "goodname/%[1]s"
    - "^files/(.*)": "%[1]s"
```

Transforms file paths using regex patterns with `%[N]s` placeholders for capture groups.

#### `template:`
```yaml
- template:
    - "templates/**"
```

Marks files as templates to be processed with template variables. Template rendering happens during Phase 4 (compositing) after all template-vars have been collected.

#### `template-vars:`
```yaml
- template-vars:
    project: ${PROJECT_NAME:-myprojectname}
    version: "1.0.0"
```

Defines variables for template rendering. Variables cascade - later definitions override earlier ones, allowing consumers to customize inherited templates.

Environment variable syntax: `${VAR_NAME:-default_value}`

#### `tools:`
```yaml
- tools:
    - pre-commit: "*"
    - rustc: ">=1.70"
    - python: "^3.9"
    - node: "~18.0"
```

Declares required tools with version constraints. The tool:
1. Checks if the tool exists in PATH
2. Attempts to get version (note: different tools use different args - this may be complex)
3. Validates version against constraint
4. Warns if tool is missing or version doesn't match

This is validation-only for now - no automatic installation.

### Fragment Merge Operators

These operators enable intelligent merging of structured configuration files.

#### `yaml:`
```yaml
- yaml:
    source: fragment.yml
    dest: config.yml
    path: metadata.labels
    append: true
```

- **source**: Fragment file to merge from
- **dest**: Target file to merge into
- **path**: Dot-notation path to merge location (optional - merges at root if omitted)
- **append**: If true and target is a list, append instead of replace

Note: More sophisticated list merging strategies (e.g., deduplication based on unique keys) may be handled by specialized operators in the future.

#### `json:`
```yaml
- json:
    source: fragment.json
    dest: package.json
    path: dependencies
    append: true
    position: end
```

Similar to `yaml:` with additional:
- **position**: Where to insert in lists (`start`, `end`, or index)

#### `toml:`
```yaml
- toml:
    source: fragment.toml
    dest: Cargo.toml
    path: dependencies
    append: true
    preserve-comments: true
```

Similar to `yaml:` with:
- **preserve-comments**: Maintain comments in the output (TOML-specific)

#### `ini:`
```yaml
- ini:
    source: fragment.ini
    dest: config.ini
    section: database
    append: true
    allow-duplicates: false
```

- **section**: INI section to merge into
- **allow-duplicates**: Whether to allow duplicate keys

#### `markdown:`
```yaml
- markdown:
    source: fragment.md
    dest: README.md
    section: "## Installation"
    append: true
    level: 2
    position: end
    create-section: true
```

- **section**: Markdown heading to merge under
- **level**: Heading level (1-6)
- **position**: Where to insert (`start`, `end`, or `before`/`after` another section)
- **create-section**: Create the section if it doesn't exist

## Advanced Features

### Plugin System (Future)

For sophisticated merge strategies that can't be handled by generic operators, we may support plugins:

```yaml
- plugin:
    name: pre-commit-merge
    source: fragment.yaml
    dest: .pre-commit-config.yaml
```

Plugins would implement custom merge logic for specific tools. This is not part of the initial implementation but the design should allow for it.

### Version Detection and Updates

The cache structure enables version checking:

1. Parse the ref string to determine if it's a semantic version
2. Query the remote repository for available tags/releases
3. Compare using semantic versioning rules
4. Notify user of available updates
5. Optionally auto-update refs in `.common-repo.yaml`

This functionality is implemented in `src/version.rs` and exposed via the `check` and `update` CLI commands.

## Performance Characteristics

### Speed Optimizations

1. **Parallel cloning**: All repos fetched simultaneously (breadth-first)
2. **Aggressive caching**: Ref-based caching means repeated runs are instant
3. **Shallow clones**: `--depth=1` minimizes data transfer
4. **In-memory operations**: No disk I/O until final write

### Performance Target

**Goal**: Execution time should approach the minimum network transfer time for cloning the inheritance tree.

With parallel cloning (breadth-first), the total time is determined by the depth of the inheritance tree, not the breadth:
- If each repo takes 1 second to clone
- And the local repo inherits from A and B (depth 1)
- And A inherits from C and D, B inherits from E (depth 2)
- Total time: ~2 seconds (A and B clone in parallel at depth 1, then C, D, E clone in parallel at depth 2)

**With caching**: Subsequent runs should be near-instant (milliseconds) as no network I/O is required.

### Determinism Guarantees

1. **Ordered operations**: Operation order is deterministic based on declaration
2. **Pinned refs**: Git refs ensure exact version locking
3. **Predictable merging**: Later operations always override earlier ones
4. **Consistent caching**: Same ref always produces same cache

## Error Handling

### Fatal Errors (Abort)

- Failed to fetch a repo and no cache exists
- Invalid `.common-repo.yaml` syntax
- Circular dependency detected
- Invalid regex in rename operations
- Template variable references undefined variable

### Warnings (Continue)

- Tool validation failures (missing tools or wrong versions)
- Cache read/write failures (fall back to fresh fetch or skip caching)
- Merge conflicts in fragment operators (use last-write-wins or configurable strategy)

## Implementation Language

The tool is implemented in Rust for:
- Performance (critical for adoption)
- Memory safety (handling complex in-memory filesystems)
- Cross-platform support
- Strong type system (helps with complex operations)
- Excellent error handling ergonomics

## CLI Design

A detailed CLI design is available in [context/cli-design.md](../context/cli-design.md). This document outlines the full command structure, options, and user experience.

## Testing Strategy

### Unit Tests

- Individual operator implementations
- Path transformation logic (rename operations)
- Template variable resolution
- Merge operation logic

### Integration Tests

- Full integration scenarios with mock repos
- Cache behavior
- Error handling paths
- Circular dependency detection

### Performance Tests

- Large repo trees (many levels of inheritance)
- Many parallel clones
- Large files
- Complex merge operations

## Open Questions

1. **Circular dependency handling**: Current design is to detect and abort. Is there a scenario for allowing cycles with depth limits?

2. **Cache invalidation**: Should cache have TTL? Or rely solely on ref changes?

3. **Merge conflict resolution**: For fragment merge operators, when there are conflicts, should we:
   - Always use last-write-wins?
   - Allow configuration per-operator?
   - Provide conflict markers like git?

4. **Template engine choice**: Which templating language? Options:
   - Simple variable substitution (`${VAR}`) with environment variable defaults (`${VAR:-default}`)
   - Jinja2-like
   - Handlebars
   - Tera (Rust-native)

   Note: The schema currently shows `${PROJECT_NAME:-myprojectname}` syntax, but this is provisional pending final template engine decision.

5. **Progress indication**: For slow network connections, should we show progress during clone operations?
