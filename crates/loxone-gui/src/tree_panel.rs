use egui::{Color32, RichText, ScrollArea, Ui};
use loxone_core::model::{LoxoneProject, TreeNode};

use crate::icons::{icon_for_block_type, icon_for_tree_node, tree_node_attr, ui_icon, LoxoneIcon};
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
    live_store: Option<&loxone_core::miniserver::LiveValueStore>,
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
                    RichText::new(crate::i18n::tr(label))
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
            .hint_text(crate::i18n::tr("Filter list…"))
            .desired_width(f32::INFINITY),
    );
    ui.add_space(4.0);
    ui.horizontal(|ui| match *active_tab {
        LeftTab::Pages => {
            if ui
                .small_button(RichText::new(crate::i18n::tr("+ Page")).color(ACCENT_BLUE))
                .on_hover_text(crate::i18n::tr("Create a new schematic page"))
                .clicked()
            {
                action = Some(TreeAction::CreatePage);
            }
        }
        LeftTab::Periphery => {
            if ui
                .small_button(RichText::new(crate::i18n::tr("+ VIn")).color(ACCENT_BLUE))
                .on_hover_text(crate::i18n::tr("Create a new Virtual Input"))
                .clicked()
            {
                action = Some(TreeAction::CreateVirtualIn);
            }
            if ui
                .small_button(RichText::new(crate::i18n::tr("+ VOut")).color(ACCENT_BLUE))
                .on_hover_text(crate::i18n::tr("Create a new Virtual Output"))
                .clicked()
            {
                action = Some(TreeAction::CreateVirtualOut);
            }
        }
        LeftTab::Rooms => {
            if ui
                .small_button(RichText::new(crate::i18n::tr("+ Room")).color(ACCENT_BLUE))
                .on_hover_text(crate::i18n::tr("Create a new Room"))
                .clicked()
            {
                action = Some(TreeAction::CreateRoom);
            }
            if ui
                .small_button(RichText::new(crate::i18n::tr("+ Category")).color(ACCENT_BLUE))
                .on_hover_text(crate::i18n::tr("Create a new Category"))
                .clicked()
            {
                action = Some(TreeAction::CreateCategory);
            }
        }
        LeftTab::System => {
            if ui
                .small_button(RichText::new(crate::i18n::tr("+ Flag")).color(ACCENT_BLUE))
                .on_hover_text(crate::i18n::tr("Create a new Memory Flag (Merker)"))
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
                                ui_icon(ui, LoxoneIcon::Page, 16.0);
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
                        if ui.button(crate::i18n::tr("🔍 Inspect Page")).clicked() {
                            action = Some(TreeAction::SelectObject(page.uuid.clone()));
                            ui.close_menu();
                        }
                    });
                }
            }
            LeftTab::Periphery => {
                for node in project
                    .system_tree
                    .iter()
                    .filter(|n| is_periphery_header_section(n))
                    .chain(project.periphery_tree.iter())
                {
                    if let Some(a) =
                        render_tree_node(ui, project, node, &q, selected_uuid, live_store)
                    {
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
                    let id = ui.make_persistent_id(&place.uuid);
                    let state = egui::collapsing_header::CollapsingState::load_with_default_open(
                        ui.ctx(),
                        id,
                        !q.is_empty(),
                    );
                    let header_res = state.show_header(ui, |ui| {
                        ui_icon(ui, LoxoneIcon::Room, 16.0);
                        if ui
                            .selectable_label(
                                selected_uuid == Some(place.uuid.as_str()),
                                RichText::new(&place.title).color(TEXT_PRIMARY).strong(),
                            )
                            .clicked()
                        {
                            action = Some(TreeAction::SelectObject(place.uuid.clone()));
                        }
                    });
                    header_res.body(|ui| {
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
                                    ui.horizontal(|ui| {
                                        ui_icon(
                                            ui,
                                            icon_for_block_type(&block.block_type, None),
                                            15.0,
                                        );
                                        if ui
                                            .selectable_label(
                                                is_sel,
                                                format!(
                                                    "{} ({})",
                                                    block.title,
                                                    crate::i18n::block_display_name(
                                                        &block.block_type
                                                    )
                                                ),
                                            )
                                            .clicked()
                                        {
                                            action = Some(TreeAction::NavigateToBlock {
                                                page_index: p_idx,
                                                block_uuid: block.uuid.clone(),
                                            });
                                        }
                                    });
                                }
                            }
                        }
                    });
                }

                if !project.categories.is_empty() {
                    ui.add_space(6.0);
                    ui.separator();
                    ui.label(
                        RichText::new(crate::i18n::tr("CATEGORIES"))
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
                        ui.horizontal(|ui| {
                            ui_icon(ui, LoxoneIcon::Category(cat.bk_color), 15.0);
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
                        });
                    }
                }
            }
            LeftTab::System => {
                for node in &project.system_tree {
                    if let Some(a) =
                        render_tree_node(ui, project, node, &q, selected_uuid, live_store)
                    {
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
    let localized_type = crate::i18n::block_display_name(&node.control_type).to_lowercase();
    if node.title.to_lowercase().contains(q)
        || node.control_type.to_lowercase().contains(q)
        || localized_type.contains(q)
        || node.uuid.to_lowercase().contains(q)
    {
        return true;
    }
    node.children.iter().any(|c| node_matches_query(c, q))
}

fn is_periphery_header_section(node: &TreeNode) -> bool {
    if node.control_type == "WeatherServer" {
        return true;
    }
    if node.control_type == "LoxCaption" {
        if let Some(ct) = tree_node_attr(node, "CaptionType") {
            return matches!(ct, "28" | "6" | "14" | "19" | "33");
        }
    }
    false
}

fn is_placeable_tree_node(control_type: &str) -> bool {
    crate::insert_modal::reference_kind_for_tree_node(control_type).is_some()
}

fn count_tree_devices_recursive(node: &TreeNode) -> usize {
    let self_count = usize::from(node.control_type == "TreeDevice");
    self_count
        + node
            .children
            .iter()
            .map(count_tree_devices_recursive)
            .sum::<usize>()
}

/// Computes the muted secondary subtitle shown in parentheses next to a Periphery/System node
/// (matching Loxone Config's `(Salon + jadalnia)`, `(Rozdzielnica R01 P1)`, `(5/30 Urządzenia)`, `(36/50 Urządzenia)`).
pub fn tree_node_subtitle(project: &LoxoneProject, node: &TreeNode) -> Option<String> {
    let icon = icon_for_tree_node(node);
    if icon == LoxoneIcon::LoxoneLink && !node.children.is_empty() {
        return Some(format!("{}/30 Urządzenia", node.children.len()));
    }
    if node.control_type == "LoxTree" {
        let dev_count = count_tree_devices_recursive(node);
        if dev_count > 0 {
            return Some(format!("{}/50 Urządzenia", dev_count));
        }
    }
    if let Some(r_uuid) = node.room_uuid.as_deref() {
        if let Some(place) = project.places.get(r_uuid) {
            if !place.title.is_empty() && place.title != "Nieprzyporządkowane" {
                return Some(place.title.clone());
            }
        }
    }
    if let Some(sb) = tree_node_attr(node, "SwitchBoard").filter(|s| !s.trim().is_empty()) {
        let row = tree_node_attr(node, "SwitchBoardRow")
            .and_then(|r| r.parse::<u32>().ok());
        let pos = tree_node_attr(node, "SwitchBoardPos")
            .filter(|p| !p.trim().is_empty());
        return match (row, pos) {
            (Some(r), Some(p)) => Some(format!("{} R{:02} P{}", sb, r, p)),
            (Some(r), None) => Some(format!("{} R{:02}", sb, r)),
            _ => Some(sb.to_string()),
        };
    }
    None
}

fn render_tree_node(
    ui: &mut Ui,
    project: &LoxoneProject,
    node: &TreeNode,
    q: &str,
    selected_uuid: Option<&str>,
    live_store: Option<&loxone_core::miniserver::LiveValueStore>,
) -> Option<TreeAction> {
    if !node_matches_query(node, q) {
        return None;
    }
    let mut action = None;
    let label_title = if node.title.is_empty() {
        crate::i18n::block_display_name(&node.control_type)
    } else {
        &node.title
    };
    let usages = project.xref.usages_for(&node.uuid).len();
    let is_selected = !node.uuid.is_empty() && selected_uuid == Some(node.uuid.as_str());
    let placeable = !node.uuid.is_empty() && is_placeable_tree_node(&node.control_type);
    let icon = icon_for_tree_node(node);
    let subtitle = tree_node_subtitle(project, node);

    if node.children.is_empty() {
        ui.horizontal(|ui| {
            ui.add_space(18.0);
            ui_icon(ui, icon, 16.0);
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
            if let Some(sub) = &subtitle {
                ui.label(RichText::new(format!("({})", sub)).size(11.5).color(TEXT_MUTED));
            }
            if placeable {
                if let Some(store) = live_store {
                    if let Some(sig) = store.resolve_tree_node_signal(node) {
                        let fg = if sig.is_active {
                            Color32::from_rgb(0x59, 0xB8, 0x28)
                        } else {
                            TEXT_MUTED
                        };
                        ui.label(RichText::new(&sig.formatted).size(11.0).strong().color(fg));
                    }
                }
            }
            if resp.clicked() && !node.uuid.is_empty() {
                action = Some(TreeAction::SelectObject(node.uuid.clone()));
            }
            if placeable {
                resp.context_menu(|ui| {
                    if ui
                        .button(crate::i18n::tr("📥 Place on Current Page"))
                        .clicked()
                    {
                        action = Some(TreeAction::PlaceOnPage(node.uuid.clone()));
                        ui.close_menu();
                    }
                });
            }
        });
    } else {
        let salt = if node.uuid.is_empty() {
            format!("caption_{}_{}", node.control_type, label_title)
        } else {
            node.uuid.clone()
        };
        let id = ui.make_persistent_id(salt);
        let default_open = !q.is_empty()
            || node.control_type == "LoxLIVE"
            || node.control_type == "Miniserver"
            || icon == LoxoneIcon::AudioFolder
            || icon == LoxoneIcon::LoxoneLink;
        let state =
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, default_open);

        let header_res = state.show_header(ui, |ui| {
            ui_icon(ui, icon, 16.0);
            let resp = ui.selectable_label(
                is_selected,
                RichText::new(label_title).color(if is_selected {
                    ACCENT_BLUE
                } else {
                    TEXT_PRIMARY
                }),
            );
            if let Some(sub) = &subtitle {
                ui.label(RichText::new(format!("({})", sub)).size(11.5).color(TEXT_MUTED));
            }
            if resp.clicked() && !node.uuid.is_empty() {
                action = Some(TreeAction::SelectObject(node.uuid.clone()));
            }
            if placeable {
                resp.context_menu(|ui| {
                    if ui
                        .button(crate::i18n::tr("📥 Place on Current Page"))
                        .clicked()
                    {
                        action = Some(TreeAction::PlaceOnPage(node.uuid.clone()));
                        ui.close_menu();
                    }
                });
            }
        });
        header_res.body(|ui| {
            for child in &node.children {
                if let Some(a) =
                    render_tree_node(ui, project, child, q, selected_uuid, live_store)
                {
                    action = Some(a);
                }
            }
        });
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

    #[test]
    fn test_tree_node_subtitles_match_reference_screenshot() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should parse");

        fn find_by_title<'a>(nodes: &'a [TreeNode], title: &str) -> Option<&'a TreeNode> {
            for n in nodes {
                if n.title == title {
                    return Some(n);
                }
                if let Some(found) = find_by_title(&n.children, title) {
                    return Some(found);
                }
            }
            None
        }

        let ms = find_by_title(&project.periphery_tree, "Klimczaki").unwrap();
        assert_eq!(
            tree_node_subtitle(&project, ms),
            Some("Rozdzielnica R01 P1".to_string())
        );

        let link = ms.children.iter().find(|n| n.title == "Link").unwrap();
        assert_eq!(
            tree_node_subtitle(&project, link),
            Some("5/30 Urządzenia".to_string())
        );

        let tree = ms.children.iter().find(|n| n.title == "Tree").unwrap();
        assert_eq!(
            tree_node_subtitle(&project, tree),
            Some("36/50 Urządzenia".to_string())
        );

        let audio_srv = find_by_title(&project.periphery_tree, "Audioserver")
            .or_else(|| find_by_title(&project.system_tree, "Audioserver"))
            .unwrap();
        assert_eq!(
            tree_node_subtitle(&project, audio_srv),
            Some("Salon + jadalnia".to_string())
        );
    }
}
