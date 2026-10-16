use egui::{Color32, RichText, ScrollArea, Ui};
use loxone_core::model::{LoxoneProject, TreeNode};

use crate::theme::{ACCENT_BLUE, TEXT_MUTED, TEXT_PRIMARY};

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
    CreatePage,
    CreateRoom,
    CreateCategory,
    CreateMemoryFlag,
    CreateVirtualIn,
    CreateVirtualOut,
    PlaceOnPage(String),
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
    ui.horizontal(|ui| match *active_tab {
        LeftTab::Pages => {
            if ui
                .small_button(RichText::new("+ Page").color(ACCENT_BLUE))
                .on_hover_text("Create a new schematic page")
                .clicked()
            {
                action = Some(TreeAction::CreatePage);
            }
        }
        LeftTab::Periphery => {
            if ui
                .small_button(RichText::new("+ VIn").color(ACCENT_BLUE))
                .on_hover_text("Create a new Virtual Input")
                .clicked()
            {
                action = Some(TreeAction::CreateVirtualIn);
            }
            if ui
                .small_button(RichText::new("+ VOut").color(ACCENT_BLUE))
                .on_hover_text("Create a new Virtual Output")
                .clicked()
            {
                action = Some(TreeAction::CreateVirtualOut);
            }
        }
        LeftTab::Rooms => {
            if ui
                .small_button(RichText::new("+ Room").color(ACCENT_BLUE))
                .on_hover_text("Create a new Room")
                .clicked()
            {
                action = Some(TreeAction::CreateRoom);
            }
            if ui
                .small_button(RichText::new("+ Category").color(ACCENT_BLUE))
                .on_hover_text("Create a new Category")
                .clicked()
            {
                action = Some(TreeAction::CreateCategory);
            }
        }
        LeftTab::System => {
            if ui
                .small_button(RichText::new("+ Flag").color(ACCENT_BLUE))
                .on_hover_text("Create a new Memory Flag (Merker)")
                .clicked()
            {
                action = Some(TreeAction::CreateMemoryFlag);
            }
        }
    });
    ui.add_space(2.0);
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
                                        let visible_count =
                                            page.blocks.iter().filter(|b| b.has_position).count();
                                        ui.label(
                                            RichText::new(format!("{}", visible_count))
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
                    resp.context_menu(|ui| {
                        if ui.button("🔍 Inspect Page").clicked() {
                            action = Some(TreeAction::SelectObject(page.uuid.clone()));
                            ui.close_menu();
                        }
                    });
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
                    let header = egui::CollapsingHeader::new(
                        RichText::new(&place.title).color(TEXT_PRIMARY).strong(),
                    )
                    .id_salt(&place.uuid)
                    .default_open(!q.is_empty())
                    .show(ui, |ui| {
                        for (p_idx, page) in project.pages.iter().enumerate() {
                            for block in &page.blocks {
                                if !block.has_position {
                                    continue;
                                }
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
                    if header.header_response.clicked() {
                        action = Some(TreeAction::SelectObject(place.uuid.clone()));
                    }
                }

                if !project.categories.is_empty() {
                    ui.add_space(6.0);
                    ui.separator();
                    ui.label(
                        RichText::new("CATEGORIES")
                            .size(10.5)
                            .color(TEXT_MUTED)
                            .strong(),
                    );
                    let mut cats: Vec<_> = project.categories.values().collect();
                    cats.sort_by(|a, b| a.title.cmp(&b.title));
                    for cat in cats {
                        if !q.is_empty() && !cat.title.to_lowercase().contains(&q) {
                            continue;
                        }
                        let is_sel = selected_uuid == Some(cat.uuid.as_str());
                        if ui
                            .selectable_label(
                                is_sel,
                                RichText::new(&cat.title).color(if is_sel {
                                    ACCENT_BLUE
                                } else {
                                    TEXT_PRIMARY
                                }),
                            )
                            .clicked()
                        {
                            action = Some(TreeAction::SelectObject(cat.uuid.clone()));
                        }
                    }
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

fn is_placeable_tree_node(control_type: &str) -> bool {
    crate::insert_modal::reference_kind_for_tree_node(control_type).is_some()
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
    let placeable = !node.uuid.is_empty() && is_placeable_tree_node(&node.control_type);

    if node.children.is_empty() {
        ui.horizontal(|ui| {
            let text = if usages > 0 {
                format!("{}  [{}]", label_title, usages)
            } else {
                label_title.to_string()
            };
            let resp = ui.selectable_label(
                is_selected,
                RichText::new(text).color(if is_selected {
                    ACCENT_BLUE
                } else {
                    TEXT_PRIMARY
                }),
            );
            if resp.clicked() && !node.uuid.is_empty() {
                action = Some(TreeAction::SelectObject(node.uuid.clone()));
            }
            if placeable {
                resp.context_menu(|ui| {
                    if ui.button("📥 Place on Current Page").clicked() {
                        action = Some(TreeAction::PlaceOnPage(node.uuid.clone()));
                        ui.close_menu();
                    }
                });
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
        if placeable {
            header.header_response.context_menu(|ui| {
                if ui.button("📥 Place on Current Page").clicked() {
                    action = Some(TreeAction::PlaceOnPage(node.uuid.clone()));
                    ui.close_menu();
                }
            });
        }
    }

    action
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use loxone_core::parser::load_loxone_file;

    #[test]
    fn test_tree_node_matches_query_on_klimczaki() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should parse");
        assert!(!project.periphery_tree.is_empty());
        assert!(node_matches_query(&project.periphery_tree[0], ""));
        assert!(node_matches_query(&project.periphery_tree[0], "loxlive"));
        assert!(!node_matches_query(&project.periphery_tree[0], "nonexistent_xyz_device_999"));
    }
}
