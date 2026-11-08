use egui::{Color32, CornerRadius, Margin, RichText, ScrollArea, Sense, Stroke, Ui, Vec2};
use loxone_core::model::{LoxoneProject, TreeNode};

use crate::icons::{icon_for_block_type, icon_for_tree_node, tree_node_attr, ui_icon, LoxoneIcon};
use crate::theme::{
    ACCENT_BLUE, BORDER_SUBTLE, CARD_BG, ELEVATED_BG, LOXONE_GREEN, TEXT_MUTED, TEXT_PRIMARY,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LeftTab {
    #[default]
    Pages,
    Periphery,
    Rooms,
    System,
}

impl LeftTab {
    /// Returns the Loxone Config 17 `Project Trees` dropdown label for this tree mode.
    pub fn dropdown_label(self) -> &'static str {
        match self {
            Self::Periphery => "Periphery",
            Self::Pages => "Configuration",
            Self::Rooms => "Rooms",
            Self::System => "Devices",
        }
    }
}

/// Formats the `(Room, Category)` subtitle shown next to a block in the `Configuration` tree.
pub fn block_room_category_subtitle(
    project: &LoxoneProject,
    page: &loxone_core::model::Page,
    block: &loxone_core::model::PageBlock,
) -> Option<String> {
    let eff_room = block
        .room_uuid
        .as_deref()
        .or(page.room_uuid.as_deref())
        .and_then(|u| project.places.get(u))
        .map(|p| p.title.as_str())
        .filter(|t| !t.is_empty() && *t != "Nieprzyporządkowane" && *t != "Unassigned");
    let eff_cat = block
        .category_uuid
        .as_deref()
        .or(page.category_uuid.as_deref())
        .and_then(|u| project.categories.get(u))
        .map(|c| c.title.as_str())
        .filter(|t| !t.is_empty() && *t != "Nieprzyporządkowane" && *t != "Unassigned");

    match (eff_room, eff_cat) {
        (Some(r), Some(c)) => Some(format!("{}, {}", r, c)),
        (Some(r), None) => Some(r.to_string()),
        (None, Some(c)) => Some(c.to_string()),
        (None, None) => None,
    }
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
    OpenPeripheryProvisioning,
    OpenHardwareInventory(Option<String>),
    IdentifyDevice(String),
}

pub fn set_active_identifying_devices(
    ctx: &egui::Context,
    active: std::collections::HashMap<String, f32>,
) {
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new("loxone_identifying_devices"), active);
    });
}

pub fn device_identify_elapsed(ctx: &egui::Context, uuid: &str) -> Option<f32> {
    if uuid.is_empty() {
        return None;
    }
    ctx.data(|d| {
        d.get_temp::<std::collections::HashMap<String, f32>>(egui::Id::new(
            "loxone_identifying_devices",
        ))
        .and_then(|m| m.get(uuid).copied())
    })
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

    // 1. Loxone Config 17 Project Trees Header + 4-Mode ComboBox Dropdown
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let (badge_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
        ui.painter()
            .circle_filled(badge_rect.center(), 9.5, LOXONE_GREEN);
        ui.painter().text(
            badge_rect.center(),
            egui::Align2::CENTER_CENTER,
            "LX",
            egui::FontId::proportional(9.0),
            Color32::from_rgb(0x0B, 0x16, 0x08),
        );
        ui.label(
            RichText::new(crate::i18n::tr("Project Trees"))
                .size(12.5)
                .color(Color32::WHITE)
                .strong(),
        );
    });

    ui.add_space(4.0);

    // 4-Mode Dropdown (Periphery, Configuration, Rooms, Devices) matching Loxone Config 17
    egui::ComboBox::from_id_salt("loxone_project_trees_mode_combo")
        .selected_text(
            RichText::new(crate::i18n::tr(active_tab.dropdown_label()))
                .size(11.8)
                .color(Color32::WHITE)
                .strong(),
        )
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            for mode in [
                LeftTab::Periphery,
                LeftTab::Pages,
                LeftTab::Rooms,
                LeftTab::System,
            ] {
                let is_sel = *active_tab == mode;
                if ui
                    .selectable_label(is_sel, crate::i18n::tr(mode.dropdown_label()))
                    .clicked()
                {
                    *active_tab = mode;
                }
            }
        });

    ui.add_space(4.0);

    // 2. Encore-style Project Switcher Card (Green rounded-square icon + Project Title)
    let proj_title = if project.metadata.title.trim().is_empty() {
        crate::i18n::tr("Project")
    } else {
        project.metadata.title.as_str()
    };
    egui::Frame::new()
        .fill(CARD_BG)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(8, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let (sq_rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                ui.painter()
                    .rect_filled(sq_rect, CornerRadius::same(4), LOXONE_GREEN);
                ui.painter().rect_stroke(
                    sq_rect.shrink(4.0),
                    CornerRadius::same(1),
                    Stroke::new(1.2_f32, Color32::WHITE),
                    egui::StrokeKind::Middle,
                );
                ui.label(
                    RichText::new(proj_title)
                        .size(12.0)
                        .color(Color32::WHITE)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("⇅").size(11.0).color(TEXT_MUTED));
                });
            });
        });

    ui.add_space(6.0);

    // 3. Segmented Section Pill Bar (synced with 4-Mode Dropdown)
    egui::Frame::new()
        .fill(Color32::from_rgb(0x16, 0x16, 0x16))
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(3, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for (tab, label) in [
                    (LeftTab::Periphery, "Periphery"),
                    (LeftTab::Pages, "Pages"),
                    (LeftTab::Rooms, "Rooms"),
                    (LeftTab::System, "Devices"),
                ] {
                    let selected = *active_tab == tab;
                    let pill_fill = if selected {
                        ELEVATED_BG
                    } else {
                        Color32::TRANSPARENT
                    };
                    let pill_resp = egui::Frame::new()
                        .fill(pill_fill)
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(Margin::symmetric(6, 3))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(crate::i18n::tr(label))
                                    .size(11.0)
                                    .color(if selected {
                                        Color32::WHITE
                                    } else {
                                        TEXT_MUTED
                                    })
                                    .strong(),
                            );
                        })
                        .response
                        .interact(Sense::click());
                    if pill_resp.clicked() {
                        *active_tab = tab;
                    }
                }
            });
        });

    ui.add_space(6.0);
    ui.add(
        egui::TextEdit::singleline(filter)
            .hint_text(crate::i18n::tr("Filter list…"))
            .desired_width(f32::INFINITY),
    );
    ui.add_space(4.0);
    ui.horizontal(|ui| match *active_tab {
        LeftTab::Pages => {
            if ui
                .small_button(RichText::new(crate::i18n::tr("+ Page")).color(LOXONE_GREEN))
                .on_hover_text(crate::i18n::tr("Create a new schematic page"))
                .clicked()
            {
                action = Some(TreeAction::CreatePage);
            }
        }
        LeftTab::Periphery => {
            if ui
                .small_button(RichText::new(crate::i18n::tr("+ Hardware")).color(LOXONE_GREEN))
                .on_hover_text(crate::i18n::tr(
                    "Provision Extensions, Loxone Tree devices, or Loxone Air devices",
                ))
                .clicked()
            {
                action = Some(TreeAction::OpenPeripheryProvisioning);
            }
            if ui
                .small_button(RichText::new(crate::i18n::tr("Serials")).color(ACCENT_BLUE))
                .on_hover_text(crate::i18n::tr(
                    "Hardware Serial Inventory & In-Place Serial Swap",
                ))
                .clicked()
            {
                action = Some(TreeAction::OpenHardwareInventory(
                    selected_uuid.map(str::to_string),
                ));
            }
            if ui
                .small_button(RichText::new(crate::i18n::tr("+ VIn")).color(LOXONE_GREEN))
                .on_hover_text(crate::i18n::tr("Create a new Virtual Input"))
                .clicked()
            {
                action = Some(TreeAction::CreateVirtualIn);
            }
            if ui
                .small_button(RichText::new(crate::i18n::tr("+ VOut")).color(LOXONE_GREEN))
                .on_hover_text(crate::i18n::tr("Create a new Virtual Output"))
                .clicked()
            {
                action = Some(TreeAction::CreateVirtualOut);
            }
        }
        LeftTab::Rooms => {
            if ui
                .small_button(RichText::new(crate::i18n::tr("+ Room")).color(LOXONE_GREEN))
                .on_hover_text(crate::i18n::tr("Create a new Room"))
                .clicked()
            {
                action = Some(TreeAction::CreateRoom);
            }
            if ui
                .small_button(RichText::new(crate::i18n::tr("+ Category")).color(LOXONE_GREEN))
                .on_hover_text(crate::i18n::tr("Create a new Category"))
                .clicked()
            {
                action = Some(TreeAction::CreateCategory);
            }
        }
        LeftTab::System => {
            if ui
                .small_button(RichText::new(crate::i18n::tr("Serials")).color(ACCENT_BLUE))
                .on_hover_text(crate::i18n::tr(
                    "Hardware Serial Inventory & In-Place Serial Swap",
                ))
                .clicked()
            {
                action = Some(TreeAction::OpenHardwareInventory(
                    selected_uuid.map(str::to_string),
                ));
            }
            if ui
                .small_button(RichText::new(crate::i18n::tr("+ Flag")).color(LOXONE_GREEN))
                .on_hover_text(crate::i18n::tr("Create a new Memory Flag (Merker)"))
                .clicked()
            {
                action = Some(TreeAction::CreateMemoryFlag);
            }
        }
    });
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(2.0);

    let q = filter.trim().to_lowercase();

    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        match *active_tab {
            LeftTab::Pages => {
                // Loxone Config 17 Configuration tree: expandable pages with visible blocks & (Room, Category) subtitles
                for (idx, page) in project.pages.iter().enumerate() {
                    let page_matches =
                        q.is_empty() || page.title.to_lowercase().contains(&q);
                    let matching_blocks: Vec<&loxone_core::model::PageBlock> = page
                        .blocks
                        .iter()
                        .filter(|b| {
                            b.has_position
                                && !b.title.is_empty()
                                && (page_matches
                                    || b.title.to_lowercase().contains(&q)
                                    || b.block_type.to_lowercase().contains(&q))
                        })
                        .collect();
                    if !page_matches && matching_blocks.is_empty() {
                        continue;
                    }
                    let is_active = idx == active_page_idx;
                    let id = ui.make_persistent_id(format!("cfg_page_tree_{}", page.uuid));
                    let state = egui::collapsing_header::CollapsingState::load_with_default_open(
                        ui.ctx(),
                        id,
                        is_active || !q.is_empty(),
                    );
                    let header_res = state.show_header(ui, |ui| {
                        ui_icon(ui, LoxoneIcon::Page, 15.0);
                        let title_rt = RichText::new(&page.title).size(12.0).color(
                            if is_active {
                                Color32::WHITE
                            } else {
                                TEXT_PRIMARY
                            },
                        );
                        let resp = ui.selectable_label(
                            is_active,
                            if is_active { title_rt.strong() } else { title_rt },
                        );
                        let visible_count =
                            page.blocks.iter().filter(|b| b.has_position).count();
                        ui.label(
                            RichText::new(format!("({})", visible_count))
                                .size(10.8)
                                .color(if is_active {
                                    LOXONE_GREEN
                                } else {
                                    TEXT_MUTED
                                }),
                        );
                        if resp.clicked() {
                            action = Some(TreeAction::SelectPage(idx));
                        }
                        resp.context_menu(|ui| {
                            if ui.button(crate::i18n::tr("🔍 Inspect Page")).clicked() {
                                action = Some(TreeAction::SelectObject(page.uuid.clone()));
                                ui.close_menu();
                            }
                        });
                    });
                    header_res.body(|ui| {
                        for block in matching_blocks {
                            let is_sel = selected_uuid == Some(block.uuid.as_str());
                            ui.horizontal(|ui| {
                                ui_icon(
                                    ui,
                                    icon_for_block_type(&block.block_type, None),
                                    14.0,
                                );
                                let resp = ui.selectable_label(
                                    is_sel,
                                    RichText::new(&block.title).size(11.4).color(if is_sel {
                                        ACCENT_BLUE
                                    } else {
                                        TEXT_PRIMARY
                                    }),
                                );
                                if let Some(sub) =
                                    block_room_category_subtitle(project, page, block)
                                {
                                    ui.label(
                                        RichText::new(format!("({})", sub))
                                            .size(10.8)
                                            .color(TEXT_MUTED),
                                    );
                                }
                                if resp.clicked() {
                                    action = Some(TreeAction::NavigateToBlock {
                                        page_index: idx,
                                        block_uuid: block.uuid.clone(),
                                    });
                                }
                            });
                        }
                    });
                }
            }
            LeftTab::Periphery => {
                // 1. Rooms section in Periphery tree (matching Loxone Config 17)
                if !project.places.is_empty() {
                    let rooms_id = ui.make_persistent_id("periphery_rooms_folder");
                    let rooms_state =
                        egui::collapsing_header::CollapsingState::load_with_default_open(
                            ui.ctx(),
                            rooms_id,
                            !q.is_empty(),
                        );
                    let rooms_hdr = rooms_state.show_header(ui, |ui| {
                        ui_icon(ui, LoxoneIcon::Room, 16.0);
                        ui.label(
                            RichText::new(crate::i18n::tr("Rooms"))
                                .color(TEXT_PRIMARY)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!("({})", project.places.len()))
                                .size(11.0)
                                .color(TEXT_MUTED),
                        );
                    });
                    rooms_hdr.body(|ui| {
                        let mut places: Vec<_> = project.places.values().collect();
                        places.sort_by(|a, b| a.title.cmp(&b.title));
                        for place in places {
                            if !q.is_empty() && !place.title.to_lowercase().contains(&q) {
                                continue;
                            }
                            let is_sel = selected_uuid == Some(place.uuid.as_str());
                            ui.horizontal(|ui| {
                                ui_icon(ui, LoxoneIcon::Room, 14.0);
                                if ui
                                    .selectable_label(
                                        is_sel,
                                        RichText::new(&place.title).size(11.4).color(if is_sel {
                                            ACCENT_BLUE
                                        } else {
                                            TEXT_PRIMARY
                                        }),
                                    )
                                    .clicked()
                                {
                                    action = Some(TreeAction::SelectObject(place.uuid.clone()));
                                }
                            });
                        }
                    });
                }

                // 2. Categories section in Periphery tree (matching Loxone Config 17)
                if !project.categories.is_empty() {
                    let cats_id = ui.make_persistent_id("periphery_categories_folder");
                    let cats_state =
                        egui::collapsing_header::CollapsingState::load_with_default_open(
                            ui.ctx(),
                            cats_id,
                            !q.is_empty(),
                        );
                    let cats_hdr = cats_state.show_header(ui, |ui| {
                        ui_icon(ui, LoxoneIcon::Category(None), 16.0);
                        ui.label(
                            RichText::new(crate::i18n::tr("Categories"))
                                .color(TEXT_PRIMARY)
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!("({})", project.categories.len()))
                                .size(11.0)
                                .color(TEXT_MUTED),
                        );
                    });
                    cats_hdr.body(|ui| {
                        let mut cats: Vec<_> = project.categories.values().collect();
                        cats.sort_by(|a, b| a.title.cmp(&b.title));
                        for cat in cats {
                            if !q.is_empty() && !cat.title.to_lowercase().contains(&q) {
                                continue;
                            }
                            let is_sel = selected_uuid == Some(cat.uuid.as_str());
                            ui.horizontal(|ui| {
                                ui_icon(ui, LoxoneIcon::Category(cat.bk_color), 14.0);
                                if ui
                                    .selectable_label(
                                        is_sel,
                                        RichText::new(&cat.title).size(11.4).color(if is_sel {
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
                    });
                }

                // 3. All global system_tree sections + periphery_tree hardware hierarchy
                for node in project
                    .system_tree
                    .iter()
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
                // Devices & System tree: Hardware hierarchy + System sections
                for node in project
                    .periphery_tree
                    .iter()
                    .chain(project.system_tree.iter())
                {
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
        return Some(crate::i18n::tr_fmt(
            "{count}/{max} Devices",
            &[("count", &node.children.len().to_string()), ("max", "30")],
        ));
    }
    if node.control_type == "LoxTree" {
        let dev_count = count_tree_devices_recursive(node);
        if dev_count > 0 {
            return Some(crate::i18n::tr_fmt(
                "{count}/{max} Devices",
                &[("count", &dev_count.to_string()), ("max", "50")],
            ));
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

fn subtree_has_identifying(ctx: &egui::Context, node: &TreeNode) -> bool {
    for child in &node.children {
        if device_identify_elapsed(ctx, &child.uuid).is_some()
            || subtree_has_identifying(ctx, child)
        {
            return true;
        }
    }
    false
}

fn render_identifying_badge(ui: &mut Ui, node: &TreeNode, elapsed_secs: f32) {
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(120));
    let blink_green = ((elapsed_secs * 4.5) as u32) % 2 == 0;
    let led_color = if blink_green {
        LOXONE_GREEN
    } else {
        Color32::from_rgb(0xF5, 0x9E, 0x0B)
    };
    let serial_suffix = node
        .attr("Serial")
        .or_else(|| node.attr("MAC"))
        .filter(|s| !s.trim().is_empty())
        .map(|s| format!(" · {}", s.trim()))
        .unwrap_or_default();
    egui::Frame::new()
        .fill(led_color.gamma_multiply(0.22))
        .stroke(Stroke::new(1.0_f32, led_color))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::symmetric(5, 1))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(7.0), Sense::hover());
                ui.painter().circle_filled(dot_rect.center(), 3.2, led_color);
                ui.label(
                    RichText::new(format!(
                        "{}{}",
                        crate::i18n::tr("IDENTIFYING"),
                        serial_suffix
                    ))
                    .size(9.8)
                    .color(Color32::WHITE)
                    .strong(),
                );
            });
        });
}

fn attach_tree_node_context_menu(
    resp: &egui::Response,
    project: &LoxoneProject,
    node: &TreeNode,
    placeable: bool,
    action: &mut Option<TreeAction>,
) {
    let is_hw_device = loxone_core::periphery_provisioning::is_identifiable_hardware_node(node);
    let can_identify = is_hw_device
        || (!node.uuid.is_empty()
            && loxone_core::periphery_provisioning::resolve_device_identify_target(
                project, &node.uuid,
            )
            .is_some());
    if !placeable && !can_identify {
        return;
    }
    resp.context_menu(|ui| {
        if can_identify {
            if ui
                .button(
                    RichText::new(crate::i18n::tr("🔦 Identify Device"))
                        .color(LOXONE_GREEN)
                        .strong(),
                )
                .clicked()
            {
                *action = Some(TreeAction::IdentifyDevice(node.uuid.clone()));
                ui.close_menu();
            }
        }
        if is_hw_device {
            if ui
                .button(crate::i18n::tr("🔢 Swap Serial Number…"))
                .clicked()
            {
                *action = Some(TreeAction::OpenHardwareInventory(Some(node.uuid.clone())));
                ui.close_menu();
            }
        }
        if placeable {
            if can_identify {
                ui.separator();
            }
            if ui
                .button(crate::i18n::tr("📥 Place on Current Page"))
                .clicked()
            {
                *action = Some(TreeAction::PlaceOnPage(node.uuid.clone()));
                ui.close_menu();
            }
        }
    });
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
    let identifying_elapsed = device_identify_elapsed(ui.ctx(), &node.uuid);

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
                is_selected || identifying_elapsed.is_some(),
                RichText::new(text).color(if identifying_elapsed.is_some() {
                    LOXONE_GREEN
                } else if is_selected {
                    ACCENT_BLUE
                } else {
                    TEXT_PRIMARY
                }),
            );
            if let Some(sub) = &subtitle {
                ui.label(RichText::new(format!("({})", sub)).size(11.5).color(TEXT_MUTED));
            }
            if let Some(elapsed) = identifying_elapsed {
                if elapsed < 0.25 {
                    resp.scroll_to_me(Some(egui::Align::Center));
                }
                render_identifying_badge(ui, node, elapsed);
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
            attach_tree_node_context_menu(&resp, project, node, placeable, &mut action);
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
        let mut state =
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, default_open);
        if subtree_has_identifying(ui.ctx(), node) {
            state.set_open(true);
        }

        let header_res = state.show_header(ui, |ui| {
            ui_icon(ui, icon, 16.0);
            let resp = ui.selectable_label(
                is_selected || identifying_elapsed.is_some(),
                RichText::new(label_title).color(if identifying_elapsed.is_some() {
                    LOXONE_GREEN
                } else if is_selected {
                    ACCENT_BLUE
                } else {
                    TEXT_PRIMARY
                }),
            );
            if let Some(sub) = &subtitle {
                ui.label(RichText::new(format!("({})", sub)).size(11.5).color(TEXT_MUTED));
            }
            if let Some(elapsed) = identifying_elapsed {
                if elapsed < 0.25 {
                    resp.scroll_to_me(Some(egui::Align::Center));
                }
                render_identifying_badge(ui, node, elapsed);
            }
            if resp.clicked() && !node.uuid.is_empty() {
                action = Some(TreeAction::SelectObject(node.uuid.clone()));
            }
            attach_tree_node_context_menu(&resp, project, node, placeable, &mut action);
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
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Sample.Loxone");
        let project = load_loxone_file(&path).expect("Sample.Loxone should parse");
        assert!(!project.periphery_tree.is_empty());
        assert!(node_matches_query(&project.periphery_tree[0], ""));
        assert!(node_matches_query(&project.periphery_tree[0], "loxlive"));
        assert!(!node_matches_query(&project.periphery_tree[0], "nonexistent_xyz_device_999"));
    }

    #[test]
    fn test_tree_node_subtitles_match_reference_screenshot() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Sample.Loxone");
        let project = load_loxone_file(&path).expect("Sample.Loxone should parse");

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

        let ms = find_by_title(&project.periphery_tree, "Sample").unwrap();
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

    #[test]
    fn test_four_mode_project_trees_and_block_subtitles() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Sample.Loxone");
        let project = load_loxone_file(&path).expect("Sample.Loxone should parse");

        assert_eq!(LeftTab::Periphery.dropdown_label(), "Periphery");
        assert_eq!(LeftTab::Pages.dropdown_label(), "Configuration");
        assert_eq!(LeftTab::Rooms.dropdown_label(), "Rooms");
        assert_eq!(LeftTab::System.dropdown_label(), "Devices");

        let page = &project.pages[0];
        let has_subtitle = page
            .blocks
            .iter()
            .filter(|b| b.has_position)
            .any(|b| block_room_category_subtitle(&project, page, b).is_some());
        assert!(has_subtitle);

        let ctx = egui::Context::default();
        for mut tab in [
            LeftTab::Periphery,
            LeftTab::Pages,
            LeftTab::Rooms,
            LeftTab::System,
        ] {
            let mut filter = String::new();
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let _ = render_left_explorer(ui, &project, &mut tab, &mut filter, 0, None, None);
                });
            });
        }
    }
}
