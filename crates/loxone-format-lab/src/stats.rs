//! Corpus-wide statistics: which XML types exist, which attributes/pins they
//! carry, and how often each value occurs.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Result};
use loxone_core::model::RawNode;
use serde::Serialize;

use crate::corpus::{discover, load_file, FileSummary};
use crate::dom::{
    collect_wire_sources, is_structural_type, node_title, node_type, node_uuid, walk,
};

const MAX_EXAMPLES: usize = 6;
const MAX_TITLES: usize = 8;
const MAX_ENUM_VALUES: usize = 24;
const MAX_ENUM_VALUE_LEN: usize = 40;

#[derive(Debug, Default, Clone, Serialize)]
pub struct AttrStats {
    pub count: usize,
    pub distinct: usize,
    pub examples: Vec<String>,
    #[serde(skip)]
    values: BTreeMap<String, usize>,
}

impl AttrStats {
    pub fn observe(&mut self, value: &str) {
        self.count += 1;
        if self.values.contains_key(value) {
            *self.values.get_mut(value).unwrap() += 1;
        } else {
            self.distinct += 1;
            if self.examples.len() < MAX_EXAMPLES && !value.is_empty() {
                self.examples.push(value.to_string());
            }
            self.values.insert(value.to_string(), 1);
        }
    }

    pub fn observed_values(&self) -> impl Iterator<Item = (&str, usize)> {
        self.values.iter().map(|(v, n)| (v.as_str(), *n))
    }

    pub fn most_common(&self) -> Option<(&str, usize)> {
        self.values
            .iter()
            .max_by(|a, b| a.1.cmp(b.1).then_with(|| a.0.cmp(b.0)))
            .map(|(v, n)| (v.as_str(), *n))
    }

    /// A coarse, human-readable guess at how Loxone uses this attribute.
    pub fn inferred_kind(&self) -> &'static str {
        if self.count == 0 {
            return "unknown";
        }
        let values: Vec<&str> = self.values.keys().map(String::as_str).collect();
        if values.iter().all(|v| is_uuid(v)) {
            return "uuid";
        }
        if values
            .iter()
            .all(|v| matches!(*v, "0" | "1" | "true" | "false" | "True" | "False"))
        {
            return "bool";
        }
        if values.iter().all(|v| v.parse::<i64>().is_ok()) {
            return "int";
        }
        if values.iter().all(|v| v.parse::<f64>().is_ok()) {
            return "float";
        }
        if values.iter().all(|v| is_rgb(v)) {
            return "color";
        }
        if values.iter().all(|v| v.contains(','))
            && values.iter().all(|v| {
                v.split(',')
                    .all(|part| part.trim().parse::<f64>().is_ok() || is_uuid(part.trim()))
            })
        {
            return "csv";
        }
        if self.is_enum_candidate() {
            return "enum";
        }
        "text"
    }

    pub fn is_enum_candidate(&self) -> bool {
        self.distinct > 1
            && self.distinct <= MAX_ENUM_VALUES
            && self
                .values
                .keys()
                .all(|v| v.len() <= MAX_ENUM_VALUE_LEN)
    }

    pub fn enum_values(&self) -> Vec<String> {
        if !self.is_enum_candidate() {
            return Vec::new();
        }
        let mut values: Vec<(String, usize)> = self
            .values
            .iter()
            .map(|(v, n)| (v.clone(), *n))
            .collect();
        values.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        values.into_iter().map(|(v, _)| v).collect()
    }
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct PinStats {
    pub occurrences: usize,
    pub defaults: AttrStats,
    pub incoming_links: usize,
    pub hidden_links: usize,
    pub detached_links: usize,
    pub seen_as_wire_source: bool,
    pub attributes: BTreeMap<String, AttrStats>,
}

impl PinStats {
    pub fn is_probably_output(&self, key: &str) -> bool {
        if self.seen_as_wire_source {
            return true;
        }
        let default = (self.defaults.count > 0).then_some("0");
        loxone_core::model::classify_pin_kind(key, default, self.incoming_links > 0, false)
            == loxone_core::model::PinKind::Output
    }
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct ChildStats {
    pub count: usize,
    pub attributes: BTreeMap<String, AttrStats>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct TypeStats {
    pub occurrences: usize,
    pub root_sections: BTreeMap<String, usize>,
    pub parent_types: BTreeMap<String, usize>,
    pub attributes: BTreeMap<String, AttrStats>,
    pub pins: BTreeMap<String, PinStats>,
    pub children: BTreeMap<String, ChildStats>,
    pub titles: Vec<String>,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Totals {
    pub files: usize,
    pub pages: usize,
    pub blocks: usize,
    pub pins: usize,
    pub typed_nodes: usize,
    pub unique_types: usize,
    pub nodes: usize,
    pub xml_bytes: usize,
    pub file_bytes: u64,
}

#[derive(Debug, Default, Serialize)]
pub struct CorpusStats {
    pub totals: Totals,
    pub files: Vec<FileSummary>,
    pub errors: Vec<String>,
    pub types: BTreeMap<String, TypeStats>,
}

impl CorpusStats {
    pub fn structural_types(&self) -> Vec<&str> {
        self.types
            .keys()
            .map(String::as_str)
            .filter(|ty| is_structural_type(ty))
            .collect()
    }

    pub fn observed_type_names(&self) -> BTreeSet<String> {
        self.types.keys().cloned().collect()
    }
}

/// Scans every project below the given paths and aggregates raw XML statistics.
pub fn scan(paths: &[std::path::PathBuf]) -> Result<CorpusStats> {
    let files = discover(paths)?;
    if files.is_empty() {
        bail!(
            "no .Loxone, .LoxCC, or .zip project files found under {:?}",
            paths
        );
    }

    let mut stats = CorpusStats::default();
    for path in files {
        match load_file(&path) {
            Ok(loaded) => {
                stats.files.push(loaded.summary.clone());
                stats.totals.files += 1;
                stats.totals.pages += loaded.summary.pages;
                stats.totals.blocks += loaded.summary.blocks;
                stats.totals.pins += loaded.summary.pins;
                stats.totals.typed_nodes += loaded.summary.typed_nodes;
                stats.totals.nodes += loaded.summary.nodes;
                stats.totals.xml_bytes += loaded.summary.xml_bytes;
                stats.totals.file_bytes += loaded.summary.file_bytes;
                aggregate_roots(&loaded.roots, &mut stats);
            }
            Err(err) => stats
                .errors
                .push(format!("{}: {:#}", path.display(), err)),
        }
    }

    stats.totals.unique_types = stats.types.len();
    Ok(stats)
}

fn aggregate_roots(roots: &[RawNode], stats: &mut CorpusStats) {
    let wire_sources = collect_wire_sources(roots);
    for root in roots {
        aggregate_node(root, None, None, &wire_sources, stats);
    }
}

fn aggregate_node(
    node: &RawNode,
    parent_type: Option<&str>,
    root_section: Option<&str>,
    wire_sources: &BTreeSet<String>,
    stats: &mut CorpusStats,
) {
    let current_type = node_type(node);

    // The Document element is the skeleton; its direct children (Program,
    // LoxLIVE, category/place captions, system nodes) are the meaningful
    // sections for context reporting.
    let next_root = if current_type == Some("Document") || root_section == Some("Document") {
        current_type.or(root_section)
    } else {
        root_section
    };

    if let Some(ty) = current_type {
        let type_stats = stats.types.entry(ty.to_string()).or_default();
        type_stats.occurrences += 1;
        *type_stats
            .root_sections
            .entry(next_root.unwrap_or("<root>").to_string())
            .or_default() += 1;
        *type_stats
            .parent_types
            .entry(parent_type.unwrap_or("<root>").to_string())
            .or_default() += 1;

        if let Some(title) = node_title(node) {
            if type_stats.titles.len() < MAX_TITLES && !type_stats.titles.iter().any(|t| t == title)
            {
                type_stats.titles.push(title.to_string());
            }
        }

        for (name, value) in &node.attributes {
            if name == "Type" {
                continue;
            }
            type_stats
                .attributes
                .entry(name.clone())
                .or_default()
                .observe(value);
        }

        for child in &node.children {
            if child.tag == "Co" {
                observe_pin(type_stats, child, wire_sources);
            } else {
                let child_stats = type_stats.children.entry(child.tag.clone()).or_default();
                child_stats.count += 1;
                for (name, value) in &child.attributes {
                    child_stats
                        .attributes
                        .entry(name.clone())
                        .or_default()
                        .observe(value);
                }
            }
        }
    }

    let child_parent = current_type.or(parent_type);
    for child in &node.children {
        aggregate_node(child, child_parent, next_root, wire_sources, stats);
    }
}

fn observe_pin(type_stats: &mut TypeStats, pin: &RawNode, wire_sources: &BTreeSet<String>) {
    let Some(key) = pin.attr("K").filter(|k| !k.is_empty()) else {
        return;
    };
    let pin_stats = type_stats.pins.entry(key.to_string()).or_default();
    pin_stats.occurrences += 1;

    if let Some(default) = pin.attr("Def") {
        pin_stats.defaults.observe(default);
    }
    for (name, value) in &pin.attributes {
        if matches!(name.as_str(), "K" | "Def" | "U") {
            continue;
        }
        pin_stats
            .attributes
            .entry(name.clone())
            .or_default()
            .observe(value);
    }

    if let Some(uuid) = node_uuid(pin) {
        if wire_sources.contains(uuid) {
            pin_stats.seen_as_wire_source = true;
        }
    }

    for child in &pin.children {
        if child.tag != "In" {
            continue;
        }
        pin_stats.incoming_links += 1;
        match child.attr("FLG") {
            Some("1") => pin_stats.hidden_links += 1,
            Some("2") => pin_stats.detached_links += 1,
            _ => {}
        }
    }
}

/// Convenience used by the `dump` command and tests: counts nodes carrying a
/// `Type` attribute.
pub fn count_typed_nodes(roots: &[RawNode]) -> usize {
    let mut count = 0;
    walk(roots, "", &mut |node, _| {
        if node_type(node).is_some() {
            count += 1;
        }
    });
    count
}

fn is_uuid(value: &str) -> bool {
    value.len() == 36
        && value.as_bytes().iter().enumerate().all(|(i, byte)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                *byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

fn is_rgb(value: &str) -> bool {
    let mut parts = value.split(',').map(str::trim);
    let Some(r) = parts.next().and_then(|v| v.parse::<u8>().ok()) else {
        return false;
    };
    let Some(g) = parts.next().and_then(|v| v.parse::<u8>().ok()) else {
        return false;
    };
    let Some(b) = parts.next().and_then(|v| v.parse::<u8>().ok()) else {
        return false;
    };
    let _ = (r, g, b);
    parts.next().is_none()
}
