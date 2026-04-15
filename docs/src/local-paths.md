# Local filesystem repo references

A `repo:` operation can reference a sibling directory on disk instead of a
git URL. Use a relative (`./foo`, `../foo`) or absolute (`/path/to/foo`)
path as the `url`. No `ref:` is required, and no git operations are performed.

```yaml
- repo:
    url: ../shared-config
```

## Path resolution

Relative URLs resolve against the directory containing the
`.common-repo.yaml` that declares the reference. Resolution is
recursive: a local repo's own `.common-repo.yaml` establishes its
own base directory.

Absolute URLs (`/foo/bar`) are used as-is but still pass through
`fs::canonicalize` so that symlinks in the path are normalised and
cycle detection identifies equivalent spellings.

## No ref required

Setting `ref:` on a local URL logs a warning and the ref is
ignored. This usually happens when a git repo entry is copy-pasted
and then edited to point at a sibling directory.

```yaml
- repo:
    url: ../sibling
    ref: main   # warning: ref 'main' ignored on local-path repo ../sibling
```

## Always-fresh reads

Local filesystem repos are never cached. Every pipeline run re-reads
the directory. This matches the expectation that the sibling
directory is under active development.

## What gets loaded

- Regular files are copied into the pipeline's memory filesystem.
- `.common-repo.yaml` and `.commonrepo.yaml` are stripped (same as
  git upstreams) so configuration does not bleed into consumers.
- `.git/` directories are skipped.
- Symbolic links are skipped. If you need content from a symlink
  target, copy the file directly.

## Filtering and transformation with `with:`

A local `repo:` reference accepts the same `with:` clause as a git
reference. Use it to include, exclude, rename, or otherwise transform
the content before it merges into the consumer.

```yaml
- repo:
    url: ../shared-config
    with:
      - include:
          - ".editorconfig"
          - "scripts/**"
      - rename:
          from: "scripts/(.*)"
          to: "tools/$1"
```

The `with:` operations run in order and produce the composite filesystem
that the consumer sees, exactly as with a git upstream.

## Error conditions

- URL points at a non-existent path: `Local path not found: <url>`
- URL resolves to a file rather than a directory: `Local path is not a directory`
- Two local URLs resolving (via `canonicalize`) to the same directory in
  the same inheritance chain: `Cycle detected`
