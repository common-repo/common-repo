# Composition pipeline

- Six stages: discovery/cloning, per-repository processing, deterministic ordering, composite construction, local merging, disk writing.
- Stage modules: `discovery.rs`, `processing.rs`, `ordering.rs`, `composite.rs`, `local_merge.rs`, `write.rs`; `orchestrator.rs` coordinates them.
- `RepoNode` carries URL, ref, children, and inline operations. Operation fingerprints keep differently configured `with:` references distinct.
- Deferred and auto-merge handling crosses composite and local-merge stages; preserve that ordering.
- Write rule (`orchestrator::PullOutcome::local_output`): with one or more `self:` blocks only their output is written; the source result stays in memory. Without `self:`, the source result is written.
