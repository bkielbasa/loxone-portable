//! Small helpers for inspecting the raw Loxone XML DOM.
//!
//! The core parser already gives us every tag and attribute; these helpers add
//! the path/label/context conventions used by the research reports.

use std::collections::{BTreeMap, BTreeSet};

use loxone_core::model::RawNode;

pub fn node_type(node: &RawNode) -> Option<&str> {
    node.attr("Type").filter(|s| !s.is_empty())
}

pub fn node_uuid(node: &RawNode) -> Option<&str> {
    node.attr("U").filter(|s| !s.is_empty())
}

pub fn node_title(node: &RawNode) -> Option<&str> {
    node.attr("Title").filter(|s| !s.is_empty())
}

/// Short human-readable label for a node, used in paths and diffs.
pub fn node_label(node: &RawNode) -> String {
    match node_type(node) {
        Some(ty) => format!("{}[{}]", node.tag, ty),
        None => node.tag.clone(),
    }
}

/// Walks the DOM depth-first and calls `visit(node, path)`.
///
/// Sibling indices are only added when a label is ambiguous, so ordinary paths
/// stay readable (`ControlList/C[Document]/C[Program]/C[Page]/C[Switch]`).
pub fn walk<'a, F>(roots: &'a [RawNode], parent_path: &str, visit: &mut F)
where
    F: FnMut(&'a RawNode, &str),
{
    walk_slice(roots, parent_path, visit);
}

fn walk_slice<'a, F>(nodes: &'a [RawNode], parent_path: &str, visit: &mut F)
where
    F: FnMut(&'a RawNode, &str),
{
    let mut totals: BTreeMap<String, usize> = BTreeMap::new();
    for node in nodes {
        *totals.entry(node_label(node)).or_default() += 1;
    }

    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for node in nodes {
        let label = node_label(node);
        let index = seen.entry(label.clone()).or_insert(0);
        let segment = if totals.get(&label).copied().unwrap_or(0) > 1 {
            format!("{}[{}]", label, index)
        } else {
            label
        };
        *index += 1;

        let path = if parent_path.is_empty() {
            segment
        } else {
            format!("{}/{}", parent_path, segment)
        };
        visit(node, &path);
        walk_slice(&node.children, &path, visit);
    }
}

/// Collects every pin UUID referenced by an `<In Input="...">` link.
///
/// Hidden links (`FLG="1"`) are included: they still prove that the source pin
/// acts as an output even though the wire is not drawn.
pub fn collect_wire_sources(roots: &[RawNode]) -> BTreeSet<String> {
    let mut sources = BTreeSet::new();
    for root in roots {
        collect_wire_sources_rec(root, &mut sources);
    }
    sources
}

fn collect_wire_sources_rec(node: &RawNode, sources: &mut BTreeSet<String>) {
    if node.tag == "In" {
        if let Some(input) = node.attr("Input").filter(|s| !s.is_empty()) {
            sources.insert(input.to_string());
        }
    }
    for child in &node.children {
        collect_wire_sources_rec(child, sources);
    }
}

/// Returns true for the handful of XML types that form the document skeleton
/// and are handled directly by the parser rather than by a block schema.
pub fn is_structural_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "Document"
            | "CategoryCaption"
            | "Category"
            | "PlaceCaption"
            | "Place"
            | "Program"
            | "Page"
    )
}

/// A compact description of a node, used in diff output.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NodeBrief {
    pub uuid: Option<String>,
    pub tag: String,
    pub type_name: Option<String>,
    pub title: Option<String>,
    pub path: String,
}

impl NodeBrief {
    pub fn from_node(node: &RawNode, path: &str) -> Self {
        Self {
            uuid: node_uuid(node).map(str::to_string),
            tag: node.tag.clone(),
            type_name: node_type(node).map(str::to_string),
            title: node_title(node).map(str::to_string),
            path: path.to_string(),
        }
    }
}
