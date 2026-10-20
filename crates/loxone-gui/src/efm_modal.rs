use egui::{
    Align, Color32, ComboBox, Context, CornerRadius, Id, Key, Layout, RichText, ScrollArea,
    Stroke, TextEdit, Vec2,
};
use loxone_core::efm_config::{
    collect_efm_candidate_objects, resolve_efm_object_label, EfmConfig, EfmNode, EfmNodeType,
    EfmObjectCandidate, StatInterval,
};
use loxone_core::model::LoxoneProject;
use loxone_core::writer::ProjectEdit;

use crate::i18n::tr;
use crate::theme::{BORDER_SUBTLE, CARD_BG, PANEL_BG, TEXT_MUTED, TEXT_PRIMARY};

const LOXONE_GREEN: Color32 = Color32::from_rgb(0x69, 0xC8, 0x32);
const ROW_BG_EVEN: Color32 = Color32::from_rgb(0x23, 0x27, 0x2E);
const ROW_BG_ODD: Color32 = Color32::from_rgb(0x29, 0x2E, 0x36);
const ROW_BG_SELECTED: Color32 = Color32::from_rgb(0x2C, 0x3E, 0x28);
const HEADER_BG: Color32 = Color32::from_rgb(0x1D, 0x20, 0x26);

#[derive(Debug, Clone, Default)]
pub struct EfmModalState {
    pub is_open: bool,
    pub page_uuid: String,
    pub block_uuid: String,
    pub block_title: String,
    pub config: Option<EfmConfig>,
    pub candidates: Vec<EfmObjectCandidate>,
    pub selected_node_uuid: Option<String>,
}

impl EfmModalState {
    pub fn open_for_block(&mut self, project: &LoxoneProject, block_uuid: &str) -> bool {
        let mut found = None;
        for page in &project.pages {
            if let Some(blk) = page.blocks.iter().find(|b| b.uuid == block_uuid) {
                if EfmConfig::is_efm_block(&blk.block_type) {
                    found = Some((page.uuid.clone(), blk.clone()));
                }
                break;
            }
        }
        let Some((page_uuid, block)) = found else {
            return false;
        };

        let cfg = EfmConfig::from_block(&block);
        let first_sel = cfg.root.children.first().map(|n| n.uuid.clone());

        self.is_open = true;
        self.page_uuid = page_uuid;
        self.block_uuid = block.uuid.clone();
        self.block_title = if block.title.is_empty() {
            tr("Energy Flow Monitor").to_string()
        } else {
            block.title.clone()
        };
        self.config = Some(cfg);
        self.candidates = collect_efm_candidate_objects(project);
        self.selected_node_uuid = first_sel;
        true
    }
}

fn translate_efm_node_type(nt: EfmNodeType) -> &'static str {
    match nt {
        EfmNodeType::Grid => tr("Grid"),
        EfmNodeType::Battery => tr("Battery"),
        EfmNodeType::Consumer => tr("Consumer"),
        EfmNodeType::Production => tr("Production"),
        EfmNodeType::Group => tr("Group"),
    }
}

fn efm_node_icon(nt: EfmNodeType) -> &'static str {
    match nt {
        EfmNodeType::Grid => "⚡",
        EfmNodeType::Battery => "🔋",
        EfmNodeType::Consumer => "🔌",
        EfmNodeType::Production => "☀",
        EfmNodeType::Group => "📁",
    }
}

fn translate_stat_interval(si: StatInterval) -> &'static str {
    match si {
        StatInterval::Off => tr("Off"),
        StatInterval::Min5 => tr("5 Minutes"),
        StatInterval::Min15 => tr("15 Minutes"),
        StatInterval::Min30 => tr("30 Minutes"),
        StatInterval::Min60 => tr("60 Minutes"),
    }
}

pub fn render_efm_modal(
    ctx: &Context,
    state: &mut EfmModalState,
    project: Option<&LoxoneProject>,
) -> Option<ProjectEdit> {
    if !state.is_open {
        return None;
    }

    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        state.is_open = false;
        return None;
    }

    let mut pending_edit: Option<ProjectEdit> = None;
    let mut close_requested = false;

    let screen_rect = ctx.screen_rect();
    let modal_w = (screen_rect.width() * 0.86).clamp(760.0, 1080.0);
    let modal_h = (screen_rect.height() * 0.80).clamp(480.0, 760.0);

    let (backdrop_clicked, popup_was_open) =
        crate::theme::draw_modal_backdrop(ctx, "efm_modal_backdrop", egui::Order::Foreground, 150);

    let win_resp = egui::Window::new(tr("Energy Flow Monitor"))
        .id(Id::new("efm_config_modal_window"))
        .order(egui::Order::Tooltip)
        .collapsible(false)
        .resizable(true)
        .title_bar(false)
        .fixed_size(Vec2::new(modal_w, modal_h))
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(PANEL_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(egui::Margin::same(14)),
        )
        .show(ctx, |ui| {
            let Some(cfg) = state.config.as_mut() else {
                close_requested = true;
                return;
            };

            // Header bar matching feat02-energy.png
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("⚡ {} — {}", tr("Energy Flow Monitor"), cfg.root.title))
                        .size(16.0)
                        .strong()
                        .color(TEXT_PRIMARY),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if crate::light_controller_modal::draw_close_button(ui) {
                        close_requested = true;
                    }
                });
            });

            ui.add_space(6.0);

            // Top settings bar: Root Name, Power/Energy Units, and Statistics Intervals (feat02-energy.png)
            egui::Frame::new()
                .fill(CARD_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .corner_radius(CornerRadius::same(4))
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(format!("{}:", tr("Name"))).size(12.0).color(TEXT_PRIMARY));
                        ui.add(TextEdit::singleline(&mut cfg.root.title).desired_width(150.0));

                        ui.add_space(10.0);
                        ui.checkbox(&mut cfg.root.calc_rest, "Oblicz niezmierzone zużycie");
                        if cfg.root.calc_rest {
                            ui.add(
                                TextEdit::singleline(&mut cfg.root.rest_name)
                                    .desired_width(120.0)
                                    .hint_text("Nieokreślone"),
                            );
                        }

                        ui.add_space(10.0);
                        ui.label(RichText::new(format!("{}:", tr("Statistic Interval"))).size(12.0).color(TEXT_PRIMARY));
                        ComboBox::from_id_salt("efm_stat_power")
                            .width(110.0)
                            .selected_text(format!("Moc: {}", translate_stat_interval(cfg.stat_power_interval)))
                            .show_ui(ui, |ui| {
                                for &si in StatInterval::all() {
                                    ui.selectable_value(
                                        &mut cfg.stat_power_interval,
                                        si,
                                        translate_stat_interval(si),
                                    );
                                }
                            });
                        ComboBox::from_id_salt("efm_stat_energy")
                            .width(120.0)
                            .selected_text(format!("Energia: {}", translate_stat_interval(cfg.stat_energy_interval)))
                            .show_ui(ui, |ui| {
                                for &si in StatInterval::all() {
                                    ui.selectable_value(
                                        &mut cfg.stat_energy_interval,
                                        si,
                                        translate_stat_interval(si),
                                    );
                                }
                            });
                    });
                });

            ui.add_space(8.0);

            // Action toolbar (+ Add Node, + Add Group, Delete)
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(tr("+ Add Node"))
                                .size(12.5)
                                .strong()
                                .color(Color32::BLACK),
                        )
                        .fill(LOXONE_GREEN)
                        .corner_radius(CornerRadius::same(4))
                        .min_size(Vec2::new(110.0, 26.0)),
                    )
                    .clicked()
                {
                    let new_child = EfmNode::new_child("", EfmNodeType::Consumer, "");
                    let new_uuid = new_child.uuid.clone();
                    // If a group is selected, add inside that group; otherwise add at root level
                    let mut added_to_group = false;
                    if let Some(sel) = &state.selected_node_uuid {
                        if let Some(grp) = cfg
                            .root
                            .children
                            .iter_mut()
                            .find(|n| &n.uuid == sel && n.node_type == EfmNodeType::Group)
                        {
                            grp.children.push(new_child.clone());
                            added_to_group = true;
                        }
                    }
                    if !added_to_group {
                        cfg.root.children.push(new_child);
                    }
                    state.selected_node_uuid = Some(new_uuid);
                }

                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(tr("+ Add Group"))
                                .size(12.5)
                                .color(TEXT_PRIMARY),
                        )
                        .fill(CARD_BG)
                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                        .corner_radius(CornerRadius::same(4))
                        .min_size(Vec2::new(110.0, 26.0)),
                    )
                    .clicked()
                {
                    let new_grp = EfmNode::new_child(&tr("Group"), EfmNodeType::Group, "");
                    let new_uuid = new_grp.uuid.clone();
                    cfg.root.children.push(new_grp);
                    state.selected_node_uuid = Some(new_uuid);
                }

                let can_delete = state.selected_node_uuid.is_some();
                if ui
                    .add_enabled(
                        can_delete,
                        egui::Button::new(
                            RichText::new(format!("🗑 {}", tr("Delete")))
                                .size(12.5)
                                .color(if can_delete { TEXT_PRIMARY } else { TEXT_MUTED }),
                        )
                        .fill(CARD_BG)
                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                        .corner_radius(CornerRadius::same(4))
                        .min_size(Vec2::new(90.0, 26.0)),
                    )
                    .clicked()
                {
                    if let Some(sel_u) = state.selected_node_uuid.take() {
                        remove_efm_node_by_uuid(&mut cfg.root.children, &sel_u);
                    }
                }
            });

            ui.add_space(8.0);

            // Table container matching feat02-energy.png
            let footer_h = 46.0;
            let table_h = (ui.available_height() - footer_h).max(180.0);

            egui::Frame::new()
                .fill(CARD_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .corner_radius(CornerRadius::same(4))
                .show(ui, |ui| {
                    ui.set_min_height(table_h);
                    ui.set_max_height(table_h);

                    let col_name_w = 240.0;
                    let col_type_w = 185.0;
                    let col_object_w = 340.0;
                    let col_rest_w = 150.0;

                    egui::Frame::new()
                        .fill(HEADER_BG)
                        .inner_margin(egui::Margin::symmetric(10, 7))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.allocate_ui(Vec2::new(col_name_w, 18.0), |ui| {
                                    ui.label(
                                        RichText::new(tr("Name"))
                                            .size(12.0)
                                            .strong()
                                            .color(TEXT_PRIMARY),
                                    );
                                });
                                ui.allocate_ui(Vec2::new(col_type_w, 18.0), |ui| {
                                    ui.label(
                                        RichText::new(tr("Type"))
                                            .size(12.0)
                                            .strong()
                                            .color(TEXT_PRIMARY),
                                    );
                                });
                                ui.allocate_ui(Vec2::new(col_object_w, 18.0), |ui| {
                                    ui.label(
                                        RichText::new(tr("Object"))
                                            .size(12.0)
                                            .strong()
                                            .color(TEXT_PRIMARY),
                                    );
                                });
                                ui.allocate_ui(Vec2::new(col_rest_w, 18.0), |ui| {
                                    ui.label(
                                        RichText::new("Niezmierzone zużycie")
                                            .size(12.0)
                                            .strong()
                                            .color(TEXT_PRIMARY),
                                    );
                                });
                            });
                        });

                    let mut delete_uuid: Option<String> = None;
                    let mut vis_row = 0usize;

                    ScrollArea::vertical()
                        .id_salt("efm_nodes_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for top_node in &mut cfg.root.children {
                                render_efm_node_row(
                                    ui,
                                    top_node,
                                    0,
                                    &mut vis_row,
                                    &state.candidates,
                                    project,
                                    &mut state.selected_node_uuid,
                                    &mut delete_uuid,
                                    col_name_w,
                                    col_type_w,
                                    col_object_w,
                                    col_rest_w,
                                );
                            }
                        });

                    if let Some(del_u) = delete_uuid {
                        remove_efm_node_by_uuid(&mut cfg.root.children, &del_u);
                        if state.selected_node_uuid.as_deref() == Some(del_u.as_str()) {
                            state.selected_node_uuid = None;
                        }
                    }
                });

            ui.add_space(8.0);

            // Bottom OK / Cancel bar
            ui.horizontal(|ui| {
                let total_nodes: usize = cfg
                    .root
                    .children
                    .iter()
                    .map(|n| 1 + n.children.len())
                    .sum();
                ui.label(
                    RichText::new(format!("{} {}", total_nodes, tr("Objects")))
                        .size(12.0)
                        .color(TEXT_MUTED),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(tr("Cancel")).size(13.0).color(TEXT_PRIMARY),
                            )
                            .fill(CARD_BG)
                            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                            .corner_radius(CornerRadius::same(4))
                            .min_size(Vec2::new(88.0, 28.0)),
                        )
                        .clicked()
                    {
                        close_requested = true;
                    }

                    ui.add_space(6.0);

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(tr("OK"))
                                    .size(13.0)
                                    .strong()
                                    .color(Color32::BLACK),
                            )
                            .fill(LOXONE_GREEN)
                            .corner_radius(CornerRadius::same(4))
                            .min_size(Vec2::new(88.0, 28.0)),
                        )
                        .clicked()
                    {
                        if let Some(proj) = project {
                            for page in &proj.pages {
                                if page.uuid == state.page_uuid {
                                    if let Some(blk) =
                                        page.blocks.iter().find(|b| b.uuid == state.block_uuid)
                                    {
                                        pending_edit = Some(cfg.to_project_edits(blk));
                                        break;
                                    }
                                }
                            }
                        }
                        close_requested = true;
                    }
                });
            });
        });

    let win_rect = win_resp.map(|r| r.response.rect);
    if crate::theme::should_close_modal_on_outside_click(
        ctx,
        "efm_config_modal_window",
        backdrop_clicked,
        popup_was_open,
        win_rect,
    ) {
        close_requested = true;
    }

    if close_requested {
        state.is_open = false;
    }

    pending_edit
}

#[allow(clippy::too_many_arguments)]
fn render_efm_node_row(
    ui: &mut egui::Ui,
    node: &mut EfmNode,
    depth: usize,
    vis_row: &mut usize,
    candidates: &[EfmObjectCandidate],
    project: Option<&LoxoneProject>,
    selected_node_uuid: &mut Option<String>,
    delete_uuid: &mut Option<String>,
    col_name_w: f32,
    col_type_w: f32,
    col_object_w: f32,
    col_rest_w: f32,
) {
    let is_selected = selected_node_uuid.as_deref() == Some(node.uuid.as_str());
    let row_bg = if is_selected {
        ROW_BG_SELECTED
    } else if *vis_row % 2 == 0 {
        ROW_BG_EVEN
    } else {
        ROW_BG_ODD
    };
    *vis_row += 1;

    let node_uuid = node.uuid.clone();
    egui::Frame::new()
        .fill(row_bg)
        .inner_margin(egui::Margin::symmetric(10, 5))
        .show(ui, |ui| {
            let row_resp = ui.horizontal(|ui| {
                // 1. Name / Group Title (with tree indentation)
                ui.allocate_ui(Vec2::new(col_name_w, 24.0), |ui| {
                    ui.horizontal(|ui| {
                        if depth > 0 {
                            ui.add_space(18.0 * depth as f32);
                            ui.label(RichText::new("└").size(12.0).color(TEXT_MUTED));
                        }
                        ui.label(RichText::new(efm_node_icon(node.node_type)).size(13.5));
                        let edit_w = (col_name_w - 42.0 - 18.0 * depth as f32).max(80.0);
                        let hint = if !node.ctrl_uuid.is_empty() {
                            project
                                .map(|p| resolve_efm_object_label(p, &node.ctrl_uuid))
                                .unwrap_or_default()
                        } else {
                            translate_efm_node_type(node.node_type).to_string()
                        };
                        ui.add(
                            TextEdit::singleline(&mut node.title)
                                .desired_width(edit_w)
                                .hint_text(hint),
                        );
                    });
                });

                // 2. Type column
                ui.allocate_ui(Vec2::new(col_type_w, 24.0), |ui| {
                    ComboBox::from_id_salt(("efm_type", &node.uuid))
                        .width(col_type_w - 14.0)
                        .selected_text(translate_efm_node_type(node.node_type))
                        .show_ui(ui, |ui| {
                            for &nt in EfmNodeType::all_child_types() {
                                if ui
                                    .selectable_label(
                                        node.node_type == nt,
                                        format!("{} {}", efm_node_icon(nt), translate_efm_node_type(nt)),
                                    )
                                    .clicked()
                                {
                                    node.node_type = nt;
                                    if nt == EfmNodeType::Group {
                                        node.ctrl_uuid.clear();
                                    }
                                }
                            }
                        });
                });

                // 3. Linked Meter / Block Object column
                ui.allocate_ui(Vec2::new(col_object_w, 24.0), |ui| {
                    if node.node_type == EfmNodeType::Group {
                        ui.label(
                            RichText::new(format!("({} elementów)", node.children.len()))
                                .size(12.0)
                                .color(TEXT_MUTED),
                        );
                    } else {
                        let current_label = if node.ctrl_uuid.is_empty() {
                            tr("(none)").to_string()
                        } else {
                            candidates
                                .iter()
                                .find(|c| c.uuid == node.ctrl_uuid)
                                .map(|c| c.display_label.clone())
                                .or_else(|| {
                                    project.map(|p| resolve_efm_object_label(p, &node.ctrl_uuid))
                                })
                                .unwrap_or_else(|| node.ctrl_uuid.clone())
                        };

                        ComboBox::from_id_salt(("efm_obj", &node.uuid))
                            .width(col_object_w - 14.0)
                            .selected_text(current_label)
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(node.ctrl_uuid.is_empty(), tr("(none)"))
                                    .clicked()
                                {
                                    node.ctrl_uuid.clear();
                                }
                                for cand in candidates {
                                    let is_sel = node.ctrl_uuid == cand.uuid;
                                    if ui
                                        .selectable_label(is_sel, &cand.display_label)
                                        .clicked()
                                    {
                                        node.ctrl_uuid = cand.uuid.clone();
                                    }
                                }
                            });
                    }
                });

                // 4. Group unmeasured consumption toggle + Delete button
                ui.allocate_ui(Vec2::new(col_rest_w, 24.0), |ui| {
                    ui.horizontal(|ui| {
                        if node.node_type == EfmNodeType::Group {
                            ui.checkbox(&mut node.calc_rest, "Oblicz resztę");
                        } else {
                            ui.add_space(92.0);
                        }
                        if ui
                            .small_button(RichText::new("🗑").size(12.0).color(TEXT_MUTED))
                            .on_hover_text(tr("Delete"))
                            .clicked()
                        {
                            *delete_uuid = Some(node.uuid.clone());
                        }
                    });
                });
            });

            if row_resp.response.clicked() {
                *selected_node_uuid = Some(node_uuid);
            }
        });

    for child in &mut node.children {
        render_efm_node_row(
            ui,
            child,
            depth + 1,
            vis_row,
            candidates,
            project,
            selected_node_uuid,
            delete_uuid,
            col_name_w,
            col_type_w,
            col_object_w,
            col_rest_w,
        );
    }
}

fn remove_efm_node_by_uuid(nodes: &mut Vec<EfmNode>, target_uuid: &str) -> bool {
    if let Some(pos) = nodes.iter().position(|n| n.uuid == target_uuid) {
        nodes.remove(pos);
        return true;
    }
    for n in nodes {
        if remove_efm_node_by_uuid(&mut n.children, target_uuid) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_efm_modal_open_and_hierarchy_from_klimczaki() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Klimczaki.Loxone");
        let xml = std::fs::read_to_string(&path).expect("read Klimczaki.Loxone");
        let project = loxone_core::parser::parse_loxone_xml(&xml).expect("parse Klimczaki.Loxone");

        let mut modal = EfmModalState::default();
        assert!(modal.open_for_block(&project, "1d04656b-0128-8d7a-ffff51535bcccc04"));
        assert!(modal.is_open);
        let cfg = modal.config.as_ref().expect("efm config loaded");
        assert_eq!(cfg.root.title, "Monitor przepływu energii");
        assert_eq!(cfg.root.children.len(), 2);
        assert_eq!(cfg.root.children[0].title, "Fotowoltaika");
        assert_eq!(cfg.root.children[1].title, "Licznik główny");
    }
}
