use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::model::{
    parse_rgb_color, Category, ConnectorPin, LoxoneProject, Page, PageBlock, PinKind, Place,
    RawNode, TreeNode, TreeSectionKind, WireLink,
};
use crate::parser::{parse_raw_dom, LoxoneError};

#[derive(Debug, Clone, PartialEq)]
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
    MovePageBlock {
        page_uuid: String,
        block_uuid: String,
        px: f32,
        py: f32,
    },
    ResizePageBlock {
        page_uuid: String,
        block_uuid: String,
        px2: f32,
        py2: f32,
    },
    AddPageBlock {
        page_uuid: String,
        block: PageBlock,
    },
    RemovePageBlock {
        page_uuid: String,
        block_uuid: String,
    },
    AddWireLink {
        target_block_uuid: String,
        target_pin_key: String,
        source_pin_uuid: String,
    },
    RemoveWireLink {
        target_block_uuid: String,
        target_pin_key: String,
        source_pin_uuid: String,
    },
    AddPage {
        page: Page,
    },
    RemovePage {
        page_uuid: String,
    },
    AddRoom {
        room: Place,
    },
    RemoveRoom {
        room_uuid: String,
    },
    AddCategory {
        category: Category,
    },
    RemoveCategory {
        category_uuid: String,
    },
    AddTreeNode {
        section: TreeSectionKind,
        parent_uuid: Option<String>,
        node: TreeNode,
    },
    RemoveTreeNode {
        node_uuid: String,
    },
    Batch(Vec<ProjectEdit>),
}

pub fn apply_edit(project: &mut LoxoneProject, edit: &ProjectEdit) -> Result<(), LoxoneError> {
    apply_edit_with_inverse(project, edit).map(|_| ())
}

pub fn apply_edit_with_inverse(
    project: &mut LoxoneProject,
    edit: &ProjectEdit,
) -> Result<ProjectEdit, LoxoneError> {
    let mut needs_xref_rebuild = false;
    let inverse = apply_edit_inner(project, edit, &mut needs_xref_rebuild)?;
    if needs_xref_rebuild {
        project.xref = crate::xref::XrefIndex::build(project);
    }
    Ok(inverse)
}

fn format_coord_attr(v: f32) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i32)
    } else {
        format!("{}", v.round() as i32)
    }
}

fn get_vec_attr(attributes: &[(String, String)], name: &str) -> Option<String> {
    attributes
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.clone())
}

fn apply_edit_inner(
    project: &mut LoxoneProject,
    edit: &ProjectEdit,
    needs_xref_rebuild: &mut bool,
) -> Result<ProjectEdit, LoxoneError> {
    match edit {
        ProjectEdit::SetControlAttr {
            control_uuid,
            attr_name,
            value,
        } => {
            let mut old_value = None;
            if let Some(block) = find_page_block_mut(project, control_uuid) {
                old_value = get_vec_attr(&block.raw_attributes, attr_name);
                set_vec_attr(&mut block.raw_attributes, attr_name, value.as_deref());
                sync_page_block_attr(block, attr_name, value.as_deref());
                if matches!(
                    attr_name.as_str(),
                    "Title" | "Ref" | "UO" | "SpStates" | "rec"
                ) {
                    *needs_xref_rebuild = true;
                }
            } else if let Some(tree_node) = find_tree_node_mut(project, control_uuid) {
                old_value = get_vec_attr(&tree_node.raw_attributes, attr_name);
                set_vec_attr(&mut tree_node.raw_attributes, attr_name, value.as_deref());
                sync_tree_node_attr(tree_node, attr_name, value.as_deref());
                if attr_name == "Title" {
                    *needs_xref_rebuild = true;
                }
            } else if let Some(cat) = project.categories.get_mut(control_uuid) {
                match attr_name.as_str() {
                    "Title" => {
                        old_value = Some(cat.title.clone());
                        cat.title = value.clone().unwrap_or_default();
                        *needs_xref_rebuild = true;
                    }
                    "BkColor" | "Cl" => {
                        old_value = cat.bk_color.map(|[r, g, b]| format!("{},{},{}", r, g, b));
                        cat.bk_color = value.as_deref().and_then(parse_rgb_color);
                    }
                    "CatGroup" => {
                        old_value = cat.cat_group.clone();
                        cat.cat_group = value.clone();
                    }
                    "Icon" => {
                        old_value = cat.icon.clone();
                        cat.icon = value.clone();
                    }
                    _ => {}
                }
            } else if let Some(place) = project.places.get_mut(control_uuid) {
                match attr_name.as_str() {
                    "Title" => {
                        old_value = Some(place.title.clone());
                        place.title = value.clone().unwrap_or_default();
                        *needs_xref_rebuild = true;
                    }
                    "Sqm" => {
                        old_value = place.sqm.clone();
                        place.sqm = value.clone();
                    }
                    "PType" => {
                        old_value = place.p_type.clone();
                        place.p_type = value.clone();
                    }
                    "Icon" => {
                        old_value = place.icon.clone();
                        place.icon = value.clone();
                    }
                    _ => {}
                }
            } else if let Some(page) = project.pages.iter_mut().find(|p| p.uuid == *control_uuid) {
                if attr_name == "Title" {
                    old_value = Some(page.title.clone());
                    page.title = value.clone().unwrap_or_default();
                    *needs_xref_rebuild = true;
                }
            }
            Ok(ProjectEdit::SetControlAttr {
                control_uuid: control_uuid.clone(),
                attr_name: attr_name.clone(),
                value: old_value,
            })
        }
        ProjectEdit::SetPinDefault {
            control_uuid,
            pin_key,
            default_value,
        } => {
            let mut old_default = None;
            if let Some(block) = find_page_block_mut(project, control_uuid) {
                old_default = block
                    .pins
                    .iter()
                    .find(|p| p.key == *pin_key)
                    .and_then(|p| p.default_value.clone());
                if set_pin_default_on_slice(&mut block.pins, control_uuid, pin_key, default_value) {
                    *needs_xref_rebuild = true;
                }
            } else if let Some(tree_node) = find_tree_node_mut(project, control_uuid) {
                old_default = tree_node
                    .pins
                    .iter()
                    .find(|p| p.key == *pin_key)
                    .and_then(|p| p.default_value.clone());
                if set_pin_default_on_slice(
                    &mut tree_node.pins,
                    control_uuid,
                    pin_key,
                    default_value,
                ) {
                    *needs_xref_rebuild = true;
                }
            }
            Ok(ProjectEdit::SetPinDefault {
                control_uuid: control_uuid.clone(),
                pin_key: pin_key.clone(),
                default_value: old_default,
            })
        }
        ProjectEdit::SetChildTagAttr {
            control_uuid,
            child_tag,
            attr_name,
            value,
        } => {
            let mut old_value = None;
            if let Some(block) = find_page_block_mut(project, control_uuid) {
                old_value = block
                    .extra_children
                    .iter()
                    .find(|c| c.tag == *child_tag)
                    .and_then(|c| c.attr(attr_name))
                    .map(str::to_string);
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
                    *needs_xref_rebuild = true;
                } else if child_tag == "LoggerMailer" && attr_name == "RefCaller" {
                    let has_ref_attr = block.raw_attributes.iter().any(|(k, _)| k == "Ref");
                    if !has_ref_attr {
                        block.ref_uuid = value.clone();
                        *needs_xref_rebuild = true;
                    }
                }
            } else if let Some(tree_node) = find_tree_node_mut(project, control_uuid) {
                old_value = tree_node
                    .extra_children
                    .iter()
                    .find(|c| c.tag == *child_tag)
                    .and_then(|c| c.attr(attr_name))
                    .map(str::to_string);
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
                    *needs_xref_rebuild = true;
                }
            } else if let Some(page) = project.pages.iter_mut().find(|p| p.uuid == *control_uuid) {
                if child_tag == "IoData" {
                    if attr_name == "Pr" {
                        old_value = page.room_uuid.clone();
                        page.room_uuid = value.clone();
                    } else if attr_name == "Cr" {
                        old_value = page.category_uuid.clone();
                        page.category_uuid = value.clone();
                    }
                    *needs_xref_rebuild = true;
                }
            }
            Ok(ProjectEdit::SetChildTagAttr {
                control_uuid: control_uuid.clone(),
                child_tag: child_tag.clone(),
                attr_name: attr_name.clone(),
                value: old_value,
            })
        }
        ProjectEdit::MovePageBlock {
            page_uuid,
            block_uuid,
            px,
            py,
        } => {
            let mut old_px = *px;
            let mut old_py = *py;
            let loc = project
                .pages
                .iter()
                .position(|p| p.uuid == *page_uuid)
                .and_then(|pi| {
                    project.pages[pi]
                        .blocks
                        .iter()
                        .position(|b| b.uuid == *block_uuid)
                        .map(|bi| (pi, bi))
                });
            let block_opt = if let Some((pi, bi)) = loc {
                Some(&mut project.pages[pi].blocks[bi])
            } else {
                find_page_block_mut(project, block_uuid)
            };
            if let Some(block) = block_opt {
                old_px = block.px;
                old_py = block.py;
                let dx = *px - old_px;
                let dy = *py - old_py;
                block.px = *px;
                block.py = *py;
                block.px2 += dx;
                block.py2 += dy;
                block.has_position = true;
                let px_str = format_coord_attr(block.px);
                let py_str = format_coord_attr(block.py);
                let px2_str = format_coord_attr(block.px2);
                let py2_str = format_coord_attr(block.py2);
                set_vec_attr(&mut block.raw_attributes, "Px", Some(&px_str));
                set_vec_attr(&mut block.raw_attributes, "Py", Some(&py_str));
                set_vec_attr(&mut block.raw_attributes, "Px2", Some(&px2_str));
                set_vec_attr(&mut block.raw_attributes, "Py2", Some(&py2_str));
            }
            Ok(ProjectEdit::MovePageBlock {
                page_uuid: page_uuid.clone(),
                block_uuid: block_uuid.clone(),
                px: old_px,
                py: old_py,
            })
        }
        ProjectEdit::ResizePageBlock {
            page_uuid,
            block_uuid,
            px2,
            py2,
        } => {
            let mut old_px2 = *px2;
            let mut old_py2 = *py2;
            let loc = project
                .pages
                .iter()
                .position(|p| p.uuid == *page_uuid)
                .and_then(|pi| {
                    project.pages[pi]
                        .blocks
                        .iter()
                        .position(|b| b.uuid == *block_uuid)
                        .map(|bi| (pi, bi))
                });
            let block_opt = if let Some((pi, bi)) = loc {
                Some(&mut project.pages[pi].blocks[bi])
            } else {
                find_page_block_mut(project, block_uuid)
            };
            if let Some(block) = block_opt {
                old_px2 = block.px2;
                old_py2 = block.py2;
                block.px2 = (*px2).max(block.px + 64.0);
                block.py2 = (*py2).max(block.py + 32.0);
                let px2_str = format_coord_attr(block.px2);
                let py2_str = format_coord_attr(block.py2);
                set_vec_attr(&mut block.raw_attributes, "Px2", Some(&px2_str));
                set_vec_attr(&mut block.raw_attributes, "Py2", Some(&py2_str));
            }
            Ok(ProjectEdit::ResizePageBlock {
                page_uuid: page_uuid.clone(),
                block_uuid: block_uuid.clone(),
                px2: old_px2,
                py2: old_py2,
            })
        }
        ProjectEdit::AddPageBlock { page_uuid, block } => {
            if let Some(page) = project.pages.iter_mut().find(|p| p.uuid == *page_uuid) {
                if !page.blocks.iter().any(|b| b.uuid == block.uuid) {
                    page.blocks.push(block.clone());
                    *needs_xref_rebuild = true;
                }
            }
            Ok(ProjectEdit::RemovePageBlock {
                page_uuid: page_uuid.clone(),
                block_uuid: block.uuid.clone(),
            })
        }
        ProjectEdit::RemovePageBlock {
            page_uuid,
            block_uuid,
        } => {
            let mut removed_block_opt: Option<(String, PageBlock)> = None;
            if let Some(page) = project.pages.iter_mut().find(|p| p.uuid == *page_uuid) {
                if let Some(pos) = page.blocks.iter().position(|b| b.uuid == *block_uuid) {
                    removed_block_opt = Some((page.uuid.clone(), page.blocks.remove(pos)));
                }
            }
            if removed_block_opt.is_none() {
                for page in &mut project.pages {
                    if let Some(pos) = page.blocks.iter().position(|b| b.uuid == *block_uuid) {
                        removed_block_opt = Some((page.uuid.clone(), page.blocks.remove(pos)));
                        break;
                    }
                }
            }

            if let Some((actual_page_uuid, removed_block)) = removed_block_opt {
                *needs_xref_rebuild = true;
                let removed_pin_uuids: HashSet<String> = removed_block
                    .pins
                    .iter()
                    .filter(|p| !p.uuid.is_empty())
                    .map(|p| p.uuid.clone())
                    .collect();

                let mut wire_restores = Vec::new();
                if !removed_pin_uuids.is_empty() {
                    for page in &mut project.pages {
                        for blk in &mut page.blocks {
                            for pin in &mut blk.pins {
                                for link in &pin.incoming_links {
                                    if removed_pin_uuids.contains(&link.source_pin_uuid) {
                                        wire_restores.push(ProjectEdit::AddWireLink {
                                            target_block_uuid: blk.uuid.clone(),
                                            target_pin_key: pin.key.clone(),
                                            source_pin_uuid: link.source_pin_uuid.clone(),
                                        });
                                    }
                                }
                                pin.incoming_links
                                    .retain(|l| !removed_pin_uuids.contains(&l.source_pin_uuid));
                            }
                        }
                    }
                }

                let add_block_edit = ProjectEdit::AddPageBlock {
                    page_uuid: actual_page_uuid,
                    block: removed_block,
                };
                if wire_restores.is_empty() {
                    Ok(add_block_edit)
                } else {
                    let mut batch = Vec::with_capacity(1 + wire_restores.len());
                    batch.push(add_block_edit);
                    batch.extend(wire_restores);
                    Ok(ProjectEdit::Batch(batch))
                }
            } else {
                Ok(ProjectEdit::Batch(Vec::new()))
            }
        }
        ProjectEdit::AddWireLink {
            target_block_uuid,
            target_pin_key,
            source_pin_uuid,
        } => {
            if let Some(block) = find_page_block_mut(project, target_block_uuid) {
                if !block.pins.iter().any(|p| p.key == *target_pin_key) {
                    crate::factory::ensure_schema_pins_on_block(block);
                }
                if !block.pins.iter().any(|p| p.key == *target_pin_key) {
                    block.pins.push(ConnectorPin {
                        uuid: crate::factory::generate_loxone_uuid(),
                        key: target_pin_key.clone(),
                        default_value: None,
                        pin_kind: PinKind::Input,
                        incoming_links: Vec::new(),
                    });
                }
                let is_hidden_link = !block.has_position
                    || target_pin_key.starts_with("HI")
                    || target_pin_key.starts_with("InputAir");
                if let Some(pin) = block.pins.iter_mut().find(|p| p.key == *target_pin_key) {
                    if pin.pin_kind == PinKind::Parameter {
                        pin.pin_kind = PinKind::Input;
                    }
                    if !pin
                        .incoming_links
                        .iter()
                        .any(|l| l.source_pin_uuid == *source_pin_uuid)
                    {
                        pin.incoming_links.push(WireLink {
                            source_pin_uuid: source_pin_uuid.clone(),
                            flags: if is_hidden_link {
                                Some("1".to_string())
                            } else {
                                None
                            },
                        });
                    }
                }
                *needs_xref_rebuild = true;
            } else if let Some(tree_node) = find_tree_node_mut(project, target_block_uuid) {
                if !tree_node.pins.iter().any(|p| p.key == *target_pin_key) {
                    tree_node.pins.push(ConnectorPin {
                        uuid: crate::factory::generate_loxone_uuid(),
                        key: target_pin_key.clone(),
                        default_value: None,
                        pin_kind: PinKind::Input,
                        incoming_links: Vec::new(),
                    });
                }
                if let Some(pin) = tree_node.pins.iter_mut().find(|p| p.key == *target_pin_key) {
                    if !pin
                        .incoming_links
                        .iter()
                        .any(|l| l.source_pin_uuid == *source_pin_uuid)
                    {
                        pin.incoming_links.push(WireLink {
                            source_pin_uuid: source_pin_uuid.clone(),
                            flags: None,
                        });
                    }
                }
                *needs_xref_rebuild = true;
            }
            Ok(ProjectEdit::RemoveWireLink {
                target_block_uuid: target_block_uuid.clone(),
                target_pin_key: target_pin_key.clone(),
                source_pin_uuid: source_pin_uuid.clone(),
            })
        }
        ProjectEdit::RemoveWireLink {
            target_block_uuid,
            target_pin_key,
            source_pin_uuid,
        } => {
            if let Some(block) = find_page_block_mut(project, target_block_uuid) {
                if let Some(pin) = block.pins.iter_mut().find(|p| p.key == *target_pin_key) {
                    pin.incoming_links
                        .retain(|l| l.source_pin_uuid != *source_pin_uuid);
                    *needs_xref_rebuild = true;
                }
            } else if let Some(tree_node) = find_tree_node_mut(project, target_block_uuid) {
                if let Some(pin) = tree_node.pins.iter_mut().find(|p| p.key == *target_pin_key) {
                    pin.incoming_links
                        .retain(|l| l.source_pin_uuid != *source_pin_uuid);
                    *needs_xref_rebuild = true;
                }
            }
            Ok(ProjectEdit::AddWireLink {
                target_block_uuid: target_block_uuid.clone(),
                target_pin_key: target_pin_key.clone(),
                source_pin_uuid: source_pin_uuid.clone(),
            })
        }
        ProjectEdit::AddPage { page } => {
            if !project.pages.iter().any(|p| p.uuid == page.uuid) {
                project.pages.push(page.clone());
                *needs_xref_rebuild = true;
            }
            Ok(ProjectEdit::RemovePage {
                page_uuid: page.uuid.clone(),
            })
        }
        ProjectEdit::RemovePage { page_uuid } => {
            if let Some(pos) = project.pages.iter().position(|p| p.uuid == *page_uuid) {
                let removed = project.pages.remove(pos);
                *needs_xref_rebuild = true;
                Ok(ProjectEdit::AddPage { page: removed })
            } else {
                Ok(ProjectEdit::Batch(Vec::new()))
            }
        }
        ProjectEdit::AddRoom { room } => {
            project.places.insert(room.uuid.clone(), room.clone());
            *needs_xref_rebuild = true;
            Ok(ProjectEdit::RemoveRoom {
                room_uuid: room.uuid.clone(),
            })
        }
        ProjectEdit::RemoveRoom { room_uuid } => {
            if let Some(removed) = project.places.shift_remove(room_uuid) {
                *needs_xref_rebuild = true;
                Ok(ProjectEdit::AddRoom { room: removed })
            } else {
                Ok(ProjectEdit::Batch(Vec::new()))
            }
        }
        ProjectEdit::AddCategory { category } => {
            project
                .categories
                .insert(category.uuid.clone(), category.clone());
            *needs_xref_rebuild = true;
            Ok(ProjectEdit::RemoveCategory {
                category_uuid: category.uuid.clone(),
            })
        }
        ProjectEdit::RemoveCategory { category_uuid } => {
            if let Some(removed) = project.categories.shift_remove(category_uuid) {
                *needs_xref_rebuild = true;
                Ok(ProjectEdit::AddCategory { category: removed })
            } else {
                Ok(ProjectEdit::Batch(Vec::new()))
            }
        }
        ProjectEdit::AddTreeNode {
            section,
            parent_uuid,
            node,
        } => {
            let mut added = false;
            if let Some(p_uuid) = parent_uuid {
                if let Some(parent) = find_tree_node_mut(project, p_uuid) {
                    if !parent.children.iter().any(|c| c.uuid == node.uuid) {
                        let mut child = node.clone();
                        child.section = parent.section;
                        parent.children.push(child);
                    }
                    added = true;
                }
            }
            if !added {
                let root_vec = match section {
                    TreeSectionKind::Periphery => &mut project.periphery_tree,
                    TreeSectionKind::System => &mut project.system_tree,
                };
                if !root_vec.iter().any(|n| n.uuid == node.uuid) {
                    let mut n = node.clone();
                    n.section = *section;
                    root_vec.push(n);
                }
            }
            *needs_xref_rebuild = true;
            Ok(ProjectEdit::RemoveTreeNode {
                node_uuid: node.uuid.clone(),
            })
        }
        ProjectEdit::RemoveTreeNode { node_uuid } => {
            let removed = remove_from_tree_vec(
                &mut project.periphery_tree,
                TreeSectionKind::Periphery,
                node_uuid,
            )
            .or_else(|| {
                remove_from_tree_vec(&mut project.system_tree, TreeSectionKind::System, node_uuid)
            });
            if let Some((section, parent_uuid, node)) = removed {
                *needs_xref_rebuild = true;
                Ok(ProjectEdit::AddTreeNode {
                    section,
                    parent_uuid,
                    node,
                })
            } else {
                Ok(ProjectEdit::Batch(Vec::new()))
            }
        }
        ProjectEdit::Batch(edits) => {
            let mut inverses = Vec::with_capacity(edits.len());
            for sub_edit in edits {
                let inv = apply_edit_inner(project, sub_edit, needs_xref_rebuild)?;
                inverses.push(inv);
            }
            inverses.reverse();
            Ok(ProjectEdit::Batch(inverses))
        }
    }
}

fn remove_from_tree_vec(
    nodes: &mut Vec<TreeNode>,
    section: TreeSectionKind,
    target_uuid: &str,
) -> Option<(TreeSectionKind, Option<String>, TreeNode)> {
    if let Some(pos) = nodes.iter().position(|n| n.uuid == target_uuid) {
        let removed = nodes.remove(pos);
        return Some((section, None, removed));
    }
    for node in nodes {
        if let Some(found) = remove_from_tree_node_children(node, target_uuid) {
            return Some(found);
        }
    }
    None
}

fn remove_from_tree_node_children(
    parent: &mut TreeNode,
    target_uuid: &str,
) -> Option<(TreeSectionKind, Option<String>, TreeNode)> {
    if let Some(pos) = parent.children.iter().position(|c| c.uuid == target_uuid) {
        let removed = parent.children.remove(pos);
        return Some((parent.section, Some(parent.uuid.clone()), removed));
    }
    for child in &mut parent.children {
        if let Some(found) = remove_from_tree_node_children(child, target_uuid) {
            return Some(found);
        }
    }
    None
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

/// Deep-clones a slice of `PageBlock`s for Copy/Paste or Duplicate (`Cmd+C`/`Cmd+V`/`Cmd+D`),
/// allocating fresh block and pin UUIDs, offsetting coordinates by `(offset_px, offset_py)`,
/// and remapping internal `<In Input="..."/>` wire links between the copied blocks while
/// stripping external incoming wires.
pub fn clone_blocks_for_paste(
    blocks: &[PageBlock],
    offset_px: f32,
    offset_py: f32,
) -> Vec<PageBlock> {
    let mut pin_uuid_map: HashMap<String, String> = HashMap::new();
    for block in blocks {
        for pin in &block.pins {
            if !pin.uuid.is_empty() {
                pin_uuid_map.insert(pin.uuid.clone(), crate::factory::generate_loxone_uuid());
            }
        }
    }

    blocks
        .iter()
        .map(|orig| {
            let mut cloned = orig.clone();
            let new_uuid = crate::factory::generate_loxone_uuid();
            cloned.uuid = new_uuid.clone();
            cloned.px += offset_px;
            cloned.py += offset_py;
            cloned.px2 += offset_px;
            cloned.py2 += offset_py;
            cloned.has_position = true;

            let px_str = format_coord_attr(cloned.px);
            let py_str = format_coord_attr(cloned.py);
            let px2_str = format_coord_attr(cloned.px2);
            let py2_str = format_coord_attr(cloned.py2);
            set_vec_attr(&mut cloned.raw_attributes, "U", Some(&new_uuid));
            set_vec_attr(&mut cloned.raw_attributes, "Px", Some(&px_str));
            set_vec_attr(&mut cloned.raw_attributes, "Py", Some(&py_str));
            set_vec_attr(&mut cloned.raw_attributes, "Px2", Some(&px2_str));
            set_vec_attr(&mut cloned.raw_attributes, "Py2", Some(&py2_str));

            for pin in &mut cloned.pins {
                pin.uuid = pin_uuid_map
                    .get(&pin.uuid)
                    .cloned()
                    .unwrap_or_else(crate::factory::generate_loxone_uuid);
                pin.incoming_links = pin
                    .incoming_links
                    .iter()
                    .filter_map(|link| {
                        pin_uuid_map.get(&link.source_pin_uuid).map(|new_src| WireLink {
                            source_pin_uuid: new_src.clone(),
                            flags: link.flags.clone(),
                        })
                    })
                    .collect();
            }

            cloned
        })
        .collect()
}

/// Builds a single atomic `ProjectEdit::Batch` that sends an output pin of `source_block_uuid`
/// on `source_page_uuid` to `target_page_uuid` via a paired Flag (`Memory`) reference block:
/// 1. Creates a `Memory` tree node under `MemoryCaption` in the system tree.
/// 2. Places a writer `Memory` reference block next to `source_block_uuid` on `source_page_uuid`
///    and wires `source_pin_key` into its `"I"` input.
/// 3. Places a reader `Memory` reference block on `target_page_uuid` at `(target_px, target_py)`.
pub fn create_cross_page_link_edits(
    project: &LoxoneProject,
    source_page_uuid: &str,
    source_block_uuid: &str,
    source_pin_key: &str,
    target_page_uuid: &str,
    target_px: f32,
    target_py: f32,
) -> Option<ProjectEdit> {
    let source_page = project.pages.iter().find(|p| p.uuid == source_page_uuid)?;
    let source_block = source_page
        .blocks
        .iter()
        .find(|b| b.uuid == source_block_uuid)?;
    let source_pin = source_block
        .pins
        .iter()
        .find(|p| p.key == source_pin_key && !p.uuid.is_empty())?;

    let flag_title = if source_block.title.is_empty() {
        format!("{} ({})", source_block.block_type, source_pin_key)
    } else {
        format!("{} ({})", source_block.title, source_pin_key)
    };
    let merker_node = crate::factory::new_tree_node("Memory", &flag_title);
    let mem_caption_uuid = project
        .system_tree
        .iter()
        .find(|n| n.control_type == "MemoryCaption")
        .map(|n| n.uuid.clone());

    let mut writer_ref = crate::factory::new_reference_block(
        "Memory",
        &merker_node,
        source_block.px2 + 128.0,
        source_block.py,
    );
    for p in &mut writer_ref.pins {
        p.uuid = crate::factory::generate_loxone_uuid();
    }
    let mut reader_ref =
        crate::factory::new_reference_block("Memory", &merker_node, target_px, target_py);
    for p in &mut reader_ref.pins {
        p.uuid = crate::factory::generate_loxone_uuid();
    }

    let writer_uuid = writer_ref.uuid.clone();
    Some(ProjectEdit::Batch(vec![
        ProjectEdit::AddTreeNode {
            section: TreeSectionKind::System,
            parent_uuid: mem_caption_uuid,
            node: merker_node,
        },
        ProjectEdit::AddPageBlock {
            page_uuid: source_page_uuid.to_string(),
            block: writer_ref,
        },
        ProjectEdit::AddWireLink {
            target_block_uuid: writer_uuid,
            target_pin_key: "I".to_string(),
            source_pin_uuid: source_pin.uuid.clone(),
        },
        ProjectEdit::AddPageBlock {
            page_uuid: target_page_uuid.to_string(),
            block: reader_ref,
        },
    ]))
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

fn extract_io_data_from_children(children: &[RawNode]) -> (Option<&str>, Option<&str>) {
    for child in children {
        if child.tag == "IoData" {
            return (child.attr("Pr"), child.attr("Cr"));
        }
    }
    (None, None)
}

fn sync_raw_node(
    node: &mut RawNode,
    project: &LoxoneProject,
    blocks_by_uuid: &HashMap<&str, &PageBlock>,
    tree_nodes_by_uuid: &HashMap<&str, &TreeNode>,
    pages_by_uuid: &HashMap<&str, &Page>,
) {
    if node.tag == "C" {
        let node_type = node.attr("Type").unwrap_or_default().to_string();
        match node_type.as_str() {
            "Document" => {
                if !project.metadata.c_date.is_empty() {
                    node.set_attr("CDate", Some(&project.metadata.c_date));
                }
                if let Some(ds) = project.metadata.date_s {
                    let ds_str = ds.to_string();
                    node.set_attr("DateS", Some(&ds_str));
                }
                if let Some(sl) = project.metadata.save_level {
                    let sl_str = sl.to_string();
                    node.set_attr("SL", Some(&sl_str));
                }
                if !project.metadata.config_version.is_empty() {
                    node.set_attr("ConfigVersion", Some(&project.metadata.config_version));
                }
                node.children.retain(|c| {
                    if c.tag != "C" {
                        return true;
                    }
                    if matches!(
                        c.attr("Type"),
                        Some("CategoryCaption" | "PlaceCaption" | "Program")
                    ) {
                        return true;
                    }
                    c.attr("U")
                        .map(|u| u.is_empty() || tree_nodes_by_uuid.contains_key(u))
                        .unwrap_or(true)
                });
                for child in &mut node.children {
                    sync_raw_node(
                        child,
                        project,
                        blocks_by_uuid,
                        tree_nodes_by_uuid,
                        pages_by_uuid,
                    );
                }
                let seen_tree_uuids: HashSet<String> = node
                    .children
                    .iter()
                    .filter(|c| c.tag == "C")
                    .filter_map(|c| c.attr("U").map(str::to_string))
                    .collect();
                for root_tree in project
                    .periphery_tree
                    .iter()
                    .chain(project.system_tree.iter())
                {
                    if !root_tree.uuid.is_empty() && !seen_tree_uuids.contains(&root_tree.uuid) {
                        node.children.push(tree_node_to_raw_node(root_tree));
                    }
                }
                return;
            }
            "CategoryCaption" => {
                node.children.retain(|c| {
                    c.tag != "C"
                        || c.attr("Type") != Some("Category")
                        || c.attr("U")
                            .is_some_and(|u| project.categories.contains_key(u))
                });
                for child in &mut node.children {
                    sync_raw_node(
                        child,
                        project,
                        blocks_by_uuid,
                        tree_nodes_by_uuid,
                        pages_by_uuid,
                    );
                }
                let seen_cats: HashSet<String> = node
                    .children
                    .iter()
                    .filter(|c| c.tag == "C" && c.attr("Type") == Some("Category"))
                    .filter_map(|c| c.attr("U").map(str::to_string))
                    .collect();
                for (cat_uuid, cat) in &project.categories {
                    if !cat_uuid.is_empty() && !seen_cats.contains(cat_uuid) {
                        node.children.push(category_to_raw_node(cat));
                    }
                }
                return;
            }
            "PlaceCaption" => {
                node.children.retain(|c| {
                    c.tag != "C"
                        || c.attr("Type") != Some("Place")
                        || c.attr("U").is_some_and(|u| project.places.contains_key(u))
                });
                for child in &mut node.children {
                    sync_raw_node(
                        child,
                        project,
                        blocks_by_uuid,
                        tree_nodes_by_uuid,
                        pages_by_uuid,
                    );
                }
                let seen_places: HashSet<String> = node
                    .children
                    .iter()
                    .filter(|c| c.tag == "C" && c.attr("Type") == Some("Place"))
                    .filter_map(|c| c.attr("U").map(str::to_string))
                    .collect();
                for (place_uuid, place) in &project.places {
                    if !place_uuid.is_empty() && !seen_places.contains(place_uuid) {
                        node.children.push(place_to_raw_node(place));
                    }
                }
                return;
            }
            "Program" => {
                node.children.retain(|c| {
                    c.tag != "C"
                        || c.attr("Type") != Some("Page")
                        || c.attr("U").is_some_and(|u| pages_by_uuid.contains_key(u))
                });
                for child in &mut node.children {
                    sync_raw_node(
                        child,
                        project,
                        blocks_by_uuid,
                        tree_nodes_by_uuid,
                        pages_by_uuid,
                    );
                }
                let seen_pages: HashSet<String> = node
                    .children
                    .iter()
                    .filter(|c| c.tag == "C" && c.attr("Type") == Some("Page"))
                    .filter_map(|c| c.attr("U").map(str::to_string))
                    .collect();
                for page in &project.pages {
                    if !page.uuid.is_empty() && !seen_pages.contains(&page.uuid) {
                        node.children.push(page_to_raw_node(page));
                    }
                }
                return;
            }
            _ => {}
        }

        let uuid = node.attr("U").unwrap_or_default().to_string();
        if !uuid.is_empty() {
            if let Some(block) = blocks_by_uuid.get(uuid.as_str()) {
                node.attributes = block.raw_attributes.clone();
                let mut new_children = sync_co_children(&node.children, &block.pins);
                new_children.extend(block.extra_children.iter().cloned());
                if (block.room_uuid.is_some() || block.category_uuid.is_some())
                    && !new_children.iter().any(|c| c.tag == "IoData")
                {
                    let mut io = RawNode {
                        tag: "IoData".to_string(),
                        attributes: Vec::new(),
                        text: None,
                        children: Vec::new(),
                    };
                    io.set_attr("Cr", block.category_uuid.as_deref());
                    io.set_attr("Pr", block.room_uuid.as_deref());
                    new_children.push(io);
                }
                node.children = new_children;
                return;
            } else if let Some(tree_node) = tree_nodes_by_uuid.get(uuid.as_str()) {
                node.attributes = tree_node.raw_attributes.clone();
                let mut new_children = sync_co_children(&node.children, &tree_node.pins);
                new_children.extend(tree_node.extra_children.iter().cloned());
                let valid_child_uuids: HashSet<&str> = tree_node
                    .children
                    .iter()
                    .map(|c| c.uuid.as_str())
                    .collect();
                let mut seen_child_uuids: HashSet<String> = HashSet::new();
                for mut c_child in node
                    .children
                    .iter()
                    .filter(|c| c.tag == "C")
                    .cloned()
                    .collect::<Vec<_>>()
                {
                    let c_uuid = c_child.attr("U").unwrap_or_default().to_string();
                    if c_uuid.is_empty() || valid_child_uuids.contains(c_uuid.as_str()) {
                        if !c_uuid.is_empty() {
                            seen_child_uuids.insert(c_uuid);
                        }
                        sync_raw_node(
                            &mut c_child,
                            project,
                            blocks_by_uuid,
                            tree_nodes_by_uuid,
                            pages_by_uuid,
                        );
                        new_children.push(c_child);
                    }
                }
                for child in &tree_node.children {
                    if !child.uuid.is_empty() && !seen_child_uuids.contains(&child.uuid) {
                        new_children.push(tree_node_to_raw_node(child));
                    }
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
                let (cur_room, cur_cat) = extract_io_data_from_children(&node.children);
                if cur_room != page.room_uuid.as_deref()
                    || cur_cat != page.category_uuid.as_deref()
                {
                    set_child_tag_attr_on_vec(
                        &mut node.children,
                        "IoData",
                        "Cr",
                        page.category_uuid.as_deref(),
                    );
                    set_child_tag_attr_on_vec(
                        &mut node.children,
                        "IoData",
                        "Pr",
                        page.room_uuid.as_deref(),
                    );
                }
                let valid_block_uuids: HashSet<&str> =
                    page.blocks.iter().map(|b| b.uuid.as_str()).collect();
                node.children.retain(|c| {
                    c.tag != "C"
                        || c.attr("U")
                            .map(|u| u.is_empty() || valid_block_uuids.contains(u))
                            .unwrap_or(true)
                });
                for child in &mut node.children {
                    sync_raw_node(
                        child,
                        project,
                        blocks_by_uuid,
                        tree_nodes_by_uuid,
                        pages_by_uuid,
                    );
                }
                let seen_blocks: HashSet<String> = node
                    .children
                    .iter()
                    .filter(|c| c.tag == "C")
                    .filter_map(|c| c.attr("U").map(str::to_string))
                    .collect();
                for block in &page.blocks {
                    if !block.uuid.is_empty() && !seen_blocks.contains(&block.uuid) {
                        node.children.push(page_block_to_raw_node(block));
                    }
                }
                return;
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

fn in_children_match(co: &RawNode, links: &[WireLink]) -> bool {
    let existing_ins: Vec<(&str, Option<&str>)> = co
        .children
        .iter()
        .filter(|c| c.tag == "In")
        .filter_map(|c| c.attr("Input").map(|inp| (inp, c.attr("FLG"))))
        .collect();
    if existing_ins.len() != links.len() {
        return false;
    }
    existing_ins
        .iter()
        .zip(links.iter())
        .all(|((inp, flg), link)| *inp == link.source_pin_uuid && *flg == link.flags.as_deref())
}

fn sync_co_children(existing_children: &[RawNode], pins: &[ConnectorPin]) -> Vec<RawNode> {
    let mut co_nodes: Vec<RawNode> = existing_children
        .iter()
        .filter(|c| c.tag == "Co")
        .cloned()
        .collect();

    for pin in pins {
        if let Some(co) = co_nodes.iter_mut().find(|c| {
            (!pin.uuid.is_empty() && c.attr("U") == Some(pin.uuid.as_str()))
                || c.attr("K") == Some(pin.key.as_str())
        }) {
            co.set_attr("Def", pin.default_value.as_deref());
            if !in_children_match(co, &pin.incoming_links) {
                if pin.incoming_links.is_empty() {
                    co.set_attr("Nc", None);
                } else {
                    let nc_str = pin.incoming_links.len().to_string();
                    co.set_attr("Nc", Some(&nc_str));
                }
                co.children.retain(|c| c.tag != "In");
                for link in &pin.incoming_links {
                    let mut in_attrs = vec![("Input".to_string(), link.source_pin_uuid.clone())];
                    if let Some(flg) = &link.flags {
                        in_attrs.push(("FLG".to_string(), flg.clone()));
                    }
                    co.children.push(RawNode {
                        tag: "In".to_string(),
                        attributes: in_attrs,
                        text: None,
                        children: Vec::new(),
                    });
                }
            }
        } else {
            co_nodes.push(pin_to_raw_node(pin));
        }
    }

    co_nodes
}

fn pin_to_raw_node(pin: &ConnectorPin) -> RawNode {
    let mut co = RawNode {
        tag: "Co".to_string(),
        attributes: vec![
            ("K".to_string(), pin.key.clone()),
            ("U".to_string(), pin.uuid.clone()),
        ],
        text: None,
        children: Vec::new(),
    };
    co.set_attr("Def", pin.default_value.as_deref());
    if !pin.incoming_links.is_empty() {
        let nc_str = pin.incoming_links.len().to_string();
        co.set_attr("Nc", Some(&nc_str));
        for link in &pin.incoming_links {
            let mut in_attrs = vec![("Input".to_string(), link.source_pin_uuid.clone())];
            if let Some(flg) = &link.flags {
                in_attrs.push(("FLG".to_string(), flg.clone()));
            }
            co.children.push(RawNode {
                tag: "In".to_string(),
                attributes: in_attrs,
                text: None,
                children: Vec::new(),
            });
        }
    }
    co
}

fn page_block_to_raw_node(block: &PageBlock) -> RawNode {
    let mut children: Vec<RawNode> = block.pins.iter().map(pin_to_raw_node).collect();
    children.extend(block.extra_children.iter().cloned());
    if (block.room_uuid.is_some() || block.category_uuid.is_some())
        && !children.iter().any(|c| c.tag == "IoData")
    {
        let mut io = RawNode {
            tag: "IoData".to_string(),
            attributes: Vec::new(),
            text: None,
            children: Vec::new(),
        };
        io.set_attr("Cr", block.category_uuid.as_deref());
        io.set_attr("Pr", block.room_uuid.as_deref());
        children.push(io);
    }
    RawNode {
        tag: "C".to_string(),
        attributes: block.raw_attributes.clone(),
        text: None,
        children,
    }
}

fn page_to_raw_node(page: &Page) -> RawNode {
    let mut children = Vec::new();
    if page.room_uuid.is_some() || page.category_uuid.is_some() {
        let mut io = RawNode {
            tag: "IoData".to_string(),
            attributes: Vec::new(),
            text: None,
            children: Vec::new(),
        };
        io.set_attr("Cr", page.category_uuid.as_deref());
        io.set_attr("Pr", page.room_uuid.as_deref());
        children.push(io);
    }
    for block in &page.blocks {
        children.push(page_block_to_raw_node(block));
    }
    RawNode {
        tag: "C".to_string(),
        attributes: vec![
            ("Type".to_string(), "Page".to_string()),
            ("Title".to_string(), page.title.clone()),
            ("U".to_string(), page.uuid.clone()),
        ],
        text: None,
        children,
    }
}

fn place_to_raw_node(place: &Place) -> RawNode {
    let mut node = RawNode {
        tag: "C".to_string(),
        attributes: vec![
            ("Type".to_string(), "Place".to_string()),
            ("Title".to_string(), place.title.clone()),
            ("U".to_string(), place.uuid.clone()),
        ],
        text: None,
        children: Vec::new(),
    };
    node.set_attr("Sqm", place.sqm.as_deref());
    node.set_attr("PType", place.p_type.as_deref());
    node.set_attr("Icon", place.icon.as_deref());
    node
}

fn category_to_raw_node(cat: &Category) -> RawNode {
    let mut node = RawNode {
        tag: "C".to_string(),
        attributes: vec![
            ("Type".to_string(), "Category".to_string()),
            ("Title".to_string(), cat.title.clone()),
            ("U".to_string(), cat.uuid.clone()),
        ],
        text: None,
        children: Vec::new(),
    };
    let bk = cat
        .bk_color
        .map(|[r, g, b]| format!("{},{},{}", r, g, b));
    node.set_attr("BkColor", bk.as_deref());
    node.set_attr("CatGroup", cat.cat_group.as_deref());
    node.set_attr("Icon", cat.icon.as_deref());
    node
}

fn tree_node_to_raw_node(tree_node: &TreeNode) -> RawNode {
    let mut children: Vec<RawNode> = tree_node.pins.iter().map(pin_to_raw_node).collect();
    children.extend(tree_node.extra_children.iter().cloned());
    if (tree_node.room_uuid.is_some() || tree_node.category_uuid.is_some())
        && !children.iter().any(|c| c.tag == "IoData")
    {
        let mut io = RawNode {
            tag: "IoData".to_string(),
            attributes: Vec::new(),
            text: None,
            children: Vec::new(),
        };
        io.set_attr("Cr", tree_node.category_uuid.as_deref());
        io.set_attr("Pr", tree_node.room_uuid.as_deref());
        children.push(io);
    }
    for child in &tree_node.children {
        children.push(tree_node_to_raw_node(child));
    }
    let attributes = if tree_node.raw_attributes.is_empty() {
        vec![
            ("Type".to_string(), tree_node.control_type.clone()),
            ("Title".to_string(), tree_node.title.clone()),
            ("U".to_string(), tree_node.uuid.clone()),
        ]
    } else {
        tree_node.raw_attributes.clone()
    };
    RawNode {
        tag: "C".to_string(),
        attributes,
        text: None,
        children,
    }
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

    #[test]
    fn test_lossless_roundtrip_unmodified_klimczaki() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should load");
        let xml = serialize_loxone_xml(&project).expect("should serialize unmodified project");
        let reparsed = parse_loxone_xml(&xml).expect("reparsed XML should be valid");

        assert_eq!(reparsed.pages, project.pages);
        assert_eq!(reparsed.places, project.places);
        assert_eq!(reparsed.categories, project.categories);
        assert_eq!(reparsed.periphery_tree, project.periphery_tree);
        assert_eq!(reparsed.system_tree, project.system_tree);
        assert_eq!(
            reparsed.xref.pins_by_uuid.len(),
            project.xref.pins_by_uuid.len()
        );
        assert_eq!(
            reparsed.xref.outgoing_wires_by_pin_uuid.len(),
            project.xref.outgoing_wires_by_pin_uuid.len()
        );
    }

    #[test]
    fn test_add_move_remove_page_block_and_undo_roundtrip() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let mut project = load_loxone_file(&path).expect("Klimczaki.Loxone should load");

        let page_uuid = project.pages[0].uuid.clone();
        let orig_block_count = project.pages[0].blocks.len();

        let new_block =
            crate::factory::new_page_block("StairwellLight", "Stairway Timer", 100.0, 200.0);
        let block_uuid = new_block.uuid.clone();

        let inv_add = apply_edit_with_inverse(
            &mut project,
            &ProjectEdit::AddPageBlock {
                page_uuid: page_uuid.clone(),
                block: new_block,
            },
        )
        .expect("AddPageBlock should succeed");
        assert_eq!(project.pages[0].blocks.len(), orig_block_count + 1);
        assert!(project.xref.objects_by_uuid.contains_key(&block_uuid));

        let inv_move = apply_edit_with_inverse(
            &mut project,
            &ProjectEdit::MovePageBlock {
                page_uuid: page_uuid.clone(),
                block_uuid: block_uuid.clone(),
                px: 350.0,
                py: 480.0,
            },
        )
        .expect("MovePageBlock should succeed");

        let moved_blk = project.pages[0]
            .blocks
            .iter()
            .find(|b| b.uuid == block_uuid)
            .unwrap();
        assert_eq!(moved_blk.px, 350.0);
        assert_eq!(moved_blk.py, 480.0);
        assert_eq!(moved_blk.px2, 350.0 + 2112.0);
        assert_eq!(moved_blk.py2, 480.0 + 768.0);

        let xml = serialize_loxone_xml(&project).expect("should serialize");
        let reparsed = parse_loxone_xml(&xml).expect("should reparse");
        let reparsed_blk = reparsed.pages[0]
            .blocks
            .iter()
            .find(|b| b.uuid == block_uuid)
            .expect("new block should exist in reparsed XML");
        assert_eq!(reparsed_blk.title, "Stairway Timer");
        assert_eq!(reparsed_blk.px, 350.0);
        assert_eq!(reparsed_blk.py, 480.0);

        // Undo move, then undo add
        apply_edit_with_inverse(&mut project, &inv_move).expect("undo move should succeed");
        let restored_blk = project.pages[0]
            .blocks
            .iter()
            .find(|b| b.uuid == block_uuid)
            .unwrap();
        assert_eq!(restored_blk.px, 100.0);
        assert_eq!(restored_blk.py, 200.0);

        apply_edit_with_inverse(&mut project, &inv_add).expect("undo add should succeed");
        assert_eq!(project.pages[0].blocks.len(), orig_block_count);
        assert!(!project.xref.objects_by_uuid.contains_key(&block_uuid));
    }

    #[test]
    fn test_add_and_remove_wire_link_and_undo_roundtrip() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let mut project = load_loxone_file(&path).expect("Klimczaki.Loxone should load");

        let page_uuid = project.pages[0].uuid.clone();
        let block_a = crate::factory::new_page_block("Pushbutton", "Button A", 100.0, 100.0);
        let block_b = crate::factory::new_page_block("StairwellLight", "Light B", 700.0, 100.0);
        let block_a_uuid = block_a.uuid.clone();
        let block_b_uuid = block_b.uuid.clone();
        let source_pin_uuid = block_a
            .pins
            .iter()
            .find(|p| p.key == "Q")
            .unwrap()
            .uuid
            .clone();

        apply_edit(
            &mut project,
            &ProjectEdit::AddPageBlock {
                page_uuid: page_uuid.clone(),
                block: block_a,
            },
        )
        .unwrap();
        apply_edit(
            &mut project,
            &ProjectEdit::AddPageBlock {
                page_uuid: page_uuid.clone(),
                block: block_b,
            },
        )
        .unwrap();

        let inv_wire = apply_edit_with_inverse(
            &mut project,
            &ProjectEdit::AddWireLink {
                target_block_uuid: block_b_uuid.clone(),
                target_pin_key: "Tr".to_string(),
                source_pin_uuid: source_pin_uuid.clone(),
            },
        )
        .expect("AddWireLink should succeed");

        assert!(project
            .xref
            .outgoing_wires_by_pin_uuid
            .get(&source_pin_uuid)
            .is_some_and(|targets| targets.iter().any(|t| t.target_block_uuid == block_b_uuid)));

        let xml = serialize_loxone_xml(&project).expect("should serialize");
        let reparsed = parse_loxone_xml(&xml).expect("should reparse");
        let reparsed_b = reparsed.pages[0]
            .blocks
            .iter()
            .find(|b| b.uuid == block_b_uuid)
            .unwrap();
        let tr_pin = reparsed_b.pins.iter().find(|p| p.key == "Tr").unwrap();
        assert_eq!(tr_pin.incoming_links.len(), 1);
        assert_eq!(tr_pin.incoming_links[0].source_pin_uuid, source_pin_uuid);
        assert!(reparsed
            .xref
            .outgoing_wires_by_pin_uuid
            .get(&source_pin_uuid)
            .is_some_and(|targets| targets.iter().any(|t| t.target_block_uuid == block_b_uuid)));
        assert!(reparsed
            .xref
            .usages_for(&block_a_uuid)
            .iter()
            .any(|u| u.block_uuid == block_b_uuid));

        // Undo wire link
        apply_edit_with_inverse(&mut project, &inv_wire).expect("undo wire should succeed");
        let blk_b = project.pages[0]
            .blocks
            .iter()
            .find(|b| b.uuid == block_b_uuid)
            .unwrap();
        let tr_after = blk_b.pins.iter().find(|p| p.key == "Tr").unwrap();
        assert!(tr_after.incoming_links.is_empty());
        assert!(!project
            .xref
            .outgoing_wires_by_pin_uuid
            .contains_key(&source_pin_uuid));
    }

    #[test]
    fn test_remove_block_cleans_up_dangling_wires_and_restores_on_undo() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let mut project = load_loxone_file(&path).expect("Klimczaki.Loxone should load");

        let page_uuid = project.pages[0].uuid.clone();
        let block_a = crate::factory::new_page_block("Pushbutton", "Source Block A", 100.0, 100.0);
        let block_b =
            crate::factory::new_page_block("StairwellLight", "Target Block B", 700.0, 100.0);
        let block_a_uuid = block_a.uuid.clone();
        let block_b_uuid = block_b.uuid.clone();
        let source_pin_uuid = block_a
            .pins
            .iter()
            .find(|p| p.key == "Q")
            .unwrap()
            .uuid
            .clone();

        apply_edit(
            &mut project,
            &ProjectEdit::AddPageBlock {
                page_uuid: page_uuid.clone(),
                block: block_a,
            },
        )
        .unwrap();
        apply_edit(
            &mut project,
            &ProjectEdit::AddPageBlock {
                page_uuid: page_uuid.clone(),
                block: block_b,
            },
        )
        .unwrap();
        apply_edit(
            &mut project,
            &ProjectEdit::AddWireLink {
                target_block_uuid: block_b_uuid.clone(),
                target_pin_key: "Tr".to_string(),
                source_pin_uuid: source_pin_uuid.clone(),
            },
        )
        .unwrap();

        let inv_remove = apply_edit_with_inverse(
            &mut project,
            &ProjectEdit::RemovePageBlock {
                page_uuid: page_uuid.clone(),
                block_uuid: block_a_uuid.clone(),
            },
        )
        .expect("RemovePageBlock should succeed");

        assert!(matches!(inv_remove, ProjectEdit::Batch(_)));
        assert!(!project
            .pages[0]
            .blocks
            .iter()
            .any(|b| b.uuid == block_a_uuid));

        // Verify dangling incoming wire on Block B was cleaned up
        let blk_b = project.pages[0]
            .blocks
            .iter()
            .find(|b| b.uuid == block_b_uuid)
            .unwrap();
        let tr_pin = blk_b.pins.iter().find(|p| p.key == "Tr").unwrap();
        assert!(tr_pin.incoming_links.is_empty());

        // Apply inverse Batch to undo removal and restore both Block A and Block B's wire
        apply_edit_with_inverse(&mut project, &inv_remove)
            .expect("undo RemovePageBlock batch should succeed");
        assert!(project
            .pages[0]
            .blocks
            .iter()
            .any(|b| b.uuid == block_a_uuid));
        let blk_b_restored = project.pages[0]
            .blocks
            .iter()
            .find(|b| b.uuid == block_b_uuid)
            .unwrap();
        let tr_restored = blk_b_restored.pins.iter().find(|p| p.key == "Tr").unwrap();
        assert_eq!(tr_restored.incoming_links.len(), 1);
        assert_eq!(
            tr_restored.incoming_links[0].source_pin_uuid,
            source_pin_uuid
        );
    }

    #[test]
    fn test_add_and_remove_page_room_category_and_tree_node_roundtrip() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let mut project = load_loxone_file(&path).expect("Klimczaki.Loxone should load");

        let orig_pages = project.pages.len();
        let orig_places = project.places.len();
        let orig_cats = project.categories.len();

        let new_page = crate::factory::new_page("Attic Automation");
        let new_room = crate::factory::new_room("Attic Studio");
        let new_cat = crate::factory::new_category("Energy Management", "10");
        let new_merker = crate::factory::new_tree_node("Memory", "Night Mode Flag");

        let page_uuid = new_page.uuid.clone();
        let room_uuid = new_room.uuid.clone();
        let cat_uuid = new_cat.uuid.clone();
        let merker_uuid = new_merker.uuid.clone();

        let mem_caption_uuid = project
            .system_tree
            .iter()
            .find(|n| n.control_type == "MemoryCaption")
            .map(|n| n.uuid.clone());

        let inv_page = apply_edit_with_inverse(
            &mut project,
            &ProjectEdit::AddPage { page: new_page },
        )
        .unwrap();
        let inv_room = apply_edit_with_inverse(
            &mut project,
            &ProjectEdit::AddRoom { room: new_room },
        )
        .unwrap();
        let inv_cat = apply_edit_with_inverse(
            &mut project,
            &ProjectEdit::AddCategory { category: new_cat },
        )
        .unwrap();
        let inv_tree = apply_edit_with_inverse(
            &mut project,
            &ProjectEdit::AddTreeNode {
                section: crate::model::TreeSectionKind::System,
                parent_uuid: mem_caption_uuid,
                node: new_merker,
            },
        )
        .unwrap();

        let xml = serialize_loxone_xml(&project).expect("should serialize");
        let reparsed = parse_loxone_xml(&xml).expect("should reparse");

        assert_eq!(reparsed.pages.len(), orig_pages + 1);
        assert!(reparsed.pages.iter().any(|p| p.uuid == page_uuid && p.title == "Attic Automation"));
        assert_eq!(reparsed.places.len(), orig_places + 1);
        assert_eq!(reparsed.places.get(&room_uuid).unwrap().title, "Attic Studio");
        assert_eq!(reparsed.categories.len(), orig_cats + 1);
        assert_eq!(
            reparsed.categories.get(&cat_uuid).unwrap().title,
            "Energy Management"
        );
        assert!(reparsed.xref.objects_by_uuid.contains_key(&merker_uuid));

        // Undo all 4 additions
        apply_edit_with_inverse(&mut project, &inv_tree).unwrap();
        apply_edit_with_inverse(&mut project, &inv_cat).unwrap();
        apply_edit_with_inverse(&mut project, &inv_room).unwrap();
        apply_edit_with_inverse(&mut project, &inv_page).unwrap();

        assert_eq!(project.pages.len(), orig_pages);
        assert_eq!(project.places.len(), orig_places);
        assert_eq!(project.categories.len(), orig_cats);
        assert!(!project.xref.objects_by_uuid.contains_key(&merker_uuid));

        let xml_after = serialize_loxone_xml(&project).expect("should serialize after undo");
        let reparsed_after = parse_loxone_xml(&xml_after).expect("should reparse after undo");
        assert_eq!(reparsed_after.pages.len(), orig_pages);
        assert_eq!(reparsed_after.places.len(), orig_places);
        assert_eq!(reparsed_after.categories.len(), orig_cats);
        assert!(!reparsed_after.xref.objects_by_uuid.contains_key(&merker_uuid));
    }

    #[test]
    fn test_resize_paste_clone_and_cross_page_link_edits() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let mut project = load_loxone_file(&path).expect("Klimczaki.Loxone should load");

        let page_0_uuid = project.pages[0].uuid.clone();
        let page_1_uuid = project.pages[1].uuid.clone();

        // 1. Add a Memo block and resize it
        let memo = crate::factory::new_page_block("Memo", "Design Note", 100.0, 200.0);
        let memo_uuid = memo.uuid.clone();
        apply_edit(
            &mut project,
            &ProjectEdit::AddPageBlock {
                page_uuid: page_0_uuid.clone(),
                block: memo,
            },
        )
        .unwrap();

        let inv_resize = apply_edit_with_inverse(
            &mut project,
            &ProjectEdit::ResizePageBlock {
                page_uuid: page_0_uuid.clone(),
                block_uuid: memo_uuid.clone(),
                px2: 1800.0,
                py2: 1100.0,
            },
        )
        .unwrap();
        let resized = project.pages[0]
            .blocks
            .iter()
            .find(|b| b.uuid == memo_uuid)
            .unwrap();
        assert_eq!((resized.px2, resized.py2), (1800.0, 1100.0));
        apply_edit_with_inverse(&mut project, &inv_resize).unwrap();

        // 2. Test clone_blocks_for_paste preserving internal wires
        let blk_a = crate::factory::new_page_block("Switch", "Sw A", 300.0, 300.0);
        let mut blk_b = crate::factory::new_page_block("StairwellLight", "Delay B", 900.0, 300.0);
        let a_out_uuid = blk_a.pins.iter().find(|p| p.key == "Q").unwrap().uuid.clone();
        blk_b
            .pins
            .iter_mut()
            .find(|p| p.key == "Tr")
            .unwrap()
            .incoming_links
            .push(WireLink {
                source_pin_uuid: a_out_uuid.clone(),
                flags: None,
            });
        let pasted = clone_blocks_for_paste(&[blk_a.clone(), blk_b.clone()], 64.0, 64.0);
        assert_eq!(pasted.len(), 2);
        assert_ne!(pasted[0].uuid, blk_a.uuid);
        assert_ne!(pasted[1].uuid, blk_b.uuid);
        assert_eq!((pasted[0].px, pasted[0].py), (364.0, 364.0));
        let pasted_a_out = pasted[0]
            .pins
            .iter()
            .find(|p| p.key == "Q")
            .unwrap()
            .uuid
            .clone();
        assert_ne!(pasted_a_out, a_out_uuid);
        let pasted_b_in = &pasted[1]
            .pins
            .iter()
            .find(|p| p.key == "Tr")
            .unwrap()
            .incoming_links;
        assert_eq!(pasted_b_in.len(), 1);
        assert_eq!(pasted_b_in[0].source_pin_uuid, pasted_a_out);

        // 3. Test create_cross_page_link_edits
        apply_edit(
            &mut project,
            &ProjectEdit::AddPageBlock {
                page_uuid: page_0_uuid.clone(),
                block: blk_a.clone(),
            },
        )
        .unwrap();
        let xpage_edit = create_cross_page_link_edits(
            &project,
            &page_0_uuid,
            &blk_a.uuid,
            "Q",
            &page_1_uuid,
            512.0,
            512.0,
        )
        .expect("should build cross-page link batch");
        let inv_xpage = apply_edit_with_inverse(&mut project, &xpage_edit).unwrap();
        assert!(project.pages[1]
            .blocks
            .iter()
            .any(|b| b.block_type == "Memory" && b.title == "Sw A (Q)"));
        apply_edit_with_inverse(&mut project, &inv_xpage).unwrap();
    }
}

