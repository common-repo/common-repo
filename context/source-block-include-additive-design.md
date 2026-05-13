# Source-block include additive — design

## Guiding principle

Common-repo operations are well-ordered and deterministic. A reader should be
able to predict the resulting composite from a top-down read of the config.
Auto-merge declares a property of a path — "any contribution to this path is
merged, not replaced" — and that property travels with the path through
integration boundaries.

## Scope

This design covers three intertwined changes shipped on the same branch:

1. **Additive include in source-block + `repo:`-integrated pipelines.** A
   second `include:` after a first must add files, not replace them.
   Implemented in `src/phases/orchestrator.rs` (Fix 1) and
   `src/phases/processing.rs` (Fix 2). The shape change: each pipeline run
   loads a read-only source FS from the working directory at function entry;
   `include` pulls matching files from the source FS into the composite
   additively. The source FS is never modified, so include-after-exclude
   re-adds previously excluded files.

2. **Empty-start composite per the operator spec.** Both self and source
   blocks begin with an empty composite. Files enter only via `include` or
   `repo:` integration.

3. **`if-exists: auto` default and the auto-merge × include rule** (this
   design). When a write to the composite targets a path that has been tagged
   for auto-merge anywhere in the resolved pipeline, the write performs a
   format-aware merge instead of an overwrite.

## Defaults

| Operator field | Default |
|---|---|
| `if-exists` on files included from `include:` | `auto` (new default; was `overwrite`) |
| `array_mode` on `yaml: auto-merge` etc. | `append_unique` |
| Position for append_unique | end (target items first, unique-from-source appended) |

`if-exists: auto` semantics:
- If the target path is tagged for auto-merge and already exists in the
  composite, merge the incoming source-FS file into the composite file using
  the tag's format and mode.
- Otherwise the source-FS file overwrites.

`if-exists: skip` leaves an existing composite file untouched.

## Algorithm

### Phase 1 — Tag pre-pass

Walk the resolved pipeline tree (this pipeline plus everything reachable via
`repo:` integration) and build a tag set:

```
tag_set: map<path, { format: yaml|toml|json|ini|markdown|xml,
                     mode: append | replace | append_unique,
                     if_exists: auto | overwrite | skip }>
```

Tags from upstream `repo:` integrations propagate to consumer pipelines via
the existing `merge_operations` list on `IntermediateFS`. A later declaration
for the same path may upgrade or downgrade the mode (last writer wins for the
tag; the merge action itself is bounded by what each operator wrote).

### Phase 2 — Inline merge at write time

When the executor is about to write a path `P` into the composite (from an
`include`, or from `repo:` integration's sub-composite flush):

```
existing = composite.get(P)
incoming = source.get(P)            # source FS for include, sub-composite for repo:
tag      = tag_set.get(P)

if existing is None:
    composite.put(P, incoming)
elif tag is None or tag.if_exists == Overwrite:
    composite.put(P, incoming)      # current behaviour
elif tag.if_exists == Skip:
    pass
else:                               # tagged + auto
    merged = format_merge(tag.format, tag.mode, target=incoming, source=existing)
    composite.put(P, merged)
```

**Merge direction:** `target=incoming, source=existing-in-composite`. This
matches the existing inheritance-merge captured behaviour: in
`upstream-intermediate` apply, intermediate's local `merge.yaml` is the
"target" (incoming via include), base's earlier-pulled value is the "source"
(existing in composite). For mappings, target's keys come first, then source's
new keys appended at end. For append_unique sequences, target's items first,
then unique-from-source appended.

`format_merge` dispatches to the existing `merge::yaml::merge_yaml_values`,
`merge::toml::...`, etc.

### Phase 3 — Deferred merge phase

For paths whose tag declarations weren't satisfied by an inline merge during
Phase 2 (i.e., the path was tagged but never written by an `include` or
integration), the existing deferred merge phase continues to run unchanged.
For paths that Phase 2 already merged, the deferred phase is a no-op.

The explicit `yaml: { source: ..., dest: ... }` splice operator continues to
run unchanged in this phase.

## Examples

### Example A — Inheritance-merge fixture

```yaml
# upstream-base/.common-repo.yaml
- include: ["**"]
- yaml: { auto-merge: merge.yaml, array_mode: append_unique }

# upstream-intermediate/.common-repo.yaml
- repo: { url: ../upstream-base }
- include: ["**"]
- yaml: { auto-merge: merge.yaml, array_mode: append_unique }

# upstream-overlay/.common-repo.yaml
- repo: { url: ../upstream-intermediate }
- include: ["**"]
- yaml: { auto-merge: merge.yaml, array_mode: append_unique }

# consumer/.common-repo.yaml
- repo: { url: ../upstream-overlay }
```

Trace at `apply` in `upstream-intermediate`:

| Step | Operation | Composite[merge.yaml] | Tags |
|---|---|---|---|
| 0 | (start) | empty | {} |
| 1 | `repo: ../upstream-base` integrates base's source block. Base's pipeline runs: include → composite[merge.yaml]=base(top=1.0); tag merge.yaml. Sub-composite returned with merge.yaml(top=1.0) and tag. | base(top=1.0, list=[alpha,fnord]) | {merge.yaml: yaml/append_unique} |
| 2 | `include: ['**']` pulls intermediate's merge.yaml(top=2.0, list=[beta,fnord]). Path is tagged → merge with target=intermediate, source=base. Scalar overwrite per merge_yaml_values: source wins → top=1.0. List append_unique end: [beta,fnord,alpha]. | merged(top=1.0, list=[beta,fnord,alpha]) | (unchanged) |
| 3 | `yaml: { auto-merge: merge.yaml }` — tag already present, idempotent. | (unchanged) | (unchanged) |

Final `merge.yaml`: `top_level_key: 1.0`, `list_within_key: [beta, fnord, alpha]` — matches the existing captured assertions in `tests/cli_e2e_inheritance_merge.rs`.

### Example B — Production `upstream/.common-repo.yaml` source block

```yaml
- include: ["src/**"]
- template: ["src/.github/workflows/release.yaml"]
- rename: ["^src/(.*)$": "$1"]
- yaml: { auto-merge: .pre-commit-config.yaml, array_mode: append_unique }
- toml: { auto-merge: cog.toml }
- repo: { url: .../semantic-release, ref: v0.5.2 }
- template-vars: { ... }
```

Trace when consumed by a downstream `repo: upstream`:

| Step | Operation | Outcome |
|---|---|---|
| 1 | `include: src/**` | Composite has `src/.pre-commit-config.yaml`, `src/cog.toml`, `src/.github/workflows/release.yaml`, etc. |
| 2 | `rename: src/→root` | Composite paths rewritten: `.pre-commit-config.yaml`, `cog.toml`, `.github/workflows/release.yaml`. |
| 3 | `yaml: auto-merge: .pre-commit-config.yaml` | Tag set for `.pre-commit-config.yaml` (yaml/append_unique). |
| 4 | `toml: auto-merge: cog.toml` | Tag set for `cog.toml` (toml). |
| 5 | `repo: semantic-release` | Sub-composite returns its own `.pre-commit-config.yaml`. Tagged → merge with target=semantic-release's, source=existing-in-composite (the local one). |

### Example C — Consumer inheriting tags

When a consumer does `- repo: { url: pre-commit }`, the tag for
`.pre-commit-config.yaml` propagates through the integration boundary. If the
consumer later writes that path (via its own `include` or a second `repo:`),
the inline merge fires automatically. The consumer doesn't need to re-declare
`yaml: auto-merge`.

## Test plan

- TDD with `tests/cli_e2e_inheritance_merge::merge_at_intermediate_combines_base`
  — that test pre-exists and currently fails under the un-implemented rule.
  Watch it fail with the additive include changes in place, implement the
  rule per Phase 1 + Phase 2, watch it pass.
- Walk through remaining failures: `merge_at_overlay_chains_through_intermediate`,
  `merge_at_consumer_propagates_full_chain`, then `cli_e2e_ls_filtering`,
  `cli_e2e_sequential_ops`, `cli_e2e_upstream_ops` suites.
- New byte-exact `<name>.expected/` fixtures:
  - `include-additive-simple/` — two sequential `include:` operators.
  - `three-tier-merge/` — passes already; pins down 3-tier auto-merge chain.

## Open questions

- Should `if-exists: auto` apply to untagged paths if any upstream tier
  declared a tag for that path? Per the design above: yes, the tag travels.
  Verify this matches consumer expectations in production configs.
- Phase 1 walks the resolved tree to build the tag set before Phase 2 fires
  inline merges. Walking happens via the existing `merge_operations`
  propagation on `IntermediateFS` — confirm no ordering issues with deeply
  nested `repo:` blocks.
