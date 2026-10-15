# Loxone Portable GUI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a clean, minimalist cross-platform Rust GUI application (`loxone-portable`) for inspecting, navigating, and porting `.Loxone` project files with a 2D function-block schematic canvas, cross-reference navigator, and `Ctrl+K` / `Cmd+K` command palette.

**Architecture:** Two-crate Cargo workspace separating `loxone-core` (lossless `quick-xml` parser, typed domain model for pages/blocks/pins/periphery/rooms/categories, and $O(N)$ UUID & cross-reference index) from `loxone-gui` (pure-Rust `eframe`/`egui` 3-pane minimalist desktop workbench with a 2D vector schematic canvas and keyboard-first `Ctrl+K` command palette).

**Tech Stack:** Rust (2021 edition), `quick-xml = "0.37"`, `indexmap = "2.7"`, `thiserror = "2.0"`, `eframe = "0.31"`, `egui = "0.31"`, `rfd = "0.15"`.

**Spec:** `docs/superpowers/specs/2026-10-15-loxone-portable-gui-design.md`

## Global Constraints

- **Toolchain PATH:** Always ensure `/opt/homebrew/bin` and `$HOME/.cargo/bin` are on `PATH` when invoking `cargo` or `rustc`.
- **Reference Project Validation:** All integration tests must validate against `/Users/bartlomiej.klimczak/Projects/loxone-portable/Klimczaki.Loxone` (`ControlList` v274, Document title `"Klimczaki"`, ConfigVersion `"17020828"`, `28` pages, `16` places/rooms, `33` categories, `1,493` `<C>` controls, `5,433` total `<Co>` pins — `4,336` on page blocks and `1,097` on periphery/system nodes).
- **Minimalist Visual Language:** Deep matte canvas (`#111317`) with subtle grid dots (`#1E222B`), side panels & top bar (`#161920`) with hairline `1px` borders (`#242936`), cards (`#1B1F2A` fill, `#2C3344` border, `#5E81FF` selection glow), desaturated `Cl` block header tints (`35%` Loxone RGB + `65%` dark surface) with a crisp `3px` full-saturation left accent strip.
- **Keyboard-First Navigation:** `Ctrl+K` and `Cmd+K` open the floating Command Palette from anywhere in the app; `Ctrl+B`/`Cmd+B` toggles the Left Explorer; `Ctrl+I`/`Cmd+I` toggles the Right Inspector.

---

### Task 1: Workspace Setup, Domain Model & Pin/Color Primitives

**Files:**
- Create: `Cargo.toml`
- Create: `.gitignore`
- Create: `crates/loxone-core/Cargo.toml`
- Create: `crates/loxone-core/src/lib.rs`
- Create: `crates/loxone-core/src/model.rs`
- Create: `crates/loxone-gui/Cargo.toml`
- Create: `crates/loxone-gui/src/main.rs`

**Interfaces:**
- Consumes: None (foundational workspace and domain types).
- Produces:
  - `loxone_core::model::{RawNode, ProjectMetadata, Category, Place, PinKind, WireLink, ConnectorPin, PageBlock, Page, TreeSectionKind, TreeNode, LoxoneProject}`
  - `loxone_core::model::parse_rgb_color(cl: &str) -> Option<[u8; 3]>`
  - `loxone_core::model::classify_pin_kind(key: &str, def: Option<&str>, has_incoming_links: bool, is_wire_source: bool) -> PinKind`

- [ ] **Step 1: Ensure Rust toolchain is installed and scaffold workspace manifests**

Run if `cargo` is not installed:
```bash
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
which cargo || brew install rust
cargo --version
```

Create `.gitignore`:
```gitignore
/target
.DS_Store
```

Create root `Cargo.toml`:
```toml
[workspace]
resolver = "2"
members = [
    "crates/loxone-core",
    "crates/loxone-gui",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
```

Create `crates/loxone-core/Cargo.toml`:
```toml
[package]
name = "loxone-core"
version.workspace = true
edition.workspace = true

[dependencies]
indexmap = "2.7"
quick-xml = "0.37"
thiserror = "2.0"
```

Create `crates/loxone-gui/Cargo.toml`:
```toml
[package]
name = "loxone-gui"
version.workspace = true
edition.workspace = true

[dependencies]
loxone-core = { path = "../loxone-core" }
eframe = "0.31"
egui = "0.31"
rfd = "0.15"
```

Create placeholder `crates/loxone-gui/src/main.rs`:
```rust
fn main() {
    println!("loxone-gui stub");
}
```

- [ ] **Step 2: Write the failing unit tests in `crates/loxone-core/src/model.rs`**

Create `crates/loxone-core/src/lib.rs`:
```rust
pub mod model;
```

Create `crates/loxone-core/src/model.rs` with type definitions and initial failing function bodies (`todo!()`):
```rust
use indexmap::IndexMap;

#[derive(Debug, Clone, PartialEq)]
pub struct RawNode {
    pub tag: String,
    pub attributes: Vec<(String, String)>,
    pub text: Option<String>,
    pub children: Vec<RawNode>,
}

impl RawNode {
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProjectMetadata {
    pub title: String,
    pub uuid: String,
    pub config_version: String,
    pub control_list_version: String,
    pub creator: String,
    pub street: String,
    pub town: String,
    pub postcode: String,
    pub country: String,
    pub latitude: String,
    pub longitude: String,
    pub currency: String,
    pub app_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Category {
    pub uuid: String,
    pub title: String,
    pub cat_group: Option<String>,
    pub bk_color: Option<[u8; 3]>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    pub uuid: String,
    pub title: String,
    pub sqm: Option<String>,
    pub p_type: Option<String>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinKind {
    Input,
    Parameter,
    Output,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireLink {
    pub source_pin_uuid: String,
    pub flags: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectorPin {
    pub uuid: String,
    pub key: String,
    pub default_value: Option<String>,
    pub pin_kind: PinKind,
    pub incoming_links: Vec<WireLink>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PageBlock {
    pub uuid: String,
    pub block_type: String,
    pub title: String,
    pub px: f32,
    pub py: f32,
    pub px2: f32,
    pub py2: f32,
    pub color_rgb: Option<[u8; 3]>,
    pub ref_uuid: Option<String>,
    pub extra_ref_uuids: Vec<String>,
    pub room_uuid: Option<String>,
    pub category_uuid: Option<String>,
    pub text_content: Option<String>,
    pub pins: Vec<ConnectorPin>,
    pub raw_attributes: Vec<(String, String)>,
    pub extra_children: Vec<RawNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub uuid: String,
    pub title: String,
    pub room_uuid: Option<String>,
    pub category_uuid: Option<String>,
    pub blocks: Vec<PageBlock>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeSectionKind {
    Periphery,
    System,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TreeNode {
    pub uuid: String,
    pub control_type: String,
    pub title: String,
    pub section: TreeSectionKind,
    pub room_uuid: Option<String>,
    pub category_uuid: Option<String>,
    pub pins: Vec<ConnectorPin>,
    pub raw_attributes: Vec<(String, String)>,
    pub extra_children: Vec<RawNode>,
    pub children: Vec<TreeNode>,
}

#[derive(Debug, Clone)]
pub struct LoxoneProject {
    pub metadata: ProjectMetadata,
    pub categories: IndexMap<String, Category>,
    pub places: IndexMap<String, Place>,
    pub pages: Vec<Page>,
    pub periphery_tree: Vec<TreeNode>,
    pub system_tree: Vec<TreeNode>,
}

pub fn parse_rgb_color(_cl: &str) -> Option<[u8; 3]> {
    todo!()
}

pub fn classify_pin_kind(
    _key: &str,
    _def: Option<&str>,
    _has_incoming_links: bool,
    _is_wire_source: bool,
) -> PinKind {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rgb_color() {
        assert_eq!(parse_rgb_color("255,0,84"), Some([255, 0, 84]));
        assert_eq!(parse_rgb_color("164, 112, 255"), Some([164, 112, 255]));
        assert_eq!(parse_rgb_color("0,0,0"), Some([0, 0, 0]));
        assert_eq!(parse_rgb_color("invalid"), None);
        assert_eq!(parse_rgb_color("255,0"), None);
    }

    #[test]
    fn test_classify_pin_kind() {
        assert_eq!(classify_pin_kind("Delay", Some("600"), false, false), PinKind::Parameter);
        assert_eq!(classify_pin_kind("Delay", Some("600"), true, false), PinKind::Input);
        assert_eq!(classify_pin_kind("I1", None, false, false), PinKind::Input);
        assert_eq!(classify_pin_kind("AI", None, true, false), PinKind::Input);
        assert_eq!(classify_pin_kind("Q", None, false, false), PinKind::Output);
        assert_eq!(classify_pin_kind("AQ1", None, false, false), PinKind::Output);
        assert_eq!(classify_pin_kind("OutAlarm", None, false, false), PinKind::Output);
        assert_eq!(classify_pin_kind("OutputAPI", None, false, false), PinKind::Output);
        assert_eq!(classify_pin_kind("Level1", None, false, false), PinKind::Output);
        assert_eq!(classify_pin_kind("CustomPin", None, false, true), PinKind::Output);
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p loxone-core`
Expected: FAIL with `not yet implemented` panic in `test_parse_rgb_color` and `test_classify_pin_kind`.

- [ ] **Step 4: Implement `parse_rgb_color` and `classify_pin_kind` in `crates/loxone-core/src/model.rs`**

Replace `parse_rgb_color` and `classify_pin_kind` in `crates/loxone-core/src/model.rs`:
```rust
pub fn parse_rgb_color(cl: &str) -> Option<[u8; 3]> {
    let mut parts = cl.split(',').map(|p| p.trim().parse::<u8>().ok());
    let r = parts.next()??;
    let g = parts.next()??;
    let b = parts.next()??;
    if parts.next().is_some() {
        return None;
    }
    Some([r, g, b])
}

pub fn classify_pin_kind(
    key: &str,
    def: Option<&str>,
    has_incoming_links: bool,
    is_wire_source: bool,
) -> PinKind {
    if is_wire_source {
        return PinKind::Output;
    }
    if has_incoming_links {
        return PinKind::Input;
    }
    if def.is_some() {
        return PinKind::Parameter;
    }
    let is_named_output = key == "Q"
        || key.starts_with('Q')
        || key == "AQ"
        || key.starts_with("AQ")
        || key.starts_with("Out")
        || key.starts_with("TQ")
        || key.starts_with("TxQ")
        || key.starts_with("Level")
        || matches!(
            key,
            "Scene"
                | "RoomOff"
                | "HouseOff"
                | "TargetPos"
                | "Light"
                | "Pulse"
                | "Rest"
                | "API"
                | "OutputAPI"
                | "ActiveLevel"
                | "LastStart"
                | "LastStop"
                | "ConfirmCause"
                | "ConfirmTime"
                | "OPf"
                | "OMr"
                | "OMr1"
                | "OMr2"
                | "OGpwr"
                | "OPpwr"
                | "OCpwr"
                | "OSpwr"
        )
        || key.starts_with("OR");
    if is_named_output {
        PinKind::Output
    } else {
        PinKind::Input
    }
}
```

- [ ] **Step 5: Run test to verify it passes and commit**

Run: `cargo test -p loxone-core`
Expected: PASS (2 tests passing).

```bash
git add Cargo.toml .gitignore crates/loxone-core crates/loxone-gui
git commit -m "feat(core): scaffold workspace and implement Loxone domain model primitives"
```

---

### Task 2: Lossless `.Loxone` XML Parser (`crates/loxone-core/src/parser.rs`)

**Files:**
- Create: `crates/loxone-core/src/parser.rs`
- Modify: `crates/loxone-core/src/lib.rs`

**Interfaces:**
- Consumes: `loxone_core::model::*`
- Produces:
  - `loxone_core::parser::LoxoneError`
  - `loxone_core::parser::parse_loxone_xml(xml: &str) -> Result<LoxoneProject, LoxoneError>`
  - `loxone_core::parser::load_loxone_file(path: &std::path::Path) -> Result<LoxoneProject, LoxoneError>`

- [ ] **Step 1: Write the failing parser unit & integration tests**

Update `crates/loxone-core/src/lib.rs`:
```rust
pub mod model;
pub mod parser;
```

Create `crates/loxone-core/src/parser.rs` with test cases and stub functions:
```rust
use std::collections::HashSet;
use std::path::Path;

use indexmap::IndexMap;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use thiserror::Error;

use crate::model::{
    classify_pin_kind, parse_rgb_color, Category, ConnectorPin, LoxoneProject, Page, PageBlock,
    PinKind, Place, ProjectMetadata, RawNode, TreeNode, TreeSectionKind, WireLink,
};

#[derive(Debug, Error)]
pub enum LoxoneError {
    #[error("I/O error reading Loxone file: {0}")]
    Io(#[from] std::io::Error),
    #[error("XML parse error: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("Attribute decode error: {0}")]
    Attr(#[from] quick_xml::events::attributes::AttrError),
    #[error("Missing root <ControlList> or <C Type=\"Document\"> element")]
    MissingDocumentRoot,
}

pub fn load_loxone_file(_path: &Path) -> Result<LoxoneProject, LoxoneError> {
    todo!()
}

pub fn parse_loxone_xml(_xml: &str) -> Result<LoxoneProject, LoxoneError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_minimal_xml_with_bom() {
        let xml = "\u{FEFF}<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n\
        <ControlList Version=\"274\">\
            <C Type=\"Document\" U=\"doc-1\" Title=\"TestHome\" ConfigVersion=\"17020828\" APPID=\"ABCD\">\
                <C Type=\"CategoryCaption\" U=\"cat-cap\" Title=\"Kategorie\">\
                    <C Type=\"Category\" U=\"cat-1\" Title=\"Lighting\" BkColor=\"164,112,255\"/>\
                </C>\
                <C Type=\"PlaceCaption\" U=\"place-cap\" Title=\"Pomieszczenia\">\
                    <C Type=\"Place\" U=\"room-1\" Title=\"Living Room\" Sqm=\"30\"/>\
                </C>\
                <C Type=\"LoxLIVE\" U=\"ms-1\" Title=\"TestHome\">\
                    <C Type=\"InputCaption\" U=\"in-cap\" Title=\"Digital Inputs\">\
                        <C Type=\"DigitalIn\" U=\"din-1\" Title=\"Wall Switch\">\
                            <Co K=\"Q\" U=\"pin-din-q\"/>\
                        </C>\
                    </C>\
                </C>\
                <C Type=\"Program\" U=\"prog-1\" Title=\"TestHome\">\
                    <C Type=\"Page\" U=\"page-1\" Title=\"Lighting Page\">\
                        <IoData Cr=\"cat-1\" Pr=\"room-1\"/>\
                        <C Type=\"InputRef\" U=\"blk-1\" Title=\"Wall Switch\" Px=\"2880\" Py=\"7680\" Px2=\"4992\" Py2=\"7872\" Cl=\"0,0,0\" Ref=\"din-1\">\
                            <Co K=\"AQ\" U=\"pin-1-aq\"/>\
                            <Co K=\"Q\" U=\"pin-1-q\"/>\
                        </C>\
                        <C Type=\"LightController2\" U=\"blk-2\" Title=\"Main Light\" Px=\"7392\" Py=\"576\" Px2=\"10080\" Py2=\"1848\" Cl=\"255,200,0\">\
                            <Co K=\"I1\" Nc=\"1\" U=\"pin-2-i1\">\
                                <In Input=\"pin-1-q\"/>\
                            </Co>\
                            <Co K=\"Step\" Def=\"10\" U=\"pin-2-step\"/>\
                            <Co K=\"AQ1\" U=\"pin-2-aq1\"/>\
                        </C>\
                    </C>\
                </C>\
            </C>\
        </ControlList>";

        let project = parse_loxone_xml(xml).expect("should parse minimal XML");
        assert_eq!(project.metadata.title, "TestHome");
        assert_eq!(project.metadata.control_list_version, "274");
        assert_eq!(project.categories.len(), 1);
        assert_eq!(project.places.len(), 1);
        assert_eq!(project.pages.len(), 1);
        assert_eq!(project.pages[0].blocks.len(), 2);
        assert_eq!(project.periphery_tree.len(), 1);
    }

    #[test]
    fn test_load_klimczaki_loxone_project() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should parse");

        assert_eq!(project.metadata.title, "Klimczaki");
        assert_eq!(project.metadata.config_version, "17020828");
        assert_eq!(project.metadata.control_list_version, "274");
        assert_eq!(project.pages.len(), 28);
        assert_eq!(project.places.len(), 16);
        assert_eq!(project.categories.len(), 33);
        assert!(!project.periphery_tree.is_empty());
        assert!(!project.system_tree.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p loxone-core`
Expected: FAIL with `not yet implemented` in `test_parse_minimal_xml_with_bom` and `test_load_klimczaki_loxone_project`.

- [ ] **Step 3: Implement `load_loxone_file` and `parse_loxone_xml` in `crates/loxone-core/src/parser.rs`**

Replace the body of `crates/loxone-core/src/parser.rs` (keeping the tests module at the bottom):
```rust
pub fn load_loxone_file(path: &Path) -> Result<LoxoneProject, LoxoneError> {
    let raw = std::fs::read_to_string(path)?;
    parse_loxone_xml(&raw)
}

pub fn parse_loxone_xml(xml: &str) -> Result<LoxoneProject, LoxoneError> {
    let clean_xml = xml.trim_start_matches('\u{FEFF}');
    let root_nodes = parse_raw_dom(clean_xml)?;

    let control_list = root_nodes
        .iter()
        .find(|n| n.tag == "ControlList")
        .ok_or(LoxoneError::MissingDocumentRoot)?;

    let control_list_version = control_list.attr("Version").unwrap_or_default().to_string();

    let doc_node = control_list
        .children
        .iter()
        .find(|n| n.tag == "C" && n.attr("Type") == Some("Document"))
        .ok_or(LoxoneError::MissingDocumentRoot)?;

    let metadata = ProjectMetadata {
        title: doc_node.attr("Title").unwrap_or_default().to_string(),
        uuid: doc_node.attr("U").unwrap_or_default().to_string(),
        config_version: doc_node.attr("ConfigVersion").unwrap_or_default().to_string(),
        control_list_version,
        creator: doc_node.attr("Creator").unwrap_or_default().to_string(),
        street: doc_node.attr("Street").unwrap_or_default().to_string(),
        town: doc_node.attr("Town").unwrap_or_default().to_string(),
        postcode: doc_node.attr("PostCode").unwrap_or_default().to_string(),
        country: doc_node.attr("Ctry").unwrap_or_default().to_string(),
        latitude: doc_node.attr("Latitude").unwrap_or_default().to_string(),
        longitude: doc_node.attr("Longitude").unwrap_or_default().to_string(),
        currency: doc_node.attr("Currency").unwrap_or_default().to_string(),
        app_id: doc_node.attr("APPID").unwrap_or_default().to_string(),
    };

    // First collect all wire source pin UUIDs across the entire document
    let mut wire_sources = HashSet::new();
    collect_wire_sources(doc_node, &mut wire_sources);

    let mut categories = IndexMap::new();
    let mut places = IndexMap::new();
    let mut pages = Vec::new();
    let mut periphery_tree = Vec::new();
    let mut system_tree = Vec::new();

    for child in &doc_node.children {
        if child.tag != "C" {
            continue;
        }
        match child.attr("Type").unwrap_or_default() {
            "CategoryCaption" => {
                for c in &child.children {
                    if c.tag == "C" && c.attr("Type") == Some("Category") {
                        let uuid = c.attr("U").unwrap_or_default().to_string();
                        categories.insert(
                            uuid.clone(),
                            Category {
                                uuid,
                                title: c.attr("Title").unwrap_or_default().to_string(),
                                cat_group: c.attr("CatGroup").map(str::to_string),
                                bk_color: c.attr("BkColor").and_then(parse_rgb_color),
                                icon: c.attr("Icon").map(str::to_string),
                            },
                        );
                    }
                }
            }
            "PlaceCaption" => {
                for p in &child.children {
                    if p.tag == "C" && p.attr("Type") == Some("Place") {
                        let uuid = p.attr("U").unwrap_or_default().to_string();
                        places.insert(
                            uuid.clone(),
                            Place {
                                uuid,
                                title: p.attr("Title").unwrap_or_default().to_string(),
                                sqm: p.attr("Sqm").map(str::to_string),
                                p_type: p.attr("PType").map(str::to_string),
                                icon: p.attr("Icon").map(str::to_string),
                            },
                        );
                    }
                }
            }
            "Program" => {
                for page_node in &child.children {
                    if page_node.tag == "C" && page_node.attr("Type") == Some("Page") {
                        pages.push(parse_page(page_node, &wire_sources));
                    }
                }
            }
            "LoxLIVE" => {
                periphery_tree.push(parse_tree_node(
                    child,
                    TreeSectionKind::Periphery,
                    &wire_sources,
                ));
            }
            _ => {
                system_tree.push(parse_tree_node(
                    child,
                    TreeSectionKind::System,
                    &wire_sources,
                ));
            }
        }
    }

    Ok(LoxoneProject {
        metadata,
        categories,
        places,
        pages,
        periphery_tree,
        system_tree,
    })
}

fn parse_raw_dom(xml: &str) -> Result<Vec<RawNode>, LoxoneError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut stack: Vec<RawNode> = Vec::new();
    let mut roots: Vec<RawNode> = Vec::new();

    loop {
        match reader.read_event()? {
            Event::Start(e) => {
                stack.push(start_to_raw_node(&e)?);
            }
            Event::Empty(e) => {
                let node = start_to_raw_node(&e)?;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    roots.push(node);
                }
            }
            Event::Text(e) => {
                let txt = e.unescape()?.into_owned();
                if !txt.is_empty() {
                    if let Some(top) = stack.last_mut() {
                        top.text = Some(txt);
                    }
                }
            }
            Event::End(_) => {
                if let Some(node) = stack.pop() {
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(node);
                    } else {
                        roots.push(node);
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(roots)
}

fn start_to_raw_node(e: &BytesStart<'_>) -> Result<RawNode, LoxoneError> {
    let tag = String::from_utf8_lossy(e.name().as_ref()).into_owned();
    let mut attributes = Vec::new();
    for attr in e.attributes() {
        let attr = attr?;
        let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
        let val = attr.unescape_value()?.into_owned();
        attributes.push((key, val));
    }
    Ok(RawNode {
        tag,
        attributes,
        text: None,
        children: Vec::new(),
    })
}

fn collect_wire_sources(node: &RawNode, sources: &mut HashSet<String>) {
    if node.tag == "In" {
        if let Some(src) = node.attr("Input") {
            sources.insert(src.to_string());
        }
    }
    for child in &node.children {
        collect_wire_sources(child, sources);
    }
}

fn extract_io_data(node: &RawNode) -> (Option<String>, Option<String>) {
    for child in &node.children {
        if child.tag == "IoData" {
            let room = child.attr("Pr").map(str::to_string);
            let cat = child.attr("Cr").map(str::to_string);
            return (room, cat);
        }
    }
    (None, None)
}

fn parse_pins(node: &RawNode, wire_sources: &HashSet<String>) -> Vec<ConnectorPin> {
    let mut pins = Vec::new();
    for child in &node.children {
        if child.tag == "Co" {
            let uuid = child.attr("U").unwrap_or_default().to_string();
            let key = child.attr("K").unwrap_or_default().to_string();
            let default_value = child.attr("Def").map(str::to_string);
            let mut incoming_links = Vec::new();
            for sub in &child.children {
                if sub.tag == "In" {
                    if let Some(src) = sub.attr("Input") {
                        incoming_links.push(WireLink {
                            source_pin_uuid: src.to_string(),
                            flags: sub.attr("FLG").map(str::to_string),
                        });
                    }
                }
            }
            let is_wire_source = wire_sources.contains(&uuid);
            let pin_kind = classify_pin_kind(
                &key,
                default_value.as_deref(),
                !incoming_links.is_empty(),
                is_wire_source,
            );
            pins.push(ConnectorPin {
                uuid,
                key,
                default_value,
                pin_kind,
                incoming_links,
            });
        }
    }
    pins
}

fn parse_page(page_node: &RawNode, wire_sources: &HashSet<String>) -> Page {
    let (room_uuid, category_uuid) = extract_io_data(page_node);
    let mut blocks = Vec::new();
    for child in &page_node.children {
        if child.tag == "C" {
            blocks.push(parse_page_block(child, wire_sources));
        }
    }
    Page {
        uuid: page_node.attr("U").unwrap_or_default().to_string(),
        title: page_node.attr("Title").unwrap_or_default().to_string(),
        room_uuid,
        category_uuid,
        blocks,
    }
}

fn parse_page_block(node: &RawNode, wire_sources: &HashSet<String>) -> PageBlock {
    let (room_uuid, category_uuid) = extract_io_data(node);
    let mut ref_uuid = node.attr("Ref").map(str::to_string);
    if ref_uuid.is_none() {
        for child in &node.children {
            if child.tag == "LoggerMailer" {
                if let Some(rc) = child.attr("RefCaller") {
                    ref_uuid = Some(rc.to_string());
                }
            }
        }
    }

    let mut extra_ref_uuids = Vec::new();
    for attr_name in ["UO", "SpStates", "rec"] {
        if let Some(val) = node.attr(attr_name) {
            for part in val.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                extra_ref_uuids.push(part.to_string());
            }
        }
    }

    let pins = parse_pins(node, wire_sources);
    let extra_children = node
        .children
        .iter()
        .filter(|c| c.tag != "Co")
        .cloned()
        .collect();

    PageBlock {
        uuid: node.attr("U").unwrap_or_default().to_string(),
        block_type: node.attr("Type").unwrap_or_default().to_string(),
        title: node.attr("Title").unwrap_or_default().to_string(),
        px: node.attr("Px").and_then(|v| v.parse().ok()).unwrap_or(0.0),
        py: node.attr("Py").and_then(|v| v.parse().ok()).unwrap_or(0.0),
        px2: node.attr("Px2").and_then(|v| v.parse().ok()).unwrap_or(0.0),
        py2: node.attr("Py2").and_then(|v| v.parse().ok()).unwrap_or(0.0),
        color_rgb: node.attr("Cl").and_then(parse_rgb_color),
        ref_uuid,
        extra_ref_uuids,
        room_uuid,
        category_uuid,
        text_content: node.attr("Text").map(str::to_string),
        pins,
        raw_attributes: node.attributes.clone(),
        extra_children,
    }
}

fn parse_tree_node(
    node: &RawNode,
    section: TreeSectionKind,
    wire_sources: &HashSet<String>,
) -> TreeNode {
    let (room_uuid, category_uuid) = extract_io_data(node);
    let pins = parse_pins(node, wire_sources);
    let mut children = Vec::new();
    let mut extra_children = Vec::new();

    for child in &node.children {
        if child.tag == "C" {
            children.push(parse_tree_node(child, section, wire_sources));
        } else if child.tag != "Co" {
            extra_children.push(child.clone());
        }
    }

    TreeNode {
        uuid: node.attr("U").unwrap_or_default().to_string(),
        control_type: node.attr("Type").unwrap_or_default().to_string(),
        title: node.attr("Title").unwrap_or_default().to_string(),
        section,
        room_uuid,
        category_uuid,
        pins,
        raw_attributes: node.attributes.clone(),
        extra_children,
        children,
    }
}
```

- [ ] **Step 4: Run tests to verify they pass and commit**

Run: `cargo test -p loxone-core`
Expected: PASS (4 tests passing, including `test_load_klimczaki_loxone_project`).

```bash
git add crates/loxone-core
git commit -m "feat(core): implement lossless Loxone XML parser validated against Klimczaki.Loxone"
```

---

### Task 3: Cross-Reference & UUID Index (`crates/loxone-core/src/xref.rs`)

**Files:**
- Create: `crates/loxone-core/src/xref.rs`
- Modify: `crates/loxone-core/src/model.rs`
- Modify: `crates/loxone-core/src/parser.rs`
- Modify: `crates/loxone-core/src/lib.rs`

**Interfaces:**
- Consumes: `loxone_core::model::{LoxoneProject, Page, PageBlock, TreeNode, ConnectorPin}`
- Produces:
  - `loxone_core::xref::{ObjectLocation, ObjectSummary, PinLocation, OutgoingWireTarget, CrossRefKind, CrossRefUsage, XrefIndex}`
  - `LoxoneProject.xref: XrefIndex`

- [ ] **Step 1: Write the failing cross-reference tests in `crates/loxone-core/src/xref.rs`**

Update `crates/loxone-core/src/lib.rs`:
```rust
pub mod model;
pub mod parser;
pub mod xref;
```

Add `pub xref: crate::xref::XrefIndex` to `LoxoneProject` in `crates/loxone-core/src/model.rs`:
```rust
#[derive(Debug, Clone, Default)]
pub struct LoxoneProject {
    pub metadata: ProjectMetadata,
    pub categories: IndexMap<String, Category>,
    pub places: IndexMap<String, Place>,
    pub pages: Vec<Page>,
    pub periphery_tree: Vec<TreeNode>,
    pub system_tree: Vec<TreeNode>,
    pub xref: crate::xref::XrefIndex,
}
```

Create `crates/loxone-core/src/xref.rs`:
```rust
use std::collections::HashMap;

use crate::model::{ConnectorPin, LoxoneProject, Page, PageBlock, TreeNode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectLocation {
    DocumentRoot,
    Category,
    Place,
    Page { page_index: usize },
    PageBlock { page_index: usize, block_index: usize, page_uuid: String, page_title: String },
    TreeNode { section: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectSummary {
    pub uuid: String,
    pub title: String,
    pub object_type: String,
    pub location: ObjectLocation,
    pub room_uuid: Option<String>,
    pub category_uuid: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinLocation {
    pub pin_uuid: String,
    pub pin_key: String,
    pub owner_uuid: String,
    pub owner_title: String,
    pub owner_type: String,
    pub page_index: Option<usize>,
    pub page_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingWireTarget {
    pub target_pin_uuid: String,
    pub target_pin_key: String,
    pub target_block_uuid: String,
    pub target_block_title: String,
    pub target_page_index: usize,
    pub target_page_title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrossRefKind {
    DirectRef,
    ExtraDeviceRef,
    WireFromPin { source_pin_key: String, target_pin_key: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossRefUsage {
    pub page_index: usize,
    pub page_uuid: String,
    pub page_title: String,
    pub block_uuid: String,
    pub block_title: String,
    pub block_type: String,
    pub kind: CrossRefKind,
}

#[derive(Debug, Clone, Default)]
pub struct XrefIndex {
    pub objects_by_uuid: HashMap<String, ObjectSummary>,
    pub pins_by_uuid: HashMap<String, PinLocation>,
    pub outgoing_wires_by_pin_uuid: HashMap<String, Vec<OutgoingWireTarget>>,
    pub usages_by_target_uuid: HashMap<String, Vec<CrossRefUsage>>,
}

impl XrefIndex {
    pub fn build(_project: &LoxoneProject) -> Self {
        todo!()
    }

    pub fn usages_for(&self, target_uuid: &str) -> &[CrossRefUsage] {
        self.usages_by_target_uuid
            .get(target_uuid)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::parser::load_loxone_file;

    #[test]
    fn test_klimczaki_xref_counts_and_usages() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should parse");

        // All 5,433 <Co> pins across pages and periphery/system trees are indexed
        assert_eq!(project.xref.pins_by_uuid.len(), 5433);

        // Verify a peripheral referenced by an InputRef on Page 0 ("Alarm 1") has recorded page usages
        let page0 = &project.pages[0];
        let first_input_ref = page0
            .blocks
            .iter()
            .find(|b| b.block_type == "InputRef" && b.ref_uuid.is_some())
            .expect("Page 0 should have an InputRef");
        let target_uuid = first_input_ref.ref_uuid.as_deref().unwrap();

        let usages = project.xref.usages_for(target_uuid);
        assert!(
            !usages.is_empty(),
            "Peripheral {} referenced by InputRef should have cross-reference usages",
            target_uuid
        );
        assert!(usages.iter().any(|u| u.block_uuid == first_input_ref.uuid));
    }
}
```

In `crates/loxone-core/src/parser.rs`, update the end of `parse_loxone_xml` to populate `project.xref`:
```rust
    let mut project = LoxoneProject {
        metadata,
        categories,
        places,
        pages,
        periphery_tree,
        system_tree,
        xref: crate::xref::XrefIndex::default(),
    };
    project.xref = crate::xref::XrefIndex::build(&project);
    Ok(project)
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p loxone-core`
Expected: FAIL with `not yet implemented` in `XrefIndex::build`.

- [ ] **Step 3: Implement `XrefIndex::build` in `crates/loxone-core/src/xref.rs`**

Replace `XrefIndex::build` in `crates/loxone-core/src/xref.rs`:
```rust
impl XrefIndex {
    pub fn build(project: &LoxoneProject) -> Self {
        let mut index = XrefIndex::default();

        if !project.metadata.uuid.is_empty() {
            index.objects_by_uuid.insert(
                project.metadata.uuid.clone(),
                ObjectSummary {
                    uuid: project.metadata.uuid.clone(),
                    title: project.metadata.title.clone(),
                    object_type: "Document".to_string(),
                    location: ObjectLocation::DocumentRoot,
                    room_uuid: None,
                    category_uuid: None,
                },
            );
        }

        for cat in project.categories.values() {
            index.objects_by_uuid.insert(
                cat.uuid.clone(),
                ObjectSummary {
                    uuid: cat.uuid.clone(),
                    title: cat.title.clone(),
                    object_type: "Category".to_string(),
                    location: ObjectLocation::Category,
                    room_uuid: None,
                    category_uuid: Some(cat.uuid.clone()),
                },
            );
        }

        for place in project.places.values() {
            index.objects_by_uuid.insert(
                place.uuid.clone(),
                ObjectSummary {
                    uuid: place.uuid.clone(),
                    title: place.title.clone(),
                    object_type: "Place".to_string(),
                    location: ObjectLocation::Place,
                    room_uuid: Some(place.uuid.clone()),
                    category_uuid: None,
                },
            );
        }

        for node in &project.periphery_tree {
            Self::index_tree_node(&mut index, node, "Periphery");
        }
        for node in &project.system_tree {
            Self::index_tree_node(&mut index, node, "System");
        }

        for (page_index, page) in project.pages.iter().enumerate() {
            index.objects_by_uuid.insert(
                page.uuid.clone(),
                ObjectSummary {
                    uuid: page.uuid.clone(),
                    title: page.title.clone(),
                    object_type: "Page".to_string(),
                    location: ObjectLocation::Page { page_index },
                    room_uuid: page.room_uuid.clone(),
                    category_uuid: page.category_uuid.clone(),
                },
            );

            for (block_index, block) in page.blocks.iter().enumerate() {
                let effective_room = block.room_uuid.clone().or_else(|| page.room_uuid.clone());
                let effective_cat = block
                    .category_uuid
                    .clone()
                    .or_else(|| page.category_uuid.clone());

                index.objects_by_uuid.insert(
                    block.uuid.clone(),
                    ObjectSummary {
                        uuid: block.uuid.clone(),
                        title: block.title.clone(),
                        object_type: block.block_type.clone(),
                        location: ObjectLocation::PageBlock {
                            page_index,
                            block_index,
                            page_uuid: page.uuid.clone(),
                            page_title: page.title.clone(),
                        },
                        room_uuid: effective_room,
                        category_uuid: effective_cat,
                    },
                );

                Self::index_pins(
                    &mut index,
                    &block.pins,
                    &block.uuid,
                    &block.title,
                    &block.block_type,
                    Some(page_index),
                    Some(&page.title),
                );
            }
        }

        // Second pass over page blocks to record references and wire links
        for (page_index, page) in project.pages.iter().enumerate() {
            for block in &page.blocks {
                if let Some(ref_uuid) = &block.ref_uuid {
                    index
                        .usages_by_target_uuid
                        .entry(ref_uuid.clone())
                        .or_default()
                        .push(CrossRefUsage {
                            page_index,
                            page_uuid: page.uuid.clone(),
                            page_title: page.title.clone(),
                            block_uuid: block.uuid.clone(),
                            block_title: block.title.clone(),
                            block_type: block.block_type.clone(),
                            kind: CrossRefKind::DirectRef,
                        });
                }

                for extra_uuid in &block.extra_ref_uuids {
                    index
                        .usages_by_target_uuid
                        .entry(extra_uuid.clone())
                        .or_default()
                        .push(CrossRefUsage {
                            page_index,
                            page_uuid: page.uuid.clone(),
                            page_title: page.title.clone(),
                            block_uuid: block.uuid.clone(),
                            block_title: block.title.clone(),
                            block_type: block.block_type.clone(),
                            kind: CrossRefKind::ExtraDeviceRef,
                        });
                }

                for pin in &block.pins {
                    for link in &pin.incoming_links {
                        index
                            .outgoing_wires_by_pin_uuid
                            .entry(link.source_pin_uuid.clone())
                            .or_default()
                            .push(OutgoingWireTarget {
                                target_pin_uuid: pin.uuid.clone(),
                                target_pin_key: pin.key.clone(),
                                target_block_uuid: block.uuid.clone(),
                                target_block_title: block.title.clone(),
                                target_page_index: page_index,
                                target_page_title: page.title.clone(),
                            });

                        if let Some(src_pin) = index.pins_by_uuid.get(&link.source_pin_uuid).cloned()
                        {
                            index
                                .usages_by_target_uuid
                                .entry(src_pin.owner_uuid.clone())
                                .or_default()
                                .push(CrossRefUsage {
                                    page_index,
                                    page_uuid: page.uuid.clone(),
                                    page_title: page.title.clone(),
                                    block_uuid: block.uuid.clone(),
                                    block_title: block.title.clone(),
                                    block_type: block.block_type.clone(),
                                    kind: CrossRefKind::WireFromPin {
                                        source_pin_key: src_pin.pin_key,
                                        target_pin_key: pin.key.clone(),
                                    },
                                });
                        }
                    }
                }
            }
        }

        index
    }

    fn index_tree_node(index: &mut XrefIndex, node: &TreeNode, section_label: &str) {
        if !node.uuid.is_empty() {
            index.objects_by_uuid.insert(
                node.uuid.clone(),
                ObjectSummary {
                    uuid: node.uuid.clone(),
                    title: node.title.clone(),
                    object_type: node.control_type.clone(),
                    location: ObjectLocation::TreeNode {
                        section: section_label.to_string(),
                    },
                    room_uuid: node.room_uuid.clone(),
                    category_uuid: node.category_uuid.clone(),
                },
            );
        }
        Self::index_pins(
            index,
            &node.pins,
            &node.uuid,
            &node.title,
            &node.control_type,
            None,
            None,
        );
        for child in &node.children {
            Self::index_tree_node(index, child, section_label);
        }
    }

    fn index_pins(
        index: &mut XrefIndex,
        pins: &[ConnectorPin],
        owner_uuid: &str,
        owner_title: &str,
        owner_type: &str,
        page_index: Option<usize>,
        page_title: Option<&str>,
    ) {
        for pin in pins {
            if !pin.uuid.is_empty() {
                index.pins_by_uuid.insert(
                    pin.uuid.clone(),
                    PinLocation {
                        pin_uuid: pin.uuid.clone(),
                        pin_key: pin.key.clone(),
                        owner_uuid: owner_uuid.to_string(),
                        owner_title: owner_title.to_string(),
                        owner_type: owner_type.to_string(),
                        page_index,
                        page_title: page_title.map(str::to_string),
                    },
                );
            }
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass and commit**

Run: `cargo test -p loxone-core`
Expected: PASS (5 tests passing).

```bash
git add crates/loxone-core
git commit -m "feat(core): add UUID and cross-reference index across pages and periphery"
```

---

### Task 4: Minimalist Theme & `Ctrl+K` Command Palette (`crates/loxone-gui/src/theme.rs` & `crates/loxone-gui/src/command_palette.rs`)

**Files:**
- Create: `crates/loxone-gui/src/theme.rs`
- Create: `crates/loxone-gui/src/command_palette.rs`
- Modify: `crates/loxone-gui/src/main.rs`

**Interfaces:**
- Consumes: `loxone_core::model::LoxoneProject`, `loxone_core::xref::ObjectLocation`
- Produces:
  - `crate::theme::{CANVAS_BG, GRID_DOT, PANEL_BG, CARD_BG, CARD_BORDER, BORDER_SUBTLE, ACCENT_BLUE, TEXT_PRIMARY, TEXT_MUTED, WIRE_DIGITAL, WIRE_ANALOG, apply_minimalist_theme, blend_block_header_color}`
  - `crate::command_palette::{PaletteAction, PaletteBadge, PaletteEntry, CommandPaletteState, build_palette_entries}`

- [ ] **Step 1: Create `crates/loxone-gui/src/theme.rs` and write failing command palette tests in `crates/loxone-gui/src/command_palette.rs`**

Create `crates/loxone-gui/src/theme.rs`:
```rust
use egui::{Color32, Context, CornerRadius, Stroke, Visuals};

pub const CANVAS_BG: Color32 = Color32::from_rgb(0x11, 0x13, 0x17);
pub const GRID_DOT: Color32 = Color32::from_rgb(0x1E, 0x22, 0x2B);
pub const PANEL_BG: Color32 = Color32::from_rgb(0x16, 0x19, 0x20);
pub const CARD_BG: Color32 = Color32::from_rgb(0x1B, 0x1F, 0x2A);
pub const CARD_BORDER: Color32 = Color32::from_rgb(0x2C, 0x33, 0x44);
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(0x24, 0x29, 0x36);
pub const ACCENT_BLUE: Color32 = Color32::from_rgb(0x5E, 0x81, 0xFF);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xE6, 0xEA, 0xF2);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x8B, 0x95, 0xA7);
pub const WIRE_DIGITAL: Color32 = Color32::from_rgb(0x38, 0xBD, 0xF8);
pub const WIRE_ANALOG: Color32 = Color32::from_rgb(0xF5, 0x9E, 0x0B);

pub fn apply_minimalist_theme(ctx: &Context) {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = PANEL_BG;
    visuals.window_fill = PANEL_BG;
    visuals.extreme_bg_color = CANVAS_BG;
    visuals.faint_bg_color = CARD_BG;
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(0x5E, 0x81, 0xFF, 60);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT_BLUE);
    visuals.widgets.noninteractive.bg_fill = CARD_BG;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_SUBTLE);
    visuals.widgets.inactive.bg_fill = CARD_BG;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_SUBTLE);
    visuals.widgets. inactive.corner_radius = CornerRadius::same(6);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x24, 0x2A, 0x38);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT_BLUE);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
    visuals.widgets.active.bg_fill = Color32::from_rgb(0x2A, 0x32, 0x45);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);
    ctx.set_visuals(visuals);
}

/// Blends Loxone block RGB (`Cl`) at 35% with dark surface (`#1B1F2A`) for a clean, desaturated header
/// and returns `(blended_header_fill, full_accent_strip_color)`.
pub fn blend_block_header_color(cl: Option<[u8; 3]>) -> (Color32, Color32) {
    let [r, g, b] = cl.unwrap_or([0x4B, 0x55, 0x63]);
    let is_near_black = r < 15 && g < 15 && b < 15;
    let [ar, ag, ab] = if is_near_black {
        [0x5E, 0x81, 0xFF]
    } else {
        [r, g, b]
    };
    let blend = |c: u8, base: u8| -> u8 { ((c as u16 * 35 + base as u16 * 65) / 100) as u8 };
    let header_fill = Color32::from_rgb(blend(ar, 0x1B), blend(ag, 0x1F), blend(ab, 0x2A));
    let accent_strip = Color32::from_rgb(ar, ag, ab);
    (header_fill, accent_strip)
}
```

Create `crates/loxone-gui/src/command_palette.rs` with unit tests and `todo!()` implementation:
```rust
use egui::{Align2, Color32, Context, Frame, Id, Key, Margin, Order, RichText, ScrollArea, Sense, Stroke, Vec2};
use loxone_core::model::LoxoneProject;
use loxone_core::xref::ObjectLocation;

use crate::theme::{ACCENT_BLUE, BORDER_SUBTLE, CARD_BG, PANEL_BG, TEXT_MUTED, TEXT_PRIMARY};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteAction {
    OpenFile,
    FitPage,
    ResetZoom,
    ToggleCompactPins,
    ToggleLeftPanel,
    ToggleRightPanel,
    NavigateToPage(usize),
    NavigateToBlock { page_index: usize, block_uuid: String },
    SelectObject(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteBadge {
    Action,
    Page,
    Block,
    Periphery,
    Room,
    Category,
    System,
}

impl PaletteBadge {
    pub fn label(self) -> &'static str {
        match self {
            Self::Action => "ACTION",
            Self::Page => "PAGE",
            Self::Block => "BLOCK",
            Self::Periphery => "DEVICE",
            Self::Room => "ROOM",
            Self::Category => "CATEGORY",
            Self::System => "SYSTEM",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteEntry {
    pub badge: PaletteBadge,
    pub title: String,
    pub subtitle: String,
    pub shortcut: Option<&'static str>,
    pub action: PaletteAction,
}

#[derive(Debug, Default)]
pub struct CommandPaletteState {
    pub is_open: bool,
    pub query: String,
    pub selected_index: usize,
    pub focus_requested: bool,
}

pub fn build_palette_entries(_project: Option<&LoxoneProject>, _query: &str) -> Vec<PaletteEntry> {
    todo!()
}

impl CommandPaletteState {
    pub fn open(&mut self) {
        self.is_open = true;
        self.query.clear();
        self.selected_index = 0;
        self.focus_requested = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn show(&mut self, _ctx: &Context, _project: Option<&LoxoneProject>) -> Option<PaletteAction> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use loxone_core::parser::load_loxone_file;

    #[test]
    fn test_command_palette_default_and_search_on_klimczaki() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should parse");

        // Empty query returns built-in actions + pages
        let initial = build_palette_entries(Some(&project), "");
        assert!(initial.iter().any(|e| e.action == PaletteAction::FitPage));
        assert!(initial.iter().any(|e| e.badge == PaletteBadge::Page));

        // Searching "audio" matches the "Audio centralne" page and CentralMusic block
        let audio_results = build_palette_entries(Some(&project), "audio");
        assert!(audio_results.iter().any(|e| e.title.contains("Audio centralne")));

        // Searching "lightcontroller2" matches LightController2 blocks
        let lc_results = build_palette_entries(Some(&project), "lightcontroller2");
        assert!(lc_results.iter().any(|e| e.badge == PaletteBadge::Block));
    }
}
```

Update `crates/loxone-gui/src/main.rs` so `cargo test -p loxone-gui` compiles:
```rust
mod command_palette;
mod theme;

fn main() {
    println!("loxone-gui");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p loxone-gui`
Expected: FAIL with `not yet implemented` in `test_command_palette_default_and_search_on_klimczaki`.

- [ ] **Step 3: Implement `build_palette_entries` and `CommandPaletteState::show` in `crates/loxone-gui/src/command_palette.rs`**

Replace `build_palette_entries` and `CommandPaletteState::show` in `crates/loxone-gui/src/command_palette.rs`:
```rust
pub fn build_palette_entries(project: Option<&LoxoneProject>, query: &str) -> Vec<PaletteEntry> {
    let q = query.trim().to_lowercase();
    let mut entries = Vec::new();

    let built_in_actions = [
        PaletteEntry {
            badge: PaletteBadge::Action,
            title: "Fit Active Page to View".to_string(),
            subtitle: "Center and scale all blocks on the current page".to_string(),
            shortcut: Some("F"),
            action: PaletteAction::FitPage,
        },
        PaletteEntry {
            badge: PaletteBadge::Action,
            title: "Toggle Compact Pins (Connected vs All Pins)".to_string(),
            subtitle: "Declutter function blocks by collapsing unconnected pins".to_string(),
            shortcut: Some("P"),
            action: PaletteAction::ToggleCompactPins,
        },
        PaletteEntry {
            badge: PaletteBadge::Action,
            title: "Reset Canvas Zoom (100%)".to_string(),
            subtitle: "Reset viewport scale to default".to_string(),
            shortcut: Some("0"),
            action: PaletteAction::ResetZoom,
        },
        PaletteEntry {
            badge: PaletteBadge::Action,
            title: "Toggle Left Explorer Sidebar".to_string(),
            subtitle: "Show or hide the Project & Periphery tree".to_string(),
            shortcut: Some("Ctrl+B"),
            action: PaletteAction::ToggleLeftPanel,
        },
        PaletteEntry {
            badge: PaletteBadge::Action,
            title: "Toggle Right Inspector Sidebar".to_string(),
            subtitle: "Show or hide Properties & Cross-References".to_string(),
            shortcut: Some("Ctrl+I"),
            action: PaletteAction::ToggleRightPanel,
        },
        PaletteEntry {
            badge: PaletteBadge::Action,
            title: "Open .Loxone Project File…".to_string(),
            subtitle: "Load another Loxone Config XML file from disk".to_string(),
            shortcut: Some("Ctrl+O"),
            action: PaletteAction::OpenFile,
        },
    ];

    for act in built_in_actions {
        if q.is_empty()
            || act.title.to_lowercase().contains(&q)
            || act.subtitle.to_lowercase().contains(&q)
        {
            entries.push(act);
        }
    }

    if let Some(proj) = project {
        for (idx, page) in proj.pages.iter().enumerate() {
            let subtitle = format!("Page • {} blocks", page.blocks.len());
            if q.is_empty()
                || page.title.to_lowercase().contains(&q)
                || page.uuid.to_lowercase().contains(&q)
            {
                entries.push(PaletteEntry {
                    badge: PaletteBadge::Page,
                    title: page.title.clone(),
                    subtitle,
                    shortcut: None,
                    action: PaletteAction::NavigateToPage(idx),
                });
            }
        }

        if !q.is_empty() {
            for summary in proj.xref.objects_by_uuid.values() {
                if summary.object_type == "Page" || summary.object_type == "Document" {
                    continue;
                }
                let room_name = summary
                    .room_uuid
                    .as_deref()
                    .and_then(|u| proj.places.get(u))
                    .map(|p| p.title.as_str())
                    .unwrap_or("");
                let hay = format!(
                    "{} {} {} {}",
                    summary.title, summary.object_type, room_name, summary.uuid
                )
                .to_lowercase();

                if !hay.contains(&q) {
                    continue;
                }

                match &summary.location {
                    ObjectLocation::PageBlock {
                        page_index,
                        page_title,
                        ..
                    } => {
                        let title = if summary.title.is_empty() {
                            summary.object_type.clone()
                        } else {
                            summary.title.clone()
                        };
                        entries.push(PaletteEntry {
                            badge: PaletteBadge::Block,
                            title,
                            subtitle: format!("{} on page '{}'", summary.object_type, page_title),
                            shortcut: None,
                            action: PaletteAction::NavigateToBlock {
                                page_index: *page_index,
                                block_uuid: summary.uuid.clone(),
                            },
                        });
                    }
                    ObjectLocation::Place => {
                        entries.push(PaletteEntry {
                            badge: PaletteBadge::Room,
                            title: summary.title.clone(),
                            subtitle: "Room".to_string(),
                            shortcut: None,
                            action: PaletteAction::SelectObject(summary.uuid.clone()),
                        });
                    }
                    ObjectLocation::Category => {
                        entries.push(PaletteEntry {
                            badge: PaletteBadge::Category,
                            title: summary.title.clone(),
                            subtitle: "Category".to_string(),
                            shortcut: None,
                            action: PaletteAction::SelectObject(summary.uuid.clone()),
                        });
                    }
                    ObjectLocation::TreeNode { section } => {
                        let badge = if section == "Periphery" {
                            PaletteBadge::Periphery
                        } else {
                            PaletteBadge::System
                        };
                        let usages = proj.xref.usages_for(&summary.uuid).len();
                        entries.push(PaletteEntry {
                            badge,
                            title: if summary.title.is_empty() {
                                summary.object_type.clone()
                            } else {
                                summary.title.clone()
                            },
                            subtitle: format!("{} • {} page refs", summary.object_type, usages),
                            shortcut: None,
                            action: PaletteAction::SelectObject(summary.uuid.clone()),
                        });
                    }
                    ObjectLocation::DocumentRoot | ObjectLocation::Page { .. } => {}
                }
            }
        }
    }

    entries.truncate(60);
    entries
}
```

And replace `CommandPaletteState::show` in `crates/loxone-gui/src/command_palette.rs`:
```rust
    pub fn show(&mut self, ctx: &Context, project: Option<&LoxoneProject>) -> Option<PaletteAction> {
        if !self.is_open {
            return None;
        }

        let entries = build_palette_entries(project, &self.query);
        if entries.is_empty() {
            self.selected_index = 0;
        } else if self.selected_index >= entries.len() {
            self.selected_index = entries.len() - 1;
        }

        let mut triggered_action = None;

        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.close();
            return None;
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowDown)) && !entries.is_empty() {
            self.selected_index = (self.selected_index + 1) % entries.len();
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowUp)) && !entries.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                entries.len() - 1
            } else {
                self.selected_index - 1
            };
        }
        if ctx.input(|i| i.key_pressed(Key::Enter)) {
            if let Some(entry) = entries.get(self.selected_index) {
                triggered_action = Some(entry.action.clone());
            }
        }

        // Subtle backdrop dimmer
        let screen_rect = ctx.screen_rect();
        let backdrop_layer = egui::LayerId::new(Order::Foreground, Id::new("cmd_palette_backdrop"));
        ctx.layer_painter(backdrop_layer).rect_filled(
            screen_rect,
            0.0,
            Color32::from_black_alpha(140),
        );

        egui::Area::new(Id::new("cmd_palette_modal"))
            .order(Order::Foreground)
            .anchor(Align2::CENTER_TOP, Vec2::new(0.0, 72.0))
            .show(ctx, |ui| {
                Frame::new()
                    .fill(PANEL_BG)
                    .stroke(Stroke::new(1.0, ACCENT_BLUE))
                    .corner_radius(10)
                    .inner_margin(Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_width(540.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("⌘K").color(ACCENT_BLUE).strong());
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut self.query)
                                    .hint_text("Type a command, page, block, device, room, or UUID…")
                                    .desired_width(450.0)
                                    .frame(false),
                            );
                            if self.focus_requested {
                                resp.request_focus();
                                self.focus_requested = false;
                            }
                            if resp.changed() {
                                self.selected_index = 0;
                            }
                        });

                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);

                        ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
                            if entries.is_empty() {
                                ui.label(RichText::new("No matching commands or objects").color(TEXT_MUTED));
                            }
                            for (idx, entry) in entries.iter().enumerate() {
                                let is_selected = idx == self.selected_index;
                                let bg = if is_selected {
                                    Color32::from_rgba_unmultiplied(0x5E, 0x81, 0xFF, 45)
                                } else {
                                    CARD_BG
                                };
                                let border = if is_selected { ACCENT_BLUE } else { BORDER_SUBTLE };

                                let resp = Frame::new()
                                    .fill(bg)
                                    .stroke(Stroke::new(1.0, border))
                                    .corner_radius(6)
                                    .inner_margin(Margin::symmetric(10, 6))
                                    .show(ui, |ui| {
                                        ui.set_width(512.0);
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(entry.badge.label())
                                                    .size(10.0)
                                                    .color(ACCENT_BLUE)
                                                    .strong(),
                                            );
                                            ui.label(
                                                RichText::new(&entry.title)
                                                    .color(TEXT_PRIMARY)
                                                    .strong(),
                                            );
                                            ui.label(
                                                RichText::new(&entry.subtitle)
                                                    .size(11.5)
                                                    .color(TEXT_MUTED),
                                            );
                                            if let Some(sc) = entry.shortcut {
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        ui.label(
                                                            RichText::new(sc)
                                                                .size(11.0)
                                                                .color(TEXT_MUTED),
                                                        );
                                                    },
                                                );
                                            }
                                        });
                                    })
                                    .response
                                    .interact(Sense::click());

                                if resp.clicked() {
                                    triggered_action = Some(entry.action.clone());
                                }
                                ui.add_space(3.0);
                            }
                        });
                    });
            });

        if triggered_action.is_some() {
            self.close();
        }
        triggered_action
    }
```

- [ ] **Step 4: Run tests to verify they pass and commit**

Run: `cargo test -p loxone-gui`
Expected: PASS.

```bash
git add crates/loxone-gui
git commit -m "feat(gui): implement minimalist theme and Ctrl+K command palette"
```

---

### Task 5: 2D Schematic Page Canvas & Wire Renderer (`crates/loxone-gui/src/canvas.rs`)

**Files:**
- Create: `crates/loxone-gui/src/canvas.rs`
- Modify: `crates/loxone-gui/src/main.rs`

**Interfaces:**
- Consumes: `loxone_core::model::{ConnectorPin, LoxoneProject, Page, PageBlock}`, `loxone_core::xref::XrefIndex`, `crate::theme::*`
- Produces:
  - `crate::canvas::{CanvasViewport, CanvasAction, render_page_canvas}`

- [ ] **Step 1: Write failing unit tests for coordinate transforms, fit-to-page, and pin classification in `crates/loxone-gui/src/canvas.rs`**

Create `crates/loxone-gui/src/canvas.rs`:
```rust
use std::collections::HashMap;

use egui::{
    epaint::CubicBezierShape, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Ui,
    Vec2,
};
use loxone_core::model::{ConnectorPin, LoxoneProject, Page, PageBlock};
use loxone_core::xref::XrefIndex;

use crate::theme::{
    blend_block_header_color, ACCENT_BLUE, BORDER_SUBTLE, CANVAS_BG, CARD_BG, CARD_BORDER,
    GRID_DOT, TEXT_MUTED, TEXT_PRIMARY, WIRE_ANALOG, WIRE_DIGITAL,
};

pub const LOXONE_UNITS_PER_PX: f32 = 8.0;

#[derive(Debug, Clone)]
pub struct CanvasViewport {
    pub pan: Vec2,
    pub zoom: f32,
    pub compact_pins: bool,
}

impl Default for CanvasViewport {
    fn default() -> Self {
        Self {
            pan: Vec2::new(40.0, 40.0),
            zoom: 1.0,
            compact_pins: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanvasAction {
    SelectObject(String),
    ClearSelection,
    NavigateToBlock { page_index: usize, block_uuid: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinSide {
    Input,
    Output,
}

impl CanvasViewport {
    pub fn loxone_to_screen(&self, canvas_origin: Pos2, lx: f32, ly: f32) -> Pos2 {
        todo!()
    }

    pub fn loxone_rect_to_screen(&self, canvas_origin: Pos2, block: &PageBlock) -> Rect {
        todo!()
    }

    pub fn fit_to_page(&mut self, page: &Page, view_size: Vec2) {
        todo!()
    }

    pub fn center_on_block(&mut self, block: &PageBlock, view_size: Vec2) {
        todo!()
    }
}

pub fn classify_pin_side(_block: &PageBlock, _pin: &ConnectorPin, _xref: &XrefIndex) -> PinSide {
    todo!()
}

pub fn is_pin_connected(_pin: &ConnectorPin, _xref: &XrefIndex) -> bool {
    todo!()
}

pub fn render_page_canvas(
    _ui: &mut Ui,
    _project: &LoxoneProject,
    _page_index: usize,
    _viewport: &mut CanvasViewport,
    _selected_uuid: Option<&str>,
) -> Option<CanvasAction> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use loxone_core::parser::load_loxone_file;

    #[test]
    fn test_canvas_coordinate_transforms_and_fit_to_page() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should parse");
        let page0 = &project.pages[0];

        let mut vp = CanvasViewport::default();
        let origin = Pos2::new(100.0, 50.0);
        let p = vp.loxone_to_screen(origin, 192.0, 192.0);
        // 192 / 8.0 = 24.0 + pan(40,40) + origin(100,50) = (164.0, 114.0)
        assert_eq!(p, Pos2::new(164.0, 114.0));

        vp.fit_to_page(page0, Vec2::new(1200.0, 800.0));
        assert!(vp.zoom >= 0.2 && vp.zoom <= 2.5);
    }

    #[test]
    fn test_pin_side_and_connection_classification() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should parse");
        let page0 = &project.pages[0];

        let input_ref = page0
            .blocks
            .iter()
            .find(|b| b.block_type == "InputRef")
            .unwrap();
        for pin in &input_ref.pins {
            assert_eq!(classify_pin_side(input_ref, pin, &project.xref), PinSide::Output);
        }

        let output_ref = page0
            .blocks
            .iter()
            .find(|b| b.block_type == "OutputRef")
            .unwrap();
        for pin in &output_ref.pins {
            assert_eq!(classify_pin_side(output_ref, pin, &project.xref), PinSide::Input);
        }
    }
}
```

Add `mod canvas;` to `crates/loxone-gui/src/main.rs`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p loxone-gui`
Expected: FAIL with `not yet implemented` in `test_canvas_coordinate_transforms_and_fit_to_page`.

- [ ] **Step 3: Implement coordinate transforms, pin layout, block rendering, and cubic bezier wire routing in `crates/loxone-gui/src/canvas.rs`**

Replace the `todo!()` implementations in `crates/loxone-gui/src/canvas.rs`:
```rust
impl CanvasViewport {
    pub fn loxone_to_screen(&self, canvas_origin: Pos2, lx: f32, ly: f32) -> Pos2 {
        let x = canvas_origin.x + self.pan.x + (lx / LOXONE_UNITS_PER_PX) * self.zoom;
        let y = canvas_origin.y + self.pan.y + (ly / LOXONE_UNITS_PER_PX) * self.zoom;
        Pos2::new(x, y)
    }

    pub fn loxone_rect_to_screen(&self, canvas_origin: Pos2, block: &PageBlock) -> Rect {
        let min = self.loxone_to_screen(canvas_origin, block.px as f32, block.py as f32);
        let max_lx = block.px2.max(block.px + 512) as f32;
        let max_ly = block.py2.max(block.py + 192) as f32;
        let max = self.loxone_to_screen(canvas_origin, max_lx, max_ly);
        Rect::from_min_max(min, max)
    }

    pub fn fit_to_page(&mut self, page: &Page, view_size: Vec2) {
        if page.blocks.is_empty() || view_size.x <= 10.0 || view_size.y <= 10.0 {
            self.pan = Vec2::new(40.0, 40.0);
            self.zoom = 1.0;
            return;
        }
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        for b in &page.blocks {
            min_x = min_x.min(b.px);
            min_y = min_y.min(b.py);
            max_x = max_x.max(b.px2.max(b.px + 512));
            max_y = max_y.max(b.py2.max(b.py + 192));
        }
        let width_px = ((max_x - min_x).max(512) as f32) / LOXONE_UNITS_PER_PX;
        let height_px = ((max_y - min_y).max(192) as f32) / LOXONE_UNITS_PER_PX;
        let margin = 64.0;
        let zx = ((view_size.x - margin) / width_px).clamp(0.25, 2.0);
        let zy = ((view_size.y - margin) / height_px).clamp(0.25, 2.0);
        self.zoom = zx.min(zy);
        self.pan = Vec2::new(
            margin * 0.5 - (min_x as f32 / LOXONE_UNITS_PER_PX) * self.zoom,
            margin * 0.5 - (min_y as f32 / LOXONE_UNITS_PER_PX) * self.zoom,
        );
    }

    pub fn center_on_block(&mut self, block: &PageBlock, view_size: Vec2) {
        let cx = ((block.px + block.px2.max(block.px + 512)) as f32 * 0.5) / LOXONE_UNITS_PER_PX;
        let cy = ((block.py + block.py2.max(block.py + 192)) as f32 * 0.5) / LOXONE_UNITS_PER_PX;
        self.pan = Vec2::new(
            view_size.x * 0.5 - cx * self.zoom,
            view_size.y * 0.5 - cy * self.zoom,
        );
    }
}

pub fn classify_pin_side(block: &PageBlock, pin: &ConnectorPin, xref: &XrefIndex) -> PinSide {
    if matches!(block.block_type.as_str(), "InputRef" | "Memory") {
        return PinSide::Output;
    }
    if block.block_type == "OutputRef" {
        return PinSide::Input;
    }
    if !pin.incoming_links.is_empty() || pin.default_value.is_some() {
        return PinSide::Input;
    }
    if xref.outgoing_wires_by_pin_uuid.contains_key(&pin.uuid) {
        return PinSide::Output;
    }
    if pin.key.starts_with('Q')
        || pin.key.starts_with('O')
        || pin.key.starts_with("AQ")
        || pin.key.starts_with("TQ")
    {
        PinSide::Output
    } else {
        PinSide::Input
    }
}

pub fn is_pin_connected(pin: &ConnectorPin, xref: &XrefIndex) -> bool {
    !pin.incoming_links.is_empty()
        || pin.default_value.is_some()
        || xref.outgoing_wires_by_pin_uuid.contains_key(&pin.uuid)
}
```

Now implement `render_page_canvas` in `crates/loxone-gui/src/canvas.rs`:
```rust
pub fn render_page_canvas(
    ui: &mut Ui,
    project: &LoxoneProject,
    page_index: usize,
    viewport: &mut CanvasViewport,
    selected_uuid: Option<&str>,
) -> Option<CanvasAction> {
    let page = project.pages.get(page_index)?;
    let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let canvas_rect = response.rect;
    let origin = canvas_rect.min;

    painter.rect_filled(canvas_rect, 0.0, CANVAS_BG);

    // Handle pan and cursor-centered zoom
    if response.dragged() {
        viewport.pan += response.drag_delta();
    }
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta);
        let zoom_delta = ui.input(|i| i.zoom_delta());
        if ( zoom_delta - 1.0 ).abs() > 0.001 {
            if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let old_zoom = viewport.zoom;
                viewport.zoom = (viewport.zoom * zoom_delta).clamp(0.2, 3.0);
                let rel = hover_pos - origin - viewport.pan;
                viewport.pan -= rel * (viewport.zoom / old_zoom - 1.0);
            }
        } else if scroll.y.abs() > 0.1 && ui.input(|i| i.modifiers.command || i.modifiers.ctrl) {
            if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let factor = (1.0 + scroll.y * 0.003).clamp(0.8, 1.25);
                let old_zoom = viewport.zoom;
                viewport.zoom = (viewport.zoom * factor).clamp(0.2, 3.0);
                let rel = hover_pos - origin - viewport.pan;
                viewport.pan -= rel * (viewport.zoom / old_zoom - 1.0);
            }
        } else if scroll.length_sq() > 0.1 {
            viewport.pan += scroll;
        }
    }

    // Subtle dot grid
    let step = (24.0 * viewport.zoom).max(14.0);
    let start_x = origin.x + (viewport.pan.x % step);
    let start_y = origin.y + (viewport.pan.y % step);
    let mut gx = start_x;
    while gx < canvas_rect.max.x {
        let mut gy = start_y;
        while gy < canvas_rect.max.y {
            painter.circle_filled(Pos2::new(gx, gy), 1.0, GRID_DOT);
            gy += step;
        }
        gx += step;
    }

    let mut action: Option<CanvasAction> = None;
    let mut pin_screen_positions: HashMap<String, Pos2> = HashMap::new();
    let pointer_pos = response.interact_pointer_pos();
    let clicked = response.clicked();
    let mut clicked_any_block = false;

    // Pass 1: Draw blocks and record pin anchor coordinates
    for block in &page.blocks {
        let rect = viewport.loxone_rect_to_screen(origin, block);
        let is_selected = selected_uuid == Some(block.uuid.as_str());
        let (header_fill, accent_color) = blend_block_header_color(block.color_rgb);

        if clicked {
            if let Some(pos) = pointer_pos {
                if rect.contains(pos) {
                    clicked_any_block = true;
                    action = Some(CanvasAction::SelectObject(block.uuid.clone()));
                }
            }
        }

        let border_stroke = if is_selected {
            Stroke::new(1.5, ACCENT_BLUE)
        } else {
            Stroke::new(1.0, CARD_BORDER)
        };

        // Sticky Note
        if block.block_type == "Note" {
            let note_bg = Color32::from_rgb(0x28, 0x24, 0x18);
            painter.rect(rect, CornerRadius::same(6), note_bg, border_stroke, egui::StrokeKind::Inside);
            painter.text(
                rect.min + Vec2::new(8.0 * viewport.zoom, 6.0 * viewport.zoom),
                Align2::LEFT_TOP,
                &block.title,
                FontId::proportional((11.0 * viewport.zoom).clamp(8.0, 16.0)),
                TEXT_PRIMARY,
            );
            continue;
        }

        // Reference Bar (InputRef, OutputRef, Memory)
        let is_ref_bar = matches!(
            block.block_type.as_str(),
            "InputRef" | "OutputRef" | "Memory"
        );
        if is_ref_bar {
            painter.rect(rect, CornerRadius::same(5), header_fill, border_stroke, egui::StrokeKind::Inside);
            let strip = Rect::from_min_max(
                rect.min,
                Pos2::new((rect.min.x + 3.0 * viewport.zoom).min(rect.max.x), rect.max.y),
            );
            painter.rect_filled(strip, CornerRadius::same(2), accent_color);

            let label = if !block.title.is_empty() {
                block.title.clone()
            } else if let Some(ref_uuid) = &block.ref_uuid {
                project
                    .xref
                    .objects_by_uuid
                    .get(ref_uuid)
                    .map(|o| o.title.clone())
                    .unwrap_or_else(|| block.block_type.clone())
            } else {
                block.block_type.clone()
            };

            painter.text(
                Pos2::new(rect.min.x + 8.0 * viewport.zoom, rect.center().y),
                Align2::LEFT_CENTER,
                label,
                FontId::proportional((11.0 * viewport.zoom).clamp(8.0, 14.0)),
                TEXT_PRIMARY,
            );

            for pin in &block.pins {
                let side = classify_pin_side(block, pin, &project.xref);
                let anchor = match side {
                    PinSide::Output => Pos2::new(rect.max.x, rect.center().y),
                    PinSide::Input => Pos2::new(rect.min.x, rect.center().y),
                };
                painter.circle_filled(anchor, (3.2 * viewport.zoom).clamp(2.0, 5.0), WIRE_DIGITAL);
                pin_screen_positions.insert(pin.uuid.clone(), anchor);
            }
            continue;
        }

        // Standard Function Block Card
        let header_h = (22.0 * viewport.zoom).clamp(14.0, 32.0).min(rect.height());
        let header_rect = Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.min.y + header_h));

        painter.rect(rect, CornerRadius::same(6), CARD_BG, border_stroke, egui::StrokeKind::Inside);
        painter.rect_filled(header_rect, CornerRadius::same(6), header_fill);
        let strip = Rect::from_min_max(
            header_rect.min,
            Pos2::new((header_rect.min.x + 3.0 * viewport.zoom).min(header_rect.max.x), header_rect.max.y),
        );
        painter.rect_filled(strip, CornerRadius::same(2), accent_color);

        let title = if block.title.is_empty() {
            &block.block_type
        } else {
            &block.title
        };
        painter.text(
            Pos2::new(header_rect.min.x + 8.0 * viewport.zoom, header_rect.center().y),
            Align2::LEFT_CENTER,
            title,
            FontId::proportional((11.5 * viewport.zoom).clamp(8.0, 15.0)),
            TEXT_PRIMARY,
        );

        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        for pin in &block.pins {
            if viewport.compact_pins && !is_pin_connected(pin, &project.xref) {
                continue;
            }
            match classify_pin_side(block, pin, &project.xref) {
                PinSide::Input => inputs.push(pin),
                PinSide::Output => outputs.push(pin),
            }
        }

        let body_top = header_rect.max.y + 6.0 * viewport.zoom;
        let row_h = (14.0 * viewport.zoom).clamp(9.0, 20.0);

        for (i, pin) in inputs.iter().enumerate() {
            let py = (body_top + (i as f32 + 0.5) * row_h).min(rect.max.y - 4.0);
            let anchor = Pos2::new(rect.min.x, py);
            painter.circle_filled(anchor, (3.0 * viewport.zoom).clamp(2.0, 4.5), WIRE_DIGITAL);
            painter.text(
                Pos2::new(rect.min.x + 6.0 * viewport.zoom, py),
                Align2::LEFT_CENTER,
                &pin.key,
                FontId::monospace((9.5 * viewport.zoom).clamp(7.0, 12.0)),
                TEXT_MUTED,
            );
            pin_screen_positions.insert(pin.uuid.clone(), anchor);
        }

        for (i, pin) in outputs.iter().enumerate() {
            let py = (body_top + (i as f32 + 0.5) * row_h).min(rect.max.y - 4.0);
            let anchor = Pos2::new(rect.max.x, py);
            painter.circle_filled(anchor, (3.0 * viewport.zoom).clamp(2.0, 4.5), WIRE_ANALOG);
            painter.text(
                Pos2::new(rect.max.x - 6.0 * viewport.zoom, py),
                Align2::RIGHT_CENTER,
                &pin.key,
                FontId::monospace((9.5 * viewport.zoom).clamp(7.0, 12.0)),
                TEXT_MUTED,
            );
            pin_screen_positions.insert(pin.uuid.clone(), anchor);
        }
    }

    // Pass 2: Draw wires (on-page cubic beziers + off-page/periphery clickable pill tags)
    for block in &page.blocks {
        let block_rect = viewport.loxone_rect_to_screen(origin, block);
        for (pin_idx, pin) in block.pins.iter().enumerate() {
            if pin.incoming_links.is_empty() {
                continue;
            }
            let dst_pos = pin_screen_positions
                .get(&pin.uuid)
                .copied()
                .unwrap_or_else(|| {
                    Pos2::new(
                        block_rect.min.x,
                        block_rect.min.y + 24.0 + (pin_idx as f32) * 12.0,
                    )
                });

            for link in &pin.incoming_links {
                if let Some(&src_pos) = pin_screen_positions.get(&link.source_pin_uuid) {
                    let dx = ((dst_pos.x - src_pos.x).abs() * 0.45).max(28.0 * viewport.zoom);
                    let bezier = CubicBezierShape::from_points_stroke(
                        [
                            src_pos,
                            Pos2::new(src_pos.x + dx, src_pos.y),
                            Pos2::new(dst_pos.x - dx, dst_pos.y),
                            dst_pos,
                        ],
                        false,
                        Color32::TRANSPARENT,
                        Stroke::new((1.6 * viewport.zoom).clamp(1.0, 2.5), WIRE_DIGITAL),
                    );
                    painter.add(bezier);
                } else if let Some(src_meta) = project.xref.pins_by_uuid.get(&link.source_pin_uuid) {
                    // Off-page or direct periphery link pill tag
                    let pill_w = (96.0 * viewport.zoom).clamp(60.0, 140.0);
                    let pill_h = (14.0 * viewport.zoom).clamp(10.0, 18.0);
                    let pill_rect = Rect::from_min_max(
                        Pos2::new(dst_pos.x - pill_w - 6.0, dst_pos.y - pill_h * 0.5),
                        Pos2::new(dst_pos.x - 6.0, dst_pos.y + pill_h * 0.5),
                    );
                    painter.line_segment(
                        [Pos2::new(pill_rect.max.x, dst_pos.y), dst_pos],
                        Stroke::new(1.2, BORDER_SUBTLE),
                    );
                    painter.rect(
                        pill_rect,
                        CornerRadius::same(4),
                        CARD_BG,
                        Stroke::new(1.0, BORDER_SUBTLE),
                        egui::StrokeKind::Inside,
                    );
                    let tag = format!("🔗 {}:{}", src_meta.owner_title, src_meta.pin_key);
                    painter.text(
                        pill_rect.center(),
                        Align2::CENTER_CENTER,
                        tag,
                        FontId::proportional((9.0 * viewport.zoom).clamp(7.0, 11.0)),
                        ACCENT_BLUE,
                    );
                    if clicked {
                        if let Some(pos) = pointer_pos {
                            if pill_rect.contains(pos) {
                                clicked_any_block = true;
                                if let Some(src_page_idx) = src_meta.page_index {
                                    action = Some(CanvasAction::NavigateToBlock {
                                        page_index: src_page_idx,
                                        block_uuid: src_meta.owner_uuid.clone(),
                                    });
                                } else {
                                    action = Some(CanvasAction::SelectObject(
                                        src_meta.owner_uuid.clone(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if clicked && !clicked_any_block {
        action = Some(CanvasAction::ClearSelection);
    }

    action
}
```

- [ ] **Step 4: Run tests to verify they pass and commit**

Run: `cargo test -p loxone-gui`
Expected: PASS.

```bash
git add crates/loxone-gui
git commit -m "feat(gui): implement 2D schematic page canvas and bezier wire renderer"
```

---

### Task 6: Three-Pane Minimalist Workspace, Tree Explorer & Inspector (`tree_panel.rs`, `inspector.rs`, `app.rs`, `main.rs`)

**Files:**
- Create: `crates/loxone-gui/src/tree_panel.rs`
- Create: `crates/loxone-gui/src/inspector.rs`
- Create: `crates/loxone-gui/src/app.rs`
- Modify: `crates/loxone-gui/src/main.rs`

**Interfaces:**
- Consumes: `loxone_core::model::*`, `loxone_core::xref::*`, `crate::theme::*`, `crate::command_palette::*`, `crate::canvas::*`
- Produces:
  - `crate::tree_panel::{LeftTab, TreeAction, render_left_explorer}`
  - `crate::inspector::{InspectorAction, render_right_inspector}`
  - `crate::app::LoxonePortableApp`

- [ ] **Step 1: Create `crates/loxone-gui/src/tree_panel.rs` and `crates/loxone-gui/src/inspector.rs`**

Create `crates/loxone-gui/src/tree_panel.rs`:
```rust
use egui::{Color32, RichText, ScrollArea, Ui};
use loxone_core::model::{LoxoneProject, TreeNode};

use crate::theme::{ACCENT_BLUE, BORDER_SUBTLE, CARD_BG, TEXT_MUTED, TEXT_PRIMARY};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LeftTab {
    #[default]
    Pages,
    Periphery,
    Rooms,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeAction {
    SelectPage(usize),
    SelectObject(String),
    NavigateToBlock { page_index: usize, block_uuid: String },
}

pub fn render_left_explorer(
    ui: &mut Ui,
    project: &LoxoneProject,
    active_tab: &mut LeftTab,
    filter: &mut String,
    active_page_idx: usize,
    selected_uuid: Option<&str>,
) -> Option<TreeAction> {
    let mut action = None;

    ui.add_space(6.0);
    ui.horizontal(|ui| {
        for (tab, label) in [
            (LeftTab::Pages, "Pages"),
            (LeftTab::Periphery, "Periphery"),
            (LeftTab::Rooms, "Rooms"),
            (LeftTab::System, "System"),
        ] {
            let selected = *active_tab == tab;
            if ui
                .selectable_label(
                    selected,
                    RichText::new(label)
                        .size(12.0)
                        .color(if selected { ACCENT_BLUE } else { TEXT_MUTED }),
                )
                .clicked()
            {
                *active_tab = tab;
            }
        }
    });

    ui.add_space(4.0);
    ui.add(
        egui::TextEdit::singleline(filter)
            .hint_text("Filter list…")
            .desired_width(f32::INFINITY),
    );
    ui.add_space(4.0);
    ui.separator();

    let q = filter.trim().to_lowercase();

    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        match *active_tab {
            LeftTab::Pages => {
                for (idx, page) in project.pages.iter().enumerate() {
                    if !q.is_empty() && !page.title.to_lowercase().contains(&q) {
                        continue;
                    }
                    let is_active = idx == active_page_idx;
                    let bg = if is_active {
                        Color32::from_rgba_unmultiplied(0x5E, 0x81, 0xFF, 36)
                    } else {
                        Color32::TRANSPARENT
                    };
                    let resp = egui::Frame::new()
                        .fill(bg)
                        .corner_radius(5)
                        .inner_margin(egui::Margin::symmetric(8, 5))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(&page.title)
                                        .color(if is_active { ACCENT_BLUE } else { TEXT_PRIMARY }),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new(format!("{}", page.blocks.len()))
                                                .size(11.0)
                                                .color(TEXT_MUTED),
                                        );
                                    },
                                );
                            });
                        })
                        .response
                        .interact(egui::Sense::click());
                    if resp.clicked() {
                        action = Some(TreeAction::SelectPage(idx));
                    }
                }
            }
            LeftTab::Periphery => {
                for node in &project.periphery_tree {
                    if let Some(a) = render_tree_node(ui, project, node, &q, selected_uuid) {
                        action = Some(a);
                    }
                }
            }
            LeftTab::Rooms => {
                let mut places: Vec<_> = project.places.values().collect();
                places.sort_by(|a, b| a.title.cmp(&b.title));
                for place in places {
                    if !q.is_empty() && !place.title.to_lowercase().contains(&q) {
                        continue;
                    }
                    egui::CollapsingHeader::new(
                        RichText::new(&place.title).color(TEXT_PRIMARY).strong(),
                    )
                    .id_salt(&place.uuid)
                    .default_open(!q.is_empty())
                    .show(ui, |ui| {
                        for (p_idx, page) in project.pages.iter().enumerate() {
                            for block in &page.blocks {
                                let eff_room = block
                                    .room_uuid
                                    .as_deref()
                                    .or(page.room_uuid.as_deref());
                                if eff_room == Some(place.uuid.as_str()) && !block.title.is_empty()
                                {
                                    let is_sel = selected_uuid == Some(block.uuid.as_str());
                                    if ui
                                        .selectable_label(
                                            is_sel,
                                            format!("{} ({})", block.title, block.block_type),
                                        )
                                        .clicked()
                                    {
                                        action = Some(TreeAction::NavigateToBlock {
                                            page_index: p_idx,
                                            block_uuid: block.uuid.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    });
                }
            }
            LeftTab::System => {
                for node in &project.system_tree {
                    if let Some(a) = render_tree_node(ui, project, node, &q, selected_uuid) {
                        action = Some(a);
                    }
                }
            }
        }
    });

    action
}

fn node_matches_query(node: &TreeNode, q: &str) -> bool {
    if q.is_empty() {
        return true;
    }
    if node.title.to_lowercase().contains(q)
        || node.control_type.to_lowercase().contains(q)
        || node.uuid.to_lowercase().contains(q)
    {
        return true;
    }
    node.children.iter().any(|c| node_matches_query(c, q))
}

fn render_tree_node(
    ui: &mut Ui,
    project: &LoxoneProject,
    node: &TreeNode,
    q: &str,
    selected_uuid: Option<&str>,
) -> Option<TreeAction> {
    if !node_matches_query(node, q) {
        return None;
    }
    let mut action = None;
    let label_title = if node.title.is_empty() {
        &node.control_type
    } else {
        &node.title
    };
    let usages = project.xref.usages_for(&node.uuid).len();
    let is_selected = !node.uuid.is_empty() && selected_uuid == Some(node.uuid.as_str());

    if node.children.is_empty() {
        ui.horizontal(|ui| {
            let text = if usages > 0 {
                format!("{}  [{}]", label_title, usages)
            } else {
                label_title.to_string()
            };
            if ui
                .selectable_label(
                    is_selected,
                    RichText::new(text).color(if is_selected {
                        ACCENT_BLUE
                    } else {
                        TEXT_PRIMARY
                    }),
                )
                .clicked()
                && !node.uuid.is_empty()
            {
                action = Some(TreeAction::SelectObject(node.uuid.clone()));
            }
        });
    } else {
        let header = egui::CollapsingHeader::new(
            RichText::new(format!("{} ({})", label_title, node.control_type)).color(
                if is_selected {
                    ACCENT_BLUE
                } else {
                    TEXT_PRIMARY
                },
            ),
        )
        .id_salt(if node.uuid.is_empty() {
            label_title
        } else {
            &node.uuid
        })
        .default_open(!q.is_empty() || node.control_type == "LoxLIVE" || node.control_type == "Miniserver")
        .show(ui, |ui| {
            for child in &node.children {
                if let Some(a) = render_tree_node(ui, project, child, q, selected_uuid) {
                    action = Some(a);
                }
            }
        });
        if header.header_response.clicked() && !node.uuid.is_empty() {
            action = Some(TreeAction::SelectObject(node.uuid.clone()));
        }
    }

    let _ = (CARD_BG, BORDER_SUBTLE);
    action
}
```

Create `crates/loxone-gui/src/inspector.rs`:
```rust
use egui::{RichText, ScrollArea, Stroke, Ui};
use loxone_core::model::{ConnectorPin, LoxoneProject, PageBlock, TreeNode};
use loxone_core::xref::{CrossRefKind, ObjectLocation};

use crate::theme::{ACCENT_BLUE, BORDER_SUBTLE, CARD_BG, TEXT_MUTED, TEXT_PRIMARY};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectorAction {
    NavigateToBlock { page_index: usize, block_uuid: String },
    SelectObject(String),
}

pub fn render_right_inspector(
    ui: &mut Ui,
    project: &LoxoneProject,
    selected_uuid: Option<&str>,
) -> Option<InspectorAction> {
    let mut action = None;

    ui.add_space(6.0);
    ui.label(RichText::new("INSPECTOR").size(11.0).color(TEXT_MUTED).strong());
    ui.add_space(4.0);
    ui.separator();

    let Some(uuid) = selected_uuid else {
        ui.add_space(24.0);
        ui.label(
            RichText::new("Select any function block, peripheral, or page item to inspect properties and cross-references.")
                .color(TEXT_MUTED),
        );
        return None;
    };

    let Some(summary) = project.xref.objects_by_uuid.get(uuid) else {
        ui.label(RichText::new(format!("Object {} not found", uuid)).color(TEXT_MUTED));
        return None;
    };

    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        // Section 1: Identity Card
        egui::Frame::new()
            .fill(CARD_BG)
            .stroke(Stroke::new(1.0, BORDER_SUBTLE))
            .corner_radius(6)
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let title = if summary.title.is_empty() {
                    &summary.object_type
                } else {
                    &summary.title
                };
                ui.label(RichText::new(title).size(14.5).color(TEXT_PRIMARY).strong());
                ui.label(
                    RichText::new(&summary.object_type)
                        .size(11.5)
                        .color(ACCENT_BLUE),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("UUID: {}", summary.uuid))
                        .monospace()
                        .size(10.5)
                        .color(TEXT_MUTED),
                );
                if let Some(r_uuid) = &summary.room_uuid {
                    if let Some(place) = project.places.get(r_uuid) {
                        ui.label(
                            RichText::new(format!("Room: {}", place.title))
                                .size(11.5)
                                .color(TEXT_MUTED),
                        );
                    }
                }
                if let Some(c_uuid) = &summary.category_uuid {
                    if let Some(cat) = project.categories.get(c_uuid) {
                        ui.label(
                            RichText::new(format!("Category: {} ({})", cat.title, cat.category_type))
                                .size(11.5)
                                .color(TEXT_MUTED),
                        );
                    }
                }
            });

        ui.add_space(10.0);

        // Section 2: Used On Pages (Cross-References)
        let usages = project.xref.usages_for(uuid);
        ui.label(
            RichText::new(format!("USED ON PAGES ({})", usages.len()))
                .size(11.0)
                .color(TEXT_MUTED)
                .strong(),
        );
        ui.add_space(4.0);

        if usages.is_empty() {
            ui.label(RichText::new("No page references recorded").size(11.5).color(TEXT_MUTED));
        } else {
            for usage in usages {
                let kind_desc = match &usage.kind {
                    CrossRefKind::DirectRef => "Direct Ref".to_string(),
                    CrossRefKind::ExtraDeviceRef => "Device Link".to_string(),
                    CrossRefKind::WireFromPin {
                        source_pin_key,
                        target_pin_key,
                    } => format!("Wire {} → {}", source_pin_key, target_pin_key),
                };
                let label = format!(
                    "↗ {} • {} ({})",
                    usage.page_title,
                    if usage.block_title.is_empty() {
                        &usage.block_type
                    } else {
                        &usage.block_title
                    },
                    kind_desc
                );
                if ui
                    .button(RichText::new(label).size(11.5).color(ACCENT_BLUE))
                    .clicked()
                {
                    action = Some(InspectorAction::NavigateToBlock {
                        page_index: usage.page_index,
                        block_uuid: usage.block_uuid.clone(),
                    });
                }
            }
        }

        ui.add_space(10.0);

        // Section 3: Detailed Block or TreeNode attributes, pins, and raw XML
        match &summary.location {
            ObjectLocation::PageBlock {
                page_index,
                block_index,
                ..
            } => {
                if let Some(block) = project
                    .pages
                    .get(*page_index)
                    .and_then(|p| p.blocks.get(*block_index))
                {
                    if let Some(a) = render_block_details(ui, project, block) {
                        action = Some(a);
                    }
                }
            }
            ObjectLocation::TreeNode { .. } => {
                if let Some(node) = find_tree_node(&project.periphery_tree, uuid)
                    .or_else(|| find_tree_node(&project.system_tree, uuid))
                {
                    render_tree_node_details(ui, node);
                }
            }
            _ => {}
        }
    });

    action
}

fn render_block_details(
    ui: &mut Ui,
    project: &LoxoneProject,
    block: &PageBlock,
) -> Option<InspectorAction> {
    let mut action = None;

    if let Some(ref_uuid) = &block.ref_uuid {
        if let Some(target) = project.xref.objects_by_uuid.get(ref_uuid) {
            ui.label(
                RichText::new("REFERENCES TARGET")
                    .size(11.0)
                    .color(TEXT_MUTED)
                    .strong(),
            );
            if ui
                .button(
                    RichText::new(format!("🎯 {} ({})", target.title, target.object_type))
                        .color(ACCENT_BLUE),
                )
                .clicked()
            {
                action = Some(InspectorAction::SelectObject(ref_uuid.clone()));
            }
            ui.add_space(8.0);
        }
    }

    render_pins_list(ui, &block.pins);

    if !block.raw_attributes.is_empty() {
        ui.add_space(8.0);
        egui::CollapsingHeader::new(
            RichText::new(format!("Attributes ({})", block.raw_attributes.len()))
                .size(11.5)
                .color(TEXT_MUTED),
        )
        .default_open(true)
        .show(ui, |ui| {
            for (k, v) in &block.raw_attributes {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{}:", k)).monospace().size(11.0).color(TEXT_MUTED));
                    ui.label(RichText::new(v).monospace().size(11.0).color(TEXT_PRIMARY));
                });
            }
        });
    }

    if !block.raw_xml.is_empty() {
        ui.add_space(8.0);
        egui::CollapsingHeader::new(RichText::new("Raw XML").size(11.5).color(TEXT_MUTED))
            .default_open(false)
            .show(ui, |ui| {
                ui.code(&block.raw_xml);
            });
    }

    action
}

fn render_tree_node_details(ui: &mut Ui, node: &TreeNode) {
    render_pins_list(ui, &node.pins);

    if !node.attributes.is_empty() {
        ui.add_space(8.0);
        egui::CollapsingHeader::new(
            RichText::new(format!("Attributes ({})", node.attributes.len()))
                .size(11.5)
                .color(TEXT_MUTED),
        )
        .default_open(true)
        .show(ui, |ui| {
            for (k, v) in &node.attributes {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{}:", k)).monospace().size(11.0).color(TEXT_MUTED));
                    ui.label(RichText::new(v).monospace().size(11.0).color(TEXT_PRIMARY));
                });
            }
        });
    }

    if !node.raw_xml.is_empty() {
        ui.add_space(8.0);
        egui::CollapsingHeader::new(RichText::new("Raw XML").size(11.5).color(TEXT_MUTED))
            .default_open(false)
            .show(ui, |ui| {
                ui.code(&node.raw_xml);
            });
    }
}

fn render_pins_list(ui: &mut Ui, pins: &[ConnectorPin]) {
    if pins.is_empty() {
        return;
    }
    ui.label(
        RichText::new(format!("CONNECTOR PINS ({})", pins.len()))
            .size(11.0)
            .color(TEXT_MUTED)
            .strong(),
    );
    ui.add_space(4.0);
    for pin in pins {
        let mut details = Vec::new();
        if let Some(def) = &pin.default_value {
            details.push(format!("Def={}", def));
        }
        if !pin.incoming_links.is_empty() {
            details.push(format!("{} in", pin.incoming_links.len()));
        }
        let suffix = if details.is_empty() {
            String::new()
        } else {
            format!(" ({})", details.join(", "))
        };
        ui.label(
            RichText::new(format!("• {}{}", pin.key, suffix))
                .monospace()
                .size(11.0)
                .color(TEXT_PRIMARY),
        );
    }
}

fn find_tree_node<'a>(nodes: &'a [TreeNode], uuid: &str) -> Option<&'a TreeNode> {
    for node in nodes {
        if node.uuid == uuid {
            return Some(node);
        }
        if let Some(found) = find_tree_node(&node.children, uuid) {
            return Some(found);
        }
    }
    None
}
```

- [ ] **Step 2: Create `crates/loxone-gui/src/app.rs` and wire `crates/loxone-gui/src/main.rs` with unit tests**

Create `crates/loxone-gui/src/app.rs`:
```rust
use std::path::{Path, PathBuf};

use egui::{Context, Key, RichText, Stroke};
use loxone_core::model::LoxoneProject;
use loxone_core::parser::load_loxone_file;

use crate::canvas::{render_page_canvas, CanvasAction, CanvasViewport};
use crate::command_palette::{CommandPaletteState, PaletteAction};
use crate::inspector::{render_right_inspector, InspectorAction};
use crate::theme::{
    apply_minimalist_theme, ACCENT_BLUE, BORDER_SUBTLE, CARD_BG, PANEL_BG, TEXT_MUTED, TEXT_PRIMARY,
};
use crate::tree_panel::{render_left_explorer, LeftTab, TreeAction};

pub struct LoxonePortableApp {
    pub project: Option<LoxoneProject>,
    pub loaded_path: Option<PathBuf>,
    pub error_banner: Option<String>,
    pub active_page_index: usize,
    pub selected_uuid: Option<String>,
    pub left_tab: LeftTab,
    pub tree_filter: String,
    pub show_left_panel: bool,
    pub show_right_panel: bool,
    pub viewport: CanvasViewport,
    pub palette: CommandPaletteState,
    pub needs_initial_fit: bool,
    pub pending_center_block_uuid: Option<String>,
}

impl Default for LoxonePortableApp {
    fn default() -> Self {
        Self {
            project: None,
            loaded_path: None,
            error_banner: None,
            active_page_index: 0,
            selected_uuid: None,
            left_tab: LeftTab::Pages,
            tree_filter: String::new(),
            show_left_panel: true,
            show_right_panel: true,
            viewport: CanvasViewport::default(),
            palette: CommandPaletteState::default(),
            needs_initial_fit: true,
            pending_center_block_uuid: None,
        }
    }
}

impl LoxonePortableApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial_path: Option<PathBuf>) -> Self {
        apply_minimalist_theme(&cc.egui_ctx);
        let mut app = Self::default();
        if let Some(path) = initial_path {
            app.load_file(&path);
        }
        app
    }

    pub fn load_file(&mut self, path: &Path) {
        match load_loxone_file(path) {
            Ok(proj) => {
                self.project = Some(proj);
                self.loaded_path = Some(path.to_path_buf());
                self.error_banner = None;
                self.active_page_index = 0;
                self.selected_uuid = None;
                self.needs_initial_fit = true;
            }
            Err(err) => {
                self.error_banner = Some(format!("Failed to load {}: {}", path.display(), err));
            }
        }
    }

    pub fn execute_palette_action(&mut self, action: PaletteAction) {
        match action {
            PaletteAction::OpenFile => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Loxone Project", &["Loxone", "loxone", "xml"])
                    .pick_file()
                {
                    self.load_file(&path);
                }
            }
            PaletteAction::FitPage => {
                self.needs_initial_fit = true;
            }
            PaletteAction::ResetZoom => {
                self.viewport.zoom = 1.0;
                self.viewport.pan = egui::Vec2::new(40.0, 40.0);
            }
            PaletteAction::ToggleCompactPins => {
                self.viewport.compact_pins = !self.viewport.compact_pins;
            }
            PaletteAction::ToggleLeftPanel => {
                self.show_left_panel = !self.show_left_panel;
            }
            PaletteAction::ToggleRightPanel => {
                self.show_right_panel = !self.show_right_panel;
            }
            PaletteAction::NavigateToPage(idx) => {
                self.active_page_index = idx;
                self.needs_initial_fit = true;
            }
            PaletteAction::NavigateToBlock {
                page_index,
                block_uuid,
            } => {
                self.active_page_index = page_index;
                self.selected_uuid = Some(block_uuid.clone());
                self.pending_center_block_uuid = Some(block_uuid);
                self.show_right_panel = true;
            }
            PaletteAction::SelectObject(uuid) => {
                self.selected_uuid = Some(uuid);
                self.show_right_panel = true;
            }
        }
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        let cmd_or_ctrl = ctx.input(|i| i.modifiers.command || i.modifiers.ctrl);

        if cmd_or_ctrl && ctx.input(|i| i.key_pressed(Key::K)) {
            if self.palette.is_open {
                self.palette.close();
            } else {
                self.palette.open();
            }
        }
        if self.palette.is_open {
            return;
        }

        if cmd_or_ctrl && ctx.input(|i| i.key_pressed(Key::B)) {
            self.show_left_panel = !self.show_left_panel;
        }
        if cmd_or_ctrl && ctx.input(|i| i.key_pressed(Key::I)) {
            self.show_right_panel = !self.show_right_panel;
        }
        if cmd_or_ctrl && ctx.input(|i| i.key_pressed(Key::O)) {
            self.execute_palette_action(PaletteAction::OpenFile);
        }

        if !ctx.wants_keyboard_input() {
            if ctx.input(|i| i.key_pressed(Key::F)) {
                self.needs_initial_fit = true;
            }
            if ctx.input(|i| i.key_pressed(Key::P)) {
                self.viewport.compact_pins = !self.viewport.compact_pins;
            }
            if ctx.input(|i| i.key_pressed(Key::Num0)) {
                self.execute_palette_action(PaletteAction::ResetZoom);
            }
        }
    }
}

impl eframe::App for LoxonePortableApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);

        // Top Minimalist Header Bar
        egui::TopBottomPanel::top("top_header")
            .exact_height(38.0)
            .frame(
                egui::Frame::new()
                    .fill(PANEL_BG)
                    .stroke(Stroke::new(1.0, BORDER_SUBTLE))
                    .inner_margin(egui::Margin::symmetric(12, 6)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new("☰").color(TEXT_MUTED))
                        .on_hover_text("Toggle Explorer (Ctrl+B)")
                        .clicked()
                    {
                        self.show_left_panel = !self.show_left_panel;
                    }

                    let title_str = self
                        .project
                        .as_ref()
                        .map(|p| {
                            let page_name = p
                                .pages
                                .get(self.active_page_index)
                                .map(|pg| pg.title.as_str())
                                .unwrap_or("No Page");
                            format!("{}  /  {}", p.metadata.title, page_name)
                        })
                        .unwrap_or_else(|| "Loxone Portable".to_string());
                    ui.label(RichText::new(title_str).color(TEXT_PRIMARY).strong());

                    ui.add_space(16.0);
                    let pill = egui::Frame::new()
                        .fill(CARD_BG)
                        .stroke(Stroke::new(1.0, BORDER_SUBTLE))
                        .corner_radius(6)
                        .inner_margin(egui::Margin::symmetric(12, 3))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("🔍  Search or jump to page, block, device…   ⌘K / Ctrl+K")
                                    .size(11.5)
                                    .color(TEXT_MUTED),
                            );
                        })
                        .response
                        .interact(egui::Sense::click());
                    if pill.clicked() {
                        self.palette.open();
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(RichText::new("ℹ").color(TEXT_MUTED))
                            .on_hover_text("Toggle Inspector (Ctrl+I)")
                            .clicked()
                        {
                            self.show_right_panel = !self.show_right_panel;
                        }
                        if ui
                            .button(RichText::new("Fit (F)").size(11.5).color(TEXT_PRIMARY))
                            .clicked()
                        {
                            self.needs_initial_fit = true;
                        }
                        let pin_mode_label = if self.viewport.compact_pins {
                            "Pins: Connected (P)"
                        } else {
                            "Pins: All (P)"
                        };
                        if ui
                            .button(RichText::new(pin_mode_label).size(11.5).color(ACCENT_BLUE))
                            .clicked()
                        {
                            self.viewport.compact_pins = !self.viewport.compact_pins;
                        }
                        ui.label(
                            RichText::new(format!("{:.0}%", self.viewport.zoom * 100.0))
                                .monospace()
                                .size(11.5)
                                .color(TEXT_MUTED),
                        );
                    });
                });
            });

        if let Some(err) = &self.error_banner {
            egui::TopBottomPanel::top("error_banner").show(ctx, |ui| {
                ui.colored_label(egui::Color32::LIGHT_RED, err);
            });
        }

        // Left Explorer Panel
        if self.show_left_panel {
            if let Some(proj) = &self.project {
                let mut tree_act = None;
                egui::SidePanel::left("left_explorer")
                    .default_width(250.0)
                    .frame(
                        egui::Frame::new()
                            .fill(PANEL_BG)
                            .stroke(Stroke::new(1.0, BORDER_SUBTLE))
                            .inner_margin(egui::Margin::symmetric(10, 6)),
                    )
                    .show(ctx, |ui| {
                        tree_act = render_left_explorer(
                            ui,
                            proj,
                            &mut self.left_tab,
                            &mut self.tree_filter,
                            self.active_page_index,
                            self.selected_uuid.as_deref(),
                        );
                    });

                if let Some(act) = tree_act {
                    match act {
                        TreeAction::SelectPage(idx) => {
                            self.active_page_index = idx;
                            self.needs_initial_fit = true;
                        }
                        TreeAction::SelectObject(uuid) => {
                            self.selected_uuid = Some(uuid);
                            self.show_right_panel = true;
                        }
                        TreeAction::NavigateToBlock {
                            page_index,
                            block_uuid,
                        } => {
                            self.execute_palette_action(PaletteAction::NavigateToBlock {
                                page_index,
                                block_uuid,
                            });
                        }
                    }
                }
            }
        }

        // Right Inspector Panel
        if self.show_right_panel {
            if let Some(proj) = &self.project {
                let mut insp_act = None;
                egui::SidePanel::right("right_inspector")
                    .default_width(300.0)
                    .frame(
                        egui::Frame::new()
                            .fill(PANEL_BG)
                            .stroke(Stroke::new(1.0, BORDER_SUBTLE))
                            .inner_margin(egui::Margin::symmetric(10, 6)),
                    )
                    .show(ctx, |ui| {
                        insp_act =
                            render_right_inspector(ui, proj, self.selected_uuid.as_deref());
                    });

                if let Some(act) = insp_act {
                    match act {
                        InspectorAction::NavigateToBlock {
                            page_index,
                            block_uuid,
                        } => {
                            self.execute_palette_action(PaletteAction::NavigateToBlock {
                                page_index,
                                block_uuid,
                            });
                        }
                        InspectorAction::SelectObject(uuid) => {
                            self.selected_uuid = Some(uuid);
                        }
                    }
                }
            }
        }

        // Central Schematic Page Canvas
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill( crate::theme::CANVAS_BG ))
            .show(ctx, |ui| {
                let view_size = ui.available_size();
                if let Some(proj) = &self.project {
                    if self.needs_initial_fit {
                        if let Some(page) = proj.pages.get(self.active_page_index) {
                            self.viewport.fit_to_page(page, view_size);
                        }
                        self.needs_initial_fit = false;
                    }
                    if let Some(target_uuid) = self.pending_center_block_uuid.take() {
                        if let Some(page) = proj.pages.get(self.active_page_index) {
                            if let Some(block) =
                                page.blocks.iter().find(|b| b.uuid == target_uuid)
                            {
                                self.viewport.center_on_block(block, view_size);
                            }
                        }
                    }

                    if let Some(canvas_act) = render_page_canvas(
                        ui,
                        proj,
                        self.active_page_index,
                        &mut self.viewport,
                        self.selected_uuid.as_deref(),
                    ) {
                        match canvas_act {
                            CanvasAction::SelectObject(uuid) => {
                                self.selected_uuid = Some(uuid);
                                self.show_right_panel = true;
                            }
                            CanvasAction::ClearSelection => {
                                self.selected_uuid = None;
                            }
                            CanvasAction::NavigateToBlock {
                                page_index,
                                block_uuid,
                            } => {
                                self.execute_palette_action(PaletteAction::NavigateToBlock {
                                    page_index,
                                    block_uuid,
                                });
                            }
                        }
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        if ui
                            .button("Open .Loxone Project File (Ctrl+O)…")
                            .clicked()
                        {
                            self.execute_palette_action(PaletteAction::OpenFile);
                        }
                    });
                }
            });

        // Ctrl+K / Cmd+K Command Palette Modal Overlay
        if let Some(palette_act) = self.palette.show(ctx, self.project.as_ref()) {
            self.execute_palette_action(palette_act);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_load_and_navigate_klimczaki() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let mut app = LoxonePortableApp::default();
        app.load_file(&path);

        assert!(app.project.is_some());
        assert!(app.error_banner.is_none());

        // Navigate via Command Palette action to Page 5
        app.execute_palette_action(PaletteAction::NavigateToPage(5));
        assert_eq!(app.active_page_index, 5);

        // Toggle compact pins
        assert!(app.viewport.compact_pins);
        app.execute_palette_action(PaletteAction::ToggleCompactPins);
        assert!(!app.viewport.compact_pins);
    }
}
```

Update `crates/loxone-gui/src/main.rs`:
```rust
mod app;
mod canvas;
mod command_palette;
mod inspector;
mod theme;
mod tree_panel;

use std::path::PathBuf;

use app::LoxonePortableApp;

fn resolve_initial_project_path() -> Option<PathBuf> {
    if let Some(arg) = std::env::args().nth(1) {
        let p = PathBuf::from(arg);
        if p.exists() {
            return Some(p);
        }
    }
    let default_path = PathBuf::from("Klimczaki.Loxone");
    if default_path.exists() {
        return Some(default_path);
    }
    None
}

fn main() -> eframe::Result<()> {
    let initial_path = resolve_initial_project_path();
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 600.0])
            .with_title("Loxone Portable — Project Viewer & Schematic Inspector"),
        ..Default::default()
    };

    eframe::run_native(
        "Loxone Portable",
        native_options,
        Box::new(move |cc| Ok(Box::new(LoxonePortableApp::new(cc, initial_path)))),
    )
}
```

- [ ] **Step 3: Run full workspace tests and verify build**

Run: `cargo test --workspace`
Expected: PASS (all unit and integration tests across `loxone-core` and `loxone-gui` pass).

- [ ] **Step 4: Commit**

```bash
git add crates/loxone-gui
git commit -m "feat(gui): complete minimalist 3-pane workspace, tree explorer, and inspector"
```
