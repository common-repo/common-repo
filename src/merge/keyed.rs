//! Keyed array merge shared by the YAML, JSON, TOML and XML engines
//! (`array_mode: merge_by_key`).
//!
//! The algorithm is format-agnostic: each engine supplies a [`KeyedItems`]
//! adapter that says how to read a key field from an item and when two
//! items are the same, and gets back a [`KeyedMergePlan`] describing which
//! destination items to replace and which source items to insert. The
//! `Vec`-backed formats apply that plan through [`merge_vec_by_key`]; XML
//! applies it to its DOM in `xml.rs`.

use log::info;

use super::ARRAY_KEY_FIELD_CANDIDATES;
use crate::config::InsertPosition;

/// How a format exposes its array items to the keyed merge.
pub(crate) trait KeyedItems {
    /// One array item (a YAML mapping, a TOML table, an XML node handle).
    type Item;
    /// A key field's value, compared with `==` to find the matching item.
    type Key: PartialEq;

    /// The value of `field` on `item`, or `None` when the field is absent or
    /// is not a scalar. Only scalars can act as keys.
    fn key_value(&self, item: &Self::Item, field: &str) -> Option<Self::Key>;

    /// Whole-item equality, used to dedupe items that carry no key.
    fn items_equal(&self, a: &Self::Item, b: &Self::Item) -> bool;

    /// Plain text for a key value in log output (no quotes or type wrappers).
    fn key_text(&self, key: &Self::Key) -> String;

    /// Fields tried in order when no explicit `key:` is configured.
    fn key_candidates(&self) -> &'static [&'static str] {
        &ARRAY_KEY_FIELD_CANDIDATES
    }

    /// Whether a replacement actually changed the item. Drives the per-item
    /// log line, so an idempotent re-apply stays quiet.
    fn item_changed(&self, old: &Self::Item, new: &Self::Item) -> bool {
        !self.items_equal(old, new)
    }
}

/// One in-place replacement: `source[source]` takes the slot of `dest[dest]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Replacement {
    pub source: usize,
    pub dest: usize,
    /// The key field the two items matched on.
    pub field: String,
    /// The matched key value, rendered for logging.
    pub key: String,
}

/// The outcome of matching a source array against a destination array.
///
/// Both lists are in source order. Destination items not named in
/// `replaced` are untouched.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct KeyedMergePlan {
    pub replaced: Vec<Replacement>,
    /// Indices into the source array of items to insert at `position`.
    pub new_items: Vec<usize>,
}

/// Decide, for each source item, whether it replaces a destination item or
/// is new.
///
/// A source item's key field is the explicit `key` when the item carries it
/// as a scalar, otherwise the first candidate field the item carries as a
/// scalar. A keyed item replaces the first not-yet-consumed destination item
/// with an equal key value, so the edit lands in place instead of piling up
/// as a duplicate (issue #370); with no match it is new. A keyless item is
/// new unless it equals an unconsumed destination item or an item already
/// marked new.
pub(crate) fn plan_keyed_merge<A: KeyedItems>(
    adapter: &A,
    dest: &[A::Item],
    source: &[A::Item],
    explicit_key: Option<&str>,
) -> KeyedMergePlan {
    let mut consumed = vec![false; dest.len()];
    let mut plan = KeyedMergePlan::default();

    for (src_idx, src_item) in source.iter().enumerate() {
        match resolve_key(adapter, src_item, explicit_key) {
            Some((field, key)) => {
                let matched = dest.iter().enumerate().position(|(dst_idx, dst_item)| {
                    !consumed[dst_idx] && adapter.key_value(dst_item, field).as_ref() == Some(&key)
                });
                match matched {
                    Some(dst_idx) => {
                        consumed[dst_idx] = true;
                        plan.replaced.push(Replacement {
                            source: src_idx,
                            dest: dst_idx,
                            field: field.to_string(),
                            key: adapter.key_text(&key),
                        });
                    }
                    None => plan.new_items.push(src_idx),
                }
            }
            None => {
                let in_dest = dest.iter().enumerate().any(|(dst_idx, dst_item)| {
                    !consumed[dst_idx] && adapter.items_equal(dst_item, src_item)
                });
                let already_new = plan
                    .new_items
                    .iter()
                    .any(|&new_idx| adapter.items_equal(&source[new_idx], src_item));
                if !in_dest && !already_new {
                    plan.new_items.push(src_idx);
                }
            }
        }
    }

    plan
}

/// The key field and value for `item`: the explicit key when the item
/// carries it as a scalar, else the first candidate it carries as a scalar.
pub(crate) fn resolve_key<'f, A: KeyedItems>(
    adapter: &A,
    item: &A::Item,
    explicit_key: Option<&'f str>,
) -> Option<(&'f str, A::Key)> {
    let fields: &[&'f str] = match explicit_key {
        Some(ref field) => std::slice::from_ref(field),
        None => adapter.key_candidates(),
    };
    fields
        .iter()
        .find_map(|&field| adapter.key_value(item, field).map(|key| (field, key)))
}

/// Prefix under which the composite stages an upstream's copy of an
/// auto-merge target before merging it into the file at the real path.
pub(crate) const AUTO_MERGE_UPSTREAM_PREFIX: &str = ".__common_repo_auto_merge_temp__";
/// Prefix under which the composite stages a repo's own copy for the second
/// pass that lets own items win (see `merge_into_composite_file`).
pub(crate) const AUTO_MERGE_OWN_PREFIX: &str = ".__common_repo_auto_merge_own__";

/// Where a merge is happening, for log lines.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MergeSite<'a> {
    pub path: &'a str,
    pub src_file: &'a str,
    pub dst_file: &'a str,
}

/// Log one replaced item. Auto-merge stages copies under temporary names
/// ending in the destination path, which are noise to a reader, so those
/// read "from upstream" or "from this repo's own copy" instead.
pub(crate) fn log_replacement(site: &MergeSite<'_>, replacement: &Replacement) {
    let path = if site.path.is_empty() {
        "<root>"
    } else {
        site.path
    };
    let origin = if site.src_file.starts_with(AUTO_MERGE_OWN_PREFIX) {
        "this repo's own copy".to_string()
    } else if site.src_file.ends_with(site.dst_file) {
        "upstream".to_string()
    } else {
        site.src_file.to_string()
    };
    info!(
        "{}: updated {}[{}={}] from {}",
        site.dst_file, path, replacement.field, replacement.key, origin
    );
}

/// Apply [`plan_keyed_merge`] to a `Vec`-backed array in place, logging each
/// changed replacement, then insert the new items at `position`.
pub(crate) fn merge_vec_by_key<A>(
    adapter: &A,
    target: &mut Vec<A::Item>,
    source: &[A::Item],
    explicit_key: Option<&str>,
    position: InsertPosition,
    site: &MergeSite<'_>,
) where
    A: KeyedItems,
    A::Item: Clone,
{
    let plan = plan_keyed_merge(adapter, target, source, explicit_key);

    for replacement in &plan.replaced {
        let incoming = &source[replacement.source];
        if adapter.item_changed(&target[replacement.dest], incoming) {
            log_replacement(site, replacement);
        }
        target[replacement.dest] = incoming.clone();
    }

    let new_items: Vec<A::Item> = plan
        .new_items
        .iter()
        .map(|&idx| source[idx].clone())
        .collect();
    match position {
        InsertPosition::Start => {
            let mut merged = new_items;
            merged.append(target);
            *target = merged;
        }
        InsertPosition::End => target.extend(new_items),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal record: scalar fields only, keyed by name. Whole-item
    /// equality is field-list equality.
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Rec(Vec<(&'static str, &'static str)>);

    fn rec(fields: &[(&'static str, &'static str)]) -> Rec {
        Rec(fields.to_vec())
    }

    struct RecItems;

    impl KeyedItems for RecItems {
        type Item = Rec;
        type Key = String;

        fn key_value(&self, item: &Rec, field: &str) -> Option<String> {
            item.0
                .iter()
                .find(|(name, _)| *name == field)
                .map(|(_, value)| value.to_string())
        }

        fn items_equal(&self, a: &Rec, b: &Rec) -> bool {
            a == b
        }

        fn key_text(&self, key: &String) -> String {
            key.clone()
        }
    }

    fn site() -> MergeSite<'static> {
        MergeSite {
            path: "steps",
            src_file: "src.yaml",
            dst_file: "dst.yaml",
        }
    }

    #[test]
    fn keyed_item_replaces_first_unconsumed_match_and_unmatched_is_new() {
        let dest = vec![
            rec(&[("name", "lint"), ("run", "old")]),
            rec(&[("name", "test"), ("run", "cargo test")]),
        ];
        let source = vec![
            rec(&[("name", "lint"), ("run", "new")]),
            rec(&[("name", "docs"), ("run", "mdbook")]),
        ];

        let plan = plan_keyed_merge(&RecItems, &dest, &source, None);

        assert_eq!(
            plan,
            KeyedMergePlan {
                replaced: vec![Replacement {
                    source: 0,
                    dest: 0,
                    field: "name".to_string(),
                    key: "lint".to_string(),
                }],
                new_items: vec![1],
            }
        );
    }

    #[test]
    fn keyless_item_is_new_only_when_not_in_dest_or_already_new() {
        let dest = vec![rec(&[("run", "echo hi")])];
        let source = vec![
            rec(&[("run", "echo hi")]),
            rec(&[("run", "echo new")]),
            rec(&[("run", "echo new")]),
        ];

        let plan = plan_keyed_merge(&RecItems, &dest, &source, None);

        assert_eq!(plan.replaced, Vec::<Replacement>::new());
        assert_eq!(plan.new_items, vec![1]);
    }

    #[test]
    fn explicit_key_overrides_candidates_and_missing_explicit_key_means_keyless() {
        let dest = vec![
            rec(&[("name", "a"), ("email", "x@example.com")]),
            rec(&[("name", "b")]),
        ];
        let source = vec![
            rec(&[("name", "renamed"), ("email", "x@example.com")]),
            rec(&[("name", "b")]),
        ];

        let plan = plan_keyed_merge(&RecItems, &dest, &source, Some("email"));

        assert_eq!(
            plan.replaced,
            vec![Replacement {
                source: 0,
                dest: 0,
                field: "email".to_string(),
                key: "x@example.com".to_string(),
            }]
        );
        // `{name: b}` has no `email`, so it is keyless and dedupes by equality.
        assert_eq!(plan.new_items, Vec::<usize>::new());
    }

    #[test]
    fn candidates_are_tried_in_shared_order() {
        let dest = vec![
            rec(&[("uses", "actions/checkout@v4")]),
            rec(&[("id", "cache"), ("uses", "actions/cache@v4")]),
        ];
        let source = vec![
            rec(&[("uses", "actions/checkout@v4"), ("with", "fetch-depth=0")]),
            rec(&[("uses", "actions/cache@v5"), ("id", "cache")]),
        ];

        let plan = plan_keyed_merge(&RecItems, &dest, &source, None);

        assert_eq!(
            plan.replaced,
            vec![Replacement {
                source: 0,
                dest: 0,
                field: "uses".to_string(),
                key: "actions/checkout@v4".to_string(),
            }]
        );
        // `uses` outranks `id`, so the bumped cache step matches nothing.
        assert_eq!(plan.new_items, vec![1]);
    }

    #[test]
    fn duplicate_keys_consume_dest_items_in_order() {
        let dest = vec![
            rec(&[("name", "x"), ("v", "1")]),
            rec(&[("name", "x"), ("v", "2")]),
        ];
        let source = vec![
            rec(&[("name", "x"), ("v", "3")]),
            rec(&[("name", "x"), ("v", "4")]),
            rec(&[("name", "x"), ("v", "5")]),
        ];

        let plan = plan_keyed_merge(&RecItems, &dest, &source, None);

        let slots: Vec<(usize, usize)> = plan.replaced.iter().map(|r| (r.source, r.dest)).collect();
        assert_eq!(slots, vec![(0, 0), (1, 1)]);
        assert_eq!(plan.new_items, vec![2]);
    }

    #[test]
    fn merge_vec_by_key_replaces_in_place_and_inserts_new_at_end() {
        let mut target = vec![
            rec(&[("name", "lint"), ("run", "old")]),
            rec(&[("name", "local-only")]),
        ];
        let source = vec![
            rec(&[("name", "docs")]),
            rec(&[("name", "lint"), ("run", "new")]),
        ];

        merge_vec_by_key(
            &RecItems,
            &mut target,
            &source,
            None,
            InsertPosition::End,
            &site(),
        );

        assert_eq!(
            target,
            vec![
                rec(&[("name", "lint"), ("run", "new")]),
                rec(&[("name", "local-only")]),
                rec(&[("name", "docs")]),
            ]
        );
    }

    #[test]
    fn merge_vec_by_key_inserts_new_at_start() {
        let mut target = vec![rec(&[("name", "lint")])];
        let source = vec![rec(&[("name", "docs")]), rec(&[("name", "lint")])];

        merge_vec_by_key(
            &RecItems,
            &mut target,
            &source,
            None,
            InsertPosition::Start,
            &site(),
        );

        assert_eq!(
            target,
            vec![rec(&[("name", "docs")]), rec(&[("name", "lint")])]
        );
    }
}
