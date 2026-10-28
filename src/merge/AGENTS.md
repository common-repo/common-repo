# Format merge engines

- Format engines over `MemoryFS`: `yaml.rs`, `json.rs`, `toml.rs`, `ini.rs`, `markdown.rs`, `xml.rs`.
- `mod.rs`: shared file helpers, path expressions, and public re-exports; paths support dotted keys, quoted/bracketed keys, indexes, and escaped dots.
- `keyed.rs`: the one `merge_by_key` algorithm (`KeyedItems` adapter trait, `plan_keyed_merge`, `merge_vec_by_key`); each engine adds only an adapter, and XML applies the plan to its DOM in `xml.rs`.
- Keep format-specific behavior in its module; configuration selection and operation ordering belong in `config.rs` and `phases/`.
