# Operation Execution Model: Exploration Findings

## Current Architecture Summary

### Data Structures (config.rs)

- `Schema = Vec<Operation>` — ordered list, **order preserved from YAML**
- `SelfOp { operations: Vec<Operation> }` — same ordered list for self: blocks
- `RepoOp { url, ref, path, with: Vec<Operation> }` — inline operations preserved
- Operations are an untagged enum: Repo, Include, Exclude, Template, Rename, Tools, TemplateVars, Yaml/Json/Toml/Ini/Markdown/Xml merge, Self_

### Pipeline (orchestrator.rs)

```
execute_pull(config)
  ├─ partition_self_operations(config) → (Vec<SelfOp>, Schema)
  ├─ execute_source_pipeline(source_config) → MemoryFS   [phases 1-6]
  └─ for each self_op:
       execute_source_pipeline(self_op.operations)        [phases 1-6, isolated]
```

### 6 Phases

| Phase | File | Purpose |
|-------|------|---------|
| 1 | discovery.rs | Discover & clone repos, build RepoTree |
| 2 | processing.rs | Apply operations per-repo → IntermediateFS map |
| 3 | ordering.rs | Depth-first post-order → OperationOrder |
| 4 | composite.rs | Merge all repo FSs, handle auto-merge, collect deferred ops |
| 5 | local_merge.rs | Merge composite over local, apply consumer ops |
| 6 | write.rs | Write final MemoryFS to disk |

### Phase 2: Per-Repo Processing (processing.rs:95-143)

Operations applied **sequentially in declaration order** via `apply_operation()`:
- Include: creates filtered_fs, replaces current fs
- Exclude: in-place removal
- Rename: in-place path transform
- Template: marks files (is_template flag)
- TemplateVars: collected separately
- Merge ops: collected for Phase 4 (deferred)
- Repo: no-op (handled in Phase 1)

### Phase 5: Local Merge (local_merge.rs:49-75)

Six sub-steps, **NOT in declaration order**:
1. Load local files from working dir
2. Apply local template ops (mark + process)
3. Merge composite over local (composite wins for shared paths)
4. Execute deferred merge operations (from Phase 4)
5. Apply consumer merges — `apply_consumer_merges()` (lines 211-236)
6. Apply consumer filters — `apply_consumer_filters()` (lines 242-262)

**The bug**: Steps 5 and 6 are separate passes. All merges run before all filters.
Within each pass, operations DO execute in declaration order. But merges and filters
are never interleaved.

### Operator Semantics

| Operator | Modifies In-Place? | Notes |
|----------|--------------------|-------|
| Include | No — creates new FS | `apply(op, source, &mut target)` |
| Exclude | Yes | `apply(op, &mut target)` |
| Rename | Yes | Collects paths once, applies all mappings. **Mappings don't chain within one rename op.** |
| Template mark | Yes (metadata only) | Sets `file.is_template = true` |
| Template process | Yes (content) | Substitutes `__COMMON_REPO__VAR__` patterns |
| TemplateVars | Accumulates into HashMap | Last-write-wins |
| Merge ops | Yes (content) | Format-aware merge (yaml/json/toml/ini/md/xml) |

### MemoryFS (filesystem.rs)

```rust
struct MemoryFS { files: HashMap<PathBuf, File> }
struct File { content: Vec<u8>, permissions: u32, modified_time: SystemTime, is_template: bool }
```

Key methods: add_file, remove_file, rename_file, list_files_glob, merge (overlay)

### Allium Spec Gaps

1. **No within-repo sequential execution rule** — cross-repo ordering is defined (depth-first post-order), but within a single repo/self block, no rule mandates declaration-order execution
2. **CompositePrecedence** (spec:770-785) — constrains content of present files but not presence; composite files can be silently dropped
3. **MergeLocalFiles step 6** — says "apply consumer filter operations" without specifying interleaving with merges
4. **No include/exclude interaction rule** — what happens when both match
5. **No rename chaining rule** — multiple renames on same file

## What Needs to Change

### The Goal

All operations — in self: blocks, in top-level repo definitions, and in repo: with: clauses — execute **in declaration order** against the composite filesystem as it exists at that point. No grouping by operation type. YAML order = execution order.

### Concrete Example

```yaml
- include: ["src/**"]       # FS now has src/** files
- exclude: ["*.md"]         # FS now has src/** minus *.md
- include: ["src/readme.md"] # FS now re-adds src/readme.md
```

This must work because each operation transforms the current state.

### Key Changes Required

1. **Phase 5 (local_merge.rs)**: Merge `apply_consumer_merges()` and `apply_consumer_filters()` into a single pass that applies ALL operations in declaration order

2. **Repo operations within self/source blocks**: When a `repo:` operation appears in a sequence, it should:
   - Resolve the repo into a sub-composite
   - Merge the sub-composite into the current FS at that point in the sequence
   - Not be deferred to a later phase

3. **Allium spec updates**: Add rules for:
   - Within-repo sequential execution
   - Include/exclude interaction semantics
   - CompositePrecedence presence guarantee
   - Rename chaining behavior

### Files That Need Changes

- `src/phases/local_merge.rs` — merge the two apply functions into one sequential pass
- `src/phases/orchestrator.rs` — may need changes for repo: within sequences
- `src/phases/processing.rs` — already sequential, likely minimal changes
- `spec/common-repo.allium` — add sequential execution rules
- `spec/detailed/auto-merge-composition.allium` — update for new model
