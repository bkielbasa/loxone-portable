use std::collections::HashMap;

use egui::{
    Align, Color32, ComboBox, Context, CornerRadius, DragValue, Id, Key, Layout, Pos2, Rect,
    RichText, ScrollArea, Sense, Slider, Stroke, TextEdit, Ui, Vec2,
};
use loxone_core::light_config::{
    collect_operating_modes, AutomationAction, HclMode, LightAutomationRule, LightCircuitType,
    LightControllerConfig, LightScene,
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
const TAB_ACTIVE_BG: Color32 = Color32::from_rgb(0x30, 0x38, 0x42);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LightControllerTab {
    #[default]
    Circuits,
    Moods,
    Automation,
    DaylightControl,
}

#[derive(Debug, Clone, Default)]
pub struct LightControllerModalState {
    pub is_open: bool,
    pub page_uuid: String,
    pub block_uuid: String,
    pub block_title: String,
    pub active_tab: LightControllerTab,
    pub config: Option<LightControllerConfig>,
    pub operating_modes: Vec<(i32, String)>,
    pub connected_targets: HashMap<usize, String>,
    pub selected_scene_idx: usize,
}

impl LightControllerModalState {
    pub fn open_for_block(&mut self, project: &LoxoneProject, block_uuid: &str) -> bool {
        let mut found = None;
        for page in &project.pages {
            if let Some(blk) = page.blocks.iter().find(|b| b.uuid == block_uuid) {
                if LightControllerConfig::is_light_controller_block(&blk.block_type) {
                    found = Some((page.clone(), blk.clone()));
                }
                break;
            }
        }
        let Some((page, block)) = found else {
            return false;
        };

        self.is_open = true;
        self.page_uuid = page.uuid.clone();
        self.block_uuid = block.uuid.clone();
        self.block_title = if block.title.is_empty() {
            tr("Light Controller").to_string()
        } else {
            block.title.clone()
        };
        self.active_tab = LightControllerTab::Circuits;
        self.config = Some(LightControllerConfig::from_block(&block));
        self.operating_modes = collect_operating_modes(project, tr("No specification"));
        self.selected_scene_idx = 0;

        // Resolve connected target labels for Lc1..Lc8 (`AQ1`..`AQ8` / `Q1`..`Q8`)
        self.connected_targets.clear();
        let mut pin_uuid_to_idx: HashMap<String, usize> = HashMap::new();
        for pin in &block.pins {
            let idx_opt = pin
                .key
                .strip_prefix("AQ")
                .or_else(|| pin.key.strip_prefix("Q"))
                .or_else(|| pin.key.strip_prefix("Lc"))
                .and_then(|num| num.parse::<usize>().ok())
                .filter(|&n| (1..=8).contains(&n))
                .map(|n| n - 1);
            if let Some(idx) = idx_opt {
                pin_uuid_to_idx.insert(pin.uuid.clone(), idx);
            }
        }

        for other in &page.blocks {
            if other.uuid == block.uuid {
                continue;
            }
            for pin in &other.pins {
                for link in &pin.incoming_links {
                    if let Some(&idx) = pin_uuid_to_idx.get(&link.source_pin_uuid) {
                        let label = if !other.title.trim().is_empty() {
                            other.title.clone()
                        } else {
                            other.block_type.clone()
                        };
                        self.connected_targets
                            .entry(idx)
                            .and_modify(|existing| {
                                if !existing.contains(&label) {
                                    existing.push_str(", ");
                                    existing.push_str(&label);
                                }
                            })
                            .or_insert(label);
                    }
                }
            }
        }

        true
    }

    pub fn format_scene_circuits_summary(&self, scene: &LightScene) -> String {
        let Some(cfg) = &self.config else {
            return String::new();
        };
        if scene.sid == 778 {
            return tr("All circuits off").to_string();
        }
        let mut parts = Vec::new();
        for (i, circuit) in cfg.circuits.iter().enumerate() {
            let pct = scene.circuit_pcts[i];
            if pct > 0 {
                let name = if circuit.name.trim().is_empty() || circuit.name == "nieznane" {
                    circuit.code_label()
                } else {
                    circuit.name.clone()
                };
                if circuit.circuit_type.is_dimmer_like() {
                    parts.push(format!("{}: {}%", name, pct));
                } else {
                    parts.push(format!("{}: {}", name, tr("On")));
                }
            }
        }
        if parts.is_empty() {
            tr("All off").to_string()
        } else {
            parts.join("  •  ")
        }
    }
}

fn translate_circuit_type(ct: LightCircuitType) -> &'static str {
    match ct {
        LightCircuitType::Switch => tr("Switch (Relay)"),
        LightCircuitType::Dimmer0To100 => tr("Dimmer 0-100%"),
        LightCircuitType::Dimmer0To10V => tr("Dimmer 0-10V"),
        LightCircuitType::Dimmer1To10V => tr("Dimmer 1-10V"),
        LightCircuitType::Rgb => tr("Color Lighting"),
        LightCircuitType::LumitechDmx => tr("Lumitech DMX"),
        LightCircuitType::SmartActuator => tr("Smart Actuator (Tree/Air)"),
    }
}

fn translate_automation_action(act: AutomationAction) -> &'static str {
    match act {
        AutomationAction::ConstantBrightness => tr("Constant brightness control"),
        AutomationAction::AdjustBrightness => tr("Adjust brightness"),
        AutomationAction::KeepBrightness => tr("Do not change brightness"),
    }
}

fn translate_hcl_mode(mode: HclMode) -> &'static str {
    match mode {
        HclMode::SunriseSunset => tr("Automatic (Sunrise / Sunset)"),
        HclMode::FixedPeriod => tr("Fixed hours"),
    }
}

pub(crate) fn draw_close_button(ui: &mut Ui) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(24.0, 24.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if resp.hovered() {
            painter.rect_filled(
                rect,
                CornerRadius::same(4),
                Color32::from_rgb(0x38, 0x3F, 0x4A),
            );
        }
        let color = if resp.hovered() {
            Color32::WHITE
        } else {
            TEXT_MUTED
        };
        let pad = 6.5;
        let stroke = Stroke::new(1.8_f32, color);
        painter.line_segment(
            [
                Pos2::new(rect.left() + pad, rect.top() + pad),
                Pos2::new(rect.right() - pad, rect.bottom() - pad),
            ],
            stroke,
        );
        painter.line_segment(
            [
                Pos2::new(rect.right() - pad, rect.top() + pad),
                Pos2::new(rect.left() + pad, rect.bottom() - pad),
            ],
            stroke,
        );
    }
    resp.clicked()
}

fn draw_loxone_checkbox(ui: &mut Ui, checked: &mut bool, enabled: bool) -> bool {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), sense);
    let mut changed = false;
    if enabled && resp.clicked() {
        *checked = !*checked;
        changed = true;
    }
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if *checked {
            let fill = if enabled {
                LOXONE_GREEN
            } else {
                Color32::from_rgb(0x4A, 0x7A, 0x2E)
            };
            painter.rect(
                rect,
                CornerRadius::same(2),
                fill,
                Stroke::new(1.0_f32, fill),
                egui::StrokeKind::Inside,
            );
            let p1 = Pos2::new(rect.min.x + 3.5, rect.min.y + 8.2);
            let p2 = Pos2::new(rect.min.x + 6.8, rect.min.y + 11.5);
            let p3 = Pos2::new(rect.min.x + 12.6, rect.min.y + 4.5);
            let stroke = Stroke::new(1.9_f32, Color32::WHITE);
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
        } else {
            let border = if enabled && resp.hovered() {
                LOXONE_GREEN
            } else if enabled {
                Color32::from_rgb(0x68, 0x72, 0x82)
            } else {
                Color32::from_rgb(0x3A, 0x40, 0x4A)
            };
            let bg = if enabled {
                Color32::from_rgb(0x1A, 0x1D, 0x22)
            } else {
                Color32::from_rgb(0x20, 0x24, 0x2B)
            };
            painter.rect(
                rect,
                CornerRadius::same(2),
                bg,
                Stroke::new(1.2_f32, border),
                egui::StrokeKind::Inside,
            );
        }
    }
    changed
}

fn render_header_cell(ui: &mut Ui, width: f32, label: &str, centered: bool) {
    let layout = if centered {
        Layout::centered_and_justified(egui::Direction::LeftToRight)
    } else {
        Layout::left_to_right(Align::Center)
    };
    ui.allocate_ui_with_layout(Vec2::new(width, 20.0), layout, |ui| {
        ui.set_clip_rect(ui.clip_rect().intersect(ui.max_rect()));
        ui.add(
            egui::Label::new(
                RichText::new(label)
                    .size(11.5)
                    .strong()
                    .color(Color32::from_rgb(0xD4, 0xDA, 0xE2)),
            )
            .truncate(),
        );
    });
}

fn render_text_cell(
    ui: &mut Ui,
    width: f32,
    height: f32,
    text: &str,
    color: Color32,
    strong: bool,
    centered: bool,
) -> egui::Response {
    let layout = if centered {
        Layout::centered_and_justified(egui::Direction::LeftToRight)
    } else {
        Layout::left_to_right(Align::Center)
    };
    ui.allocate_ui_with_layout(Vec2::new(width, height), layout, |ui| {
        ui.set_clip_rect(ui.clip_rect().intersect(ui.max_rect()));
        let mut rt = RichText::new(text).size(12.5).color(color);
        if strong {
            rt = rt.strong();
        }
        ui.add(egui::Label::new(rt).truncate().sense(Sense::click()))
    })
    .inner
}

pub fn render_light_controller_modal(
    ctx: &Context,
    state: &mut LightControllerModalState,
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
    let modal_w = (screen_rect.width() * 0.88).clamp(840.0, 1140.0);
    let modal_h = (screen_rect.height() * 0.84).clamp(540.0, 780.0);

    let (backdrop_clicked, popup_was_open) = crate::theme::draw_modal_backdrop(
        ctx,
        "light_controller_modal_backdrop",
        egui::Order::Foreground,
        150,
    );

    let win_resp = egui::Window::new(tr("Light Controller"))
        .id(Id::new("light_controller_modal_window"))
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
            if state.config.is_none() {
                close_requested = true;
                return;
            }

            // Header with crisp painted bulb badge and vector X button
            ui.horizontal(|ui| {
                let (badge_rect, _) = ui.allocate_exact_size(Vec2::new(18.0, 18.0), Sense::hover());
                if ui.is_rect_visible(badge_rect) {
                    let p = ui.painter();
                    let c = badge_rect.center();
                    p.circle_filled(
                        Pos2::new(c.x, c.y - 1.5),
                        6.0,
                        Color32::from_rgb(0xF5, 0xB0, 0x41),
                    );
                    p.rect_filled(
                        Rect::from_center_size(Pos2::new(c.x, c.y + 5.5), Vec2::new(6.0, 3.0)),
                        CornerRadius::same(1),
                        Color32::from_rgb(0x9E, 0xA7, 0xB3),
                    );
                }
                ui.label(
                    RichText::new(format!(
                        "{} ({})",
                        tr("Light Controller"),
                        state.block_title
                    ))
                    .size(16.0)
                    .strong()
                    .color(TEXT_PRIMARY),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if draw_close_button(ui) {
                        close_requested = true;
                    }
                });
            });

            ui.add_space(8.0);

            // 4-tab bar matching light01..light05
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let tabs = [
                    (LightControllerTab::Circuits, tr("Lighting Circuits")),
                    (LightControllerTab::Moods, tr("Moods")),
                    (LightControllerTab::Automation, tr("Automation")),
                    (
                        LightControllerTab::DaylightControl,
                        tr("Daylight Control Settings"),
                    ),
                ];
                for (tab, label) in tabs {
                    let is_active = state.active_tab == tab;
                    let btn = egui::Button::new(
                        RichText::new(label)
                            .size(12.5)
                            .strong()
                            .color(if is_active { LOXONE_GREEN } else { TEXT_PRIMARY }),
                    )
                    .fill(if is_active { TAB_ACTIVE_BG } else { CARD_BG })
                    .stroke(Stroke::new(
                        1.0_f32,
                        if is_active {
                            LOXONE_GREEN
                        } else {
                            BORDER_SUBTLE
                        },
                    ))
                    .corner_radius(CornerRadius::same(4))
                    .min_size(Vec2::new(135.0, 28.0));

                    if ui.add(btn).clicked() {
                        state.active_tab = tab;
                    }
                }
            });

            ui.add_space(8.0);

            // Strictly bounded body region so the bottom OK / Cancel bar is always visible
            let footer_reserve = 46.0;
            let body_h = (ui.available_height() - footer_reserve).max(260.0);
            let body_w = ui.available_width();

            ui.allocate_ui_with_layout(
                Vec2::new(body_w, body_h),
                Layout::top_down(Align::Min),
                |ui| {
                    ui.set_max_size(Vec2::new(body_w, body_h));
                    egui::Frame::new()
                        .fill(CARD_BG)
                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(egui::Margin::same(8))
                        .show(ui, |ui| {
                            let inner_h = (body_h - 18.0).max(220.0);
                            let inner_w = ui.available_width();
                            ui.allocate_ui_with_layout(
                                Vec2::new(inner_w, inner_h),
                                Layout::top_down(Align::Min),
                                |ui| {
                                    ui.set_max_size(Vec2::new(inner_w, inner_h));
                                    match state.active_tab {
                                        LightControllerTab::Circuits => {
                                            render_circuits_tab(ui, state);
                                        }
                                        LightControllerTab::Moods => {
                                            render_moods_tab(ui, state);
                                        }
                                        LightControllerTab::Automation => {
                                            render_automation_tab(ui, state);
                                        }
                                        LightControllerTab::DaylightControl => {
                                            render_hcl_tab(ui, state);
                                        }
                                    }
                                },
                            );
                        });
                },
            );

            ui.add_space(8.0);

            // Footer OK / Cancel
            ui.horizontal(|ui| {
                let scenes_len = state.config.as_ref().map_or(0, |c| c.scenes.len());
                ui.label(
                    RichText::new(format!(
                        "{}: 8  •  {}: {}",
                        tr("Lighting Circuits"),
                        tr("Moods"),
                        scenes_len
                    ))
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
                        if let (Some(proj), Some(cfg)) = (project, state.config.as_ref()) {
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
        "light_controller_modal_window",
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

fn render_circuits_tab(ui: &mut Ui, state: &mut LightControllerModalState) {
    let Some(cfg) = &mut state.config else {
        return;
    };

    let avail_w = ui.available_width();
    let col_code = 76.0;
    let col_name = 260.0;
    let col_type = 230.0;
    let col_conn = (avail_w - col_code - col_name - col_type - 48.0).max(180.0);

    egui::Frame::new()
        .fill(HEADER_BG)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                render_header_cell(ui, col_code, tr("Code"), false);
                render_header_cell(ui, col_name, tr("Name"), false);
                render_header_cell(ui, col_type, tr("Type"), false);
                render_header_cell(ui, col_conn, tr("Connected Object"), false);
            });
        });

    let scroll_h = (ui.available_height() - 4.0).max(160.0);
    ScrollArea::vertical()
        .id_salt("lc_circuits_scroll")
        .max_height(scroll_h)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for (idx, circuit) in cfg.circuits.iter_mut().enumerate() {
                let row_bg = if idx % 2 == 0 {
                    ROW_BG_EVEN
                } else {
                    ROW_BG_ODD
                };
                egui::Frame::new()
                    .fill(row_bg)
                    .inner_margin(egui::Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            render_text_cell(
                                ui,
                                col_code,
                                24.0,
                                &circuit.code_label(),
                                LOXONE_GREEN,
                                true,
                                false,
                            );

                            ui.allocate_ui_with_layout(
                                Vec2::new(col_name, 24.0),
                                Layout::left_to_right(Align::Center),
                                |ui| {
                                    ui.add(
                                        TextEdit::singleline(&mut circuit.name)
                                            .desired_width(col_name - 8.0)
                                            .hint_text("nieznane"),
                                    );
                                },
                            );

                            ui.allocate_ui_with_layout(
                                Vec2::new(col_type, 24.0),
                                Layout::left_to_right(Align::Center),
                                |ui| {
                                    ui.push_id(("lc_type_combo", idx), |ui| {
                                        ComboBox::from_id_salt(("lc_type_cb", idx))
                                            .width(col_type - 10.0)
                                            .selected_text(translate_circuit_type(
                                                circuit.circuit_type,
                                            ))
                                            .show_ui(ui, |ui| {
                                                for &ct in LightCircuitType::all() {
                                                    ui.selectable_value(
                                                        &mut circuit.circuit_type,
                                                        ct,
                                                        translate_circuit_type(ct),
                                                    );
                                                }
                                            });
                                    });
                                },
                            );

                            let connected = state
                                .connected_targets
                                .get(&idx)
                                .cloned()
                                .unwrap_or_else(|| "—".to_string());
                            render_text_cell(
                                ui,
                                col_conn,
                                24.0,
                                &connected,
                                TEXT_MUTED,
                                false,
                                false,
                            );
                        });
                    });
            }
        });
}

fn render_moods_tab(ui: &mut Ui, state: &mut LightControllerModalState) {
    // Top action bar: Add Mood / Delete Mood
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if ui
            .add(
                egui::Button::new(
                    RichText::new(tr("+ Add Mood"))
                        .size(12.5)
                        .strong()
                        .color(Color32::BLACK),
                )
                .fill(LOXONE_GREEN)
                .corner_radius(CornerRadius::same(4))
                .min_size(Vec2::new(124.0, 26.0)),
            )
            .clicked()
        {
            if let Some(cfg) = &mut state.config {
                let next_sid = cfg.next_available_sid();
                let new_name = format!("{} {}", tr("Mood"), next_sid);
                let new_scene = LightScene::new_user_scene(&new_name, next_sid, &cfg.circuits);
                // Insert before "Wył." (sid 778) if present at the end
                let off_pos = cfg.scenes.iter().position(|s| s.sid == 778);
                if let Some(pos) = off_pos {
                    cfg.scenes.insert(pos, new_scene);
                    state.selected_scene_idx = pos;
                } else {
                    cfg.scenes.push(new_scene);
                    state.selected_scene_idx = cfg.scenes.len().saturating_sub(1);
                }
            }
        }

        let can_delete = state
            .config
            .as_ref()
            .and_then(|c| c.scenes.get(state.selected_scene_idx))
            .is_some_and(|s| !s.read_only && s.sid != 778);

        if ui
            .add_enabled(
                can_delete,
                egui::Button::new(
                    RichText::new(tr("Delete Mood"))
                        .size(12.5)
                        .color(if can_delete { TEXT_PRIMARY } else { TEXT_MUTED }),
                )
                .fill(CARD_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .corner_radius(CornerRadius::same(4))
                .min_size(Vec2::new(114.0, 26.0)),
            )
            .clicked()
        {
            if let Some(cfg) = &mut state.config {
                if state.selected_scene_idx < cfg.scenes.len()
                    && !cfg.scenes[state.selected_scene_idx].read_only
                {
                    cfg.scenes.remove(state.selected_scene_idx);
                    if state.selected_scene_idx >= cfg.scenes.len() && !cfg.scenes.is_empty() {
                        state.selected_scene_idx = cfg.scenes.len() - 1;
                    }
                }
            }
        }
    });

    ui.add_space(6.0);

    let avail_w = ui.available_width();
    let col_input = 72.0;
    let col_name = 190.0;
    let col_id = 52.0;
    let col_mix = 86.0;
    let col_motion = 136.0;
    let col_circuits =
        (avail_w - col_input - col_name - col_id - col_mix - col_motion - 66.0).max(200.0);

    let scenes_count = state.config.as_ref().map_or(0, |c| c.scenes.len());
    let mut summaries = Vec::with_capacity(scenes_count);
    if let Some(cfg) = &state.config {
        for scene in &cfg.scenes {
            summaries.push(state.format_scene_circuits_summary(scene));
        }
    }

    let avail_h = ui.available_height();
    let max_table_rows_h = (avail_h * 0.38).clamp(115.0, 195.0);
    let natural_rows_h = (scenes_count as f32 * 34.0 + 4.0).max(76.0);
    let table_rows_h = natural_rows_h.min(max_table_rows_h);

    // Table header matching light02.png
    egui::Frame::new()
        .fill(HEADER_BG)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                render_header_cell(ui, col_input, tr("Input"), false);
                render_header_cell(ui, col_name, tr("Name"), false);
                render_header_cell(ui, col_id, tr("ID"), true);
                render_header_cell(ui, col_mix, tr("Mixing"), true);
                render_header_cell(ui, col_motion, tr("Default on motion"), true);
                render_header_cell(ui, col_circuits, tr("Lighting Circuits"), false);
            });
        });

    egui::Frame::new()
        .fill(ROW_BG_EVEN)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .show(ui, |ui| {
            ScrollArea::vertical()
                .id_salt("lc_moods_table_scroll")
                .max_height(table_rows_h)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let Some(cfg) = &mut state.config else {
                        return;
                    };
                    let mut move_default_selected_idx: Option<usize> = None;

                    for (idx, scene) in cfg.scenes.iter_mut().enumerate() {
                        let is_selected = idx == state.selected_scene_idx;
                        let row_bg = if is_selected {
                            ROW_BG_SELECTED
                        } else if idx % 2 == 0 {
                            ROW_BG_EVEN
                        } else {
                            ROW_BG_ODD
                        };

                        let frame_resp = egui::Frame::new()
                            .fill(row_bg)
                            .inner_margin(egui::Margin::symmetric(10, 5))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 8.0;

                                    // Column 1: Input ("Dis1", "Dis2", "-") — fixed width
                                    let input_color = if is_selected {
                                        LOXONE_GREEN
                                    } else {
                                        TEXT_PRIMARY
                                    };
                                    if render_text_cell(
                                        ui,
                                        col_input,
                                        22.0,
                                        &scene.input_label(),
                                        input_color,
                                        is_selected,
                                        false,
                                    )
                                    .clicked()
                                    {
                                        state.selected_scene_idx = idx;
                                    }

                                    // Column 2: Name (TextEdit or left-aligned read-only label)
                                    if scene.read_only {
                                        if render_text_cell(
                                            ui,
                                            col_name,
                                            22.0,
                                            &scene.name,
                                            TEXT_MUTED,
                                            false,
                                            false,
                                        )
                                        .clicked()
                                        {
                                            state.selected_scene_idx = idx;
                                        }
                                    } else {
                                        ui.allocate_ui_with_layout(
                                            Vec2::new(col_name, 22.0),
                                            Layout::left_to_right(Align::Center),
                                            |ui| {
                                                let resp = ui.add(
                                                    TextEdit::singleline(&mut scene.name)
                                                        .desired_width(col_name - 8.0),
                                                );
                                                if resp.clicked() || resp.gained_focus() {
                                                    state.selected_scene_idx = idx;
                                                }
                                            },
                                        );
                                    }

                                    // Column 3: ID (centered)
                                    if render_text_cell(
                                        ui,
                                        col_id,
                                        22.0,
                                        &scene.sid.to_string(),
                                        TEXT_MUTED,
                                        false,
                                        true,
                                    )
                                    .clicked()
                                    {
                                        state.selected_scene_idx = idx;
                                    }

                                    // Column 4: Mixing (centered Loxone checkbox)
                                    ui.allocate_ui_with_layout(
                                        Vec2::new(col_mix, 22.0),
                                        Layout::centered_and_justified(
                                            egui::Direction::LeftToRight,
                                        ),
                                        |ui| {
                                            if draw_loxone_checkbox(
                                                ui,
                                                &mut scene.mix_mode,
                                                !scene.read_only,
                                            ) {
                                                state.selected_scene_idx = idx;
                                            }
                                        },
                                    );

                                    // Column 5: Default on motion (centered Loxone checkbox)
                                    ui.allocate_ui_with_layout(
                                        Vec2::new(col_motion, 22.0),
                                        Layout::centered_and_justified(
                                            egui::Direction::LeftToRight,
                                        ),
                                        |ui| {
                                            if draw_loxone_checkbox(
                                                ui,
                                                &mut scene.move_default,
                                                !scene.read_only,
                                            ) {
                                                state.selected_scene_idx = idx;
                                                if scene.move_default {
                                                    move_default_selected_idx = Some(idx);
                                                }
                                            }
                                        },
                                    );

                                    // Column 6: Lighting Circuits summary (left-aligned)
                                    let summary = summaries.get(idx).map_or("", String::as_str);
                                    if render_text_cell(
                                        ui,
                                        col_circuits,
                                        22.0,
                                        summary,
                                        TEXT_PRIMARY,
                                        false,
                                        false,
                                    )
                                    .clicked()
                                    {
                                        state.selected_scene_idx = idx;
                                    }
                                });
                            });

                        if frame_resp.response.clicked() {
                            state.selected_scene_idx = idx;
                        }
                    }

                    if let Some(chosen_idx) = move_default_selected_idx {
                        for (i, s) in cfg.scenes.iter_mut().enumerate() {
                            if i != chosen_idx {
                                s.move_default = false;
                            }
                        }
                    }
                });
        });

    ui.add_space(8.0);

    // Bottom detail panel: circuit brightness / switch editors for selected mood (`light02.png` / `light03.png`)
    let Some(cfg) = &mut state.config else {
        return;
    };
    let circuits_snapshot = cfg.circuits.clone();
    let Some(scene) = cfg.scenes.get_mut(state.selected_scene_idx) else {
        return;
    };

    egui::Frame::new()
        .fill(HEADER_BG)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{}: {} (ID {})",
                        tr("Moods"),
                        scene.name,
                        scene.sid
                    ))
                    .size(13.0)
                    .strong()
                    .color(LOXONE_GREEN),
                );
            });
        });

    let detail_h = (ui.available_height() - 4.0).max(120.0);
    egui::Frame::new()
        .fill(ROW_BG_EVEN)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .show(ui, |ui| {
            ScrollArea::vertical()
                .id_salt("lc_mood_detail_scroll")
                .max_height(detail_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for (i, circuit) in circuits_snapshot.iter().enumerate() {
                        let title = if circuit.name.trim().is_empty() || circuit.name == "nieznane"
                        {
                            circuit.code_label()
                        } else {
                            format!("{} ({})", circuit.name, circuit.code_label())
                        };
                        let row_bg = if i % 2 == 0 { ROW_BG_EVEN } else { ROW_BG_ODD };
                        egui::Frame::new()
                            .fill(row_bg)
                            .inner_margin(egui::Margin::symmetric(10, 4))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 12.0;
                                    render_text_cell(
                                        ui,
                                        240.0,
                                        22.0,
                                        &title,
                                        TEXT_PRIMARY,
                                        false,
                                        false,
                                    );

                                    let slider_w = (ui.available_width() - 90.0).clamp(160.0, 440.0);
                                    ui.add_enabled_ui(!scene.read_only, |ui| {
                                        if circuit.circuit_type.is_dimmer_like() {
                                            ui.spacing_mut().slider_width = slider_w;
                                            ui.add(
                                                Slider::new(&mut scene.circuit_pcts[i], 0..=100)
                                                    .suffix("%")
                                                    .show_value(true),
                                            );
                                        } else {
                                            let mut is_on = scene.circuit_pcts[i] > 0;
                                            if draw_loxone_checkbox(
                                                ui,
                                                &mut is_on,
                                                !scene.read_only,
                                            ) {
                                                scene.circuit_pcts[i] = if is_on { 100 } else { 0 };
                                            }
                                            let lbl = if is_on { tr("On") } else { tr("Off") };
                                            ui.label(
                                                RichText::new(lbl).size(12.5).color(if is_on {
                                                    LOXONE_GREEN
                                                } else {
                                                    TEXT_MUTED
                                                }),
                                            );
                                        }
                                    });
                                });
                            });
                    }
                });
        });
}

fn render_automation_tab(ui: &mut Ui, state: &mut LightControllerModalState) {
    let op_modes = state.operating_modes.clone();
    let Some(cfg) = &mut state.config else {
        return;
    };

    ui.horizontal(|ui| {
        ui.add_space(10.0);
        if ui
            .add(
                egui::Button::new(
                    RichText::new(tr("+ Add Rule"))
                        .size(12.5)
                        .strong()
                        .color(Color32::BLACK),
                )
                .fill(LOXONE_GREEN)
                .corner_radius(CornerRadius::same(4))
                .min_size(Vec2::new(120.0, 26.0)),
            )
            .clicked()
        {
            let first_sid = cfg
                .scenes
                .iter()
                .find(|s| s.sid != 778)
                .map_or(1, |s| s.sid);
            cfg.automation_rules.push(LightAutomationRule {
                operating_mode_id: -1,
                scene_sid: first_sid,
                action: AutomationAction::ConstantBrightness,
            });
        }
    });

    ui.add_space(6.0);

    let col_mode = 250.0;
    let col_mood = 230.0;
    let col_behav = 270.0;

    egui::Frame::new()
        .fill(HEADER_BG)
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                render_header_cell(ui, col_mode, tr("Operating Mode"), false);
                render_header_cell(ui, col_mood, tr("Moods"), false);
                render_header_cell(ui, col_behav, tr("Behavior"), false);
            });
        });

    let scenes_choices: Vec<(i32, String)> = cfg
        .scenes
        .iter()
        .map(|s| (s.sid, format!("{} (ID {})", s.name, s.sid)))
        .collect();

    let mut remove_idx: Option<usize> = None;

    ScrollArea::vertical()
        .id_salt("lc_automation_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (idx, rule) in cfg.automation_rules.iter_mut().enumerate() {
                let row_bg = if idx % 2 == 0 {
                    ROW_BG_EVEN
                } else {
                    ROW_BG_ODD
                };
                egui::Frame::new()
                    .fill(row_bg)
                    .inner_margin(egui::Margin::symmetric(12, 7))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            let current_op_label = op_modes
                                .iter()
                                .find(|(id, _)| *id == rule.operating_mode_id)
                                .map(|(_, l)| l.clone())
                                .unwrap_or_else(|| tr("No specification").to_string());

                            ui.allocate_ui_with_layout(
                                Vec2::new(col_mode, 22.0),
                                Layout::left_to_right(Align::Center),
                                |ui| {
                                    ui.push_id(("auto_op", idx), |ui| {
                                        ComboBox::from_id_salt(("auto_op_cb", idx))
                                            .width(col_mode - 8.0)
                                            .selected_text(current_op_label)
                                            .show_ui(ui, |ui| {
                                                for (op_id, op_name) in &op_modes {
                                                    ui.selectable_value(
                                                        &mut rule.operating_mode_id,
                                                        *op_id,
                                                        op_name,
                                                    );
                                                }
                                            });
                                    });
                                },
                            );

                            let current_scene_label = scenes_choices
                                .iter()
                                .find(|(sid, _)| *sid == rule.scene_sid)
                                .map(|(_, l)| l.clone())
                                .unwrap_or_else(|| format!("ID {}", rule.scene_sid));

                            ui.allocate_ui_with_layout(
                                Vec2::new(col_mood, 22.0),
                                Layout::left_to_right(Align::Center),
                                |ui| {
                                    ui.push_id(("auto_scene", idx), |ui| {
                                        ComboBox::from_id_salt(("auto_scene_cb", idx))
                                            .width(col_mood - 8.0)
                                            .selected_text(current_scene_label)
                                            .show_ui(ui, |ui| {
                                                for (sid, s_label) in &scenes_choices {
                                                    ui.selectable_value(
                                                        &mut rule.scene_sid,
                                                        *sid,
                                                        s_label,
                                                    );
                                                }
                                            });
                                    });
                                },
                            );

                            ui.allocate_ui_with_layout(
                                Vec2::new(col_behav, 22.0),
                                Layout::left_to_right(Align::Center),
                                |ui| {
                                    ui.push_id(("auto_act", idx), |ui| {
                                        ComboBox::from_id_salt(("auto_act_cb", idx))
                                            .width(col_behav - 8.0)
                                            .selected_text(translate_automation_action(rule.action))
                                            .show_ui(ui, |ui| {
                                                for &act in AutomationAction::all() {
                                                    ui.selectable_value(
                                                        &mut rule.action,
                                                        act,
                                                        translate_automation_action(act),
                                                    );
                                                }
                                            });
                                    });
                                },
                            );

                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(tr("Delete Rule"))
                                            .size(11.5)
                                            .color(TEXT_MUTED),
                                    )
                                    .fill(CARD_BG)
                                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                                    .corner_radius(CornerRadius::same(4)),
                                )
                                .clicked()
                            {
                                remove_idx = Some(idx);
                            }
                        });
                    });
            }
        });

    if let Some(idx) = remove_idx {
        if idx < cfg.automation_rules.len() {
            cfg.automation_rules.remove(idx);
        }
    }
}

fn render_hcl_tab(ui: &mut Ui, state: &mut LightControllerModalState) {
    let Some(cfg) = &mut state.config else {
        return;
    };
    let hcl = &mut cfg.hcl;

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        ui.label(
            RichText::new(tr("Color Temperature Range"))
                .size(13.0)
                .strong()
                .color(TEXT_PRIMARY),
        );
        ui.add_space(14.0);
        ui.label(RichText::new("Min:").size(12.5).color(TEXT_MUTED));
        ui.add(
            DragValue::new(&mut hcl.tmin_kelvin)
                .range(1800..=6500)
                .speed(25.0)
                .suffix(" K"),
        );
        ui.add_space(10.0);
        ui.label(RichText::new("Max:").size(12.5).color(TEXT_MUTED));
        ui.add(
            DragValue::new(&mut hcl.tmax_kelvin)
                .range(1800..=6500)
                .speed(25.0)
                .suffix(" K"),
        );
        if hcl.tmax_kelvin < hcl.tmin_kelvin {
            hcl.tmax_kelvin = hcl.tmin_kelvin;
        }
    });

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        ui.label(
            RichText::new(tr("Daylight Curve Mode"))
                .size(13.0)
                .strong()
                .color(TEXT_PRIMARY),
        );
        ui.add_space(14.0);
        ComboBox::from_id_salt("hcl_mode_combo")
            .width(260.0)
            .selected_text(translate_hcl_mode(hcl.mode))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut hcl.mode,
                    HclMode::SunriseSunset,
                    translate_hcl_mode(HclMode::SunriseSunset),
                );
                ui.selectable_value(
                    &mut hcl.mode,
                    HclMode::FixedPeriod,
                    translate_hcl_mode(HclMode::FixedPeriod),
                );
            });

        let (mut fh, mut fm) = hcl.from_hm();
        let (mut th, mut tm) = hcl.to_hm();
        ui.add_space(16.0);
        ui.label(RichText::new(tr("From")).size(12.5).color(TEXT_MUTED));
        let f_changed = ui
            .add(DragValue::new(&mut fh).range(0..=23).suffix("h"))
            .changed()
            | ui.add(DragValue::new(&mut fm).range(0..=59).suffix("m"))
                .changed();
        if f_changed {
            hcl.set_from_hm(fh, fm);
        }

        ui.add_space(10.0);
        ui.label(RichText::new(tr("To")).size(12.5).color(TEXT_MUTED));
        let t_changed = ui
            .add(DragValue::new(&mut th).range(0..=23).suffix("h"))
            .changed()
            | ui.add(DragValue::new(&mut tm).range(0..=59).suffix("m"))
                .changed();
        if t_changed {
            hcl.set_to_hm(th, tm);
        }
    });

    ui.add_space(14.0);

    // 24h Kelvin curve plot matching `light05.png`
    let plot_w = (ui.available_width() - 28.0).max(400.0);
    let plot_h = (ui.available_height() - 24.0).clamp(180.0, 360.0);
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(plot_w, plot_h), Sense::hover());
        let painter = ui.painter_at(rect);

        painter.rect_filled(rect, 6.0, HEADER_BG);
        painter.rect_stroke(
            rect,
            6.0,
            Stroke::new(1.0_f32, BORDER_SUBTLE),
            egui::StrokeKind::Inside,
        );

        let pad_left = 54.0;
        let pad_right = 20.0;
        let pad_top = 18.0;
        let pad_bottom = 28.0;
        let chart_rect = Rect::from_min_max(
            Pos2::new(rect.left() + pad_left, rect.top() + pad_top),
            Pos2::new(rect.right() - pad_right, rect.bottom() - pad_bottom),
        );

        let k_min = 2000.0_f32;
        let k_max = 6500.0_f32;

        // Horizontal Kelvin grid lines
        for k_tick in [2700_u32, 3500, 4500, 5500, 6500] {
            let norm = ((k_tick as f32 - k_min) / (k_max - k_min)).clamp(0.0, 1.0);
            let y = chart_rect.bottom() - norm * chart_rect.height();
            painter.line_segment(
                [
                    Pos2::new(chart_rect.left(), y),
                    Pos2::new(chart_rect.right(), y),
                ],
                Stroke::new(1.0_f32, BORDER_SUBTLE),
            );
            painter.text(
                Pos2::new(chart_rect.left() - 6.0, y),
                egui::Align2::RIGHT_CENTER,
                format!("{}K", k_tick),
                egui::FontId::proportional(11.0),
                TEXT_MUTED,
            );
        }

        // Vertical hour grid lines (00:00 .. 24:00)
        for hr in (0..=24).step_by(3) {
            let x = chart_rect.left() + (hr as f32 / 24.0) * chart_rect.width();
            painter.line_segment(
                [
                    Pos2::new(x, chart_rect.top()),
                    Pos2::new(x, chart_rect.bottom()),
                ],
                Stroke::new(1.0_f32, BORDER_SUBTLE),
            );
            painter.text(
                Pos2::new(x, chart_rect.bottom() + 12.0),
                egui::Align2::CENTER_CENTER,
                format!("{:02}:00", hr),
                egui::FontId::proportional(11.0),
                TEXT_MUTED,
            );
        }

        // Sample bell curve across 96 points
        let samples = 96;
        let mut pts = Vec::with_capacity(samples + 1);
        for s in 0..=samples {
            let hr = (s as f32 / samples as f32) * 24.0;
            let kelvin = hcl.sample_kelvin_at_hour(hr).clamp(k_min, k_max);
            let x = chart_rect.left() + (hr / 24.0) * chart_rect.width();
            let norm = (kelvin - k_min) / (k_max - k_min);
            let y = chart_rect.bottom() - norm * chart_rect.height();
            pts.push(Pos2::new(x, y));
        }

        for pair in pts.windows(2) {
            painter.line_segment(
                [pair[0], pair[1]],
                Stroke::new(2.5_f32, Color32::from_rgb(0xF5, 0xB0, 0x41)),
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn load_klimczaki() -> LoxoneProject {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.pop();
        path.pop();
        path.push("Klimczaki.Loxone");
        let xml = std::fs::read_to_string(&path).expect("read Klimczaki.Loxone");
        loxone_core::parser::parse_loxone_xml(&xml).expect("Klimczaki.Loxone should parse")
    }

    #[test]
    fn test_open_light_controller_modal_on_klimczaki() {
        let project = load_klimczaki();
        let lc_uuid = project
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .find(|b| b.block_type == "LightController2")
            .expect("LightController2 block must exist in Klimczaki.Loxone")
            .uuid
            .clone();

        let mut modal = LightControllerModalState::default();
        assert!(modal.open_for_block(&project, &lc_uuid));
        assert!(modal.is_open);

        let cfg = modal.config.as_ref().expect("config should be populated");
        assert_eq!(cfg.circuits.len(), 8);
        assert!(!cfg.scenes.is_empty());
        assert!(!modal.operating_modes.is_empty());
    }

    #[test]
    fn test_render_light_controller_modal_all_tabs() {
        let project = load_klimczaki();
        let lc_uuid = project
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .find(|b| b.block_type == "LightController2")
            .expect("LightController2 block must exist in Klimczaki.Loxone")
            .uuid
            .clone();

        let mut modal = LightControllerModalState::default();
        assert!(modal.open_for_block(&project, &lc_uuid));

        let ctx = egui::Context::default();
        for tab in [
            LightControllerTab::Circuits,
            LightControllerTab::Moods,
            LightControllerTab::Automation,
            LightControllerTab::DaylightControl,
        ] {
            modal.active_tab = tab;
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                let edit = render_light_controller_modal(ctx, &mut modal, Some(&project));
                assert!(edit.is_none());
            });
            assert!(modal.is_open);
        }
    }
}
