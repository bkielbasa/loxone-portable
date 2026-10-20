use egui::{
    Align, Color32, ComboBox, Context, CornerRadius, Id, Key, Layout, RichText, ScrollArea, Sense,
    Stroke, TextEdit, Vec2,
};
use loxone_core::model::LoxoneProject;
use loxone_core::state_alarm_config::{
    AlarmClockConfig, StateBlockConfig, StateConditionOp, ALARM_MODES_ALL, ALARM_MODES_WEEKDAYS,
    ALARM_MODES_WEEKEND, ALARM_MODE_FRI, ALARM_MODE_HOLIDAY, ALARM_MODE_MON, ALARM_MODE_SAT,
    ALARM_MODE_SICK, ALARM_MODE_SUN, ALARM_MODE_THU, ALARM_MODE_TUE, ALARM_MODE_VACATION,
    ALARM_MODE_WED,
};
use loxone_core::writer::ProjectEdit;

use crate::i18n::tr;
use crate::theme::{BORDER_SUBTLE, CARD_BG, PANEL_BG, TEXT_MUTED, TEXT_PRIMARY};

const LOXONE_GREEN: Color32 = Color32::from_rgb(0x69, 0xC8, 0x32);
const ROW_BG_EVEN: Color32 = Color32::from_rgb(0x23, 0x27, 0x2E);
const ROW_BG_ODD: Color32 = Color32::from_rgb(0x29, 0x2E, 0x36);
const HEADER_BG: Color32 = Color32::from_rgb(0x1D, 0x20, 0x26);

#[derive(Debug, Clone, Default)]
pub struct StateModalState {
    pub is_open: bool,
    pub page_uuid: String,
    pub block_uuid: String,
    pub block_title: String,
    pub config: Option<StateBlockConfig>,
}

impl StateModalState {
    pub fn open_for_block(&mut self, project: &LoxoneProject, block_uuid: &str) -> bool {
        let mut found = None;
        for page in &project.pages {
            if let Some(blk) = page.blocks.iter().find(|b| b.uuid == block_uuid) {
                if StateBlockConfig::is_state_block(&blk.block_type) {
                    found = Some((page.uuid.clone(), blk.clone()));
                }
                break;
            }
        }
        let Some((page_uuid, block)) = found else {
            return false;
        };

        self.is_open = true;
        self.page_uuid = page_uuid;
        self.block_uuid = block.uuid.clone();
        self.block_title = if block.title.is_empty() {
            "Status".to_string()
        } else {
            block.title.clone()
        };
        self.config = Some(StateBlockConfig::from_block(&block));
        true
    }
}

fn decode_loxone_color(dec_str: &str) -> Color32 {
    if let Ok(val) = dec_str.trim().parse::<u32>() {
        let r = (val & 0xFF) as u8;
        let g = ((val >> 8) & 0xFF) as u8;
        let b = ((val >> 16) & 0xFF) as u8;
        if r == 0 && g == 0 && b == 0 {
            TEXT_MUTED
        } else {
            Color32::from_rgb(r, g, b)
        }
    } else {
        LOXONE_GREEN
    }
}

const COLOR_PRESETS: &[(&str, &str)] = &[
    ("Szary (domyślny)", "7405453"),
    ("Zielony Loxone", "3328105"),
    ("Pomarańczowy", "3632629"),
    ("Czerwony", "3947746"),
    ("Niebieski", "14392382"),
    ("Brak (0)", "0"),
];

pub fn render_state_modal(
    ctx: &Context,
    state: &mut StateModalState,
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
    let modal_w = (screen_rect.width() * 0.90).clamp(820.0, 1180.0);
    let modal_h = (screen_rect.height() * 0.78).clamp(460.0, 740.0);

    let (backdrop_clicked, popup_was_open) = crate::theme::draw_modal_backdrop(
        ctx,
        "state_modal_backdrop",
        egui::Order::Foreground,
        150,
    );

    let win_resp = egui::Window::new("Status")
        .id(Id::new("state_config_modal_window"))
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

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("📊 Konfiguracja bloku Status: {}", state.block_title))
                        .size(16.0)
                        .strong()
                        .color(TEXT_PRIMARY),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(RichText::new("✖").size(13.0).color(TEXT_MUTED))
                                .frame(false),
                        )
                        .clicked()
                    {
                        close_requested = true;
                    }
                    ui.add_space(10.0);
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("➕ Dodaj warunek")
                                    .size(12.5)
                                    .strong()
                                    .color(Color32::BLACK),
                            )
                            .fill(LOXONE_GREEN)
                            .corner_radius(CornerRadius::same(4)),
                        )
                        .clicked()
                    {
                        cfg.add_rule();
                    }
                });
            });

            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Reguły są sprawdzane od góry do dołu. Pierwszy spełniony warunek ustawia wyjście tekstowe (Text) i wartość (Val).",
                )
                .size(12.0)
                .color(TEXT_MUTED),
            );
            ui.add_space(8.0);

            let footer_h = 46.0;
            let table_h = (ui.available_height() - footer_h).max(180.0);

            egui::Frame::new()
                .fill(CARD_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .corner_radius(CornerRadius::same(4))
                .show(ui, |ui| {
                    ui.set_min_height(table_h);
                    ui.set_max_height(table_h);

                    egui::Frame::new()
                        .fill(HEADER_BG)
                        .inner_margin(egui::Margin::symmetric(10, 7))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.allocate_ui(Vec2::new(66.0, 18.0), |ui| {
                                    ui.label(
                                        RichText::new("Priorytet")
                                            .size(12.0)
                                            .strong()
                                            .color(TEXT_PRIMARY),
                                    );
                                });
                                ui.allocate_ui(Vec2::new(420.0, 18.0), |ui| {
                                    ui.label(
                                        RichText::new("Warunki (AI1..AI4 / DI1..DI4)")
                                            .size(12.0)
                                            .strong()
                                            .color(TEXT_PRIMARY),
                                    );
                                });
                                ui.allocate_ui(Vec2::new(72.0, 18.0), |ui| {
                                    ui.label(
                                        RichText::new("Wartość")
                                            .size(12.0)
                                            .strong()
                                            .color(TEXT_PRIMARY),
                                    );
                                });
                                ui.allocate_ui(Vec2::new(220.0, 18.0), |ui| {
                                    ui.label(
                                        RichText::new("Tekst statusu")
                                            .size(12.0)
                                            .strong()
                                            .color(TEXT_PRIMARY),
                                    );
                                });
                                ui.allocate_ui(Vec2::new(150.0, 18.0), |ui| {
                                    ui.label(
                                        RichText::new("Kolor / Akcje")
                                            .size(12.0)
                                            .strong()
                                            .color(TEXT_PRIMARY),
                                    );
                                });
                            });
                        });

                    let mut move_up: Option<usize> = None;
                    let mut move_down: Option<usize> = None;
                    let mut delete_idx: Option<usize> = None;
                    let rule_count = cfg.rules.len();

                    ScrollArea::vertical()
                        .id_salt("state_rules_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (idx, rule) in cfg.rules.iter_mut().enumerate() {
                                let row_bg = if idx % 2 == 0 { ROW_BG_EVEN } else { ROW_BG_ODD };
                                egui::Frame::new()
                                    .fill(row_bg)
                                    .inner_margin(egui::Margin::symmetric(10, 5))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.allocate_ui(Vec2::new(66.0, 24.0), |ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        RichText::new(format!("{}.", idx + 1))
                                                            .size(12.0)
                                                            .color(TEXT_MUTED),
                                                    );
                                                    if ui
                                                        .add_enabled(
                                                            idx > 0,
                                                            egui::Button::new(
                                                                RichText::new("▲").size(10.5),
                                                            ),
                                                        )
                                                        .clicked()
                                                    {
                                                        move_up = Some(idx);
                                                    }
                                                    if ui
                                                        .add_enabled(
                                                            idx + 1 < rule_count,
                                                            egui::Button::new(
                                                                RichText::new("▼").size(10.5),
                                                            ),
                                                        )
                                                        .clicked()
                                                    {
                                                        move_down = Some(idx);
                                                    }
                                                });
                                            });

                                            ui.allocate_ui(Vec2::new(420.0, 24.0), |ui| {
                                                ui.horizontal(|ui| {
                                                    if rule.is_fallback() {
                                                        ui.label(
                                                            RichText::new(
                                                                "W przeciwnym razie (domyślnie)",
                                                            )
                                                            .size(12.0)
                                                            .italics()
                                                            .color(TEXT_MUTED),
                                                        );
                                                        if ui
                                                            .small_button("+ Warunek")
                                                            .clicked()
                                                        {
                                                            rule.input0 = Some(1);
                                                        }
                                                    } else {
                                                        let i0_label = match rule.input0 {
                                                            Some(n) => format!("AI{n} / DI{n}"),
                                                            None => "Brak".to_string(),
                                                        };
                                                        ComboBox::from_id_salt(("st_in0", idx))
                                                            .width(82.0)
                                                            .selected_text(i0_label)
                                                            .show_ui(ui, |ui| {
                                                                if ui
                                                                    .selectable_label(
                                                                        rule.input0.is_none(),
                                                                        "Domyślnie (brak)",
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    rule.input0 = None;
                                                                    rule.input1 = None;
                                                                }
                                                                for n in 1..=4u8 {
                                                                    if ui
                                                                        .selectable_label(
                                                                            rule.input0 == Some(n),
                                                                            format!("AI{n} / DI{n}"),
                                                                        )
                                                                        .clicked()
                                                                    {
                                                                        rule.input0 = Some(n);
                                                                    }
                                                                }
                                                            });

                                                        ComboBox::from_id_salt(("st_op0", idx))
                                                            .width(44.0)
                                                            .selected_text(rule.op0.symbol())
                                                            .show_ui(ui, |ui| {
                                                                for op in StateConditionOp::ALL {
                                                                    if ui
                                                                        .selectable_label(
                                                                            rule.op0 == op,
                                                                            op.symbol(),
                                                                        )
                                                                        .clicked()
                                                                    {
                                                                        rule.op0 = op;
                                                                    }
                                                                }
                                                            });

                                                        ui.add(
                                                            TextEdit::singleline(&mut rule.val0)
                                                                .desired_width(42.0),
                                                        );

                                                        let i1_label = match rule.input1 {
                                                            Some(n) => format!(
                                                                "{} AI{n}",
                                                                if rule.join_or {
                                                                    "OR"
                                                                } else {
                                                                    "AND"
                                                                }
                                                            ),
                                                            None => "+ AND/OR".to_string(),
                                                        };
                                                        ComboBox::from_id_salt(("st_in1", idx))
                                                            .width(86.0)
                                                            .selected_text(i1_label)
                                                            .show_ui(ui, |ui| {
                                                                if ui
                                                                    .selectable_label(
                                                                        rule.input1.is_none(),
                                                                        "Brak 2. warunku",
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    rule.input1 = None;
                                                                }
                                                                for n in 1..=4u8 {
                                                                    if ui
                                                                        .selectable_label(
                                                                            rule.input1 == Some(n)
                                                                                && !rule.join_or,
                                                                            format!("AND AI{n}/DI{n}"),
                                                                        )
                                                                        .clicked()
                                                                    {
                                                                        rule.input1 = Some(n);
                                                                        rule.join_or = false;
                                                                    }
                                                                    if ui
                                                                        .selectable_label(
                                                                            rule.input1 == Some(n)
                                                                                && rule.join_or,
                                                                            format!("OR AI{n}/DI{n}"),
                                                                        )
                                                                        .clicked()
                                                                    {
                                                                        rule.input1 = Some(n);
                                                                        rule.join_or = true;
                                                                    }
                                                                }
                                                            });

                                                        if rule.input1.is_some() {
                                                            ComboBox::from_id_salt(("st_op1", idx))
                                                                .width(44.0)
                                                                .selected_text(rule.op1.symbol())
                                                                .show_ui(ui, |ui| {
                                                                    for op in StateConditionOp::ALL
                                                                    {
                                                                        if ui
                                                                            .selectable_label(
                                                                                rule.op1 == op,
                                                                                op.symbol(),
                                                                            )
                                                                            .clicked()
                                                                        {
                                                                            rule.op1 = op;
                                                                        }
                                                                    }
                                                                });
                                                            ui.add(
                                                                TextEdit::singleline(
                                                                    &mut rule.val1,
                                                                )
                                                                .desired_width(42.0),
                                                            );
                                                        }
                                                    }
                                                });
                                            });

                                            ui.allocate_ui(Vec2::new(72.0, 24.0), |ui| {
                                                ui.add(
                                                    TextEdit::singleline(&mut rule.value)
                                                        .desired_width(58.0),
                                                );
                                            });

                                            ui.allocate_ui(Vec2::new(220.0, 24.0), |ui| {
                                                ui.add(
                                                    TextEdit::singleline(&mut rule.text)
                                                        .desired_width(204.0)
                                                        .hint_text("(brak tekstu)"),
                                                );
                                            });

                                            ui.allocate_ui(Vec2::new(150.0, 24.0), |ui| {
                                                ui.horizontal(|ui| {
                                                    let swatch =
                                                        decode_loxone_color(&rule.icon_color);
                                                    let (rect, _) = ui.allocate_exact_size(
                                                        Vec2::new(14.0, 14.0),
                                                        Sense::hover(),
                                                    );
                                                    ui.painter().rect_filled(rect, 3.0, swatch);

                                                    ComboBox::from_id_salt(("st_col", idx))
                                                        .width(96.0)
                                                        .selected_text(&rule.icon_color)
                                                        .show_ui(ui, |ui| {
                                                            for (label, code) in COLOR_PRESETS {
                                                                if ui
                                                                    .selectable_label(
                                                                        rule.icon_color == *code,
                                                                        *label,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    rule.icon_color =
                                                                        (*code).to_string();
                                                                }
                                                            }
                                                        });

                                                    if ui
                                                        .small_button(
                                                            RichText::new("🗑")
                                                                .size(12.0)
                                                                .color(TEXT_MUTED),
                                                        )
                                                        .on_hover_text(tr("Delete"))
                                                        .clicked()
                                                    {
                                                        delete_idx = Some(idx);
                                                    }
                                                });
                                            });
                                        });
                                    });
                            }
                        });

                    if let Some(i) = move_up {
                        cfg.move_rule_up(i);
                    }
                    if let Some(i) = move_down {
                        cfg.move_rule_down(i);
                    }
                    if let Some(i) = delete_idx {
                        cfg.remove_rule(i);
                    }
                });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("Liczba reguł: {}", cfg.rules.len()))
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
                                        let edits = cfg.to_edits(blk);
                                        if !edits.is_empty() {
                                            pending_edit = Some(ProjectEdit::Batch(edits));
                                        }
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
        "state_config_modal_window",
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

#[derive(Debug, Clone, Default)]
pub struct AlarmClockModalState {
    pub is_open: bool,
    pub page_uuid: String,
    pub block_uuid: String,
    pub block_title: String,
    pub config: Option<AlarmClockConfig>,
    pub time_inputs: Vec<String>,
}

impl AlarmClockModalState {
    pub fn open_for_block(&mut self, project: &LoxoneProject, block_uuid: &str) -> bool {
        let mut found = None;
        for page in &project.pages {
            if let Some(blk) = page.blocks.iter().find(|b| b.uuid == block_uuid) {
                if AlarmClockConfig::is_alarm_clock_block(&blk.block_type) {
                    found = Some((page.uuid.clone(), blk.clone()));
                }
                break;
            }
        }
        let Some((page_uuid, block)) = found else {
            return false;
        };

        let cfg = AlarmClockConfig::from_block(&block);
        self.time_inputs = cfg.entries.iter().map(|e| e.format_time_hhmm()).collect();
        self.is_open = true;
        self.page_uuid = page_uuid;
        self.block_uuid = block.uuid.clone();
        self.block_title = if block.title.is_empty() {
            "Budzik".to_string()
        } else {
            block.title.clone()
        };
        self.config = Some(cfg);
        true
    }
}

const MODE_BITS: &[(u32, &str)] = &[
    (ALARM_MODE_MON, "Pn"),
    (ALARM_MODE_TUE, "Wt"),
    (ALARM_MODE_WED, "Śr"),
    (ALARM_MODE_THU, "Cz"),
    (ALARM_MODE_FRI, "Pt"),
    (ALARM_MODE_SAT, "So"),
    (ALARM_MODE_SUN, "Nd"),
    (ALARM_MODE_HOLIDAY, "Święto"),
    (ALARM_MODE_VACATION, "Urlop"),
    (ALARM_MODE_SICK, "Choroba"),
];

pub fn render_alarm_clock_modal(
    ctx: &Context,
    state: &mut AlarmClockModalState,
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
    let modal_w = (screen_rect.width() * 0.88).clamp(800.0, 1120.0);
    let modal_h = (screen_rect.height() * 0.78).clamp(460.0, 720.0);

    let (backdrop_clicked, popup_was_open) = crate::theme::draw_modal_backdrop(
        ctx,
        "alarm_clock_modal_backdrop",
        egui::Order::Foreground,
        150,
    );

    let win_resp = egui::Window::new("Budzik")
        .id(Id::new("alarm_clock_config_modal_window"))
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

            if state.time_inputs.len() != cfg.entries.len() {
                state.time_inputs = cfg.entries.iter().map(|e| e.format_time_hhmm()).collect();
            }

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("⏰ Konfiguracja Budzika: {}", state.block_title))
                        .size(16.0)
                        .strong()
                        .color(TEXT_PRIMARY),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(RichText::new("✖").size(13.0).color(TEXT_MUTED))
                                .frame(false),
                        )
                        .clicked()
                    {
                        close_requested = true;
                    }
                    ui.add_space(10.0);
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("➕ Dodaj budzik")
                                    .size(12.5)
                                    .strong()
                                    .color(Color32::BLACK),
                            )
                            .fill(LOXONE_GREEN)
                            .corner_radius(CornerRadius::same(4)),
                        )
                        .clicked()
                    {
                        cfg.add_entry();
                        state.time_inputs =
                            cfg.entries.iter().map(|e| e.format_time_hhmm()).collect();
                    }
                });
            });

            ui.add_space(6.0);

            // Top parameters bar
            egui::Frame::new()
                .fill(CARD_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .corner_radius(CornerRadius::same(4))
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("Czas alarmu (s):")
                                .size(12.0)
                                .color(TEXT_MUTED),
                        );
                        ui.add(egui::DragValue::new(&mut cfg.alarm_time_sec).range(5..=7200));
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new("Czas przygotowania PrepTime (s):")
                                .size(12.0)
                                .color(TEXT_MUTED),
                        );
                        ui.add(egui::DragValue::new(&mut cfg.prep_time_sec).range(0..=7200));
                        ui.add_space(12.0);
                        ui.label(RichText::new("Drzemka (s):").size(12.0).color(TEXT_MUTED));
                        ui.add(egui::DragValue::new(&mut cfg.snooze_time_sec).range(30..=3600));
                        ui.add_space(12.0);
                        ui.label(RichText::new("Dźwięk (1-9):").size(12.0).color(TEXT_MUTED));
                        ui.add(egui::DragValue::new(&mut cfg.wake_sound).range(1..=9));
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new("Głośność (%):")
                                .size(12.0)
                                .color(TEXT_MUTED),
                        );
                        ui.add(egui::DragValue::new(&mut cfg.wake_volume).range(0..=100));
                    });
                });

            ui.add_space(8.0);

            let footer_h = 46.0;
            let table_h = (ui.available_height() - footer_h).max(160.0);

            egui::Frame::new()
                .fill(CARD_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .corner_radius(CornerRadius::same(4))
                .show(ui, |ui| {
                    ui.set_min_height(table_h);
                    ui.set_max_height(table_h);

                    egui::Frame::new()
                        .fill(HEADER_BG)
                        .inner_margin(egui::Margin::symmetric(10, 7))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.allocate_ui(Vec2::new(200.0, 18.0), |ui| {
                                    ui.label(
                                        RichText::new("Nazwa budzika")
                                            .size(12.0)
                                            .strong()
                                            .color(TEXT_PRIMARY),
                                    );
                                });
                                ui.allocate_ui(Vec2::new(90.0, 18.0), |ui| {
                                    ui.label(
                                        RichText::new("Godzina (GG:MM)")
                                            .size(12.0)
                                            .strong()
                                            .color(TEXT_PRIMARY),
                                    );
                                });
                                ui.allocate_ui(Vec2::new(560.0, 18.0), |ui| {
                                    ui.label(
                                        RichText::new("Aktywne dni i tryby pracy")
                                            .size(12.0)
                                            .strong()
                                            .color(TEXT_PRIMARY),
                                    );
                                });
                            });
                        });

                    let mut delete_idx: Option<usize> = None;

                    ScrollArea::vertical()
                        .id_salt("alarm_entries_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (idx, entry) in cfg.entries.iter_mut().enumerate() {
                                let row_bg = if idx % 2 == 0 { ROW_BG_EVEN } else { ROW_BG_ODD };
                                egui::Frame::new()
                                    .fill(row_bg)
                                    .inner_margin(egui::Margin::symmetric(10, 6))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.allocate_ui(Vec2::new(200.0, 24.0), |ui| {
                                                ui.horizontal(|ui| {
                                                    if entry.is_internal_default {
                                                        ui.label(
                                                            RichText::new("🔒")
                                                                .size(11.5)
                                                                .color(TEXT_MUTED),
                                                        )
                                                        .on_hover_text("Wbudowany alarm domyślny");
                                                    }
                                                    ui.add(
                                                        TextEdit::singleline(&mut entry.name)
                                                            .desired_width(168.0),
                                                    );
                                                });
                                            });

                                            ui.allocate_ui(Vec2::new(90.0, 24.0), |ui| {
                                                if let Some(time_str) =
                                                    state.time_inputs.get_mut(idx)
                                                {
                                                    let resp = ui.add(
                                                        TextEdit::singleline(time_str)
                                                            .desired_width(68.0),
                                                    );
                                                    if resp.changed() {
                                                        let _ = entry.set_time_hhmm(time_str);
                                                    }
                                                    if resp.lost_focus() {
                                                        *time_str = entry.format_time_hhmm();
                                                    }
                                                }
                                            });

                                            ui.allocate_ui(Vec2::new(560.0, 24.0), |ui| {
                                                ui.horizontal(|ui| {
                                                    for &(bit, label) in MODE_BITS {
                                                        let mut active = entry.has_mode_bit(bit);
                                                        if ui
                                                            .selectable_label(active, label)
                                                            .clicked()
                                                        {
                                                            active = !active;
                                                            entry.set_mode_bit(bit, active);
                                                        }
                                                    }
                                                    ui.add_space(6.0);
                                                    if ui.small_button("Pn-Pt").clicked() {
                                                        entry.modes_mask = ALARM_MODES_WEEKDAYS;
                                                    }
                                                    if ui.small_button("Weekend").clicked() {
                                                        entry.modes_mask = ALARM_MODES_WEEKEND;
                                                    }
                                                    if ui.small_button("Wszystkie").clicked() {
                                                        entry.modes_mask = ALARM_MODES_ALL;
                                                    }
                                                    if !entry.is_internal_default
                                                        && ui
                                                            .small_button(
                                                                RichText::new("🗑")
                                                                    .size(12.0)
                                                                    .color(TEXT_MUTED),
                                                            )
                                                            .clicked()
                                                    {
                                                        delete_idx = Some(idx);
                                                    }
                                                });
                                            });
                                        });
                                    });
                            }
                        });

                    if let Some(i) = delete_idx {
                        cfg.remove_entry(i);
                        state.time_inputs =
                            cfg.entries.iter().map(|e| e.format_time_hhmm()).collect();
                    }
                });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("Liczba budzików: {}", cfg.entries.len()))
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
                                        let edits = cfg.to_edits(blk);
                                        if !edits.is_empty() {
                                            pending_edit = Some(ProjectEdit::Batch(edits));
                                        }
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
        "alarm_clock_config_modal_window",
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_state_and_alarm_clock_modals_open_from_klimczaki() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Klimczaki.Loxone");
        let xml = std::fs::read_to_string(&path).expect("read Klimczaki.Loxone");
        let project = loxone_core::parser::parse_loxone_xml(&xml).expect("parse Klimczaki.Loxone");

        let mut state_modal = StateModalState::default();
        assert!(state_modal.open_for_block(&project, "1abec360-009a-9ee2-ffff62ed8a266bff"));
        assert!(state_modal.is_open);
        assert_eq!(state_modal.block_title, "Ogrzewanie");
        assert_eq!(state_modal.config.as_ref().unwrap().rules.len(), 4);

        let mut alarm_modal = AlarmClockModalState::default();
        assert!(alarm_modal.open_for_block(&project, "1c83a514-0155-bc42-ffff51535bcccc04"));
        assert!(alarm_modal.is_open);
        assert_eq!(alarm_modal.block_title, "Touch Nightlight Air");
        assert_eq!(alarm_modal.config.as_ref().unwrap().entries.len(), 2);
        assert_eq!(alarm_modal.time_inputs, vec!["06:00", "07:00"]);
    }
}
