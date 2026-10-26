//! Locating and loading a corpus of `.Loxone`, `.LoxCC`, and Miniserver `.zip`
//! snapshots.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use loxone_core::loxcc::extract_loxone_xml;
use loxone_core::model::RawNode;
use loxone_core::parse_raw_dom;
use serde::Serialize;

use crate::dom::{node_type, node_uuid};

pub const PROJECT_EXTENSIONS: &[&str] = &["loxone", "loxcc", "zip"];
const SKIP_DIRS: &[&str] = &[".git", "target", "node_modules", ".cache"];

#[derive(Debug, Clone)]
pub struct LoadedFile {
    pub path: PathBuf,
    pub xml: String,
    pub roots: Vec<RawNode>,
    pub summary: FileSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileSummary {
    pub path: String,
    pub title: String,
    pub uuid: String,
    pub config_version: String,
    pub control_list_version: String,
    pub save_level: Option<u32>,
    pub pages: usize,
    pub blocks: usize,
    pub pins: usize,
    pub typed_nodes: usize,
    pub unique_types: usize,
    pub nodes: usize,
    pub file_bytes: u64,
    pub xml_bytes: usize,
}

/// Expands directories into a sorted, de-duplicated list of project files.
///
/// Explicit file paths are always accepted; directory scans only pick up the
/// known project extensions.
pub fn discover(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = BTreeSet::new();
    for path in paths {
        if path.is_file() {
            files.insert(path.clone());
        } else if path.is_dir() {
            collect_dir(path, &mut files)?;
        } else {
            bail!("path does not exist: {}", path.display());
        }
    }
    Ok(files.into_iter().collect())
}

fn collect_dir(dir: &Path, files: &mut BTreeSet<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if !SKIP_DIRS.contains(&name.as_str()) {
                collect_dir(&path, files)?;
            }
        } else if file_type.is_file() && has_project_extension(&path) {
            files.insert(path);
        }
    }
    Ok(())
}

fn has_project_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            PROJECT_EXTENSIONS
                .iter()
                .any(|known| ext.eq_ignore_ascii_case(known))
        })
        .unwrap_or(false)
}

/// Loads a project from a `.Loxone`, `.LoxCC`, or `.zip` file.
pub fn load_file(path: &Path) -> Result<LoadedFile> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let xml = extract_loxone_xml(&bytes)
        .with_context(|| format!("failed to decode Loxone container {}", path.display()))?;
    let clean = xml.trim_start_matches('\u{FEFF}');
    let roots = parse_raw_dom(clean)
        .with_context(|| format!("failed to parse XML in {}", path.display()))?;
    let summary = summarize(path, &roots, &xml, bytes.len() as u64);
    Ok(LoadedFile {
        path: path.to_path_buf(),
        xml,
        roots,
        summary,
    })
}

fn summarize(path: &Path, roots: &[RawNode], xml: &str, file_bytes: u64) -> FileSummary {
    let control_list = roots.iter().find(|n| n.tag == "ControlList");
    let document = roots
        .iter()
        .find(|n| n.tag == "ControlList")
        .and_then(|n| {
            n.children
                .iter()
                .find(|c| c.tag == "C" && node_type(c) == Some("Document"))
        });

    let mut counters = Counters::default();
    for root in roots {
        count_node(root, None, &mut counters);
    }

    FileSummary {
        path: path.display().to_string(),
        title: document
            .and_then(|d| d.attr("Title"))
            .unwrap_or_default()
            .to_string(),
        uuid: document
            .and_then(node_uuid)
            .unwrap_or_default()
            .to_string(),
        config_version: document
            .and_then(|d| d.attr("ConfigVersion"))
            .unwrap_or_default()
            .to_string(),
        control_list_version: control_list
            .and_then(|c| c.attr("Version"))
            .unwrap_or_default()
            .to_string(),
        save_level: document
            .and_then(|d| d.attr("SL"))
            .and_then(|v| v.trim().parse().ok()),
        pages: counters.pages,
        blocks: counters.blocks,
        pins: counters.pins,
        typed_nodes: counters.typed_nodes,
        unique_types: counters.types.len(),
        nodes: counters.nodes,
        file_bytes,
        xml_bytes: xml.len(),
    }
}

#[derive(Default)]
struct Counters {
    nodes: usize,
    typed_nodes: usize,
    pages: usize,
    blocks: usize,
    pins: usize,
    types: BTreeSet<String>,
}

fn count_node(node: &RawNode, parent_type: Option<&str>, counters: &mut Counters) {
    counters.nodes += 1;
    if node.tag == "Co" {
        counters.pins += 1;
    }

    let current_type = node_type(node);
    if let Some(ty) = current_type {
        counters.typed_nodes += 1;
        counters.types.insert(ty.to_string());
        if ty == "Page" {
            counters.pages += 1;
        }
        if node.tag == "C" && parent_type == Some("Page") && ty != "Page" {
            counters.blocks += 1;
        }
    }

    let child_parent = current_type.or(parent_type);
    for child in &node.children {
        count_node(child, child_parent, counters);
    }
}

/// Extracts a nice display title from the loaded XML, falling back to the file name.
pub fn display_name(loaded: &LoadedFile) -> String {
    if !loaded.summary.title.is_empty() {
        loaded.summary.title.clone()
    } else {
        loaded
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| loaded.path.display().to_string())
    }
}
