//! Structural diff between two project saves.
//!
//! This is the core feedback loop for reverse engineering: make exactly one
//! change in Loxone Config, save a second copy, and let the diff show precisely
//! which XML attributes, pins, and child elements changed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::Result;
use loxone_core::model::RawNode;
use serde::{Deserialize, Serialize};

use crate::corpus::load_file;
use crate::dom::{node_label, node_title, node_uuid, walk, NodeBrief};

#[derive(Debug, Clone)]
pub struct DiffOptions {
    /// Attributes that change on every save (or are otherwise noise).
    pub ignore_attributes: BTreeSet<String>,
    /// Restrict the report to one block/tree type.
    pub only_type: Option<String>,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            ignore_attributes: ["SL", "DateS", "CDate"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            only_type: None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DiffReport {
    pub old_path: String,
    pub new_path: String,
    pub ignored_attributes: Vec<String>,
    pub summary: DiffSummary,
    pub added: Vec<NodeBrief>,
    pub removed: Vec<NodeBrief>,
    pub changed: Vec<NodeDiff>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiffSummary {
    pub added: usize,
    pub removed: usize,
    pub changed_nodes: usize,
    pub field_changes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDiff {
    pub brief: NodeBrief,
    pub changes: Vec<FieldChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChange {
    pub field: String,
    pub old: Option<String>,
    pub new: Option<String>,
    pub note: String,
}

struct Indexed<'a> {
    node: &'a RawNode,
    path: String,
}

pub fn diff_files(old_path: &Path, new_path: &Path, options: &DiffOptions) -> Result<DiffReport> {
    let old = load_file(old_path)?;
    let new = load_file(new_path)?;
    let old_index = index_nodes(&old.roots);
    let new_index = index_nodes(&new.roots);

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();

    for (key, new_entry) in &new_index {
        if !old_index.contains_key(key) {
            added.push(NodeBrief::from_node(new_entry.node, &new_entry.path));
        }
    }
    for (key, old_entry) in &old_index {
        if !new_index.contains_key(key) {
            removed.push(NodeBrief::from_node(old_entry.node, &old_entry.path));
        }
    }

    for (key, new_entry) in &new_index {
        let Some(old_entry) = old_index.get(key) else {
            continue;
        };
        let mut changes = Vec::new();
        diff_node(old_entry.node, new_entry.node, "", &mut changes, options);
        if !changes.is_empty() {
            changed.push(NodeDiff {
                brief: NodeBrief::from_node(new_entry.node, &new_entry.path),
                changes,
            });
        }
    }

    if let Some(only) = &options.only_type {
        added.retain(|n| n.type_name.as_deref() == Some(only.as_str()));
        removed.retain(|n| n.type_name.as_deref() == Some(only.as_str()));
        changed.retain(|n| n.brief.type_name.as_deref() == Some(only.as_str()));
    }

    added.sort_by(|a, b| sort_brief(a).cmp(&sort_brief(b)));
    removed.sort_by(|a, b| sort_brief(a).cmp(&sort_brief(b)));
    changed.sort_by(|a, b| sort_brief(&a.brief).cmp(&sort_brief(&b.brief)));

    let field_changes = changed.iter().map(|n| n.changes.len()).sum();
    let summary = DiffSummary {
        added: added.len(),
        removed: removed.len(),
        changed_nodes: changed.len(),
        field_changes,
    };

    Ok(DiffReport {
        old_path: old_path.display().to_string(),
        new_path: new_path.display().to_string(),
        ignored_attributes: options.ignore_attributes.iter().cloned().collect(),
        summary,
        added,
        removed,
        changed,
    })
}

fn sort_brief(brief: &NodeBrief) -> (String, String, String) {
    (
        brief.type_name.clone().unwrap_or_default(),
        brief.title.clone().unwrap_or_default(),
        brief.path.clone(),
    )
}

fn index_nodes<'a>(roots: &'a [RawNode]) -> BTreeMap<String, Indexed<'a>> {
    let mut index = BTreeMap::new();
    walk(roots, "", &mut |node, path| {
        if node.tag == "Co" || node.tag == "In" {
            return;
        }
        let key = if let Some(uuid) = node_uuid(node) {
            format!("uuid:{uuid}")
        } else if node.tag == "ControlList" {
            "root:ControlList".to_string()
        } else {
            return;
        };

        let key = if index.contains_key(&key) {
            format!("{key}@{path}")
        } else {
            key
        };
        index.insert(
            key,
            Indexed {
                node,
                path: path.to_string(),
            },
        );
    });
    index
}

fn diff_node(
    old: &RawNode,
    new: &RawNode,
    path: &str,
    changes: &mut Vec<FieldChange>,
    options: &DiffOptions,
) {
    for (name, old_value) in &old.attributes {
        if name == "U" || name == "Type" || options.ignore_attributes.contains(name) {
            continue;
        }
        match new.attr(name) {
            Some(new_value) if new_value == old_value => {}
            Some(new_value) => changes.push(FieldChange {
                field: join_field(path, name),
                old: Some(old_value.clone()),
                new: Some(new_value.to_string()),
                note: "attribute changed".to_string(),
            }),
            None => changes.push(FieldChange {
                field: join_field(path, name),
                old: Some(old_value.clone()),
                new: None,
                note: "attribute removed".to_string(),
            }),
        }
    }
    for (name, new_value) in &new.attributes {
        if name == "U"
            || name == "Type"
            || options.ignore_attributes.contains(name)
            || old.attr(name).is_some()
        {
            continue;
        }
        changes.push(FieldChange {
            field: join_field(path, name),
            old: None,
            new: Some(new_value.clone()),
            note: "attribute added".to_string(),
        });
    }

    let old_text = old.text.as_deref().unwrap_or("");
    let new_text = new.text.as_deref().unwrap_or("");
    if old_text != new_text {
        changes.push(FieldChange {
            field: join_field(path, "#text"),
            old: (!old_text.is_empty()).then(|| old_text.to_string()),
            new: (!new_text.is_empty()).then(|| new_text.to_string()),
            note: "text changed".to_string(),
        });
    }

    let old_children = child_map(old);
    let new_children = child_map(new);

    for (key, new_child) in &new_children {
        if !old_children.contains_key(key) {
            changes.push(FieldChange {
                field: join_field(path, key),
                old: None,
                new: Some(brief_text(new_child)),
                note: "child added".to_string(),
            });
        }
    }
    for (key, old_child) in &old_children {
        if !new_children.contains_key(key) {
            changes.push(FieldChange {
                field: join_field(path, key),
                old: Some(brief_text(old_child)),
                new: None,
                note: "child removed".to_string(),
            });
        }
    }

    for (key, new_child) in &new_children {
        let Some(old_child) = old_children.get(key) else {
            continue;
        };
        let child_path = join_field(path, key);
        if old_child.tag == "Co" {
            diff_pin(old_child, new_child, &child_path, changes, options);
        } else if should_recurse(old_child) && should_recurse(new_child) {
            diff_node(old_child, new_child, &child_path, changes, options);
        }
    }
}

fn should_recurse(node: &RawNode) -> bool {
    // Nodes with their own UUID are indexed and diffed independently.
    node.tag == "In" || node_uuid(node).is_none()
}

fn diff_pin(
    old: &RawNode,
    new: &RawNode,
    path: &str,
    changes: &mut Vec<FieldChange>,
    options: &DiffOptions,
) {
    for (name, old_value) in &old.attributes {
        if name == "U" || name == "K" || options.ignore_attributes.contains(name) {
            continue;
        }
        match new.attr(name) {
            Some(new_value) if new_value == old_value => {}
            Some(new_value) => changes.push(FieldChange {
                field: join_field(path, name),
                old: Some(old_value.clone()),
                new: Some(new_value.to_string()),
                note: "pin attribute changed".to_string(),
            }),
            None => changes.push(FieldChange {
                field: join_field(path, name),
                old: Some(old_value.clone()),
                new: None,
                note: "pin attribute removed".to_string(),
            }),
        }
    }
    for (name, new_value) in &new.attributes {
        if name == "U"
            || name == "K"
            || options.ignore_attributes.contains(name)
            || old.attr(name).is_some()
        {
            continue;
        }
        changes.push(FieldChange {
            field: join_field(path, name),
            old: None,
            new: Some(new_value.clone()),
            note: "pin attribute added".to_string(),
        });
    }

    let old_links = link_set(old);
    let new_links = link_set(new);
    for link in new_links.difference(&old_links) {
        changes.push(FieldChange {
            field: join_field(path, "In"),
            old: None,
            new: Some(link.clone()),
            note: "wire added".to_string(),
        });
    }
    for link in old_links.difference(&new_links) {
        changes.push(FieldChange {
            field: join_field(path, "In"),
            old: Some(link.clone()),
            new: None,
            note: "wire removed".to_string(),
        });
    }
}

fn child_map(node: &RawNode) -> BTreeMap<String, &RawNode> {
    let mut candidates: Vec<(String, &RawNode)> = Vec::new();

    for child in &node.children {
        if child.tag == "In" {
            continue;
        }
        if child.tag != "Co" && node_uuid(child).is_some() {
            // Indexed separately by UUID; do not duplicate it in child diffs.
            continue;
        }
        let base = if child.tag == "Co" {
            match child.attr("K") {
                Some(key) if !key.is_empty() => format!("Co[{key}]"),
                _ => "Co".to_string(),
            }
        } else {
            node_label(child)
        };
        candidates.push((base, child));
    }

    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut map = BTreeMap::new();
    for (base, child) in candidates {
        let index = seen.entry(base.clone()).or_insert(0);
        let key = if *index == 0 {
            base
        } else {
            format!("{base}#{index}")
        };
        *index += 1;
        map.insert(key, child);
    }
    map
}

fn link_set(pin: &RawNode) -> BTreeSet<String> {
    pin.children
        .iter()
        .filter(|child| child.tag == "In")
        .map(|child| {
            let input = child.attr("Input").unwrap_or("");
            match child.attr("FLG") {
                Some(flag) => format!("{input} (FLG={flag})"),
                None => input.to_string(),
            }
        })
        .collect()
}

fn brief_text(node: &RawNode) -> String {
    let mut text = node_label(node);
    if let Some(title) = node_title(node) {
        text.push_str(&format!(" \"{title}\""));
    }
    text
}

fn join_field(path: &str, name: &str) -> String {
    if path.is_empty() {
        name.to_string()
    } else {
        format!("{path}.{name}")
    }
}

pub fn render_text(report: &DiffReport) -> String {
    let mut out = String::new();
    out.push_str("# Structural diff\n\n");
    out.push_str(&format!("- old: `{}`\n", report.old_path));
    out.push_str(&format!("- new: `{}`\n", report.new_path));
    if !report.ignored_attributes.is_empty() {
        out.push_str(&format!(
            "- ignored attributes: `{}`\n",
            report.ignored_attributes.join("`, `")
        ));
    }
    out.push_str(&format!(
        "- summary: {} added, {} removed, {} changed nodes, {} field changes\n",
        report.summary.added,
        report.summary.removed,
        report.summary.changed_nodes,
        report.summary.field_changes
    ));

    if !report.added.is_empty() {
        out.push_str("\n## Added\n\n");
        for node in &report.added {
            out.push_str(&format!("- {}\n", brief_line(node)));
        }
    }
    if !report.removed.is_empty() {
        out.push_str("\n## Removed\n\n");
        for node in &report.removed {
            out.push_str(&format!("- {}\n", brief_line(node)));
        }
    }
    if !report.changed.is_empty() {
        out.push_str("\n## Changed\n\n");
        for node in &report.changed {
            out.push_str(&format!("### {}\n\n", brief_line(&node.brief)));
            for change in &node.changes {
                out.push_str(&format!("- **{}** ({})", change.field, change.note));
                match (&change.old, &change.new) {
                    (Some(old), Some(new)) => {
                        out.push_str(&format!(": `{}` -> `{}`\n", one_line(old), one_line(new)));
                    }
                    (Some(old), None) => {
                        out.push_str(&format!(": removed `{}`\n", one_line(old)));
                    }
                    (None, Some(new)) => {
                        out.push_str(&format!(": added `{}`\n", one_line(new)));
                    }
                    (None, None) => out.push('\n'),
                }
            }
            out.push('\n');
        }
    }

    if report.added.is_empty() && report.removed.is_empty() && report.changed.is_empty() {
        out.push_str("\nNo structural differences found.\n");
    }
    out
}

fn brief_line(brief: &NodeBrief) -> String {
    let mut line = String::new();
    if let Some(ty) = &brief.type_name {
        line.push_str(&format!("`{ty}`"));
    } else {
        line.push_str(&format!("`{}`", brief.tag));
    }
    if let Some(title) = &brief.title {
        line.push_str(&format!(" \"{title}\""));
    }
    if let Some(uuid) = &brief.uuid {
        line.push_str(&format!(" (`{uuid}`)"));
    }
    line.push_str(&format!(" at `{}`", brief.path));
    line
}

fn one_line(value: &str) -> String {
    value.replace(['\r', '\n'], " ")
}

/// Convenience wrapper used by tests and the CLI.
pub fn diff_paths(old: &PathBuf, new: &PathBuf, options: &DiffOptions) -> Result<DiffReport> {
    diff_files(old.as_path(), new.as_path(), options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_set_and_child_map_distinguish_duplicate_pins() {
        let xml = r#"<C Type="Foo"><Co K="I1" Def="0"/><Co K="I1" Def="1"/></C>"#;
        let roots = loxone_core::parse_raw_dom(xml).unwrap();
        let node = &roots[0];
        let children = child_map(node);
        assert_eq!(children.len(), 2);
        assert!(children.contains_key("Co[I1]"));
        assert!(children.contains_key("Co[I1]#1"));
    }

    #[test]
    fn test_should_recurse_skips_indexed_nodes() {
        let xml = r#"<C Type="Page"><C Type="Switch" U="u1"/><IoData/></C>"#;
        let roots = loxone_core::parse_raw_dom(xml).unwrap();
        let page = &roots[0];
        assert!(!should_recurse(&page.children[0]));
        assert!(should_recurse(&page.children[1]));
    }
}
