use egui::{
    Align, Color32, Context, CornerRadius, Id, Key, Layout, Pos2, Rect, RichText, ScrollArea,
    Sense, Stroke, Ui, Vec2,
};
use loxone_core::model::{LoxoneProject, PageBlock};
use loxone_core::{
    collect_energy_source_candidates, collect_user_operating_modes_for_timer,
    supports_room_controller_config, EnergySourceCandidate, ProjectEdit,
    RoomControllerConfig, RoomScheduleConfig, RoomTempMode,
};

const LOXONE_GREEN: Color32 = Color32::from_rgb(0x69, 0xB3, 0x2D);
const PANEL_BG: Color32 = Color32::from_rgb(0x1E, 0x22, 0x28);
const HEADER_BG: Color32 = Color32::from_rgb(0x28, 0x2D, 0x35);
const CARD_BG: Color32 = Color32::from_rgb(0x16, 0x19, 0x1E);
const ROW_BG_EVEN: Color32 = Color32::from_rgb(0x22, 0x26, 0x2E);
const ROW_BG_ODD: Color32 = Color32::from_rgb(0x1C, 0x20, 0x26);
const BORDER_SUBTLE: Color32 = Color32::from_rgb(0x3A, 0x41, 0x4D);
const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xEE, 0xF2, 0xF6);
const TEXT_MUTED: Color32 = Color32::from_rgb(0x9A, 0xA3, 0xB2);

const COLOR_COMFORT: Color32 = Color32::from_rgb(0x5B, 0xB7, 0x3B);
const COLOR_ECONOMY: Color32 = Color32::from_rgb(0xF5, 0xF5, 0xEE);
const COLOR_PROTECTION: Color32 = Color32::from_rgb(0x6D, 0x9E, 0xEB);
const COLOR_FIXED: Color32 = Color32::from_rgb(0xD6, 0xC2, 0x5A);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoomControllerTab {
    #[default]
    EnergySources,
    Heating,
    Cooling,
}

#[derive(Debug, Clone)]
pub enum RoomControllerModalAction {
    CommitEdits(Vec<ProjectEdit>),
}

#[derive(Debug, Clone, Default)]
pub struct RoomControllerModalState {
    pub is_open: bool,
    pub page_uuid: String,
    pub block_uuid: String,
    pub block_title: String,
    pub active_tab: RoomControllerTab,
    pub config: RoomControllerConfig,
    pub candidates: Vec<EnergySourceCandidate>,
    pub selected_candidate_idx: usize,

    // Schedule sub-modal ("Edytuj harmonogram")
    pub is_schedule_open: bool,
    pub schedule_draft: RoomScheduleConfig,
    pub schedule_undo_stack: Vec<RoomScheduleConfig>,
    pub available_user_modes: Vec<(String, String)>,
    pub selected_user_mode_combo_idx: usize,
    pub selected_schedule_row: usize,
    pub edit_from_min: u16,
    pub edit_to_min: u16,
    pub edit_temp_mode: RoomTempMode,
}

impl RoomControllerModalState {
    pub fn open_for_block_uuid(&mut self, project: &LoxoneProject, block_uuid: &str) -> bool {
        for page in &project.pages {
            if let Some(block) = page.blocks.iter().find(|b| b.uuid == block_uuid) {
                return self.open_for_block(project, page.uuid.clone(), block);
            }
        }
        false
    }

    pub fn open_for_block(
        &mut self,
        project: &LoxoneProject,
        page_uuid: String,
        block: &PageBlock,
    ) -> bool {
        if !supports_room_controller_config(block) {
            return false;
        }
        let config = RoomControllerConfig::from_block(project, block);
        let existing_uuids: Vec<String> = config
            .groups
            .iter()
            .map(|g| g.object_uuid.clone())
            .collect();
        self.candidates = collect_energy_source_candidates(project, &existing_uuids);
        self.selected_candidate_idx = 0;
        self.available_user_modes = collect_user_operating_modes_for_timer(project);
        self.selected_user_mode_combo_idx = 0;

        self.is_open = true;
        self.is_schedule_open = false;
        self.page_uuid = page_uuid;
        self.block_uuid = block.uuid.clone();
        self.block_title = if block.title.trim().is_empty() {
            crate::i18n::block_display_name(&block.block_type).to_string()
        } else {
            block.title.clone()
        };
        self.active_tab = RoomControllerTab::EnergySources;
        self.schedule_draft = config.schedule.clone();
        self.schedule_undo_stack.clear();
        self.selected_schedule_row = config.schedule.user_mode_count();
        self.sync_selected_row_editor();
        self.config = config;
        true
    }

    pub fn refresh_candidates(&mut self, project: &LoxoneProject) {
        let existing_uuids: Vec<String> = self
            .config
            .groups
            .iter()
            .map(|g| g.object_uuid.clone())
            .collect();
        self.candidates = collect_energy_source_candidates(project, &existing_uuids);
        if self.selected_candidate_idx >= self.candidates.len() {
            self.selected_candidate_idx = 0;
        }
    }

    pub fn open_schedule_editor(&mut self) {
        self.schedule_draft = self.config.schedule.clone();
        self.schedule_undo_stack.clear();
        if self.selected_schedule_row >= self.schedule_draft.rows.len() {
            self.selected_schedule_row = 0;
        }
        self.sync_selected_row_editor();
        self.is_schedule_open = true;
    }

    pub fn sync_selected_row_editor(&mut self) {
        let entries = self
            .schedule_draft
            .entries_for_row(self.selected_schedule_row);
        if let Some(first) = entries.first() {
            self.edit_from_min = first.from_min;
            self.edit_to_min = first.to_min;
            self.edit_temp_mode = first.mode;
        } else {
            self.edit_from_min = 600; // 10:00
            self.edit_to_min = 1080; // 18:00
            self.edit_temp_mode = RoomTempMode::Comfort;
        }
    }

    fn push_schedule_undo(&mut self) {
        self.schedule_undo_stack.push(self.schedule_draft.clone());
    }
}

fn format_minutes_hhmm(mins: u16) -> String {
    let clamped = mins.min(1440);
    format!("{:02}:{:02}", clamped / 60, clamped % 60)
}

fn draw_loxone_checkbox(ui: &mut Ui, checked: &mut bool) -> bool {
    let desired = Vec2::new(16.0, 16.0);
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());
    let mut changed = false;
    if resp.clicked() {
        *checked = !*checked;
        changed = true;
    }
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if *checked {
            painter.rect(
                rect,
                CornerRadius::same(2),
                LOXONE_GREEN,
                Stroke::new(1.0_f32, LOXONE_GREEN),
                egui::StrokeKind::Inside,
            );
            let p1 = Pos2::new(rect.min.x + 3.5, rect.min.y + 8.2);
            let p2 = Pos2::new(rect.min.x + 6.8, rect.min.y + 11.5);
            let p3 = Pos2::new(rect.min.x + 12.6, rect.min.y + 4.5);
            let stroke = Stroke::new(1.9_f32, Color32::WHITE);
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
        } else {
            let border = if resp.hovered() {
                LOXONE_GREEN
            } else {
                Color32::from_rgb(0x68, 0x72, 0x82)
            };
            painter.rect(
                rect,
                CornerRadius::same(2),
                Color32::from_rgb(0x1A, 0x1D, 0x22),
                Stroke::new(1.2_f32, border),
                egui::StrokeKind::Inside,
            );
        }
    }
    changed
}

fn render_cell_text(ui: &mut Ui, width: f32, text: &str, color: Color32, strong: bool) {
    ui.allocate_ui_with_layout(
        Vec2::new(width, 24.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_clip_rect(ui.clip_rect().intersect(ui.max_rect()));
            let mut rt = RichText::new(text).size(12.0).color(color);
            if strong {
                rt = rt.strong();
            }
            ui.add(egui::Label::new(rt).truncate());
        },
    );
}

fn render_header_text(ui: &mut Ui, width: f32, label: &str, centered: bool) {
    let layout = if centered {
        Layout::centered_and_justified(egui::Direction::LeftToRight)
    } else {
        Layout::left_to_right(Align::Center)
    };
    ui.allocate_ui_with_layout(Vec2::new(width, 24.0), layout, |ui| {
        ui.label(
            RichText::new(label)
                .size(11.5)
                .strong()
                .color(Color32::from_rgb(0xD4, 0xDA, 0xE2)),
        );
    });
}

fn render_energy_sources_tab(
    ui: &mut Ui,
    project: &LoxoneProject,
    state: &mut RoomControllerModalState,
) {
    use crate::i18n::tr;

    let avail_w = ui.available_width();
    let col_out = 84.0;
    let col_heat = 82.0;
    let col_cool = 82.0;
    let col_pwm = 62.0;
    let col_del = 56.0;
    let flex = (avail_w - col_out - col_heat - col_cool - col_pwm - col_del - 54.0).max(280.0);
    let col_name = flex * 0.56;
    let col_type = flex * 0.44;

    egui::Frame::new()
        .fill(HEADER_BG)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .inner_margin(egui::Margin::symmetric(6, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                render_header_text(ui, col_out, tr("Wyjście"), false);
                render_header_text(ui, col_name, tr("Nazwa"), false);
                render_header_text(ui, col_type, tr("Typ obiektu"), false);
                render_header_text(ui, col_heat, tr("Ogrzewanie"), true);
                render_header_text(ui, col_cool, tr("Chłodzenie"), true);
                render_header_text(ui, col_pwm, "PWM", true);
                render_header_text(ui, col_del, tr("Usuń"), true);
            });
        });

    let table_h = (ui.available_height() - 88.0).max(180.0);
    let mut remove_idx: Option<usize> = None;
    let mut toggle_heat: Option<(usize, bool)> = None;
    let mut toggle_cool: Option<(usize, bool)> = None;

    egui::Frame::new()
        .fill(CARD_BG)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .show(ui, |ui| {
            ui.set_min_height(table_h);
            ScrollArea::vertical()
                .id_salt("room_ctrl_energy_sources_scroll")
                .max_height(table_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for (idx, group) in state.config.groups.iter_mut().enumerate() {
                        let bg = if idx % 2 == 0 {
                            ROW_BG_EVEN
                        } else {
                            ROW_BG_ODD
                        };
                        let row_resp = egui::Frame::new()
                            .fill(bg)
                            .inner_margin(egui::Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    render_cell_text(
                                        ui,
                                        col_out,
                                        &format!("{} {}", tr("Grupa"), idx + 1),
                                        TEXT_PRIMARY,
                                        false,
                                    );
                                    render_cell_text(
                                        ui,
                                        col_name,
                                        &group.display_name,
                                        TEXT_PRIMARY,
                                        true,
                                    );
                                    render_cell_text(
                                        ui,
                                        col_type,
                                        &group.object_type_label,
                                        TEXT_MUTED,
                                        false,
                                    );
                                    ui.allocate_ui_with_layout(
                                        Vec2::new(col_heat, 24.0),
                                        Layout::centered_and_justified(
                                            egui::Direction::LeftToRight,
                                        ),
                                        |ui| {
                                            let mut h = group.heating;
                                            if draw_loxone_checkbox(ui, &mut h) {
                                                toggle_heat = Some((idx, h));
                                            }
                                        },
                                    );
                                    ui.allocate_ui_with_layout(
                                        Vec2::new(col_cool, 24.0),
                                        Layout::centered_and_justified(
                                            egui::Direction::LeftToRight,
                                        ),
                                        |ui| {
                                            let mut c = group.cooling;
                                            if draw_loxone_checkbox(ui, &mut c) {
                                                toggle_cool = Some((idx, c));
                                            }
                                        },
                                    );
                                    ui.allocate_ui_with_layout(
                                        Vec2::new(col_pwm, 24.0),
                                        Layout::centered_and_justified(
                                            egui::Direction::LeftToRight,
                                        ),
                                        |ui| {
                                            draw_loxone_checkbox(ui, &mut group.pwm);
                                        },
                                    );
                                    ui.allocate_ui_with_layout(
                                        Vec2::new(col_del, 24.0),
                                        Layout::centered_and_justified(
                                            egui::Direction::LeftToRight,
                                        ),
                                        |ui| {
                                            let del_btn = egui::Button::new(
                                                RichText::new("✖")
                                                    .size(12.5)
                                                    .strong()
                                                    .color(Color32::from_rgb(0xE0, 0x52, 0x52)),
                                            )
                                            .frame(false);
                                            if ui.add(del_btn).clicked() {
                                                remove_idx = Some(idx);
                                            }
                                        },
                                    );
                                });
                            })
                            .response;
                        ui.painter().line_segment(
                            [
                                Pos2::new(row_resp.rect.min.x, row_resp.rect.max.y),
                                Pos2::new(row_resp.rect.max.x, row_resp.rect.max.y),
                            ],
                            Stroke::new(0.5_f32, Color32::from_rgb(0x36, 0x3C, 0x46)),
                        );
                    }
                });
        });

    if let Some((idx, val)) = toggle_heat {
        state.config.set_group_heating(idx, val);
    }
    if let Some((idx, val)) = toggle_cool {
        state.config.set_group_cooling(idx, val);
    }
    if let Some(idx) = remove_idx {
        state.config.remove_group(idx);
        state.refresh_candidates(project);
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let selected_label = if state.candidates.is_empty() {
            tr("Nie znaleziono żadnych odpowiednich obiektów").to_string()
        } else {
            let c = &state.candidates
                [state.selected_candidate_idx.min(state.candidates.len() - 1)];
            format!("{} — {}", c.display_name, c.object_type_label)
        };

        ui.add_enabled_ui(!state.candidates.is_empty(), |ui| {
            egui::ComboBox::from_id_salt("room_ctrl_candidate_combo")
                .width(340.0)
                .selected_text(selected_label)
                .show_ui(ui, |ui| {
                    for (idx, cand) in state.candidates.iter().enumerate() {
                        let lbl = format!("{} — {}", cand.display_name, cand.object_type_label);
                        ui.selectable_value(&mut state.selected_candidate_idx, idx, lbl);
                    }
                });

            if ui
                .add(
                    egui::Button::new(RichText::new(tr("Dodaj")).size(12.5).strong())
                        .min_size(Vec2::new(76.0, 26.0)),
                )
                .clicked()
            {
                if let Some(cand) = state
                    .candidates
                    .get(state.selected_candidate_idx)
                    .cloned()
                {
                    state.config.add_group(&cand);
                    state.refresh_candidates(project);
                }
            }
        });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let sched_btn = egui::Button::new(
                RichText::new(tr("Harmonogram"))
                    .size(12.5)
                    .strong()
                    .color(Color32::from_rgb(0x11, 0x16, 0x0D)),
            )
            .fill(LOXONE_GREEN)
            .min_size(Vec2::new(118.0, 26.0));
            if ui.add(sched_btn).clicked() {
                state.open_schedule_editor();
            }
        });
    });
}

fn render_priority_tab(
    ui: &mut Ui,
    state: &mut RoomControllerModalState,
    is_heating: bool,
) {
    use crate::i18n::tr;

    let avail_w = ui.available_width();
    let col_prio = 76.0;
    let col_up = 66.0;
    let col_down = 66.0;
    let col_cost = 120.0;
    let col_name = (avail_w - col_prio - col_up - col_down - col_cost - 46.0).max(240.0);

    egui::Frame::new()
        .fill(HEADER_BG)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .inner_margin(egui::Margin::symmetric(6, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                render_header_text(ui, col_prio, tr("Priorytet"), false);
                render_header_text(ui, col_up, tr("W górę"), true);
                render_header_text(ui, col_down, tr("W dół"), true);
                render_header_text(ui, col_name, tr("Nazwa"), false);
                render_header_text(ui, col_cost, tr("Koszty energii"), false);
            });
        });

    let table_h = (ui.available_height() - 88.0).max(180.0);
    let prio_list = if is_heating {
        state.config.prio_h.clone()
    } else {
        state.config.prio_c.clone()
    };
    let total_rows = prio_list.len();
    let mut move_up_pos: Option<usize> = None;
    let mut move_down_pos: Option<usize> = None;

    egui::Frame::new()
        .fill(CARD_BG)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .show(ui, |ui| {
            ui.set_min_height(table_h);
            ScrollArea::vertical()
                .id_salt(if is_heating {
                    "room_ctrl_prio_h_scroll"
                } else {
                    "room_ctrl_prio_c_scroll"
                })
                .max_height(table_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    if prio_list.is_empty() {
                        ui.add_space(16.0);
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.label(
                                RichText::new(if is_heating {
                                    tr("Brak przypisanych grup ogrzewania — zaznacz grupę poniżej lub w zakładce Źródła energii.")
                                } else {
                                    tr("Brak przypisanych grup chłodzenia — zaznacz grupę poniżej lub w zakładce Źródła energii.")
                                })
                                .size(12.0)
                                .color(TEXT_MUTED),
                            );
                        });
                    }
                    for (pos, &group_idx) in prio_list.iter().enumerate() {
                        let Some(group) = state.config.groups.get(group_idx) else {
                            continue;
                        };
                        let bg = if pos % 2 == 0 {
                            ROW_BG_EVEN
                        } else {
                            ROW_BG_ODD
                        };
                        let row_resp = egui::Frame::new()
                            .fill(bg)
                            .inner_margin(egui::Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    render_cell_text(
                                        ui,
                                        col_prio,
                                        &(pos + 1).to_string(),
                                        TEXT_PRIMARY,
                                        false,
                                    );
                                    ui.allocate_ui_with_layout(
                                        Vec2::new(col_up, 24.0),
                                        Layout::centered_and_justified(
                                            egui::Direction::LeftToRight,
                                        ),
                                        |ui| {
                                            let can_up = pos > 0;
                                            let up_btn = egui::Button::new(
                                                RichText::new("⬆")
                                                    .size(13.0)
                                                    .strong()
                                                    .color(if can_up {
                                                        LOXONE_GREEN
                                                    } else {
                                                        Color32::from_rgb(0x4A, 0x52, 0x5E)
                                                    }),
                                            )
                                            .frame(false);
                                            if ui.add_enabled(can_up, up_btn).clicked() {
                                                move_up_pos = Some(pos);
                                            }
                                        },
                                    );
                                    ui.allocate_ui_with_layout(
                                        Vec2::new(col_down, 24.0),
                                        Layout::centered_and_justified(
                                            egui::Direction::LeftToRight,
                                        ),
                                        |ui| {
                                            let can_down = pos + 1 < total_rows;
                                            let down_btn = egui::Button::new(
                                                RichText::new("⬇")
                                                    .size(13.0)
                                                    .strong()
                                                    .color(if can_down {
                                                        LOXONE_GREEN
                                                    } else {
                                                        Color32::from_rgb(0x4A, 0x52, 0x5E)
                                                    }),
                                            )
                                            .frame(false);
                                            if ui.add_enabled(can_down, down_btn).clicked() {
                                                move_down_pos = Some(pos);
                                            }
                                        },
                                    );
                                    render_cell_text(
                                        ui,
                                        col_name,
                                        &group.priority_row_label(group_idx + 1),
                                        TEXT_PRIMARY,
                                        false,
                                    );
                                    render_cell_text(ui, col_cost, tr("tanio"), TEXT_MUTED, false);
                                });
                            })
                            .response;
                        ui.painter().line_segment(
                            [
                                Pos2::new(row_resp.rect.min.x, row_resp.rect.max.y),
                                Pos2::new(row_resp.rect.max.x, row_resp.rect.max.y),
                            ],
                            Stroke::new(0.5_f32, Color32::from_rgb(0x36, 0x3C, 0x46)),
                        );
                    }
                });
        });

    if let Some(pos) = move_up_pos {
        if is_heating {
            state.config.move_heating_prio_up(pos);
        } else {
            state.config.move_cooling_prio_up(pos);
        }
    }
    if let Some(pos) = move_down_pos {
        if is_heating {
            state.config.move_heating_prio_down(pos);
        } else {
            state.config.move_cooling_prio_down(pos);
        }
    }

    ui.add_space(8.0);
    let mut toggle_group: Option<(usize, bool)> = None;
    ui.horizontal(|ui| {
        if !state.config.groups.is_empty() {
            ui.label(
                RichText::new(if is_heating {
                    tr("Aktywne grupy ogrzewania:")
                } else {
                    tr("Aktywne grupy chłodzenia:")
                })
                .size(11.5)
                .color(TEXT_MUTED),
            );
            for (idx, grp) in state.config.groups.iter().enumerate() {
                let mut active = if is_heating {
                    grp.heating
                } else {
                    grp.cooling
                };
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    if draw_loxone_checkbox(ui, &mut active) {
                        toggle_group = Some((idx, active));
                    }
                    ui.label(
                        RichText::new(format!("{} {}", tr("Grupa"), idx + 1))
                            .size(11.5)
                            .color(TEXT_PRIMARY),
                    );
                });
            }
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let sched_btn = egui::Button::new(
                RichText::new(tr("Harmonogram"))
                    .size(12.5)
                    .strong()
                    .color(Color32::from_rgb(0x11, 0x16, 0x0D)),
            )
            .fill(LOXONE_GREEN)
            .min_size(Vec2::new(118.0, 26.0));
            if ui.add(sched_btn).clicked() {
                state.open_schedule_editor();
            }
        });
    });

    if let Some((idx, val)) = toggle_group {
        if is_heating {
            state.config.set_group_heating(idx, val);
        } else {
            state.config.set_group_cooling(idx, val);
        }
    }
}

fn temp_mode_color(mode: RoomTempMode) -> Color32 {
    match mode {
        RoomTempMode::Comfort => COLOR_COMFORT,
        RoomTempMode::Economy => COLOR_ECONOMY,
        RoomTempMode::BuildingProtection => COLOR_PROTECTION,
        RoomTempMode::FixedValue => COLOR_FIXED,
    }
}

fn draw_legend_item(ui: &mut Ui, color: Color32, label: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
        ui.painter().circle(
            rect.center(),
            6.0,
            color,
            Stroke::new(1.0_f32, Color32::from_rgb(0x48, 0x52, 0x60)),
        );
        ui.label(RichText::new(label).size(12.0).color(TEXT_PRIMARY));
    });
}

fn render_schedule_submodal(ctx: &Context, state: &mut RoomControllerModalState) {
    use crate::i18n::tr;

    if !state.is_schedule_open {
        return;
    }

    let screen = ctx.screen_rect();
    let modal_w = (screen.width() - 90.0).clamp(760.0, 960.0);
    let modal_h = (screen.height() - 80.0).clamp(540.0, 680.0);

    let mut open_flag = state.is_schedule_open;
    let mut should_close = false;

    let (backdrop_clicked, popup_was_open) = crate::theme::draw_modal_backdrop(
        ctx,
        "room_ctrl_schedule_backdrop",
        egui::Order::Foreground,
        150,
    );

    let win_resp = egui::Window::new(format!("{} — {}", tr("Edytuj harmonogram"), state.block_title))
        .id(Id::new("loxone_room_schedule_modal_window"))
        .open(&mut open_flag)
        .collapsible(false)
        .resizable(true)
        .default_size(Vec2::new(modal_w, modal_h))
        .min_size(Vec2::new(720.0, 520.0))
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .order(egui::Order::Tooltip)
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(PANEL_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(egui::Margin::same(14)),
        )
        .show(ctx, |ui| {
            // Top section: Tryby użytkownika
            ui.label(
                RichText::new(tr("Tryby użytkownika"))
                    .size(12.5)
                    .strong()
                    .color(TEXT_PRIMARY),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let selected_um_label = state
                    .available_user_modes
                    .get(state.selected_user_mode_combo_idx)
                    .map(|(_, l)| l.clone())
                    .unwrap_or_else(|| tr("Brak trybów użytkownika").to_string());

                egui::ComboBox::from_id_salt("schedule_user_mode_combo")
                    .width(220.0)
                    .selected_text(selected_um_label)
                    .show_ui(ui, |ui| {
                        for (idx, (_, lbl)) in state.available_user_modes.iter().enumerate() {
                            ui.selectable_value(
                                &mut state.selected_user_mode_combo_idx,
                                idx,
                                lbl,
                            );
                        }
                    });

                let sel_is_user = state
                    .schedule_draft
                    .rows
                    .get(state.selected_schedule_row)
                    .map(|r| r.is_user_mode)
                    .unwrap_or(false);

                // Add (+) button
                if ui
                    .add_enabled(
                        !state.available_user_modes.is_empty(),
                        egui::Button::new(
                            RichText::new("➕").size(13.0).strong().color(LOXONE_GREEN),
                        )
                        .min_size(Vec2::new(28.0, 24.0)),
                    )
                    .on_hover_text(tr("Dodaj tryb użytkownika"))
                    .clicked()
                {
                    if let Some((u, l)) = state
                        .available_user_modes
                        .get(state.selected_user_mode_combo_idx)
                        .cloned()
                    {
                        state.push_schedule_undo();
                        let new_ix = state.schedule_draft.add_user_mode(u, l);
                        state.selected_schedule_row = new_ix;
                        state.sync_selected_row_editor();
                    }
                }

                // Remove (✖) button
                if ui
                    .add_enabled(
                        sel_is_user,
                        egui::Button::new(
                            RichText::new("✖")
                                .size(13.0)
                                .strong()
                                .color(Color32::from_rgb(0xE0, 0x52, 0x52)),
                        )
                        .min_size(Vec2::new(28.0, 24.0)),
                    )
                    .on_hover_text(tr("Usuń wybrany tryb użytkownika"))
                    .clicked()
                {
                    state.push_schedule_undo();
                    let row_ix = state.selected_schedule_row;
                    if state.schedule_draft.remove_user_mode(row_ix) {
                        state.selected_schedule_row = state
                            .selected_schedule_row
                            .min(state.schedule_draft.rows.len().saturating_sub(1));
                        state.sync_selected_row_editor();
                    }
                }

                // Move up (⬆) button
                let can_up = sel_is_user && state.selected_schedule_row > 0;
                if ui
                    .add_enabled(
                        can_up,
                        egui::Button::new(
                            RichText::new("⬆").size(13.0).strong().color(LOXONE_GREEN),
                        )
                        .min_size(Vec2::new(28.0, 24.0)),
                    )
                    .on_hover_text(tr("Przesuń w górę"))
                    .clicked()
                {
                    state.push_schedule_undo();
                    let row_ix = state.selected_schedule_row;
                    if state.schedule_draft.move_user_mode_up(row_ix) {
                        state.selected_schedule_row -= 1;
                    }
                }

                // Move down (⬇) button
                let can_down = sel_is_user
                    && state.selected_schedule_row + 1 < state.schedule_draft.user_mode_count();
                if ui
                    .add_enabled(
                        can_down,
                        egui::Button::new(
                            RichText::new("⬇").size(13.0).strong().color(LOXONE_GREEN),
                        )
                        .min_size(Vec2::new(28.0, 24.0)),
                    )
                    .on_hover_text(tr("Przesuń w dół"))
                    .clicked()
                {
                    state.push_schedule_undo();
                    let row_ix = state.selected_schedule_row;
                    if state.schedule_draft.move_user_mode_down(row_ix) {
                        state.selected_schedule_row += 1;
                    }
                }

                ui.add_space(10.0);
                ui.label(
                    RichText::new(tr(
                        "Uwaga: Jeśli kilka trybów pracy odpowiada sytuacji, użyty będzie znajdujący się najwyżej!",
                    ))
                    .size(11.5)
                    .color(TEXT_MUTED),
                );
            });

            ui.add_space(10.0);
            ui.label(
                RichText::new(tr("Harmonogram"))
                    .size(12.5)
                    .strong()
                    .color(TEXT_PRIMARY),
            );
            ui.add_space(4.0);

            // 24-hour timeline grid
            let label_col_w = 205.0;
            let total_w = ui.available_width();
            let bar_w = (total_w - label_col_w - 16.0).max(380.0);

            // Time axis labels (00:00 .. 24:00 every 3 hours)
            let (axis_rect, _) =
                ui.allocate_exact_size(Vec2::new(total_w, 18.0), Sense::hover());
            let axis_x0 = axis_rect.min.x + label_col_w + 8.0;
            for h in (0..=24).step_by(3) {
                let frac = (h as f32) / 24.0;
                let x = axis_x0 + frac * bar_w;
                ui.painter().text(
                    Pos2::new(x, axis_rect.center().y),
                    egui::Align2::CENTER_CENTER,
                    format!("{:02}:00", h),
                    egui::FontId::proportional(11.0),
                    TEXT_MUTED,
                );
            }

            let mut clicked_row_entry: Option<(usize, Option<(u16, u16, RoomTempMode)>)> = None;
            let row_count = state.schedule_draft.rows.len();

            for row_ix in 0..row_count {
                let row_label = state.schedule_draft.rows[row_ix].label.clone();
                let is_user_mode = state.schedule_draft.rows[row_ix].is_user_mode;
                let is_selected = state.selected_schedule_row == row_ix;

                let (row_rect, row_resp) =
                    ui.allocate_exact_size(Vec2::new(total_w, 24.0), Sense::click());
                let painter = ui.painter();

                if is_selected {
                    painter.rect_filled(
                        row_rect,
                        CornerRadius::same(2),
                        Color32::from_rgba_unmultiplied(0x69, 0xB3, 0x2D, 36),
                    );
                }

                // Left label
                let label_rect = Rect::from_min_max(
                    row_rect.min,
                    Pos2::new(row_rect.min.x + label_col_w, row_rect.max.y),
                );
                painter.text(
                    Pos2::new(label_rect.min.x + 6.0, label_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &row_label,
                    egui::FontId::proportional(12.0),
                    if is_selected {
                        LOXONE_GREEN
                    } else {
                        TEXT_PRIMARY
                    },
                );

                // Bar rect
                let bar_rect = Rect::from_min_max(
                    Pos2::new(axis_x0, row_rect.min.y + 3.0),
                    Pos2::new(axis_x0 + bar_w, row_rect.max.y - 3.0),
                );
                painter.rect_filled(bar_rect, CornerRadius::ZERO, COLOR_ECONOMY);

                // Draw entries on this row
                let entries = state.schedule_draft.entries_for_row(row_ix);
                let mut hit_entry: Option<(u16, u16, RoomTempMode)> = None;
                let pointer_pos = row_resp.interact_pointer_pos();

                for entry in entries {
                    let x0 = bar_rect.min.x + (entry.from_min as f32 / 1440.0) * bar_w;
                    let x1 = bar_rect.min.x + (entry.to_min as f32 / 1440.0) * bar_w;
                    let er = Rect::from_min_max(
                        Pos2::new(x0, bar_rect.min.y),
                        Pos2::new(x1, bar_rect.max.y),
                    );
                    painter.rect_filled(er, CornerRadius::ZERO, temp_mode_color(entry.mode));

                    // Diagonal white hatch lines for Ochrona budynku (matching 05.harmonogram.png)
                    if entry.mode == RoomTempMode::BuildingProtection {
                        let clipped = painter.with_clip_rect(er.intersect(ui.clip_rect()));
                        let mut hx = er.min.x - er.height();
                        while hx < er.max.x + er.height() {
                            clipped.line_segment(
                                [
                                    Pos2::new(hx, er.max.y),
                                    Pos2::new(hx + er.height(), er.min.y),
                                ],
                                Stroke::new(
                                    1.0_f32,
                                    Color32::from_rgba_unmultiplied(255, 255, 255, 110),
                                ),
                            );
                            hx += 7.0;
                        }
                    }

                    if let Some(pos) = pointer_pos {
                        if er.contains(pos) {
                            hit_entry = Some((entry.from_min, entry.to_min, entry.mode));
                        }
                    }
                }

                // Hourly grid ticks
                for h in 0..=24 {
                    let x = bar_rect.min.x + (h as f32 / 24.0) * bar_w;
                    let is_major = h % 3 == 0;
                    painter.line_segment(
                        [Pos2::new(x, bar_rect.min.y), Pos2::new(x, bar_rect.max.y)],
                        Stroke::new(
                            if is_major { 1.0_f32 } else { 0.5_f32 },
                            if is_major {
                                Color32::from_rgb(0x88, 0x90, 0x9A)
                            } else {
                                Color32::from_rgba_unmultiplied(0x98, 0xA0, 0xAA, 110)
                            },
                        ),
                    );
                }
                painter.rect_stroke(
                    bar_rect,
                    CornerRadius::ZERO,
                    Stroke::new(
                        1.0_f32,
                        if is_selected {
                            LOXONE_GREEN
                        } else {
                            Color32::from_rgb(0x64, 0x6C, 0x78)
                        },
                    ),
                    egui::StrokeKind::Inside,
                );

                // Separator line between user modes and weekdays
                if is_user_mode
                    && row_ix + 1 < row_count
                    && !state.schedule_draft.rows[row_ix + 1].is_user_mode
                {
                    painter.line_segment(
                        [
                            Pos2::new(row_rect.min.x, row_rect.max.y),
                            Pos2::new(row_rect.max.x, row_rect.max.y),
                        ],
                        Stroke::new(1.5_f32, Color32::from_rgb(0x52, 0x5B, 0x68)),
                    );
                }

                if row_resp.clicked() {
                    clicked_row_entry = Some((row_ix, hit_entry));
                }
            }

            if let Some((r_ix, maybe_entry)) = clicked_row_entry {
                state.selected_schedule_row = r_ix;
                if let Some((f, t, m)) = maybe_entry {
                    state.edit_from_min = f;
                    state.edit_to_min = t;
                    state.edit_temp_mode = m;
                } else {
                    state.sync_selected_row_editor();
                }
            }

            ui.add_space(10.0);

            // Color legend: Informacje o kolorze
            ui.label(
                RichText::new(tr("Informacje o kolorze"))
                    .size(12.0)
                    .strong()
                    .color(TEXT_PRIMARY),
            );
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 20.0;
                draw_legend_item(ui, COLOR_COMFORT, tr("Temperatura komfortowa"));
                draw_legend_item(ui, COLOR_ECONOMY, tr("Temperatura ekonomiczna"));
                draw_legend_item(ui, COLOR_PROTECTION, tr("Ochrona budynku"));
                draw_legend_item(ui, COLOR_FIXED, tr("Stała wartość"));
            });

            ui.add_space(10.0);

            // Selected row editor: Wybrany rząd
            let selected_row_name = state
                .schedule_draft
                .rows
                .get(state.selected_schedule_row)
                .map(|r| r.label.as_str())
                .unwrap_or("—");
            ui.label(
                RichText::new(format!("{} — {}", tr("Wybrany rząd"), selected_row_name))
                    .size(12.0)
                    .strong()
                    .color(TEXT_PRIMARY),
            );
            ui.add_space(4.0);

            egui::Frame::new()
                .fill(CARD_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(tr("od")).size(12.0).color(TEXT_MUTED));
                        let mut from_h = (state.edit_from_min / 60) as u32;
                        let mut from_m = (state.edit_from_min % 60) as u32;
                        let mut time_changed = false;
                        time_changed |= ui
                            .add(
                                egui::DragValue::new(&mut from_h)
                                    .range(0..=24)
                                    .custom_formatter(|n, _| format!("{:02}", n as u32)),
                            )
                            .changed();
                        ui.label(":");
                        time_changed |= ui
                            .add(
                                egui::DragValue::new(&mut from_m)
                                    .speed(15.0)
                                    .range(0..=59)
                                    .custom_formatter(|n, _| format!("{:02}", n as u32)),
                            )
                            .changed();

                        ui.add_space(8.0);
                        ui.label(RichText::new(tr("do")).size(12.0).color(TEXT_MUTED));
                        let mut to_h = (state.edit_to_min / 60) as u32;
                        let mut to_m = (state.edit_to_min % 60) as u32;
                        time_changed |= ui
                            .add(
                                egui::DragValue::new(&mut to_h)
                                    .range(0..=24)
                                    .custom_formatter(|n, _| format!("{:02}", n as u32)),
                            )
                            .changed();
                        ui.label(":");
                        time_changed |= ui
                            .add(
                                egui::DragValue::new(&mut to_m)
                                    .speed(15.0)
                                    .range(0..=59)
                                    .custom_formatter(|n, _| format!("{:02}", n as u32)),
                            )
                            .changed();

                        if time_changed {
                            state.edit_from_min = ((from_h * 60 + from_m) as u16).min(1440);
                            state.edit_to_min = ((to_h * 60 + to_m) as u16).min(1440);
                        }

                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(tr("Temperatura"))
                                .size(12.0)
                                .color(TEXT_MUTED),
                        );
                        egui::ComboBox::from_id_salt("schedule_temp_mode_combo")
                            .width(195.0)
                            .selected_text(tr(state.edit_temp_mode.label_pl()))
                            .show_ui(ui, |ui| {
                                for &m in RoomTempMode::all() {
                                    ui.selectable_value(
                                        &mut state.edit_temp_mode,
                                        m,
                                        tr(m.label_pl()),
                                    );
                                }
                            });

                        let apply_btn = egui::Button::new(
                            RichText::new(tr("Zastosuj"))
                                .size(12.0)
                                .strong()
                                .color(Color32::from_rgb(0x11, 0x16, 0x0D)),
                        )
                        .fill(LOXONE_GREEN);
                        if ui
                            .add(apply_btn)
                            .on_hover_text(format!(
                                "{} {}–{}",
                                tr("Ustaw przedział"),
                                format_minutes_hhmm(state.edit_from_min),
                                format_minutes_hhmm(state.edit_to_min)
                            ))
                            .clicked()
                        {
                            state.push_schedule_undo();
                            state.schedule_draft.set_interval(
                                state.selected_schedule_row,
                                state.edit_from_min,
                                state.edit_to_min,
                                state.edit_temp_mode,
                            );
                        }

                        if ui
                            .button("⌫")
                            .on_hover_text(tr("Wyczyść wybrany rząd"))
                            .clicked()
                        {
                            state.push_schedule_undo();
                            state.schedule_draft.clear_row(state.selected_schedule_row);
                        }

                        if ui
                            .button("📅")
                            .on_hover_text(tr("Kopiuj wybrany rząd na wszystkie dni tygodnia"))
                            .clicked()
                        {
                            state.push_schedule_undo();
                            state
                                .schedule_draft
                                .copy_row_to_weekdays(state.selected_schedule_row);
                        }
                    });
                });

            ui.add_space(10.0);

            // Schedule footer
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{}: {}",
                        tr("Wartość standardowa"),
                        tr(state.schedule_draft.default_mode.label_pl())
                    ))
                    .size(12.0)
                    .color(TEXT_MUTED),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(RichText::new(tr("Anuluj")).size(12.5))
                                .min_size(Vec2::new(82.0, 28.0)),
                        )
                        .clicked()
                    {
                        should_close = true;
                    }

                    let ok_btn = egui::Button::new(
                        RichText::new("OK")
                            .size(12.5)
                            .strong()
                            .color(Color32::from_rgb(0x11, 0x16, 0x0D)),
                    )
                    .fill(LOXONE_GREEN)
                    .min_size(Vec2::new(82.0, 28.0));
                    if ui.add(ok_btn).clicked() {
                        state.config.schedule = state.schedule_draft.clone();
                        should_close = true;
                    }

                    if ui
                        .add_enabled(
                            !state.schedule_undo_stack.is_empty(),
                            egui::Button::new(RichText::new("↩").size(13.0))
                                .min_size(Vec2::new(34.0, 28.0)),
                        )
                        .on_hover_text(tr("Cofnij zmianę w harmonogramie"))
                        .clicked()
                    {
                        if let Some(prev) = state.schedule_undo_stack.pop() {
                            state.schedule_draft = prev;
                        }
                    }
                });
            });
        });

    if crate::theme::should_close_modal_on_outside_click(
        ctx,
        "loxone_room_schedule_modal_window",
        backdrop_clicked,
        popup_was_open,
        win_resp.as_ref().map(|r| r.response.rect),
    ) {
        should_close = true;
    }

    if !open_flag || should_close {
        state.is_schedule_open = false;
    }
}

pub fn render_room_controller_modal(
    ctx: &Context,
    project: &LoxoneProject,
    state: &mut RoomControllerModalState,
) -> Option<RoomControllerModalAction> {
    use crate::i18n::tr;

    if !state.is_open {
        return None;
    }

    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        if state.is_schedule_open {
            state.is_schedule_open = false;
        } else {
            state.is_open = false;
        }
        return None;
    }

    let screen = ctx.screen_rect();
    let modal_w = (screen.width() - 120.0).clamp(680.0, 880.0);
    let modal_h = (screen.height() - 140.0).clamp(400.0, 560.0);

    let mut open_flag = state.is_open;
    let mut should_close = false;
    let mut action: Option<RoomControllerModalAction> = None;

    let (backdrop_clicked, popup_was_open) = crate::theme::draw_modal_backdrop(
        ctx,
        "room_controller_modal_backdrop",
        egui::Order::Middle,
        150,
    );

    let win_resp = egui::Window::new(format!("{} — {}", tr("Konfiguracja"), state.block_title))
        .id(Id::new("loxone_room_controller_modal_window"))
        .open(&mut open_flag)
        .collapsible(false)
        .resizable(true)
        .default_size(Vec2::new(modal_w, modal_h))
        .min_size(Vec2::new(640.0, 380.0))
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .order(if state.is_schedule_open {
            egui::Order::Middle
        } else {
            egui::Order::Foreground
        })
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(PANEL_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(egui::Margin::same(14)),
        )
        .show(ctx, |ui| {
            // Top 3-tab selector bar: Źródło energii | Ogrzewanie | Chłodzenie
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for (tab, label) in [
                    (RoomControllerTab::EnergySources, tr("Źródło energii")),
                    (RoomControllerTab::Heating, tr("Ogrzewanie")),
                    (RoomControllerTab::Cooling, tr("Chłodzenie")),
                ] {
                    let is_active = state.active_tab == tab;
                    let btn = egui::Button::new(
                        RichText::new(label)
                            .size(12.5)
                            .strong()
                            .color(if is_active {
                                Color32::WHITE
                            } else {
                                TEXT_MUTED
                            }),
                    )
                    .fill(if is_active {
                        Color32::from_rgb(0x2F, 0x36, 0x42)
                    } else {
                        Color32::from_rgb(0x18, 0x1B, 0x20)
                    })
                    .stroke(Stroke::new(
                        1.0_f32,
                        if is_active {
                            LOXONE_GREEN
                        } else {
                            BORDER_SUBTLE
                        },
                    ))
                    .min_size(Vec2::new(124.0, 28.0));
                    if ui.add(btn).clicked() {
                        state.active_tab = tab;
                    }
                }
            });

            ui.add_space(8.0);

            match state.active_tab {
                RoomControllerTab::EnergySources => {
                    render_energy_sources_tab(ui, project, state);
                }
                RoomControllerTab::Heating => {
                    render_priority_tab(ui, state, true);
                }
                RoomControllerTab::Cooling => {
                    render_priority_tab(ui, state, false);
                }
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            // Dialog Footer: OK / Anuluj
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new(tr("Anuluj")).size(12.5))
                            .min_size(Vec2::new(86.0, 28.0)),
                    )
                    .clicked()
                {
                    should_close = true;
                }

                let ok_btn = egui::Button::new(
                    RichText::new("OK")
                        .size(12.5)
                        .strong()
                        .color(Color32::from_rgb(0x11, 0x16, 0x0D)),
                )
                .fill(LOXONE_GREEN)
                .min_size(Vec2::new(86.0, 28.0));

                if ui.add(ok_btn).clicked() {
                    if let Some(block) = project
                        .pages
                        .iter()
                        .find(|p| p.uuid == state.page_uuid)
                        .and_then(|p| p.blocks.iter().find(|b| b.uuid == state.block_uuid))
                    {
                        let edits = state.config.to_edits(block);
                        if !edits.is_empty() {
                            action = Some(RoomControllerModalAction::CommitEdits(edits));
                        }
                    }
                    should_close = true;
                }
            });
        });

    let clicked_outside_main = crate::theme::should_close_modal_on_outside_click(
        ctx,
        "loxone_room_controller_modal_window",
        backdrop_clicked,
        popup_was_open,
        win_resp.as_ref().map(|r| r.response.rect),
    );
    if !state.is_schedule_open && clicked_outside_main {
        should_close = true;
    }

    if !open_flag || should_close {
        state.is_schedule_open = false;
        state.is_open = false;
    }

    if state.is_open && state.is_schedule_open {
        render_schedule_submodal(ctx, state);
    }

    action
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_open_and_render_room_controller_modal_and_schedule_on_klimczaki() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Klimczaki.Loxone");
        let project = loxone_core::parser::load_loxone_file(&path).unwrap();

        let gabinet_uuid = "19eb19a9-001b-384c-ffff51535bcccc04";
        let mut state = RoomControllerModalState::default();
        assert!(state.open_for_block_uuid(&project, gabinet_uuid));
        assert!(state.is_open);
        assert!(!state.block_title.is_empty());
        assert_eq!(state.config.groups.len(), 2);
        assert_eq!(state.active_tab, RoomControllerTab::EnergySources);

        let ctx = egui::Context::default();
        for tab in [
            RoomControllerTab::EnergySources,
            RoomControllerTab::Heating,
            RoomControllerTab::Cooling,
        ] {
            state.active_tab = tab;
            state.is_schedule_open = true;
            state.schedule_draft = state.config.schedule.clone();
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                let _ = render_room_controller_modal(ctx, &project, &mut state);
            });
            assert!(state.is_open);
            assert!(state.is_schedule_open);
        }

        // Clicking outside the schedule submodal closes the schedule submodal first
        let outside_pos = egui::Pos2::new(5.0, 5.0);
        let mut press_input = egui::RawInput::default();
        press_input.events.push(egui::Event::PointerMoved(outside_pos));
        press_input.events.push(egui::Event::PointerButton {
            pos: outside_pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        });
        let _ = ctx.run(press_input, |ctx| {
            let _ = render_room_controller_modal(ctx, &project, &mut state);
        });
        let mut release_input = egui::RawInput::default();
        release_input.events.push(egui::Event::PointerButton {
            pos: outside_pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        });
        let _ = ctx.run(release_input, |ctx| {
            let _ = render_room_controller_modal(ctx, &project, &mut state);
        });
        assert!(!state.is_schedule_open);
        assert!(state.is_open);

        // Clicking outside again closes the main RoomController modal
        let mut press_input2 = egui::RawInput::default();
        press_input2.events.push(egui::Event::PointerMoved(outside_pos));
        press_input2.events.push(egui::Event::PointerButton {
            pos: outside_pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        });
        let _ = ctx.run(press_input2, |ctx| {
            let _ = render_room_controller_modal(ctx, &project, &mut state);
        });
        let mut release_input2 = egui::RawInput::default();
        release_input2.events.push(egui::Event::PointerButton {
            pos: outside_pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        });
        let _ = ctx.run(release_input2, |ctx| {
            let _ = render_room_controller_modal(ctx, &project, &mut state);
        });
        assert!(!state.is_open);
    }
}
