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
- [Structural Operators](#structural-operators)
  - [`self` - Local-Only Operations](#self---local-only-operations)
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
| [`self`](#self---local-only-operations) | Run operations locally without exposing them to consumers |

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
- self: [ ... ]
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
    - "**/*"           # Every file, dotfiles included
    - "src/**"         # Everything under src/
    - "*.toml"         # All .toml files at any depth
```

#### Patterns

Patterns match against each file's full relative path. `*` matches any characters, including `/`, and patterns match dotfiles, so `**/*` matches every file — `.gitignore` and `.github/workflows/ci.yml` included — with no extra dotfile patterns.

- `**/*` - Every file at any depth, dotfiles included
- `*` and `**` - Same as `**/*`
- `*.rs` - All Rust files at any depth (`*` crosses `/`)
- `src/**/*.rs` - All Rust files under src/
- `.github/**` - Everything under `.github/`

`*/**` matches only files at least one directory deep; a top-level file such as `README.md` does not match. Use `**/*` to match every file.

#### `if-exists:` — control over destination collisions

The optional `if-exists:` sibling key on an `include:` operator chooses what
happens when an included file's destination already exists in the
consumer's working tree at write time:

- `overwrite` (default): the include result clobbers any local content at
  the destination. Equivalent to omitting `if-exists:`.
- `preserve`: skip the write so the consumer's content survives. Useful
  when an upstream ships a sensible-default file the consumer is free to
  customise (for example, a stub CI workflow).
- `error`: fail propagation with a clear error naming the path. Useful
  when local content at the destination should be a hard signal that the
  consumer's setup needs attention.

"Exists" means any filesystem entry at the destination path on disk — a
regular file, a symlink (valid or broken), or a directory. The check
uses on-disk metadata, so it detects entries that would be invisible to
a plain file walk.

Example using `preserve`:

```yaml
- include: [".github/workflows/ci.yaml"]
  if-exists: preserve
```

Example using `error` — choose this when the destination holds content
that must not be silently replaced, such as production credentials
shipped via `include:` from a template repo:

```yaml
- include: ["sensitive/config.yaml"]
  if-exists: error
```

`if-exists:` works the same way inside a `repo:` block's `with:` clause:

```yaml
- repo:
    url: https://github.com/common-repo/semantic-release
    ref: v1.0.0
    with:
      - include: ["**"]
        if-exists: preserve
```

#### Last-op-wins

Multiple operators on the same path follow last-op-wins by **execution
order**. A later non-deferred operator (including the `Overwrite`-tagged
output of a merge operator) replaces an earlier `Preserve` tag at the
same destination.

#### Interaction with upstream `auto-merge:`

`if-exists: preserve` does **not** opt out of an upstream `auto-merge:`
declaration. Auto-merge operations are deferred — they run at the
residual stage, after the consumer's sequential pass. The merge produces
a fresh file with the default `Overwrite` tag at the destination,
clobbering any preceding `Preserve` tag.

An `exclude:` **does** cancel an inherited `auto-merge:` for the excluded
path: a repository that excludes a path owns that path, and no deferred
merge declared by any of its upstreams runs on it. This holds at every
level — a consumer-side `exclude:` after the `repo:` entry keeps the
consumer's local file untouched, and an intermediate that excludes an
upstream's file (inside the repo entry's `with:`) stops that upstream's
declaration from reaching its own consumers. See
[Declaration Precedence and Cancellation](#declaration-precedence-and-cancellation)
for the full rules and examples.

#### Unknown sibling keys

A typo in an operator-level sibling key (for example, `if-exits:`
instead of `if-exists:`) does not abort propagation. Whether the typo
is caught depends on which YAML form the config uses:

- **Legacy bare-mapping form** (the original parser, where the operator
  key and its siblings are direct children of a list-entry mapping):
  emits a `log::warn!` at default visibility naming the unknown key.
  The key is dropped and propagation continues.
- **Standard form** (serde's derived deserializer): silently ignores
  unknown fields. A typo parses as if the key were absent, so
  `if-exits: preserve` produces no warning and `if_exists` falls back
  to its default `Overwrite`.

Most consumer configs are written in the legacy bare-mapping form, so
the warning fires for typos. Configs in standard form should be
reviewed manually — serde will not flag unknown keys.

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
    - "old-name/(.*)": "new-name/$1"
    - "^templates/(.*)": "$1"
    - "(.+)\\.template$": "$1"
```

#### Placeholders

- `$1` - First capture group
- `$2` - Second capture group
- etc.

#### Examples

**Strip a directory prefix:**
```yaml
- rename:
    - "^files/(.*)": "$1"
```
Result: `files/config.yaml` becomes `config.yaml`

**Move files to a subdirectory:**
```yaml
- rename:
    - "^(.+\\.md)$": "docs/$1"
```
Result: `README.md` becomes `docs/README.md`

**Rename file extensions:**
```yaml
- rename:
    - "(.+)\\.template$": "$1"
```
Result: `config.yaml.template` becomes `config.yaml`

### `template` - Mark Template Files

Mark files for variable substitution. Files matching these glob patterns are scanned for the `__COMMON_REPO__` prefix — if found, the file is flagged as a template for processing during composite construction.

```yaml
- template:
    - "**/*.template"
    - ".github/workflows/*.yml"
    - "Cargo.toml"
```

Template files use the `__COMMON_REPO__VARNAME__` sentinel pattern for variable placeholders:

```yaml
# In a template file (e.g., Cargo.toml)
name = "__COMMON_REPO__PROJECT_NAME__"
version = "__COMMON_REPO__VERSION__"
```

Variable names must be valid identifiers (`[A-Za-z_][A-Za-z0-9_]*`) and must not contain double underscores (`__`).

### `template-vars` - Define Variables

Define variables for template substitution. Values are literal strings.

```yaml
- template-vars:
    project_name: my-project
    author: Jane Doe
    rust_version: "1.75"
```

#### Variable Cascading

Variables cascade through the inheritance tree. When the same variable is
defined in more than one place, the value is chosen by these rules, highest
precedence first:

1. **Own blocks.** A repo's own `template-vars` blocks win over anything it
   inherits from its `repo:` entries, no matter where the blocks appear in
   its config file. For an upstream repo, "own" includes the `with:`
   operations a consumer appends to it, and those `with:` values beat the
   upstream's own `template-vars` blocks because they are applied after
   them.
2. **Later sibling.** Among sibling `repo:` entries at the same level, the
   one declared later wins. This compares each sibling's whole resolved
   result, so a value a later sibling inherits from its own ancestor still
   beats an earlier sibling's own value.
3. **Earlier sibling.** An earlier sibling's values apply wherever no later
   sibling defines them.
4. **Ancestor gap-fill.** Within one `repo:` entry's chain, a nearer level
   beats a more distant one; ancestor values only fill in variables nothing
   nearer defines.

```yaml
# Consumer's .common-repo.yaml
- repo:
    url: https://github.com/org/base-a
    ref: v1.0.0
    # base-a sets log_level: info, region: us-east, and timeout: "30"
- repo:
    url: https://github.com/org/base-b
    ref: v2.0.0
    # base-b sets log_level: debug and inherits timeout: "60" from base-c
- template-vars:
    region: eu-west
```

Here `log_level` renders as `debug` because `base-b` is the later sibling.
`timeout` renders as `60` for the same reason: `base-b` inherited it from
its own ancestor `base-c`, and a later sibling's whole resolved result beats
an earlier sibling's own value. `region` renders as `eu-west` because the
consumer's own block wins even though it comes after both `repo:` entries.
To restore an earlier sibling's value, re-declare it in your own
`template-vars`:

```yaml
- template-vars:
    log_level: info   # take base-a's value back over base-b's
```

Values passed to an upstream through `with:` count as that upstream's own
blocks and beat the ones in its config file:

```yaml
- repo:
    url: https://github.com/org/base-a
    ref: v1.0.0
    with:
      - template-vars:
          log_level: warn   # beats base-a's own log_level: info
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

## Structural Operators

### `self` - Local-Only Operations

Run a sub-pipeline of operations that apply only to the local repository. Files produced by `self:` blocks are written to the working directory but never appear in the composite filesystem that downstream consumers see.

When a config has one or more `self:` blocks, `apply` writes only the `self:` output to the working directory. The top-level operations define the source API for consumers; their output is not written locally. When a config has no `self:` block, `apply` writes the top-level output to the working directory.

This is useful when a repository is both a source (providing shared configuration to consumers) and wants to consume from its own upstream repos.

```yaml
- self:
    - repo:
        url: https://github.com/org/upstream-tooling
        ref: v1.0.0
    - exclude:
        - ".releaserc.yaml"
```

#### How It Works

1. The orchestrator partitions `self:` blocks from the rest of the config
2. The source pipeline builds the composite filesystem for consumers in memory. When the config has a `self:` block, this composite is not written to disk
3. Each `self:` block runs as an independent pipeline. It loads the working directory as it is on disk and does not see source pipeline output. When `apply` writes to the working directory (the default), this includes the output of earlier `self:` blocks. With `--output`, `diff`, or `ls`, it does not.
4. Output from `self:` blocks is written to the working directory but excluded from what consumers inherit

#### Rules

- `self:` blocks must contain at least one operation
- `self:` blocks cannot be nested (no `self:` inside `self:`)
- `self:` blocks are stripped when a repo is consumed as an upstream — consumers never see them
- Any operation valid at the top level can appear inside `self:` (repo, include, exclude, rename, template, merge operators, etc.)
- A `self:` block populates its composite filesystem through `include` (which pulls matching files from the local working directory) and/or `repo:`. Without one of these, the composite is empty and `exclude`/`rename` have nothing to operate on.
- When a config has a `self:` block, the top-level output is not written locally. An upstream that wants its own `src/**` files in its working directory must include them inside `self:` (for example `- include: ["src/**"]` followed by the `rename` inside `self:`), as `common-repo/upstream` does.
- `diff` compares the working directory against the files that `apply` would write: the `self:` output when a `self:` block is present. `ls` lists the source composite.
- Files that earlier versions wrote to the working directory from top-level operations are not removed by `apply`; delete them by hand if they are no longer wanted.

#### Example: Source Repo That Consumes Upstream Tooling

```yaml
# .common-repo.yaml for a shared-config repo

# Self operations — local only, consumers don't see these
- self:
    - include:
        - "src/**"     # apply this repo's own files locally
    - rename:
        - from: "^src/(.*)$"
          to: "$1"
    - repo:            # later operations win on shared paths
        url: https://github.com/org/ci-tooling
        ref: v2.0.0
    - exclude:
        - ".releaserc.yaml"
        - "commitlint.config.cjs"

# Source API — what consumers get; not written to this repo's working directory
- include:
    - "src/**"         # matches dotfiles under src/ too
- template:
    - "src/.github/workflows/release.yaml"
- template-vars:
    GH_APP_ID_VAR: CHRISTMAS_ISLAND_APP_ID
- rename:
    - from: "^src/(.*)$"
      to: "$1"
```

In this example, the repo applies its own `src/**` files locally and pulls CI tooling for its own use via `self:`. Consumers only see the files exposed by the top-level `include`, `template`, `template-vars`, and `rename`. Because the config has a `self:` block, `apply` does not write the top-level output to the working directory.

For a detailed guide on using `self:` when authoring upstream repositories, see [Authoring Upstream Repositories](authoring-upstream-repos.md#using-self-for-local-consumption).

## Merge Operators

Merge operators intelligently combine configuration fragments into destination files.

### Upstream-Declared Merge (defer/auto-merge)

All merge operators support two additional options for upstream-declared merge behavior:

| Option | Type | Description |
|--------|------|-------------|
| `auto-merge` | string | Shorthand: sets source=dest to this value and implies defer=true |
| `defer` | bool | When true, this operation only runs when repo is used as an upstream |

**`auto-merge`** is the preferred syntax when the source and destination filenames are the same:

```yaml
# In upstream repo's .common-repo.yaml
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

#### Declaration Precedence and Cancellation

Two rules govern inherited auto-merge declarations when a path is
declared more than once along a chain, or when a repository excludes a
path an upstream declared a merge for. They apply identically to every
merge format — `yaml`, `json`, `toml`, `ini`, `markdown`, and `xml` —
because declarations are keyed on the file path, not the format.

**1. The nearest declaration wins.** When several upstreams declare an
auto-merge for the same path, the declaration closest to the consumer
wins: the consumer's `with:` first, then the intermediate's own, then
its upstreams, nearest first. Among sibling `repo:` entries at the same
level, the later sibling wins. A differing duplicate is announced at
default visibility instead of being dropped silently:

```text
[WARN ] auto-merge declared more than once for .github/workflows/ci.yaml:
        keeping the nearer declaration (yaml array_mode=merge_by_key),
        discarding yaml array_mode=append_unique from a farther upstream
```

**2. An exclude cancels inherited declarations for that path.** A
repository that excludes a path owns that path: every `auto-merge:`
declaration inherited for it is dropped, and the file propagates as a
plain copy that overwrites whatever the consumer had. Written order
decides — the exclude cancels declarations already collected at the
point it runs (an upstream's own declarations and any `with:` exclude a
consumer appends to them), not ones integrated later.

The two rules combine for the common intermediate case — a repository
that takes over a file from its upstream:

```yaml
# mid/.common-repo.yaml — takes over ci.yaml from base
- repo:
    url: https://github.com/org/base
    with:
      - exclude: [".github/workflows/ci.yaml"]   # drop base's copy ...
- include: ["**"]                                 # ... ship our own
- yaml:                                           # ... and decide its merge ourselves
    auto-merge: .github/workflows/ci.yaml
    array_mode: merge_by_key
```

`base`'s declaration is cancelled at the exclude, so `mid`'s own
declaration is the only one its consumers see. Had `mid` not
re-declared, its file would propagate as a plain copy — no merge mode
chosen by `base` for `base`'s stub would apply to `mid`'s consumers.

The same shapes for the other formats:

```yaml
# json-mid/.common-repo.yaml — takes over deploy.json from json-base
- repo:
    url: https://github.com/org/json-base
    with:
      - exclude: ["deploy.json"]
- include: ["**"]
- json:
    auto-merge: deploy.json
    array_mode: merge_by_key
    key: name
```

```yaml
# xml-mid/.common-repo.yaml — takes over config.xml from xml-base
- repo:
    url: https://github.com/org/xml-base
    with:
      - exclude: ["config.xml"]
- include: ["**"]
- xml:
    auto-merge: config.xml
    array_mode: merge_by_key
    key: name
```

On the consumer side, a top-level `exclude:` after the `repo:` entry
cancels every inherited declaration for the path, so the consumer's own
file is never merged into:

```yaml
# consumer/.common-repo.yaml — owns its CI outright
- repo:
    url: https://github.com/org/mid
- exclude: [".github/workflows/ci.yaml"]
```

See [Upstream-Declared Merge Behavior](authoring-upstream-repos.md#upstream-declared-merge-behavior) for detailed usage.

#### Byte Stability of Unchanged Files

A merge that does not change the destination's parsed content does not write
the file. Every structured merge operator (`yaml`, `json`, `toml`, `ini`,
`xml`) compares the merged document with the destination as it was parsed and
keeps the original bytes when they are equal, so:

- `apply` twice in a row writes the same bytes both times — a consumer that
  has not edited an inherited file never sees a cosmetic rewrite of it.
- `diff` reports "No changes detected" immediately after `apply`, including
  on a freshly created consumer.
- Comments, SHA-pin notes, flow-style lists (`on: [push, pull_request]`),
  inline tables, and indentation survive verbatim in any file the merge does
  not change.

The moment a merge does change content, the destination is rewritten in the
operator's canonical format, and formatting the writers cannot represent —
comments, flow styles, original indentation — is lost on that rewrite.

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
| `defer` | No | false | Only apply when repo is used as an upstream |
| `path` | No | root | Path to merge at (see [Path Syntax](#path-syntax)) |
| `array_mode` | No | replace | Array handling: `replace`, `append`, `append_unique`, or `merge_by_key` |
| `key` | No | auto | Key field for `merge_by_key`: `name`, then `uses`, then `id` (see [Array Merge Modes](#array-merge-modes)) |
| `position` | No | end | Where to insert array items: `start` or `end` (used with the `append`, `append_unique`, and `merge_by_key` modes) |

*Either `source`+`dest` or `auto-merge` is required

#### Array Merge Modes

| Mode | Description |
|------|-------------|
| `replace` | Replace destination array with source array (default) |
| `append` | Append source items to the end of destination array |
| `append_unique` | Append only items not already in destination array |
| `merge_by_key` | Match items by a key field and replace the matched item in place |

`append_unique` compares whole items, so an upstream edit to an existing item
makes it count as "new" and it gets appended next to the stale copy. For arrays
whose items have a natural identity — GitHub Actions `steps`, pre-commit
`hooks[]` — use `merge_by_key` instead: each source item replaces the
destination item with the same key, new items are appended, and destination
items the source does not mention are preserved.

The key field auto-detects `name`, then `uses`, then `id`. Set `key:` to match
on another field (for example `repo` for pre-commit `repos[]`); for XML the
key is an attribute (`name`, then `id`). Items without a key field fall back
to `append_unique` behavior. `merge_by_key` is idempotent: re-running `apply`
after an upstream bump updates the changed items without duplicating them.

A matched item is replaced whole, not merged field by field. Consumer edits
inside an item the upstream owns (an extra `env:` or `with:` entry added to
the upstream's step, for example) are overwritten on every apply; only
consumer-added items with their own keys survive. Duplicates that
`append_unique` already created are not removed automatically: delete the
stale copies by hand once before switching an existing merge to
`merge_by_key`.

```yaml
- yaml:
    auto-merge: .github/workflows/ci.yaml
    array_mode: merge_by_key
```

`position: start` inserts source items at the front of the destination array
instead of the end. It only affects the `append`, `append_unique`, and
`merge_by_key` modes (for `merge_by_key`, where items with no matching
destination key are inserted).

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
    array_mode: append
```

**Add jobs to CI workflow:**
```yaml
# Merge shared CI jobs into existing workflow
- yaml:
    source: ci-jobs.yml
    dest: .github/workflows/ci.yml
    path: jobs
    array_mode: append
```

**Update workflow steps in place (`merge_by_key`):**

The upstream revises its steps over time; the consumer holds the previous
revision plus a step of its own. Steps are matched on their key field: the
edited `Check coverage` step is replaced in place, the consumer-only
`Deploy preview` step is kept, and the new `Audit dependencies` step is added.

```yaml
- yaml:
    source: steps.yml
    dest: .github/workflows/ci.yml
    path: jobs.test.steps
    array_mode: merge_by_key
```

Source fragment (`steps.yml`):

```yaml
- uses: actions/checkout@v4
- name: Check coverage
  run: coverage verify --threshold 80
- name: Audit dependencies
  run: cargo audit
```

Destination before apply:

```yaml
jobs:
  test:
    steps:
      - uses: actions/checkout@v4
      - name: Check coverage
        run: coverage verify --threshold 70
      - name: Deploy preview
        run: ./scripts/deploy-preview.sh
```

Destination after apply:

```yaml
jobs:
  test:
    steps:
    - uses: actions/checkout@v4
    - name: Check coverage
      run: coverage verify --threshold 80
    - name: Deploy preview
      run: ./scripts/deploy-preview.sh
    - name: Audit dependencies
      run: cargo audit
```

**A merge that changes nothing writes nothing:**

The upstream declares `auto-merge: .github/workflows/ci.yaml`. A consumer
that inherits the file and edits nothing keeps its copy byte for byte — the
header comment, the `# v4` SHA-pin note, the flow-style `on:` list, and the
two-space list indentation all survive every `apply`, and `diff` reports no
changes. Only a merge that actually changes content rewrites the file.

```yaml
# .github/workflows/ci.yaml — identical on every apply until content changes
# From upstream
name: CI
on: [push, pull_request]
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
| `defer` | No | false | Only apply when repo is used as an upstream |
| `path` | No | root | Dot-notation path to merge at |
| `array_mode` | No | replace | Array handling: `replace`, `append`, `append_unique`, or `merge_by_key` |
| `key` | No | auto | Key field for `merge_by_key`: `name`, then `uses`, then `id` |
| `position` | No | end | Where to insert array items: `start` or `end` (used with the `append`, `append_unique`, and `merge_by_key` modes) |

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
    array_mode: append
    position: start
```

**Merge TypeScript compiler options:**
```yaml
- json:
    source: strict-options.json
    dest: tsconfig.json
    path: compilerOptions
```

**Update keyed array entries in place (`merge_by_key`):**

`members.json` is the source fragment; the destination already holds an
older revision plus a consumer-only entry. Items are matched on the explicit
`key: email` field (anything other than `name`/`uses`/`id` must be
declared): the edited `a@example.com` entry is replaced in place, the
consumer-only `c@example.com` entry is preserved, and the new `b@example.com`
entry is appended.

```yaml
- json:
    source: members.json
    dest: directory.json
    path: members
    array_mode: merge_by_key
    key: email
```

Source fragment (`members.json`):

```json
{
  "members": [
    { "email": "a@example.com", "role": "admin" },
    { "email": "b@example.com", "role": "dev" }
  ]
}
```

Destination before apply:

```json
{
  "members": [
    { "email": "a@example.com", "role": "viewer" },
    { "email": "c@example.com", "role": "guest" }
  ]
}
```

Destination after apply:

```json
{
  "members": [
    { "email": "a@example.com", "role": "admin" },
    { "email": "c@example.com", "role": "guest" },
    { "email": "b@example.com", "role": "dev" }
  ]
}
```

**A merge that changes nothing writes nothing:**

With the upstream declaring `auto-merge: package.json`, a consumer that has
not edited the file keeps its copy exactly as written — the four-space
indent and the one-line `scripts` object below survive every `apply`, and
`diff` reports no changes. The file is only rewritten, in the writer's
two-space expanded format, when a merge actually changes its content.

```json
{
    "name": "demo",
    "scripts": {"test": "jest"}
}
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
| `defer` | No | false | Only apply when repo is used as an upstream |
| `path` | No | root | Path to merge at (see [Path Syntax](#path-syntax)) |
| `array_mode` | No | replace | Array handling: `replace`, `append`, `append_unique`, or `merge_by_key` |
| `key` | No | auto | Key field for `merge_by_key`: `name`, then `uses`, then `id` |
| `position` | No | end | Where to insert array items: `start` or `end` (used with the `append`, `append_unique`, and `merge_by_key` modes) |
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

**Update array-of-tables entries in place (`merge_by_key`):**

TOML arrays of tables (`[[...]]`) merge by a key field like any other array.
The destination below holds an older `mirrors` revision plus a consumer-only
mirror; after the merge the `fast.example.com` entry is updated in place
(matched on the explicit `key: url`), the consumer-only mirror is preserved,
and the new mirror is appended.

```yaml
- toml:
    source: mirrors.toml
    dest: .cargo/config.toml
    path: mirrors
    array_mode: merge_by_key
    key: url
```

Source fragment (`mirrors.toml`):

```toml
[[mirrors]]
url = "https://fast.example.com"
priority = 20

[[mirrors]]
url = "https://slow.example.com"
priority = 50
```

Destination before apply:

```toml
[[mirrors]]
url = "https://fast.example.com"
priority = 10

[[mirrors]]
url = "https://private.example.com"
priority = 5
```

Destination after apply:

```toml
[[mirrors]]
url = "https://fast.example.com"
priority = 20

[[mirrors]]
url = "https://private.example.com"
priority = 5

[[mirrors]]
url = "https://slow.example.com"
priority = 50
```

A merge that changes nothing writes nothing here either: an unedited
`Cargo.toml` keeps its comments, inline tables, and table order byte for
byte on every `apply` (see [Byte Stability of Unchanged
Files](#byte-stability-of-unchanged-files)). A merge that does change
content rewrites the file with tables sorted and inline tables expanded,
so `preserve-comments: true` is still worth setting for files you expect
to revise.

### `xml` - Merge XML Files

```yaml
- xml:
    source: fragment.xml
    dest: config.xml
```

#### Options

| Option | Required | Default | Description |
|--------|----------|---------|-------------|
| `source` | Yes* | - | Source fragment file |
| `dest` | Yes* | - | Destination file |
| `auto-merge` | No | - | Shorthand: sets source=dest, implies defer=true |
| `defer` | No | false | Only apply when repo is used as an upstream |
| `path` | No | root | Path to merge at, navigating by element local name (see [Path Syntax](#path-syntax)) |
| `array_mode` | No | replace | Array handling for repeated sibling elements: `replace`, `append`, `append_unique`, or `merge_by_key` |
| `key` | No | auto | Key **attribute** for `merge_by_key`: `name`, then `id` |
| `position` | No | end | Where to insert new elements: `start` or `end` (used with the `append`, `append_unique`, and `merge_by_key` modes) |

*Either `source`+`dest` or `auto-merge` is required

Elements merge recursively, matched on local name and namespace URI; source
attributes override destination attributes on the same element. Repeated
sibling elements with the same name are XML's arrays and follow the unified
array merge model. With `array_mode: merge_by_key`, the key is an
**attribute** on the element (auto-detected `name`, then `id`, or an
explicit `key:`), never a child element — a matched sibling is replaced in
place with the source element, siblings the source does not mention are
preserved, and elements without a key attribute fall back to text-content
uniqueness. A same-name pair is matched by key whenever either side
carries a key attribute, even when there is only one element on each
side; a lone pair with no key attribute on either side (a single config
element such as `<settings>`) still merges recursively.

```yaml
- xml:
    source: servers.xml
    dest: config.xml
    array_mode: merge_by_key
```

```xml
<!-- source: servers.xml -->
<servers>
  <server name="web" replicas="3"/>
  <server name="db" replicas="1"/>
</servers>

<!-- destination before apply: old web revision plus a consumer-only server -->
<servers>
  <server name="web" replicas="1"/>
  <server name="cache" replicas="9"/>
</servers>

<!-- destination after apply: web updated in place, cache kept, db added -->
<servers>
  <server name="web" replicas="3"/>
  <server name="cache" replicas="9"/>
  <server name="db" replicas="1"/>
</servers>
```

**A merge that changes nothing writes nothing:**

With the upstream declaring `auto-merge: config.xml`, a consumer that has
not edited the file keeps its copy exactly as written — the comment, the
`encoding` declaration, and the hand indentation below survive every
`apply`, and `diff` reports no changes. Only a merge that actually changes
content rewrites the file through the XML writer.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!-- App configuration. Comments must survive. -->
<app name="demo">
  <servers>
    <server host="alpha.example.com" port="8080"/>
  </servers>
</app>
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
| `defer` | No | false | Only apply when repo is used as an upstream |
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
| `defer` | No | false | Only apply when repo is used as an upstream |
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

**Merge CLAUDE.md rules (upstream-declared):**
```yaml
# In upstream repo - will only apply when inherited
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

`self:` blocks execute after the source pipeline builds the source composite in memory, in declaration order. When a `self:` block is present, the source composite is not written to disk; only the `self:` output is. Each `self:` block runs as an independent sequential pipeline: operations fire in YAML declaration order, and `repo:` operations resolve inline at their declaration position. The pipeline starts with local files loaded from the working directory as it is on disk. When `apply` writes to the working directory (the default), this includes the output of earlier `self:` blocks. With `--output`, `diff`, or `ls`, it does not. Because local files are loaded first, filter operators (`include`, `exclude`, `rename`) can transform the local file set before a `repo:` integrates upstream content.

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

Here's a complete configuration showing multiple operators. It is an ordinary consumer config: every operation is at top level and all output is written locally. For an upstream repo that also consumes its own tooling, see [`self`](#self---local-only-operations).

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
    - "(.+)\\.template$": "$1"

# Define template variables
- template-vars:
    project_name: my-project
    author: Your Name
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
    array_mode: append
```
