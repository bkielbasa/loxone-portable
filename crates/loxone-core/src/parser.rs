use std::collections::HashSet;
use std::path::Path;

use indexmap::IndexMap;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use thiserror::Error;

use crate::model::{
    classify_pin_kind, parse_rgb_color, Category, ConnectorPin, LoxoneProject, Page, PageBlock,
    Place, ProjectMetadata, RawNode, TreeNode, TreeSectionKind, WireLink,
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

    let mut metadata = ProjectMetadata {
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
        miniserver_ip: None,
        miniserver_serial: None,
        miniserver_hostname: None,
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
                if metadata.miniserver_ip.is_none() {
                    metadata.miniserver_ip = child
                        .attr("IntAddr")
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string);
                }
                if metadata.miniserver_serial.is_none() {
                    metadata.miniserver_serial = child
                        .attr("Serial")
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string);
                }
                if metadata.miniserver_hostname.is_none() {
                    metadata.miniserver_hostname = child
                        .attr("HoN")
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string);
                }
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

    let mut project = LoxoneProject {
        metadata,
        categories,
        places,
        pages,
        periphery_tree,
        system_tree,
        xref: crate::xref::XrefIndex::default(),
        raw_xml: Some(xml.to_string()),
    };
    project.xref = crate::xref::XrefIndex::build(&project);
    Ok(project)
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
        assert_eq!(
            project.metadata.miniserver_ip.as_deref(),
            Some("192.168.1.100")
        );
        assert_eq!(
            project.metadata.miniserver_serial.as_deref(),
            Some("504F94A0ECAD")
        );
        assert_eq!(
            project.metadata.miniserver_hostname.as_deref(),
            Some("miniserwer")
        );
        assert!(project
            .raw_xml
            .as_ref()
            .is_some_and(|xml| xml.contains("<ControlList")));
        assert_eq!(project.pages.len(), 28);
        assert_eq!(project.places.len(), 16);
        assert_eq!(project.categories.len(), 33);
        assert!(!project.periphery_tree.is_empty());
        assert!(!project.system_tree.is_empty());
    }
}
