# Format merge engines

- Format engines over `MemoryFS`: `yaml.rs`, `json.rs`, `toml.rs`, `ini.rs`, `markdown.rs`, `xml.rs`.
- `mod.rs`: shared file helpers, path expressions, and public re-exports; paths support dotted keys, quoted/bracketed keys, indexes, and escaped dots.
- `keyed.rs`: the one `merge_by_key` algorithm (`KeyedItems` adapter trait, `plan_keyed_merge`, `merge_vec_by_key`); each engine adds only an adapter, and XML applies the plan to its DOM in `xml.rs`.
- Every structured engine keeps the destination bytes when the merge does not change its parsed tree (#376): compare the pre-merge parse (xml: canonical serialization) and skip the write when equal. XML also skips text application when an element's full child text already matches, so whitespace-only text nodes are not moved on a no-op.
- Keep format-specific behavior in its module; configuration selection and operation ordering belong in `config.rs` and `phases/`.
