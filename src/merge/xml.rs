//! XML merge operations
//!
//! Merges XML documents with support for path navigation, recursive deep
//! merging, namespace-aware element matching, and repeated element handling.

use log::warn;
use xot::Xot;

use super::{read_file_as_string, read_file_as_string_optional, write_string_to_file, PathSegment};
use crate::config::{ArrayMergeMode, InsertPosition, XmlMergeOp};
use crate::error::{Error, Result};
use crate::filesystem::MemoryFS;

#[cfg(test)]
use crate::filesystem::File;

/// Context for XML merge operations, bundling diagnostic metadata.
pub(crate) struct XmlMergeContext<'a> {
    path: &'a str,
    src_file: &'a str,
    dst_file: &'a str,
}

/// Navigate to a specific path within an XML tree, returning the target element.
///
/// Path segments map to element local names. For example, path "project.dependencies"
/// navigates to the first `<dependencies>` child of the first `<project>` element.
/// Creates intermediate elements as needed for missing path segments.
pub fn navigate_xml_path(
    xot: &mut Xot,
    node: xot::Node,
    path: &[PathSegment],
) -> Result<xot::Node> {
    let mut current = node;
    for segment in path {
        match segment {
            PathSegment::Key(name) => {
                // Find existing child element with this local name
                let name_id = xot.add_name(name);
                let existing = xot
                    .children(current)
                    .find(|&child| xot.element(child).is_some_and(|el| el.name() == name_id));
                match existing {
                    Some(child) => current = child,
                    None => {
                        // Create intermediate element
                        let new_el = xot.new_element(name_id);
                        xot.append(current, new_el).map_err(|e| Error::Merge {
                            operation: "xml merge".to_string(),
                            message: format!("Failed to create element '{}': {}", name, e),
                        })?;
                        current = new_el;
                    }
                }
            }
            PathSegment::Index(_) => {
                return Err(Error::Merge {
                    operation: "xml merge".to_string(),
                    message: "Array index path segments are not supported for XML navigation"
                        .to_string(),
                });
            }
        }
    }
    Ok(current)
}

/// Recursively merge source element children into target element.
///
/// Elements merge by matching on (local name, namespace URI). Unique source
/// children are appended. Source attributes override target attributes.
/// Repeated siblings with the same name use the unified array merge model.
/// Text content uses last-write-wins.
///
/// When the source content type differs from the target content type (text vs.
/// child elements), the source wins and a warning is emitted per the
/// `TypeMismatchReplace` spec rule.
pub fn merge_xml_elements(
    xot: &mut Xot,
    target: xot::Node,
    source: xot::Node,
    mode: ArrayMergeMode,
    position: InsertPosition,
    ctx: &XmlMergeContext<'_>,
) -> Result<()> {
    // Step 1: merge attributes — source attributes override target attributes
    merge_attributes(xot, target, source)?;

    // Step 2: collect source children by (name_id) to detect repeated elements
    let source_children: Vec<xot::Node> = xot.children(source).collect();

    // Group source element children by name_id
    let mut source_groups: Vec<(xot::NameId, Vec<xot::Node>)> = Vec::new();
    let mut seen_names: Vec<xot::NameId> = Vec::new();
    // Track non-element source children separately
    let mut source_text_content: Option<String> = None;
    // Comments and PIs from source that need to be carried over
    let mut source_comments_and_pis: Vec<xot::Node> = Vec::new();

    for &child in &source_children {
        if let Some(el) = xot.element(child) {
            let name_id = el.name();
            if let Some(pos) = seen_names.iter().position(|&n| n == name_id) {
                source_groups[pos].1.push(child);
            } else {
                seen_names.push(name_id);
                source_groups.push((name_id, vec![child]));
            }
        } else if let Some(text) = xot.text_str(child) {
            source_text_content = Some(text.to_string());
        } else if xot.is_comment(child) || xot.is_processing_instruction(child) {
            source_comments_and_pis.push(child);
        }
    }

    // Check target content type for type mismatch detection
    let target_has_elements = xot
        .children(target)
        .any(|child| xot.element(child).is_some());
    let target_has_text = xot.children(target).any(|child| xot.text(child).is_some());
    let source_has_elements = !source_groups.is_empty();

    // Step 3: handle text content (last-write-wins from source)
    if let Some(src_text) = &source_text_content {
        if target_has_elements && !target_has_text {
            warn!(
                "{} -> {}: Type mismatch at path '{}': replacing child elements with text content",
                ctx.src_file, ctx.dst_file, ctx.path
            );
        }
        apply_text_content(xot, target, src_text)?;
    }

    // Step 4: carry over comments and processing instructions from source
    for &node in &source_comments_and_pis {
        clone_subtree_into(xot, target, node, None)?;
    }

    if source_has_elements && target_has_text && !target_has_elements {
        warn!(
            "{} -> {}: Type mismatch at path '{}': replacing text content with child elements",
            ctx.src_file, ctx.dst_file, ctx.path
        );
    }

    // Step 5: merge element children
    for (name_id, source_elems) in &source_groups {
        let (local_name, _ns) = xot.name_ns_str(*name_id);
        let child_path = if ctx.path.is_empty() {
            local_name.to_string()
        } else {
            format!("{}.{}", ctx.path, local_name)
        };
        let child_ctx = XmlMergeContext {
            path: &child_path,
            src_file: ctx.src_file,
            dst_file: ctx.dst_file,
        };

        let dest_elems: Vec<xot::Node> = xot
            .children(target)
            .filter(|&child| xot.element(child).is_some_and(|el| el.name() == *name_id))
            .collect();

        let is_repeated = source_elems.len() > 1 || dest_elems.len() > 1;

        if is_repeated {
            merge_repeated_elements(
                xot,
                target,
                &dest_elems,
                source_elems,
                *name_id,
                mode,
                position,
            )?;
        } else if dest_elems.len() == 1 && source_elems.len() == 1 {
            merge_xml_elements(
                xot,
                dest_elems[0],
                source_elems[0],
                mode,
                position,
                &child_ctx,
            )?;
        } else {
            for &src_elem in source_elems {
                clone_subtree_into(xot, target, src_elem, None)?;
            }
        }
    }

    Ok(())
}

/// Merge attributes from source element onto target element.
/// Source attributes override target attributes with the same name.
fn merge_attributes(xot: &mut Xot, target: xot::Node, source: xot::Node) -> Result<()> {
    // Collect source attributes
    let source_attrs: Vec<(xot::NameId, String)> = xot
        .attributes(source)
        .iter()
        .map(|(name, value)| (name, value.to_string()))
        .collect();

    // Apply to target
    let mut target_attrs = xot.attributes_mut(target);
    for (name, value) in source_attrs {
        target_attrs.insert(name, value);
    }
    Ok(())
}

/// Set text content of an element, replacing any existing text child.
fn apply_text_content(xot: &mut Xot, target: xot::Node, text: &str) -> Result<()> {
    // Find and update existing text child, or create one
    let existing_text = xot
        .children(target)
        .find(|&child| xot.text(child).is_some());
    if let Some(text_node) = existing_text {
        xot.text_mut(text_node).unwrap().set(text);
    } else {
        xot.append_text(target, text).map_err(|e| Error::Merge {
            operation: "xml merge".to_string(),
            message: format!("Failed to set text content: {}", e),
        })?;
    }
    Ok(())
}

/// Handle repeated sibling elements using the unified array merge model.
fn merge_repeated_elements(
    xot: &mut Xot,
    parent: xot::Node,
    dest_elems: &[xot::Node],
    source_elems: &[xot::Node],
    _name_id: xot::NameId,
    mode: ArrayMergeMode,
    position: InsertPosition,
) -> Result<()> {
    match mode {
        ArrayMergeMode::Replace => {
            // Remove all dest siblings with this name
            for &node in dest_elems {
                xot.remove(node).map_err(|e| Error::Merge {
                    operation: "xml merge".to_string(),
                    message: format!("Failed to remove element: {}", e),
                })?;
            }
            // Add source siblings
            for &src in source_elems {
                clone_subtree_into(xot, parent, src, None)?;
            }
        }
        ArrayMergeMode::Append => {
            match position {
                InsertPosition::Start => {
                    // Insert source siblings before the first dest sibling
                    let insert_before = dest_elems.first().copied();
                    for &src in source_elems {
                        clone_subtree_into(xot, parent, src, insert_before)?;
                    }
                }
                InsertPosition::End => {
                    // Insert source siblings after the last dest sibling
                    let insert_after = dest_elems.last().copied();
                    // Find the node after the last dest element (if any)
                    let insert_before = insert_after.and_then(|n| xot.next_sibling(n));
                    for &src in source_elems {
                        clone_subtree_into(xot, parent, src, insert_before)?;
                    }
                }
            }
        }
        ArrayMergeMode::AppendUnique => {
            // Collect text content of existing dest elements
            let dest_texts: Vec<String> = dest_elems
                .iter()
                .map(|&node| xot.text_content_str(node).unwrap_or("").to_string())
                .collect();

            // Filter source elements whose text content is not in dest
            let unique_sources: Vec<xot::Node> = source_elems
                .iter()
                .filter(|&&node| {
                    let text = xot.text_content_str(node).unwrap_or("").to_string();
                    !dest_texts.contains(&text)
                })
                .copied()
                .collect();

            match position {
                InsertPosition::Start => {
                    let insert_before = dest_elems.first().copied();
                    for &src in &unique_sources {
                        clone_subtree_into(xot, parent, src, insert_before)?;
                    }
                }
                InsertPosition::End => {
                    let insert_after = dest_elems.last().copied();
                    let insert_before = insert_after.and_then(|n| xot.next_sibling(n));
                    for &src in &unique_sources {
                        clone_subtree_into(xot, parent, src, insert_before)?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Deep-clone a subtree from source into parent (optionally before a reference node).
fn clone_subtree_into(
    xot: &mut Xot,
    parent: xot::Node,
    source: xot::Node,
    insert_before: Option<xot::Node>,
) -> Result<()> {
    let cloned = xot.clone_with_prefixes(source);
    if let Some(before) = insert_before {
        xot.insert_before(before, cloned)
            .map_err(|e| Error::Merge {
                operation: "xml merge".to_string(),
                message: format!("Failed to insert element: {}", e),
            })?;
    } else {
        xot.append(parent, cloned).map_err(|e| Error::Merge {
            operation: "xml merge".to_string(),
            message: format!("Failed to append element: {}", e),
        })?;
    }
    Ok(())
}

/// Apply an XML merge operation to the filesystem.
///
/// Reads source and destination XML files, merges them according to the
/// operation's configuration, and writes the result back to the destination.
pub fn apply_xml_merge_operation(fs: &mut MemoryFS, op: &XmlMergeOp) -> Result<()> {
    op.validate()?;
    let source_path = op.get_source().expect("source validated");
    let dest_path = op.get_dest().expect("dest validated");

    let source_content = read_file_as_string(fs, source_path)?;
    let dest_content = read_file_as_string_optional(fs, dest_path)?;

    let mut xot = Xot::new();

    // Parse source
    let source_root = xot.parse(&source_content).map_err(|e| Error::Merge {
        operation: "xml merge".to_string(),
        message: format!("Failed to parse source XML: {}", e),
    })?;
    let source_doc_el = xot
        .document_element(source_root)
        .map_err(|e| Error::Merge {
            operation: "xml merge".to_string(),
            message: format!("Source XML has no document element: {}", e),
        })?;

    // Parse or create dest
    let dest_root = if let Some(content) = dest_content {
        xot.parse(&content).map_err(|e| Error::Merge {
            operation: "xml merge".to_string(),
            message: format!("Failed to parse destination XML: {}", e),
        })?
    } else {
        // Auto-create dest with root element matching source
        let source_el = xot.element(source_doc_el).ok_or_else(|| Error::Merge {
            operation: "xml merge".to_string(),
            message: "Source root is not an element".to_string(),
        })?;
        let root_name = source_el.name();
        let new_root = xot.parse("<placeholder/>").map_err(|e| Error::Merge {
            operation: "xml merge".to_string(),
            message: format!("Failed to create destination stub: {}", e),
        })?;
        let new_doc_el = xot.document_element(new_root).unwrap();
        // Rename the placeholder to match source root
        xot.element_mut(new_doc_el).unwrap().set_name(root_name);
        new_root
    };
    let dest_doc_el = xot.document_element(dest_root).map_err(|e| Error::Merge {
        operation: "xml merge".to_string(),
        message: format!("Destination XML has no document element: {}", e),
    })?;

    // Navigate to target path within dest
    let path = super::parse_path(op.path.as_deref().unwrap_or(""));
    let target = navigate_xml_path(&mut xot, dest_doc_el, &path)?;

    // Build merge context for diagnostic logging
    let path_str = op.path.as_deref().unwrap_or("");
    let ctx = XmlMergeContext {
        path: path_str,
        src_file: source_path,
        dst_file: dest_path,
    };

    // Merge source into target
    merge_xml_elements(
        &mut xot,
        target,
        source_doc_el,
        op.array_mode,
        op.position,
        &ctx,
    )?;

    // Serialize with XML declaration
    let xml_params = xot::output::xml::Parameters {
        declaration: Some(xot::output::xml::Declaration::default()),
        ..Default::default()
    };
    let serialized = xot
        .serialize_xml_string(xml_params, dest_root)
        .map_err(|e| Error::Merge {
            operation: "xml merge".to_string(),
            message: format!("Failed to serialize XML: {}", e),
        })?;

    write_string_to_file(fs, dest_path, serialized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ArrayMergeMode;
    use crate::merge::parse_path;

    // =====================================================================
    // Spec: AutoCreateDestination (XML case)
    // When dest file does not exist, create it with XML declaration and root
    // element matching the source document's root element.
    // =====================================================================

    #[test]
    fn creates_dest_if_missing() {
        let mut fs = MemoryFS::new();
        fs.add_file(
            "source.xml",
            File::from_string("<config><key>value</key></config>"),
        )
        .unwrap();

        let op = XmlMergeOp::new().source("source.xml").dest("dest.xml");
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        assert!(result.contains("<config>"));
        assert!(result.contains("<key>value</key>"));
    }

    #[test]
    fn auto_created_dest_includes_xml_declaration() {
        let mut fs = MemoryFS::new();
        fs.add_file(
            "source.xml",
            File::from_string("<root><item>data</item></root>"),
        )
        .unwrap();

        let op = XmlMergeOp::new().source("source.xml").dest("dest.xml");
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        assert!(
            result.starts_with("<?xml"),
            "Auto-created XML dest should start with XML declaration, got: {}",
            &result[..result.len().min(80)]
        );
    }

    // =====================================================================
    // Spec: MissingSourceExplicit
    // Explicit source/dest with missing source file is an error.
    // =====================================================================

    #[test]
    fn missing_source_explicit_is_error() {
        let mut fs = MemoryFS::new();
        fs.add_file("dest.xml", File::from_string("<root/>"))
            .unwrap();

        let op = XmlMergeOp::new().source("missing.xml").dest("dest.xml");
        let result = apply_xml_merge_operation(&mut fs, &op);
        assert!(result.is_err());
    }

    #[test]
    fn test_xml_merge_missing_source_includes_sequential_hint() {
        let mut fs = MemoryFS::new();
        fs.add_file("dest.xml", File::from_string("<root/>"))
            .unwrap();

        let op = XmlMergeOp {
            source: Some("missing.xml".to_string()),
            dest: Some("dest.xml".to_string()),
            ..Default::default()
        };

        let err_msg = apply_xml_merge_operation(&mut fs, &op)
            .unwrap_err()
            .to_string();
        assert!(
            err_msg.contains("renamed or excluded by a preceding operation"),
            "missing-source error should include sequential-context hint, got: {}",
            err_msg
        );
    }

    // =====================================================================
    // Spec: XmlDeepMerge — basic recursive element merge
    // Elements merge by matching on (local name, namespace URI).
    // Source attributes override dest attributes on same element.
    // Destination-only elements and attributes are preserved.
    // Text content uses last-write-wins.
    // =====================================================================

    #[test]
    fn deep_merge_at_root() {
        let mut fs = MemoryFS::new();
        fs.add_file(
            "source.xml",
            File::from_string("<config><db><port>5433</port></db></config>"),
        )
        .unwrap();
        fs.add_file(
            "dest.xml",
            File::from_string("<config><db><host>localhost</host><port>5432</port></db></config>"),
        )
        .unwrap();

        let op = XmlMergeOp::new().source("source.xml").dest("dest.xml");
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        // Source text wins for <port>
        assert!(result.contains("<port>5433</port>"));
        // Dest-only element <host> is preserved
        assert!(result.contains("<host>localhost</host>"));
    }

    #[test]
    fn source_attributes_override_dest() {
        let mut fs = MemoryFS::new();
        fs.add_file(
            "source.xml",
            File::from_string(r#"<config><server host="new-host"/></config>"#),
        )
        .unwrap();
        fs.add_file(
            "dest.xml",
            File::from_string(r#"<config><server host="old-host" port="8080"/></config>"#),
        )
        .unwrap();

        let op = XmlMergeOp::new().source("source.xml").dest("dest.xml");
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        // Source attribute wins
        assert!(result.contains(r#"host="new-host""#));
        // Dest-only attribute preserved
        assert!(result.contains(r#"port="8080""#));
    }

    #[test]
    fn dest_only_elements_preserved() {
        let mut fs = MemoryFS::new();
        fs.add_file("source.xml", File::from_string("<root><a>1</a></root>"))
            .unwrap();
        fs.add_file(
            "dest.xml",
            File::from_string("<root><a>0</a><b>2</b></root>"),
        )
        .unwrap();

        let op = XmlMergeOp::new().source("source.xml").dest("dest.xml");
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        assert!(result.contains("<a>1</a>"));
        assert!(result.contains("<b>2</b>"));
    }

    #[test]
    fn source_new_elements_added() {
        let mut fs = MemoryFS::new();
        fs.add_file(
            "source.xml",
            File::from_string("<root><a>1</a><c>3</c></root>"),
        )
        .unwrap();
        fs.add_file("dest.xml", File::from_string("<root><a>0</a></root>"))
            .unwrap();

        let op = XmlMergeOp::new().source("source.xml").dest("dest.xml");
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        assert!(result.contains("<a>1</a>"));
        assert!(result.contains("<c>3</c>"));
    }

    // =====================================================================
    // Spec: NavigatePath — dot-notation path targeting
    // Path uses parse_path() to navigate by element local name.
    // =====================================================================

    #[test]
    fn merge_at_path() {
        let mut fs = MemoryFS::new();
        fs.add_file(
            "source.xml",
            File::from_string("<wrapper><timeout>30</timeout></wrapper>"),
        )
        .unwrap();
        fs.add_file(
            "dest.xml",
            File::from_string("<project><settings><timeout>10</timeout></settings></project>"),
        )
        .unwrap();

        let op = XmlMergeOp::new()
            .source("source.xml")
            .dest("dest.xml")
            .path("project.settings");
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        assert!(result.contains("<timeout>30</timeout>"));
    }

    #[test]
    fn path_creates_intermediate_elements() {
        let mut fs = MemoryFS::new();
        fs.add_file(
            "source.xml",
            File::from_string("<wrapper><key>val</key></wrapper>"),
        )
        .unwrap();
        fs.add_file("dest.xml", File::from_string("<root/>"))
            .unwrap();

        let op = XmlMergeOp::new()
            .source("source.xml")
            .dest("dest.xml")
            .path("root.nested.deep");
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        assert!(result.contains("<nested>"));
        assert!(result.contains("<deep>"));
        assert!(result.contains("<key>val</key>"));
    }

    // =====================================================================
    // Spec: XmlRepeatedElements — array merge modes
    // Repeated sibling elements with the same name act as arrays.
    // =====================================================================

    #[test]
    fn repeated_elements_replace() {
        let mut fs = MemoryFS::new();
        fs.add_file(
            "source.xml",
            File::from_string("<deps><dep>new-a</dep><dep>new-b</dep></deps>"),
        )
        .unwrap();
        fs.add_file(
            "dest.xml",
            File::from_string("<deps><dep>old-a</dep><dep>old-b</dep><dep>old-c</dep></deps>"),
        )
        .unwrap();

        let op = XmlMergeOp::new()
            .source("source.xml")
            .dest("dest.xml")
            .array_mode(ArrayMergeMode::Replace);
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        assert!(result.contains("<dep>new-a</dep>"));
        assert!(result.contains("<dep>new-b</dep>"));
        assert!(!result.contains("<dep>old-a</dep>"));
        assert!(!result.contains("<dep>old-c</dep>"));
    }

    #[test]
    fn repeated_elements_append_end() {
        let mut fs = MemoryFS::new();
        fs.add_file(
            "source.xml",
            File::from_string("<deps><dep>new-a</dep><dep>new-b</dep></deps>"),
        )
        .unwrap();
        fs.add_file(
            "dest.xml",
            File::from_string("<deps><dep>old-a</dep><dep>old-b</dep></deps>"),
        )
        .unwrap();

        let op = XmlMergeOp::new()
            .source("source.xml")
            .dest("dest.xml")
            .array_mode(ArrayMergeMode::Append)
            .position(InsertPosition::End);
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        // All present, old before new
        assert!(result.contains("<dep>old-a</dep>"));
        assert!(result.contains("<dep>old-b</dep>"));
        assert!(result.contains("<dep>new-a</dep>"));
        assert!(result.contains("<dep>new-b</dep>"));
        let old_pos = result.find("<dep>old-a</dep>").unwrap();
        let new_pos = result.find("<dep>new-a</dep>").unwrap();
        assert!(old_pos < new_pos);
    }

    #[test]
    fn repeated_elements_append_start() {
        let mut fs = MemoryFS::new();
        fs.add_file(
            "source.xml",
            File::from_string("<deps><dep>new-a</dep><dep>new-b</dep></deps>"),
        )
        .unwrap();
        fs.add_file(
            "dest.xml",
            File::from_string("<deps><dep>old-a</dep><dep>old-b</dep></deps>"),
        )
        .unwrap();

        let op = XmlMergeOp::new()
            .source("source.xml")
            .dest("dest.xml")
            .array_mode(ArrayMergeMode::Append)
            .position(InsertPosition::Start);
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        let new_pos = result.find("<dep>new-a</dep>").unwrap();
        let old_pos = result.find("<dep>old-a</dep>").unwrap();
        assert!(new_pos < old_pos);
    }

    #[test]
    fn repeated_elements_append_unique() {
        let mut fs = MemoryFS::new();
        fs.add_file(
            "source.xml",
            File::from_string("<deps><dep>existing</dep><dep>fresh</dep></deps>"),
        )
        .unwrap();
        fs.add_file(
            "dest.xml",
            File::from_string("<deps><dep>existing</dep></deps>"),
        )
        .unwrap();

        let op = XmlMergeOp::new()
            .source("source.xml")
            .dest("dest.xml")
            .array_mode(ArrayMergeMode::AppendUnique);
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        // "existing" appears only once
        let count = result.matches("<dep>existing</dep>").count();
        assert_eq!(count, 1);
        // "fresh" is added
        assert!(result.contains("<dep>fresh</dep>"));
    }

    // =====================================================================
    // Spec: XmlNamespaceHandling
    // Elements matched by (local name, namespace URI), not prefix.
    // Different namespace URIs are distinct elements.
    // =====================================================================

    #[test]
    fn namespace_aware_matching() {
        let mut fs = MemoryFS::new();
        fs.add_file(
            "source.xml",
            File::from_string(
                r#"<root xmlns:ns="http://example.com"><ns:item>updated</ns:item></root>"#,
            ),
        )
        .unwrap();
        fs.add_file(
            "dest.xml",
            File::from_string(
                r#"<root xmlns:ns="http://example.com"><ns:item>original</ns:item></root>"#,
            ),
        )
        .unwrap();

        let op = XmlMergeOp::new().source("source.xml").dest("dest.xml");
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        assert!(result.contains("updated"));
        assert!(!result.contains("original"));
    }

    #[test]
    fn different_namespaces_are_distinct() {
        let mut fs = MemoryFS::new();
        fs.add_file(
            "source.xml",
            File::from_string(r#"<root xmlns:a="http://ns-a"><a:item>from-a</a:item></root>"#),
        )
        .unwrap();
        fs.add_file(
            "dest.xml",
            File::from_string(r#"<root xmlns:b="http://ns-b"><b:item>from-b</b:item></root>"#),
        )
        .unwrap();

        let op = XmlMergeOp::new().source("source.xml").dest("dest.xml");
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        // Both preserved because different namespace URIs make them distinct
        assert!(result.contains("from-a"));
        assert!(result.contains("from-b"));
    }

    // =====================================================================
    // Spec: TrailingNewline invariant
    // All merge output ends with a trailing newline.
    // =====================================================================

    #[test]
    fn output_ends_with_trailing_newline() {
        let mut fs = MemoryFS::new();
        fs.add_file("source.xml", File::from_string("<r><a>1</a></r>"))
            .unwrap();
        fs.add_file("dest.xml", File::from_string("<r/>")).unwrap();

        let op = XmlMergeOp::new().source("source.xml").dest("dest.xml");
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        assert!(result.ends_with('\n'));
    }

    // =====================================================================
    // Spec: MergeNeverCorrupts invariant
    // Output must be well-formed XML.
    // =====================================================================

    #[test]
    fn output_is_well_formed_xml() {
        let mut fs = MemoryFS::new();
        fs.add_file(
            "source.xml",
            File::from_string("<root><a x=\"1\">text</a><b/></root>"),
        )
        .unwrap();
        fs.add_file(
            "dest.xml",
            File::from_string("<root><a y=\"2\">old</a><c/></root>"),
        )
        .unwrap();

        let op = XmlMergeOp::new().source("source.xml").dest("dest.xml");
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        // Must parse as valid XML
        let mut xot = Xot::new();
        assert!(xot.parse(&result).is_ok());
    }

    // =====================================================================
    // Spec: ValidateMergeOperation
    // auto_merge XOR (source + dest)
    // =====================================================================

    #[test]
    fn validate_rejects_auto_merge_with_explicit_source() {
        let op = XmlMergeOp {
            source: Some("s.xml".to_string()),
            auto_merge: Some("a.xml".to_string()),
            ..Default::default()
        };
        assert!(op.validate().is_err());
    }

    #[test]
    fn validate_rejects_missing_source_and_dest() {
        let op = XmlMergeOp {
            source: Some("s.xml".to_string()),
            ..Default::default()
        };
        assert!(op.validate().is_err());
    }

    // =====================================================================
    // Spec: Comments and PIs preserved
    // =====================================================================

    #[test]
    fn dest_comments_preserved_in_output() {
        let mut fs = MemoryFS::new();
        fs.add_file(
            "source.xml",
            File::from_string("<root><new>val</new></root>"),
        )
        .unwrap();
        fs.add_file(
            "dest.xml",
            File::from_string("<root><!-- important --><existing>1</existing></root>"),
        )
        .unwrap();

        let op = XmlMergeOp::new().source("source.xml").dest("dest.xml");
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        assert!(result.contains("<!-- important -->"));
    }

    #[test]
    fn source_comments_preserved_in_merged_output() {
        let mut fs = MemoryFS::new();
        fs.add_file(
            "source.xml",
            File::from_string("<root><!-- source comment --><item>val</item></root>"),
        )
        .unwrap();
        fs.add_file(
            "dest.xml",
            File::from_string("<root><existing>1</existing></root>"),
        )
        .unwrap();

        let op = XmlMergeOp::new().source("source.xml").dest("dest.xml");
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        assert!(
            result.contains("<!-- source comment -->"),
            "Source comments should be preserved in merged output, got: {}",
            result
        );
    }

    #[test]
    fn source_processing_instructions_preserved_in_merged_output() {
        let mut fs = MemoryFS::new();
        fs.add_file(
            "source.xml",
            File::from_string("<root><?my-pi some data?><item>val</item></root>"),
        )
        .unwrap();
        fs.add_file(
            "dest.xml",
            File::from_string("<root><existing>1</existing></root>"),
        )
        .unwrap();

        let op = XmlMergeOp::new().source("source.xml").dest("dest.xml");
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        assert!(
            result.contains("<?my-pi some data?>"),
            "Source PIs should be preserved in merged output, got: {}",
            result
        );
    }

    #[test]
    fn both_source_and_dest_comments_preserved() {
        let mut fs = MemoryFS::new();
        fs.add_file(
            "source.xml",
            File::from_string("<root><!-- from source --><item>val</item></root>"),
        )
        .unwrap();
        fs.add_file(
            "dest.xml",
            File::from_string("<root><!-- from dest --><existing>1</existing></root>"),
        )
        .unwrap();

        let op = XmlMergeOp::new().source("source.xml").dest("dest.xml");
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        assert!(
            result.contains("<!-- from dest -->"),
            "Dest comments should be preserved, got: {}",
            result
        );
        assert!(
            result.contains("<!-- from source -->"),
            "Source comments should be preserved, got: {}",
            result
        );
    }

    // =====================================================================
    // navigate_xml_path unit tests
    // =====================================================================

    #[test]
    fn navigate_empty_path_returns_node_itself() {
        let mut xot = Xot::new();
        let root = xot.parse("<root/>").unwrap();
        let doc_el = xot.document_element(root).unwrap();

        let path: Vec<PathSegment> = vec![];
        let result = navigate_xml_path(&mut xot, doc_el, &path).unwrap();
        assert_eq!(result, doc_el);
    }

    #[test]
    fn navigate_finds_existing_child() {
        let mut xot = Xot::new();
        let root = xot.parse("<root><child/></root>").unwrap();
        let doc_el = xot.document_element(root).unwrap();

        let path = parse_path("child");
        let result = navigate_xml_path(&mut xot, doc_el, &path).unwrap();
        let el = xot.element(result).unwrap();
        let (local_name, _ns) = xot.name_ns_str(el.name());
        assert_eq!(local_name, "child");
    }

    #[test]
    fn navigate_creates_missing_element() {
        let mut xot = Xot::new();
        let root = xot.parse("<root/>").unwrap();
        let doc_el = xot.document_element(root).unwrap();

        let path = parse_path("child.grandchild");
        let result = navigate_xml_path(&mut xot, doc_el, &path).unwrap();
        let el = xot.element(result).unwrap();
        let (local_name, _ns) = xot.name_ns_str(el.name());
        assert_eq!(local_name, "grandchild");
    }

    // =====================================================================
    // Spec: TypeMismatchReplace — warn on text vs elements mismatch
    // When source has text content and dest has child elements (or vice
    // versa), replace with a warning per the spec rule.
    // =====================================================================

    #[test]
    fn text_into_element_children_warns_type_mismatch() {
        testing_logger::setup();

        let mut fs = MemoryFS::new();
        // Source has only text content
        fs.add_file(
            "source.xml",
            File::from_string("<root><item>just text</item></root>"),
        )
        .unwrap();
        // Dest has child elements under <item>
        fs.add_file(
            "dest.xml",
            File::from_string("<root><item><child>nested</child></item></root>"),
        )
        .unwrap();

        let op = XmlMergeOp::new().source("source.xml").dest("dest.xml");
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        // Source wins: text replaces child elements
        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        assert!(
            result.contains("just text"),
            "Source text should replace dest children, got: {}",
            result
        );

        // A warning should have been emitted about the type mismatch
        testing_logger::validate(|captured_logs| {
            let found = captured_logs.iter().any(|log| {
                log.level == log::Level::Warn
                    && log.body.contains("Type mismatch")
                    && log.body.contains("item")
            });
            assert!(
                found,
                "Expected a type mismatch warning mentioning 'item', got: {:?}",
                captured_logs.iter().map(|l| &l.body).collect::<Vec<_>>()
            );
        });
    }

    #[test]
    fn elements_into_text_content_warns_type_mismatch() {
        testing_logger::setup();

        let mut fs = MemoryFS::new();
        // Source has child elements
        fs.add_file(
            "source.xml",
            File::from_string("<root><item><child>nested</child></item></root>"),
        )
        .unwrap();
        // Dest has only text content
        fs.add_file(
            "dest.xml",
            File::from_string("<root><item>just text</item></root>"),
        )
        .unwrap();

        let op = XmlMergeOp::new().source("source.xml").dest("dest.xml");
        apply_xml_merge_operation(&mut fs, &op).unwrap();

        // Source wins: child elements replace text
        let result = read_file_as_string(&fs, "dest.xml").unwrap();
        assert!(
            result.contains("<child>nested</child>"),
            "Source elements should be merged in, got: {}",
            result
        );

        // A warning should have been emitted about the type mismatch
        testing_logger::validate(|captured_logs| {
            let found = captured_logs.iter().any(|log| {
                log.level == log::Level::Warn
                    && log.body.contains("Type mismatch")
                    && log.body.contains("item")
            });
            assert!(
                found,
                "Expected a type mismatch warning mentioning 'item', got: {:?}",
                captured_logs.iter().map(|l| &l.body).collect::<Vec<_>>()
            );
        });
    }
}
