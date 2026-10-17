use egui::{Align, Color32, Context, CornerRadius, Id, Key, Layout, Pos2, RichText, ScrollArea, Sense, Stroke, TextEdit, Ui, Vec2};
use loxone_core::model::LoxoneProject;
use loxone_core::object_config::{build_object_config_rows, ObjectConfigRow};

use crate::theme::{BORDER_SUBTLE, CARD_BG, PANEL_BG, TEXT_MUTED, TEXT_PRIMARY};

const LOXONE_GREEN: Color32 = Color32::from_rgb(0x69, 0xC8, 0x32);
const ROW_BG_EVEN: Color32 = Color32::from_rgb(0x23, 0x27, 0x2E);
const ROW_BG_ODD: Color32 = Color32::from_rgb(0x29, 0x2E, 0x36);
const ROW_BG_HOVER: Color32 = Color32::from_rgb(0x32, 0x39, 0x44);
const HEADER_BG: Color32 = Color32::from_rgb(0x1D, 0x20, 0x26);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectConfigSortColumn {
    DefaultOrder,
    Input,
    Output,
    Name,
    DisplayName,
    Device,
    Room,
    RoomType,
    Category,
    ObjectType,
}

#[derive(Debug, Clone)]
pub enum ObjectConfigModalAction {
    Apply {
        page_uuid: String,
        block_uuid: String,
        rows: Vec<ObjectConfigRow>,
    },
    InspectObject(String),
}

#[derive(Debug, Clone)]
pub struct ObjectConfigModalState {
    pub is_open: bool,
    pub page_uuid: String,
    pub block_uuid: String,
    pub block_title: String,
    pub block_type: String,
    pub filter_by_selection: bool,
    pub search_query: String,
    pub sort_column: ObjectConfigSortColumn,
    pub sort_ascending: bool,
    pub rows: Vec<ObjectConfigRow>,
}

impl Default for ObjectConfigModalState {
    fn default() -> Self {
        Self {
            is_open: false,
            page_uuid: String::new(),
            block_uuid: String::new(),
            block_title: String::new(),
            block_type: String::new(),
            filter_by_selection: true,
            search_query: String::new(),
            sort_column: ObjectConfigSortColumn::DefaultOrder,
            sort_ascending: true,
            rows: Vec::new(),
        }
    }
}

impl ObjectConfigModalState {
    pub fn open_for_block(&mut self, project: &LoxoneProject, block_uuid: &str) -> bool {
        let mut found = None;
        for page in &project.pages {
            if let Some(blk) = page.blocks.iter().find(|b| b.uuid == block_uuid) {
                found = Some((page.uuid.clone(), blk.clone()));
                break;
            }
        }
        let Some((page_uuid, block)) = found else {
            return false;
        };

        let rows = build_object_config_rows(project, &page_uuid, &block.uuid);
        let has_any_selected = rows.iter().any(|r| r.is_input || r.is_output);

        self.is_open = true;
        self.page_uuid = page_uuid;
        self.block_uuid = block.uuid.clone();
        self.block_title = if block.title.is_empty() {
            block.block_type.clone()
        } else {
            block.title.clone()
        };
        self.block_type = block.block_type.clone();
        self.filter_by_selection = has_any_selected;
        self.search_query.clear();
        self.sort_column = ObjectConfigSortColumn::DefaultOrder;
        self.sort_ascending = true;
        self.rows = rows;
        true
    }

    pub fn visible_row_indices(&self) -> Vec<usize> {
        let q = self.search_query.trim().to_lowercase();
        let mut indices: Vec<usize> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                if self.filter_by_selection && !row.is_input && !row.is_output {
                    return false;
                }
                if !q.is_empty() {
                    let matches = row.name.to_lowercase().contains(&q)
                        || row.display_name.to_lowercase().contains(&q)
                        || row.device_name.to_lowercase().contains(&q)
                        || row.room_name.to_lowercase().contains(&q)
                        || row.room_type.to_lowercase().contains(&q)
                        || row.category_name.to_lowercase().contains(&q)
                        || row.object_type.to_lowercase().contains(&q);
                    if !matches {
                        return false;
                    }
                }
                true
            })
            .map(|(idx, _)| idx)
            .collect();

        if self.sort_column != ObjectConfigSortColumn::DefaultOrder {
            indices.sort_by(|&ia, &ib| {
                let a = &self.rows[ia];
                let b = &self.rows[ib];
                let cmp = match self.sort_column {
                    ObjectConfigSortColumn::DefaultOrder => std::cmp::Ordering::Equal,
                    ObjectConfigSortColumn::Input => b.is_input.cmp(&a.is_input),
                    ObjectConfigSortColumn::Output => b.is_output.cmp(&a.is_output),
                    ObjectConfigSortColumn::Name => a.name.cmp(&b.name),
                    ObjectConfigSortColumn::DisplayName => a.display_name.cmp(&b.display_name),
                    ObjectConfigSortColumn::Device => a.device_name.cmp(&b.device_name),
                    ObjectConfigSortColumn::Room => a.room_name.cmp(&b.room_name),
                    ObjectConfigSortColumn::RoomType => a.room_type.cmp(&b.room_type),
                    ObjectConfigSortColumn::Category => a.category_name.cmp(&b.category_name),
                    ObjectConfigSortColumn::ObjectType => a.object_type.cmp(&b.object_type),
                };
                if self.sort_ascending {
                    cmp.then_with(|| ia.cmp(&ib))
                } else {
                    cmp.reverse().then_with(|| ia.cmp(&ib))
                }
            });
        }

        indices
    }
}

fn draw_loxone_green_checkbox(ui: &mut Ui, checked: &mut bool, enabled: bool) -> bool {
    let desired = Vec2::new(16.0, 16.0);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(desired, sense);
    let mut changed = false;
    if enabled && resp.clicked() {
        *checked = !*checked;
        changed = true;
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        if !enabled {
            return false;
        }
        if *checked {
            painter.rect(
                rect,
                CornerRadius::same(2),
                LOXONE_GREEN,
                Stroke::new(1.0_f32, LOXONE_GREEN),
                egui::StrokeKind::Inside,
            );
            // Draw crisp white checkmark inside the green box
            let p1 = Pos2::new(rect.min.x + 3.5, rect.min.y + 8.2);
            let p2 = Pos2::new(rect.min.x + 6.8, rect.min.y + 11.5);
            let p3 = Pos2::new(rect.min.x + 12.6, rect.min.y + 4.5);
            let stroke = Stroke::new(1.9_f32, Color32::WHITE);
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
        } else {
            let border_col = if resp.hovered() {
                LOXONE_GREEN
            } else {
                Color32::from_rgb(0x68, 0x72, 0x82)
            };
            painter.rect(
                rect,
                CornerRadius::same(2),
                Color32::from_rgb(0x1A, 0x1D, 0x22),
                Stroke::new(1.2_f32, border_col),
                egui::StrokeKind::Inside,
            );
        }
    }

    changed
}
#[allow(clippy::too_many_arguments)]
fn render_header_cell(
    ui: &mut Ui,
    width: f32,
    label: &str,
    column: ObjectConfigSortColumn,
    current_col: &mut ObjectConfigSortColumn,
    sort_asc: &mut bool,
    centered: bool,
) {
    let layout = if centered {
        Layout::centered_and_justified(egui::Direction::LeftToRight)
    } else {
        Layout::left_to_right(Align::Center)
    };
    ui.allocate_ui_with_layout(Vec2::new(width, 24.0), layout, |ui| {
        let arrow = if *current_col == column {
            if *sort_asc { " ▲" } else { " ▼" }
        } else {
            ""
        };
        let text = format!("{}{}", label, arrow);
        let resp = ui.add(
            egui::Button::new(
                RichText::new(text)
                    .size(11.5)
                    .strong()
                    .color(if *current_col == column {
                        LOXONE_GREEN
                    } else {
                        Color32::from_rgb(0xD4, 0xDA, 0xE2)
                    }),
            )
            .frame(false),
        );
        if resp.clicked() {
            if *current_col == column {
                if *sort_asc {
                    *sort_asc = false;
                } else {
                    *current_col = ObjectConfigSortColumn::DefaultOrder;
                    *sort_asc = true;
                }
            } else {
                *current_col = column;
                *sort_asc = true;
            }
        }
    });
}

fn render_cell_label(ui: &mut Ui, width: f32, text: &str, color: Color32, strong: bool) -> egui::Response {
    ui.allocate_ui_with_layout(
        Vec2::new(width, 22.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_clip_rect(ui.max_rect());
            let mut rt = RichText::new(text).size(12.0).color(color);
            if strong {
                rt = rt.strong();
            }
            ui.add(egui::Label::new(rt).truncate().sense(Sense::click()))
        },
    )
    .inner
}

pub fn render_object_config_modal(
    ctx: &Context,
    state: &mut ObjectConfigModalState,
) -> Option<ObjectConfigModalAction> {
    if !state.is_open {
        return None;
    }

    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        state.is_open = false;
        return None;
    }

    let screen = ctx.screen_rect();
    ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        Id::new("object_config_modal_backdrop"),
    ))
    .rect_filled(screen, 0.0, Color32::from_black_alpha(150));

    let mut action: Option<ObjectConfigModalAction> = None;
    let mut open_flag = state.is_open;
    let mut should_close = false;

    let modal_w = (screen.width() - 80.0).clamp(760.0, 1180.0);
    let modal_h = (screen.height() - 90.0).clamp(460.0, 740.0);

    egui::Window::new(format!("Konfiguracja obiektu — {}", state.block_title))
        .id(Id::new("loxone_object_config_modal_window"))
        .open(&mut open_flag)
        .collapsible(false)
        .resizable(true)
        .default_size(Vec2::new(modal_w, modal_h))
        .min_size(Vec2::new(720.0, 400.0))
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .order(egui::Order::Foreground)
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(PANEL_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(egui::Margin::same(12)),
        )
        .show(ctx, |ui| {
            // Top bar: "Filtruj według wyboru" checkbox on the left + "Szukaj..." search bar on the right
            ui.horizontal(|ui| {
                draw_loxone_green_checkbox(ui, &mut state.filter_by_selection, true);
                let lbl = ui.add(
                    egui::Label::new(
                        RichText::new("Filtruj według wyboru")
                            .size(12.5)
                            .color(TEXT_PRIMARY),
                    )
                    .sense(Sense::click()),
                );
                if lbl.clicked() {
                    state.filter_by_selection = !state.filter_by_selection;
                }

                let selected_in = state.rows.iter().filter(|r| r.is_input).count();
                let selected_out = state.rows.iter().filter(|r| r.is_output).count();
                ui.add_space(10.0);
                ui.label(
                    RichText::new(format!(
                        "(Wejścia: {}  •  Wyjścia: {}  •  Wszystkich: {})",
                        selected_in,
                        selected_out,
                        state.rows.len()
                    ))
                    .size(11.5)
                    .color(TEXT_MUTED),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if !state.search_query.is_empty() && ui.small_button("✕").clicked() {
                        state.search_query.clear();
                    }
                    ui.add(
                        TextEdit::singleline(&mut state.search_query)
                            .hint_text("🔍 Szukaj...")
                            .desired_width(240.0),
                    );
                });
            });

            ui.add_space(8.0);

            // Column widths
            let avail_w = ui.available_width();
            let col_in = 58.0;
            let col_out = 58.0;
            let flex = (avail_w - col_in - col_out - 64.0).max(520.0);
            let col_name = flex * 0.19;
            let col_disp = flex * 0.17;
            let col_dev = flex * 0.16;
            let col_room = flex * 0.12;
            let col_room_type = flex * 0.12;
            let col_cat = flex * 0.11;
            let col_obj_type = flex * 0.13;

            // Table Header
            egui::Frame::new()
                .fill(HEADER_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .inner_margin(egui::Margin::symmetric(6, 3))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        render_header_cell(
                            ui,
                            col_in,
                            "Wejście",
                            ObjectConfigSortColumn::Input,
                            &mut state.sort_column,
                            &mut state.sort_ascending,
                            true,
                        );
                        render_header_cell(
                            ui,
                            col_out,
                            "Wyjście",
                            ObjectConfigSortColumn::Output,
                            &mut state.sort_column,
                            &mut state.sort_ascending,
                            true,
                        );
                        render_header_cell(
                            ui,
                            col_name,
                            "Nazwa",
                            ObjectConfigSortColumn::Name,
                            &mut state.sort_column,
                            &mut state.sort_ascending,
                            false,
                        );
                        render_header_cell(
                            ui,
                            col_disp,
                            "Wyświetlana nazwa",
                            ObjectConfigSortColumn::DisplayName,
                            &mut state.sort_column,
                            &mut state.sort_ascending,
                            false,
                        );
                        render_header_cell(
                            ui,
                            col_dev,
                            "Urządzenie",
                            ObjectConfigSortColumn::Device,
                            &mut state.sort_column,
                            &mut state.sort_ascending,
                            false,
                        );
                        render_header_cell(
                            ui,
                            col_room,
                            "Pomieszczenie",
                            ObjectConfigSortColumn::Room,
                            &mut state.sort_column,
                            &mut state.sort_ascending,
                            false,
                        );
                        render_header_cell(
                            ui,
                            col_room_type,
                            "Typ pomieszczenia",
                            ObjectConfigSortColumn::RoomType,
                            &mut state.sort_column,
                            &mut state.sort_ascending,
                            false,
                        );
                        render_header_cell(
                            ui,
                            col_cat,
                            "Kategoria",
                            ObjectConfigSortColumn::Category,
                            &mut state.sort_column,
                            &mut state.sort_ascending,
                            false,
                        );
                        render_header_cell(
                            ui,
                            col_obj_type,
                            "Typ obiektu",
                            ObjectConfigSortColumn::ObjectType,
                            &mut state.sort_column,
                            &mut state.sort_ascending,
                            false,
                        );
                    });
                });

            let visible_indices = state.visible_row_indices();
            let table_h = (ui.available_height() - 46.0).max(200.0);

            egui::Frame::new()
                .fill(CARD_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .show(ui, |ui| {
                    ui.set_min_height(table_h);
                    ScrollArea::vertical()
                        .id_salt("object_config_rows_scroll")
                        .max_height(table_h)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            if visible_indices.is_empty() {
                                ui.add_space(28.0);
                                ui.vertical_centered(|ui| {
                                    ui.label(
                                        RichText::new(
                                            "Brak pasujących obiektów. Odznacz „Filtruj według wyboru”, aby zobaczyć wszystkie dostępne obiekty w projekcie.",
                                        )
                                        .size(12.5)
                                        .color(TEXT_MUTED),
                                    );
                                });
                            }
                            for (vis_pos, row_idx) in visible_indices.into_iter().enumerate() {
                                let row = &mut state.rows[row_idx];
                                let base_bg = if vis_pos % 2 == 0 {
                                    ROW_BG_EVEN
                                } else {
                                    ROW_BG_ODD
                                };
                                let row_resp = egui::Frame::new()
                                    .fill(base_bg)
                                    .inner_margin(egui::Margin::symmetric(6, 2))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing.x = 6.0;
                                            ui.allocate_ui_with_layout(
                                                Vec2::new(col_in, 22.0),
                                                Layout::centered_and_justified(
                                                    egui::Direction::LeftToRight,
                                                ),
                                                |ui| {
                                                    draw_loxone_green_checkbox(
                                                        ui,
                                                        &mut row.is_input,
                                                        row.can_be_input,
                                                    );
                                                },
                                            );
                                            ui.allocate_ui_with_layout(
                                                Vec2::new(col_out, 22.0),
                                                Layout::centered_and_justified(
                                                    egui::Direction::LeftToRight,
                                                ),
                                                |ui| {
                                                    draw_loxone_green_checkbox(
                                                        ui,
                                                        &mut row.is_output,
                                                        row.can_be_output,
                                                    );
                                                },
                                            );
                                            let r_name = render_cell_label(
                                                ui,
                                                col_name,
                                                &row.name,
                                                TEXT_PRIMARY,
                                                true,
                                            );
                                            if r_name.clicked() {
                                                action = Some(
                                                    ObjectConfigModalAction::InspectObject(
                                                        row.target_uuid.clone(),
                                                    ),
                                                );
                                            }
                                            render_cell_label(
                                                ui,
                                                col_disp,
                                                &row.display_name,
                                                TEXT_PRIMARY,
                                                false,
                                            );
                                            render_cell_label(
                                                ui,
                                                col_dev,
                                                &row.device_name,
                                                Color32::from_rgb(0xC6, 0xCE, 0xDA),
                                                false,
                                            );
                                            render_cell_label(
                                                ui,
                                                col_room,
                                                &row.room_name,
                                                Color32::from_rgb(0xC6, 0xCE, 0xDA),
                                                false,
                                            );
                                            render_cell_label(
                                                ui,
                                                col_room_type,
                                                &row.room_type,
                                                TEXT_MUTED,
                                                false,
                                            );
                                            render_cell_label(
                                                ui,
                                                col_cat,
                                                &row.category_name,
                                                Color32::from_rgb(0xC6, 0xCE, 0xDA),
                                                false,
                                            );
                                            render_cell_label(
                                                ui,
                                                col_obj_type,
                                                &row.object_type,
                                                TEXT_MUTED,
                                                false,
                                            );
                                        });
                                    })
                                    .response;

                                if row_resp.hovered() {
                                    ui.painter().rect_stroke(
                                        row_resp.rect,
                                        CornerRadius::ZERO,
                                        Stroke::new(1.0_f32, ROW_BG_HOVER),
                                        egui::StrokeKind::Inside,
                                    );
                                }
                                // Subtle bottom divider line matching Loxone Config table grid
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

            ui.add_space(10.0);

            // Bottom action buttons: Anuluj + Potwierdź
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let confirm_btn = egui::Button::new(
                    RichText::new("  Potwierdź  ")
                        .size(13.0)
                        .strong()
                        .color(Color32::from_rgb(0x11, 0x16, 0x0D)),
                )
                .fill(LOXONE_GREEN)
                .min_size(Vec2::new(112.0, 28.0));

                if ui.add(confirm_btn).clicked() {
                    action = Some(ObjectConfigModalAction::Apply {
                        page_uuid: state.page_uuid.clone(),
                        block_uuid: state.block_uuid.clone(),
                        rows: state.rows.clone(),
                    });
                    should_close = true;
                }

                ui.add_space(8.0);

                let cancel_btn = egui::Button::new(
                    RichText::new("  Anuluj  ")
                        .size(13.0)
                        .color(TEXT_PRIMARY),
                )
                .fill(Color32::from_rgb(0x38, 0x3E, 0x48))
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .min_size(Vec2::new(100.0, 28.0));

                if ui.add(cancel_btn).clicked() {
                    should_close = true;
                }
            });
        });

    if !open_flag || should_close {
        state.is_open = false;
    }

    action
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_object_config_modal_state_filters_and_sorts_burglar_alarm() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.pop();
        path.pop();
        path.push("Klimczaki.Loxone");
        let project =
            loxone_core::parser::load_loxone_file(&path).expect("parse Klimczaki.Loxone");

        let mut modal = ObjectConfigModalState::default();
        assert!(modal.open_for_block(&project, "19eb19a9-024d-3a10-ffff51535bcccc04"));
        assert!(modal.is_open);
        assert!(modal.filter_by_selection);
        let initial_selected = modal.visible_row_indices().len();
        assert_eq!(initial_selected, 37); // 11 inputs + 26 outputs

        // Search filter narrows visible rows
        modal.search_query = "Wiatrołap".to_string();
        let filtered = modal.visible_row_indices();
        assert!(filtered.len() >= 2);

        // Unchecking "Filtruj według wyboru" shows all candidate objects
        modal.search_query.clear();
        modal.filter_by_selection = false;
        assert!(modal.visible_row_indices().len() > initial_selected);

        // Verify headless egui rendering completes without panic
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let _ = render_object_config_modal(ctx, &mut modal);
        });
    }
}
