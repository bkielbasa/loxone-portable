use std::collections::HashMap;
use std::path::Path;

use crate::model::{
    parse_rgb_color, ConnectorPin, LoxoneProject, Page, PageBlock, PinKind, RawNode, TreeNode,
};
use crate::parser::{parse_raw_dom, LoxoneError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectEdit {
    SetControlAttr {
        control_uuid: String,
        attr_name: String,
        value: Option<String>,
    },
    SetPinDefault {
        control_uuid: String,
        pin_key: String,
        default_value: Option<String>,
    },
    SetChildTagAttr {
        control_uuid: String,
        child_tag: String,
        attr_name: String,
        value: Option<String>,
    },
}

pub fn apply_edit(project: &mut LoxoneProject, edit: &ProjectEdit) -> Result<(), LoxoneError> {
    let mut needs_xref_rebuild = false;

    match edit {
        ProjectEdit::SetControlAttr {
            control_uuid,
            attr_name,
            value,
        } => {
            if let Some(block) = find_page_block_mut(project, control_uuid) {
                set_vec_attr(&mut block.raw_attributes, attr_name, value.as_deref());
                sync_page_block_attr(block, attr_name, value.as_deref());
                if matches!(
                    attr_name.as_str(),
                    "Title" | "Ref" | "UO" | "SpStates" | "rec"
                ) {
                    needs_xref_rebuild = true;
                }
            } else if let Some(tree_node) = find_tree_node_mut(project, control_uuid) {
                set_vec_attr(&mut tree_node.raw_attributes, attr_name, value.as_deref());
                sync_tree_node_attr(tree_node, attr_name, value.as_deref());
                if attr_name == "Title" {
                    needs_xref_rebuild = true;
                }
            } else if let Some(cat) = project.categories.get_mut(control_uuid) {
                match attr_name.as_str() {
                    "Title" => {
                        cat.title = value.clone().unwrap_or_default();
                        needs_xref_rebuild = true;
                    }
                    "BkColor" | "Cl" => {
                        cat.bk_color = value.as_deref().and_then(parse_rgb_color);
                    }
                    "CatGroup" => {
                        cat.cat_group = value.clone();
                    }
                    "Icon" => {
                        cat.icon = value.clone();
                    }
                    _ => {}
                }
            } else if let Some(place) = project.places.get_mut(control_uuid) {
                match attr_name.as_str() {
                    "Title" => {
                        place.title = value.clone().unwrap_or_default();
                        needs_xref_rebuild = true;
                    }
                    "Sqm" => {
                        place.sqm = value.clone();
                    }
                    "PType" => {
                        place.p_type = value.clone();
                    }
                    "Icon" => {
                        place.icon = value.clone();
                    }
                    _ => {}
                }
            } else if let Some(page) = project.pages.iter_mut().find(|p| p.uuid == *control_uuid) {
                if attr_name == "Title" {
                    page.title = value.clone().unwrap_or_default();
                    needs_xref_rebuild = true;
                }
            }
        }
        ProjectEdit::SetPinDefault {
            control_uuid,
            pin_key,
            default_value,
        } => {
            if let Some(block) = find_page_block_mut(project, control_uuid) {
                if set_pin_default_on_slice(&mut block.pins, control_uuid, pin_key, default_value) {
                    needs_xref_rebuild = true;
                }
            } else if let Some(tree_node) = find_tree_node_mut(project, control_uuid) {
                if set_pin_default_on_slice(
                    &mut tree_node.pins,
                    control_uuid,
                    pin_key,
                    default_value,
                ) {
                    needs_xref_rebuild = true;
                }
            }
        }
        ProjectEdit::SetChildTagAttr {
            control_uuid,
            child_tag,
            attr_name,
            value,
        } => {
            if let Some(block) = find_page_block_mut(project, control_uuid) {
                set_child_tag_attr_on_vec(
                    &mut block.extra_children,
                    child_tag,
                    attr_name,
                    value.as_deref(),
                );
                if child_tag == "IoData" {
                    if attr_name == "Pr" {
                        block.room_uuid = value.clone();
                    } else if attr_name == "Cr" {
                        block.category_uuid = value.clone();
                    }
                    needs_xref_rebuild = true;
                } else if child_tag == "LoggerMailer" && attr_name == "RefCaller" {
                    let has_ref_attr = block.raw_attributes.iter().any(|(k, _)| k == "Ref");
                    if !has_ref_attr {
                        block.ref_uuid = value.clone();
                        needs_xref_rebuild = true;
                    }
                }
            } else if let Some(tree_node) = find_tree_node_mut(project, control_uuid) {
                set_child_tag_attr_on_vec(
                    &mut tree_node.extra_children,
                    child_tag,
                    attr_name,
                    value.as_deref(),
                );
                if child_tag == "IoData" {
                    if attr_name == "Pr" {
                        tree_node.room_uuid = value.clone();
                    } else if attr_name == "Cr" {
                        tree_node.category_uuid = value.clone();
                    }
                    needs_xref_rebuild = true;
                }
            }
        }
    }

    if needs_xref_rebuild {
        project.xref = crate::xref::XrefIndex::build(project);
    }

    Ok(())
}

pub fn serialize_loxone_xml(project: &LoxoneProject) -> Result<String, LoxoneError> {
    let raw_xml = project
        .raw_xml
        .as_deref()
        .ok_or(LoxoneError::MissingDocumentRoot)?;
    let clean_xml = raw_xml.trim_start_matches('\u{FEFF}');
    let mut root_nodes = parse_raw_dom(clean_xml)?;

    let mut blocks_by_uuid: HashMap<&str, &PageBlock> = HashMap::new();
    let mut pages_by_uuid: HashMap<&str, &Page> = HashMap::new();
    for page in &project.pages {
        if !page.uuid.is_empty() {
            pages_by_uuid.insert(page.uuid.as_str(), page);
        }
        for block in &page.blocks {
            if !block.uuid.is_empty() {
                blocks_by_uuid.insert(block.uuid.as_str(), block);
            }
        }
    }

    let mut tree_nodes_by_uuid: HashMap<&str, &TreeNode> = HashMap::new();
    for node in &project.periphery_tree {
        collect_tree_nodes_by_uuid(node, &mut tree_nodes_by_uuid);
    }
    for node in &project.system_tree {
        collect_tree_nodes_by_uuid(node, &mut tree_nodes_by_uuid);
    }

    for root in &mut root_nodes {
        sync_raw_node(
            root,
            project,
            &blocks_by_uuid,
            &tree_nodes_by_uuid,
            &pages_by_uuid,
        );
    }

    let mut out = String::with_capacity(raw_xml.len() + 1024);
    out.push_str("\u{FEFF}<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n");
    for root in &root_nodes {
        write_raw_node(root, 0, &mut out);
    }
    Ok(out)
}

pub fn save_loxone_file(path: &Path, project: &mut LoxoneProject) -> Result<(), LoxoneError> {
    let xml = serialize_loxone_xml(project)?;
    std::fs::write(path, &xml)?;
    project.raw_xml = Some(xml);
    Ok(())
}

fn find_page_block_mut<'a>(
    project: &'a mut LoxoneProject,
    control_uuid: &str,
) -> Option<&'a mut PageBlock> {
    for page in &mut project.pages {
        for block in &mut page.blocks {
            if block.uuid == control_uuid {
                return Some(block);
            }
        }
    }
    None
}

fn find_tree_node_mut<'a>(
    project: &'a mut LoxoneProject,
    control_uuid: &str,
) -> Option<&'a mut TreeNode> {
    for node in &mut project.periphery_tree {
        if let Some(found) = find_in_tree_node_mut(node, control_uuid) {
            return Some(found);
        }
    }
    for node in &mut project.system_tree {
        if let Some(found) = find_in_tree_node_mut(node, control_uuid) {
            return Some(found);
        }
    }
    None
}

fn find_in_tree_node_mut<'a>(
    node: &'a mut TreeNode,
    control_uuid: &str,
) -> Option<&'a mut TreeNode> {
    if node.uuid == control_uuid {
        return Some(node);
    }
    for child in &mut node.children {
        if let Some(found) = find_in_tree_node_mut(child, control_uuid) {
            return Some(found);
        }
    }
    None
}

fn set_vec_attr(attributes: &mut Vec<(String, String)>, name: &str, value: Option<&str>) {
    match value {
        Some(val) => {
            if let Some((_, existing_val)) = attributes.iter_mut().find(|(k, _)| k == name) {
                *existing_val = val.to_string();
            } else {
                attributes.push((name.to_string(), val.to_string()));
            }
        }
        None => {
            attributes.retain(|(k, _)| k != name);
        }
    }
}

fn sync_page_block_attr(block: &mut PageBlock, attr_name: &str, value: Option<&str>) {
    match attr_name {
        "Title" => {
            block.title = value.unwrap_or_default().to_string();
        }
        "Cl" => {
            block.color_rgb = value.and_then(parse_rgb_color);
        }
        "Text" => {
            block.text_content = value.map(str::to_string);
        }
        "Ref" => {
            if let Some(val) = value {
                block.ref_uuid = Some(val.to_string());
            } else {
                block.ref_uuid = block
                    .extra_children
                    .iter()
                    .find(|c| c.tag == "LoggerMailer")
                    .and_then(|c| c.attr("RefCaller"))
                    .map(str::to_string);
            }
        }
        "UO" | "SpStates" | "rec" => {
            let mut extra_ref_uuids = Vec::new();
            for key in ["UO", "SpStates", "rec"] {
                if let Some((_, val)) = block.raw_attributes.iter().find(|(k, _)| k == key) {
                    for part in val.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                        extra_ref_uuids.push(part.to_string());
                    }
                }
            }
            block.extra_ref_uuids = extra_ref_uuids;
        }
        "Px" => {
            block.px = value.and_then(|v| v.parse().ok()).unwrap_or(0.0);
            block.has_position = has_attr(&block.raw_attributes, "Px")
                && has_attr(&block.raw_attributes, "Py");
        }
        "Py" => {
            block.py = value.and_then(|v| v.parse().ok()).unwrap_or(0.0);
            block.has_position = has_attr(&block.raw_attributes, "Px")
                && has_attr(&block.raw_attributes, "Py");
        }
        "Px2" => {
            block.px2 = value.and_then(|v| v.parse().ok()).unwrap_or(0.0);
        }
        "Py2" => {
            block.py2 = value.and_then(|v| v.parse().ok()).unwrap_or(0.0);
        }
        "Type" => {
            if let Some(val) = value {
                block.block_type = val.to_string();
            }
        }
        _ => {}
    }
}

fn sync_tree_node_attr(node: &mut TreeNode, attr_name: &str, value: Option<&str>) {
    match attr_name {
        "Title" => {
            node.title = value.unwrap_or_default().to_string();
        }
        "Type" => {
            if let Some(val) = value {
                node.control_type = val.to_string();
            }
        }
        _ => {}
    }
}

fn has_attr(attributes: &[(String, String)], key: &str) -> bool {
    attributes.iter().any(|(k, _)| k == key)
}

fn set_pin_default_on_slice(
    pins: &mut Vec<ConnectorPin>,
    control_uuid: &str,
    pin_key: &str,
    default_value: &Option<String>,
) -> bool {
    if let Some(pin) = pins.iter_mut().find(|p| p.key == pin_key) {
        pin.default_value = default_value.clone();
        false
    } else {
        pins.push(ConnectorPin {
            uuid: format!("{}-{}", control_uuid, pin_key),
            key: pin_key.to_string(),
            default_value: default_value.clone(),
            pin_kind: PinKind::Parameter,
            incoming_links: Vec::new(),
        });
        true
    }
}

fn set_child_tag_attr_on_vec(
    extra_children: &mut Vec<RawNode>,
    child_tag: &str,
    attr_name: &str,
    value: Option<&str>,
) {
    if let Some(child) = extra_children.iter_mut().find(|c| c.tag == child_tag) {
        child.set_attr(attr_name, value);
    } else if value.is_some() {
        let mut child = RawNode {
            tag: child_tag.to_string(),
            attributes: Vec::new(),
            text: None,
            children: Vec::new(),
        };
        child.set_attr(attr_name, value);
        extra_children.push(child);
    }
}

fn collect_tree_nodes_by_uuid<'a>(
    node: &'a TreeNode,
    map: &mut HashMap<&'a str, &'a TreeNode>,
) {
    if !node.uuid.is_empty() {
        map.insert(node.uuid.as_str(), node);
    }
    for child in &node.children {
        collect_tree_nodes_by_uuid(child, map);
    }
}

fn sync_raw_node(
    node: &mut RawNode,
    project: &LoxoneProject,
    blocks_by_uuid: &HashMap<&str, &PageBlock>,
    tree_nodes_by_uuid: &HashMap<&str, &TreeNode>,
    pages_by_uuid: &HashMap<&str, &Page>,
) {
    if node.tag == "C" {
        let uuid = node.attr("U").unwrap_or_default().to_string();
        if !uuid.is_empty() {
            if let Some(block) = blocks_by_uuid.get(uuid.as_str()) {
                node.attributes = block.raw_attributes.clone();
                let mut new_children = sync_co_children(&node.children, &block.pins);
                new_children.extend(block.extra_children.iter().cloned());
                node.children = new_children;
                return;
            } else if let Some(tree_node) = tree_nodes_by_uuid.get(uuid.as_str()) {
                node.attributes = tree_node.raw_attributes.clone();
                let mut new_children = sync_co_children(&node.children, &tree_node.pins);
                new_children.extend(tree_node.extra_children.iter().cloned());
                for mut c_child in node
                    .children
                    .iter()
                    .filter(|c| c.tag == "C")
                    .cloned()
                    .collect::<Vec<_>>()
                {
                    sync_raw_node(
                        &mut c_child,
                        project,
                        blocks_by_uuid,
                        tree_nodes_by_uuid,
                        pages_by_uuid,
                    );
                    new_children.push(c_child);
                }
                node.children = new_children;
                return;
            } else if let Some(cat) = project.categories.get(&uuid) {
                node.set_attr("Title", Some(&cat.title));
                let bk = cat
                    .bk_color
                    .map(|[r, g, b]| format!("{},{},{}", r, g, b));
                node.set_attr("BkColor", bk.as_deref());
                node.set_attr("CatGroup", cat.cat_group.as_deref());
                node.set_attr("Icon", cat.icon.as_deref());
            } else if let Some(place) = project.places.get(&uuid) {
                node.set_attr("Title", Some(&place.title));
                node.set_attr("Sqm", place.sqm.as_deref());
                node.set_attr("PType", place.p_type.as_deref());
                node.set_attr("Icon", place.icon.as_deref());
            } else if let Some(page) = pages_by_uuid.get(uuid.as_str()) {
                node.set_attr("Title", Some(&page.title));
            }
        }
    }

    for child in &mut node.children {
        sync_raw_node(
            child,
            project,
            blocks_by_uuid,
            tree_nodes_by_uuid,
            pages_by_uuid,
        );
    }
}

fn sync_co_children(existing_children: &[RawNode], pins: &[ConnectorPin]) -> Vec<RawNode> {
    let mut co_nodes: Vec<RawNode> = existing_children
        .iter()
        .filter(|c| c.tag == "Co")
        .cloned()
        .collect();

    for pin in pins {
        if let Some(co) = co_nodes
            .iter_mut()
            .find(|c| c.attr("K") == Some(pin.key.as_str()))
        {
            co.set_attr("Def", pin.default_value.as_deref());
        } else {
            let mut new_co = RawNode {
                tag: "Co".to_string(),
                attributes: vec![
                    ("K".to_string(), pin.key.clone()),
                    ("U".to_string(), pin.uuid.clone()),
                ],
                text: None,
                children: Vec::new(),
            };
            new_co.set_attr("Def", pin.default_value.as_deref());
            co_nodes.push(new_co);
        }
    }

    co_nodes
}

fn write_raw_node(node: &RawNode, depth: usize, out: &mut String) {
    for _ in 0..depth {
        out.push('\t');
    }
    out.push('<');
    out.push_str(&node.tag);
    for (k, v) in &node.attributes {
        out.push(' ');
        out.push_str(k);
        out.push_str("=\"");
        escape_xml_attr(v, out);
        out.push('"');
    }

    if node.children.is_empty() && node.text.is_none() {
        out.push_str("/>\r\n");
    } else if node.children.is_empty() {
        out.push('>');
        if let Some(txt) = &node.text {
            escape_xml_text(txt, out);
        }
        out.push_str("</");
        out.push_str(&node.tag);
        out.push_str(">\r\n");
    } else {
        out.push_str(">\r\n");
        if let Some(txt) = &node.text {
            for _ in 0..=depth {
                out.push('\t');
            }
            escape_xml_text(txt, out);
            out.push_str("\r\n");
        }
        for child in &node.children {
            write_raw_node(child, depth + 1, out);
        }
        for _ in 0..depth {
            out.push('\t');
        }
        out.push_str("</");
        out.push_str(&node.tag);
        out.push_str(">\r\n");
    }
}

fn escape_xml_attr(val: &str, out: &mut String) {
    for ch in val.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\r' => out.push_str("&#xD;"),
            '\n' => out.push_str("&#xA;"),
            _ => out.push(ch),
        }
    }
}

fn escape_xml_text(val: &str, out: &mut String) {
    for ch in val.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{load_loxone_file, parse_loxone_xml};
    use std::path::PathBuf;

    #[test]
    fn test_apply_edits_and_roundtrip_serialize_klimczaki() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let mut project = load_loxone_file(&path).expect("Klimczaki.Loxone should load");

        let target_block_uuid = project.pages[0].blocks[0].uuid.clone();
        let target_pin_key = project.pages[0].blocks[0].pins[0].key.clone();
        let first_room_uuid = project.places.keys().next().unwrap().clone();
        let first_cat_uuid = project.categories.keys().next().unwrap().clone();

        // 1. Edit block Title and Color
        apply_edit(
            &mut project,
            &ProjectEdit::SetControlAttr {
                control_uuid: target_block_uuid.clone(),
                attr_name: "Title".to_string(),
                value: Some("Edited Block Title".to_string()),
            },
        )
        .unwrap();
        apply_edit(
            &mut project,
            &ProjectEdit::SetControlAttr {
                control_uuid: target_block_uuid.clone(),
                attr_name: "Cl".to_string(),
                value: Some("12,34,56".to_string()),
            },
        )
        .unwrap();

        assert_eq!(project.pages[0].blocks[0].title, "Edited Block Title");
        assert_eq!(project.pages[0].blocks[0].color_rgb, Some([12, 34, 56]));

        // 2. Edit Pin Default value
        apply_edit(
            &mut project,
            &ProjectEdit::SetPinDefault {
                control_uuid: target_block_uuid.clone(),
                pin_key: target_pin_key.clone(),
                default_value: Some("42.5".to_string()),
            },
        )
        .unwrap();
        assert_eq!(
            project.pages[0].blocks[0].pins[0].default_value.as_deref(),
            Some("42.5")
        );

        // 3. Edit Child Tag <IoData Pr="..." Cr="..."/>
        apply_edit(
            &mut project,
            &ProjectEdit::SetChildTagAttr {
                control_uuid: target_block_uuid.clone(),
                child_tag: "IoData".to_string(),
                attr_name: "Pr".to_string(),
                value: Some(first_room_uuid.clone()),
            },
        )
        .unwrap();
        apply_edit(
            &mut project,
            &ProjectEdit::SetChildTagAttr {
                control_uuid: target_block_uuid.clone(),
                child_tag: "IoData".to_string(),
                attr_name: "Cr".to_string(),
                value: Some(first_cat_uuid.clone()),
            },
        )
        .unwrap();
        assert_eq!(
            project.pages[0].blocks[0].room_uuid.as_deref(),
            Some(first_room_uuid.as_str())
        );
        assert_eq!(
            project.pages[0].blocks[0].category_uuid.as_deref(),
            Some(first_cat_uuid.as_str())
        );

        // 4. Serialize and re-parse
        let xml = serialize_loxone_xml(&project).expect("should serialize XML");
        assert!(xml.starts_with("\u{FEFF}<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n"));

        let reparsed = parse_loxone_xml(&xml).expect("reparsed XML should be valid");
        assert_eq!(reparsed.pages.len(), 28);
        assert_eq!(reparsed.places.len(), 16);
        assert_eq!(reparsed.categories.len(), 33);
        assert_eq!(
            reparsed.xref.pins_by_uuid.len(),
            project.xref.pins_by_uuid.len()
        );
        assert_eq!(
            reparsed.xref.outgoing_wires_by_pin_uuid.len(),
            project.xref.outgoing_wires_by_pin_uuid.len()
        );

        let blk = &reparsed.pages[0].blocks[0];
        assert_eq!(blk.uuid, target_block_uuid);
        assert_eq!(blk.title, "Edited Block Title");
        assert_eq!(blk.color_rgb, Some([12, 34, 56]));
        assert_eq!(blk.pins[0].default_value.as_deref(), Some("42.5"));
        assert_eq!(blk.room_uuid.as_deref(), Some(first_room_uuid.as_str()));
        assert_eq!(blk.category_uuid.as_deref(), Some(first_cat_uuid.as_str()));

        // Verify save_loxone_file writes to disk and updates project.raw_xml
        let tmp_path = std::env::temp_dir().join("loxone_writer_roundtrip_test.Loxone");
        save_loxone_file(&tmp_path, &mut project).expect("save_loxone_file should succeed");
        let reloaded = load_loxone_file(&tmp_path).expect("saved file should reload");
        let _ = std::fs::remove_file(&tmp_path);
        assert_eq!(reloaded.pages[0].blocks[0].title, "Edited Block Title");
    }

    #[test]
    fn test_apply_edits_on_tree_node_and_child_tags() {
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
                        <C Type=\"LightController2\" U=\"blk-2\" Title=\"Main Light\" Px=\"7392\" Py=\"576\" Px2=\"10080\" Py2=\"1848\" Cl=\"255,200,0\">\
                            <Co K=\"Step\" Def=\"10\" U=\"pin-2-step\"/>\
                        </C>\
                    </C>\
                </C>\
            </C>\
        </ControlList>";

        let mut project = parse_loxone_xml(xml).expect("should parse minimal XML");

        // 1. Edit TreeNode attributes, pin default (creating new pin), and IoData
        apply_edit(
            &mut project,
            &ProjectEdit::SetControlAttr {
                control_uuid: "din-1".to_string(),
                attr_name: "Title".to_string(),
                value: Some("Hallway Switch & Sensor".to_string()),
            },
        )
        .unwrap();
        apply_edit(
            &mut project,
            &ProjectEdit::SetPinDefault {
                control_uuid: "din-1".to_string(),
                pin_key: "Debounce".to_string(),
                default_value: Some("50".to_string()),
            },
        )
        .unwrap();
        apply_edit(
            &mut project,
            &ProjectEdit::SetChildTagAttr {
                control_uuid: "din-1".to_string(),
                child_tag: "IoData".to_string(),
                attr_name: "Pr".to_string(),
                value: Some("room-1".to_string()),
            },
        )
        .unwrap();

        // 2. Edit PageBlock Text, Ref, and child tags Display, LoggerMailer, HCL, PSD
        apply_edit(
            &mut project,
            &ProjectEdit::SetControlAttr {
                control_uuid: "blk-2".to_string(),
                attr_name: "Text".to_string(),
                value: Some("Line 1\nLine 2".to_string()),
            },
        )
        .unwrap();
        apply_edit(
            &mut project,
            &ProjectEdit::SetControlAttr {
                control_uuid: "blk-2".to_string(),
                attr_name: "Ref".to_string(),
                value: Some("din-1".to_string()),
            },
        )
        .unwrap();
        for (tag, attr, val) in [
            ("Display", "Unit", "<v.1>"),
            ("LoggerMailer", "MailText", "Alert"),
            ("HCL", "Enable", "true"),
            ("PSD", "Dmin", "30"),
        ] {
            apply_edit(
                &mut project,
                &ProjectEdit::SetChildTagAttr {
                    control_uuid: "blk-2".to_string(),
                    child_tag: tag.to_string(),
                    attr_name: attr.to_string(),
                    value: Some(val.to_string()),
                },
            )
            .unwrap();
        }

        assert_eq!(
            project.pages[0].blocks[0].text_content.as_deref(),
            Some("Line 1\nLine 2")
        );
        assert_eq!(project.pages[0].blocks[0].ref_uuid.as_deref(), Some("din-1"));
        assert!(!project.xref.usages_for("din-1").is_empty());

        let serialized = serialize_loxone_xml(&project).expect("should serialize");
        let reparsed = parse_loxone_xml(&serialized).expect("should reparse");

        let din = &reparsed.periphery_tree[0].children[0].children[0];
        assert_eq!(din.title, "Hallway Switch & Sensor");
        assert_eq!(din.room_uuid.as_deref(), Some("room-1"));
        assert!(din
            .pins
            .iter()
            .any(|p| p.key == "Debounce" && p.default_value.as_deref() == Some("50")));

        let blk = &reparsed.pages[0].blocks[0];
        assert_eq!(blk.text_content.as_deref(), Some("Line 1\nLine 2"));
        assert_eq!(blk.ref_uuid.as_deref(), Some("din-1"));
        assert!(blk
            .extra_children
            .iter()
            .any(|c| c.tag == "Display" && c.attr("Unit") == Some("<v.1>")));
        assert!(blk
            .extra_children
            .iter()
            .any(|c| c.tag == "LoggerMailer" && c.attr("MailText") == Some("Alert")));
        assert!(blk
            .extra_children
            .iter()
            .any(|c| c.tag == "HCL" && c.attr("Enable") == Some("true")));
        assert!(blk
            .extra_children
            .iter()
            .any(|c| c.tag == "PSD" && c.attr("Dmin") == Some("30")));
    }
}
