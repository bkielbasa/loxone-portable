use egui::{RichText, ScrollArea, Stroke, Ui};
use loxone_core::model::{ConnectorPin, LoxoneProject, PageBlock, RawNode, TreeNode};
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
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
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
                        let cat_type = cat.cat_group.as_deref().unwrap_or("-");
                        ui.label(
                            RichText::new(format!("Category: {} ({})", cat.title, cat_type))
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

        // Section 3: Detailed Block, TreeNode, or Category attributes, pins, and raw children
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
            ObjectLocation::Category => {
                if let Some(cat) = project.categories.get(uuid) {
                    kv_row(ui, "CategoryType", cat.cat_group.as_deref().unwrap_or("-"));
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

    if !block.raw_attributes.is_empty() || !block.extra_children.is_empty() {
        ui.add_space(8.0);
        ui.collapsing("Raw Attributes & Children", |ui| {
            for (k, v) in &block.raw_attributes {
                kv_row(ui, k, v);
            }
            if !block.extra_children.is_empty() {
                ui.add_space(4.0);
                render_raw_nodes(ui, &block.extra_children);
            }
        });
    }

    action
}

fn render_tree_node_details(ui: &mut Ui, node: &TreeNode) {
    render_pins_list(ui, &node.pins);

    if !node.raw_attributes.is_empty() {
        ui.add_space(8.0);
        egui::CollapsingHeader::new(
            RichText::new(format!("Attributes ({})", node.raw_attributes.len()))
                .size(11.5)
                .color(TEXT_MUTED),
        )
        .default_open(true)
        .show(ui, |ui| {
            for (k, v) in &node.raw_attributes {
                kv_row(ui, k, v);
            }
        });
    }

    if !node.extra_children.is_empty() {
        ui.add_space(8.0);
        egui::CollapsingHeader::new(RichText::new("Raw Children").size(11.5).color(TEXT_MUTED))
            .default_open(false)
            .show(ui, |ui| {
                render_raw_nodes(ui, &node.extra_children);
            });
    }
}

fn kv_row(ui: &mut Ui, k: &str, v: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{}:", k))
                .monospace()
                .size(11.0)
                .color(TEXT_MUTED),
        );
        ui.label(RichText::new(v).monospace().size(11.0).color(TEXT_PRIMARY));
    });
}

fn render_raw_nodes(ui: &mut Ui, nodes: &[RawNode]) {
    for raw in nodes {
        let mut header_str = format!("<{}", raw.tag);
        for (k, v) in &raw.attributes {
            header_str.push_str(&format!(" {}=\"{}\"", k, v));
        }
        header_str.push('>');
        ui.label(
            RichText::new(header_str)
                .monospace()
                .size(10.5)
                .color(TEXT_PRIMARY),
        );
        if let Some(txt) = &raw.text {
            ui.label(
                RichText::new(txt)
                    .monospace()
                    .size(10.5)
                    .color(TEXT_MUTED),
            );
        }
        if !raw.children.is_empty() {
            ui.indent(&raw.tag, |ui| {
                render_raw_nodes(ui, &raw.children);
            });
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use loxone_core::parser::load_loxone_file;

    #[test]
    fn test_find_tree_node_in_klimczaki() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should parse");
        let root_uuid = &project.periphery_tree[0].uuid;
        let found = find_tree_node(&project.periphery_tree, root_uuid);
        assert!(found.is_some());
        assert_eq!(found.unwrap().uuid, *root_uuid);
    }
}
