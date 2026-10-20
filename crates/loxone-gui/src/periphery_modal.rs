use eframe::egui::{self, Color32, CornerRadius, Margin, RichText, ScrollArea, Stroke};
use loxone_core::model::LoxoneProject;
use loxone_core::{
    build_provision_hardware_edit, build_serial_swap_edit, collect_parent_bus_candidates,
    collect_provisioned_hardware, generate_mock_serial, HardwareCategory, HardwareTemplateKind,
    ProjectEdit,
};

use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PeripheryModalTab {
    #[default]
    ProvisionDevice,
    HardwareInventory,
}

#[derive(Debug, Clone, Default)]
pub struct PeripheryModalState {
    pub is_open: bool,
    pub active_tab: PeripheryModalTab,
    pub selected_category: HardwareCategory,
    pub selected_template: Option<HardwareTemplateKind>,
    pub selected_parent_uuid: String,
    pub title_draft: String,
    pub serial_draft: String,
    pub place_uuid_draft: String,
    pub category_uuid_draft: String,
    pub status_message: Option<String>,
    // Inventory tab
    pub inventory_filter: String,
    pub editing_serial_uuid: Option<String>,
    pub editing_serial_draft: String,
}

impl PeripheryModalState {
    pub fn open_provision(&mut self, project: &LoxoneProject, parent_uuid: Option<String>) {
        self.is_open = true;
        self.active_tab = PeripheryModalTab::ProvisionDevice;
        let default_tpl = HardwareTemplateKind::TreeExtension;
        self.selected_category = default_tpl.category();
        self.selected_template = Some(default_tpl);
        self.title_draft = default_tpl.display_name().to_string();
        self.serial_draft.clear();
        self.place_uuid_draft.clear();
        self.category_uuid_draft.clear();
        self.status_message = None;

        if let Some(puuid) = parent_uuid {
            self.selected_parent_uuid = puuid;
        } else {
            let candidates = collect_parent_bus_candidates(project, default_tpl.category());
            self.selected_parent_uuid = candidates
                .first()
                .map(|c| c.uuid.clone())
                .unwrap_or_default();
        }
    }

    pub fn open_inventory(&mut self, focus_uuid: Option<String>, project: &LoxoneProject) {
        self.is_open = true;
        self.active_tab = PeripheryModalTab::HardwareInventory;
        self.status_message = None;
        if let Some(uuid) = focus_uuid {
            let items = collect_provisioned_hardware(project);
            if let Some(found) = items.into_iter().find(|i| i.uuid == uuid) {
                self.editing_serial_uuid = Some(found.uuid);
                self.editing_serial_draft = found.serial;
            }
        }
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.status_message = None;
        self.editing_serial_uuid = None;
    }

    fn select_template(&mut self, project: &LoxoneProject, tpl: HardwareTemplateKind) {
        self.selected_category = tpl.category();
        self.selected_template = Some(tpl);
        self.title_draft = tpl.display_name().to_string();
        let candidates = collect_parent_bus_candidates(project, tpl.category());
        if !candidates
            .iter()
            .any(|c| c.uuid == self.selected_parent_uuid)
        {
            self.selected_parent_uuid = candidates
                .first()
                .map(|c| c.uuid.clone())
                .unwrap_or_default();
        }
    }
}

pub fn render_periphery_modal(
    ctx: &egui::Context,
    state: &mut PeripheryModalState,
    project: Option<&LoxoneProject>,
) -> Vec<ProjectEdit> {
    let mut edits = Vec::new();
    if !state.is_open {
        return edits;
    }
    let Some(project) = project else {
        state.close();
        return edits;
    };

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.close();
        return edits;
    }

    let (backdrop_clicked, popup_was_open) = theme::draw_modal_backdrop(
        ctx,
        "periphery_modal_backdrop",
        egui::Order::Foreground,
        150,
    );

    let window_resp = egui::Window::new("periphery_hardware_modal")
        .id(egui::Id::new("periphery_hardware_modal"))
        .order(egui::Order::Tooltip)
        .title_bar(false)
        .collapsible(false)
        .resizable(true)
        .default_size([880.0, 620.0])
        .min_size([740.0, 480.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(theme::BG_PANEL)
                .stroke(Stroke::new(1.5_f32, theme::ACCENT_PRIMARY))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::same(16)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("🔌 Periphery Hardware & Device Provisioning")
                        .size(16.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("✕ Close").size(12.0).color(theme::TEXT_PRIMARY),
                            )
                            .fill(theme::BG_WIDGET)
                            .corner_radius(CornerRadius::same(6)),
                        )
                        .clicked()
                    {
                        state.close();
                    }
                });
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                for (tab, label) in [
                    (
                        PeripheryModalTab::ProvisionDevice,
                        "➕ Provision Extension / Tree / Air Device",
                    ),
                    (
                        PeripheryModalTab::HardwareInventory,
                        "🔢 Hardware Serial Inventory & Swap",
                    ),
                ] {
                    let active = state.active_tab == tab;
                    let fill = if active {
                        theme::ACCENT_GREEN_DIM
                    } else {
                        theme::BG_WIDGET
                    };
                    let stroke = if active {
                        Stroke::new(1.0_f32, theme::ACCENT_PRIMARY)
                    } else {
                        Stroke::new(1.0_f32, theme::BORDER_SUBTLE)
                    };
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(label).size(12.5).strong().color(if active {
                                    theme::ACCENT_PRIMARY
                                } else {
                                    theme::TEXT_SECONDARY
                                }),
                            )
                            .fill(fill)
                            .stroke(stroke)
                            .corner_radius(CornerRadius::same(6)),
                        )
                        .clicked()
                    {
                        state.active_tab = tab;
                    }
                }
            });

            if let Some(msg) = &state.status_message {
                ui.add_space(6.0);
                egui::Frame::NONE
                    .fill(theme::ACCENT_GREEN_DIM)
                    .stroke(Stroke::new(1.0_f32, theme::ACCENT_PRIMARY))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.label(RichText::new(msg).size(11.5).color(theme::ACCENT_PRIMARY));
                    });
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            match state.active_tab {
                PeripheryModalTab::ProvisionDevice => {
                    if let Some(edit) = render_provision_tab(ui, state, project) {
                        edits.push(edit);
                    }
                }
                PeripheryModalTab::HardwareInventory => {
                    if let Some(edit) = render_inventory_tab(ui, state, project) {
                        edits.push(edit);
                    }
                }
            }
        });

    let win_rect = window_resp.map(|r| r.response.rect);
    if state.is_open
        && theme::should_close_modal_on_outside_click(
            ctx,
            "periphery_hardware_modal",
            backdrop_clicked,
            popup_was_open,
            win_rect,
        )
    {
        state.close();
    }

    edits
}
fn render_provision_tab(
    ui: &mut egui::Ui,
    state: &mut PeripheryModalState,
    project: &LoxoneProject,
) -> Option<ProjectEdit> {
    let mut emitted_edit = None;

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Category:")
                .size(12.0)
                .color(theme::TEXT_SECONDARY),
        );
        for cat in HardwareCategory::ALL {
            let active = state.selected_category == cat;
            let btn = egui::Button::new(RichText::new(cat.label()).size(12.0).color(if active {
                theme::ACCENT_PRIMARY
            } else {
                theme::TEXT_PRIMARY
            }))
            .fill(if active {
                theme::ACCENT_GREEN_DIM
            } else {
                theme::BG_WIDGET
            })
            .stroke(Stroke::new(
                1.0_f32,
                if active {
                    theme::ACCENT_PRIMARY
                } else {
                    theme::BORDER_SUBTLE
                },
            ))
            .corner_radius(CornerRadius::same(6));
            if ui.add(btn).clicked() {
                state.selected_category = cat;
                if let Some(first_tpl) = HardwareTemplateKind::ALL
                    .into_iter()
                    .find(|t| t.category() == cat)
                {
                    state.select_template(project, first_tpl);
                }
            }
        }
    });

    ui.add_space(10.0);

    ui.columns(2, |cols| {
        // Left column: Hardware Templates catalog
        cols[0].label(
            RichText::new("1. Select Hardware Model")
                .size(13.0)
                .strong()
                .color(theme::TEXT_PRIMARY),
        );
        cols[0].add_space(6.0);
        let selected_cat = state.selected_category;
        ScrollArea::vertical()
            .id_salt("provision_tpl_scroll")
            .max_height(420.0)
            .show(&mut cols[0], |ui| {
                for tpl in HardwareTemplateKind::ALL
                    .into_iter()
                    .filter(move |t| t.category() == selected_cat)
                {
                    let selected = state.selected_template == Some(tpl);
                    let fill = if selected {
                        theme::ACCENT_GREEN_DIM
                    } else {
                        theme::BG_DARK
                    };
                    let stroke = if selected {
                        Stroke::new(1.5_f32, theme::ACCENT_PRIMARY)
                    } else {
                        Stroke::new(1.0_f32, theme::BORDER_SUBTLE)
                    };
                    let resp = egui::Frame::NONE
                        .fill(fill)
                        .stroke(stroke)
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::symmetric(12, 8))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(tpl.display_name())
                                        .size(13.0)
                                        .strong()
                                        .color(if selected {
                                            theme::ACCENT_PRIMARY
                                        } else {
                                            theme::TEXT_PRIMARY
                                        }),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new(tpl.node_type())
                                                .size(10.5)
                                                .monospace()
                                                .color(theme::TEXT_MUTED),
                                        );
                                    },
                                );
                            });
                            ui.label(
                                RichText::new(tpl.summary())
                                    .size(11.0)
                                    .color(theme::TEXT_SECONDARY),
                            );
                        })
                        .response
                        .interact(egui::Sense::click());

                    if resp.clicked() {
                        state.select_template(project, tpl);
                    }
                    ui.add_space(6.0);
                }
            });

        // Right column: Configuration & Parent Bus
        cols[1].label(
            RichText::new("2. Configure Device & Bus Target")
                .size(13.0)
                .strong()
                .color(theme::TEXT_PRIMARY),
        );
        cols[1].add_space(6.0);

        let Some(tpl) = state.selected_template else {
            cols[1].label(
                RichText::new("Select a hardware template on the left.")
                    .color(theme::TEXT_MUTED),
            );
            return;
        };

        egui::Frame::NONE
            .fill(theme::BG_DARK)
            .stroke(Stroke::new(1.0_f32, theme::BORDER_SUBTLE))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(14))
            .show(&mut cols[1], |ui| {
                let candidates = collect_parent_bus_candidates(project, tpl.category());
                ui.label(
                    RichText::new("Parent Bus / Extension")
                        .size(11.5)
                        .color(theme::TEXT_SECONDARY),
                );
                let current_parent_label = candidates
                    .iter()
                    .find(|c| c.uuid == state.selected_parent_uuid)
                    .map(|c| c.label.clone())
                    .unwrap_or_else(|| "Auto / Periphery Root".to_string());

                egui::ComboBox::from_id_salt("provision_parent_bus_combo")
                    .selected_text(current_parent_label)
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for cand in &candidates {
                            ui.selectable_value(
                                &mut state.selected_parent_uuid,
                                cand.uuid.clone(),
                                format!("{} [{}]", cand.label, cand.node_type),
                            );
                        }
                    });

                ui.add_space(10.0);
                ui.label(
                    RichText::new("Device Title")
                        .size(11.5)
                        .color(theme::TEXT_SECONDARY),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut state.title_draft)
                        .desired_width(ui.available_width()),
                );

                ui.add_space(10.0);
                ui.label(
                    RichText::new(format!(
                        "Serial Number (optional — prefix {} auto-generated if blank)",
                        tpl.serial_prefix()
                    ))
                    .size(11.5)
                    .color(theme::TEXT_SECONDARY),
                );
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut state.serial_draft)
                            .hint_text(format!("{}XXXX", tpl.serial_prefix()))
                            .desired_width(ui.available_width() - 95.0),
                    );
                    if ui
                        .add(
                            egui::Button::new(RichText::new("🎲 Generate").size(11.5))
                                .fill(theme::BG_WIDGET)
                                .corner_radius(CornerRadius::same(6)),
                        )
                        .clicked()
                    {
                        state.serial_draft = generate_mock_serial(tpl);
                    }
                });

                ui.add_space(10.0);
                ui.label(
                    RichText::new("Default Room (optional — propagates to child I/O channels)")
                        .size(11.5)
                        .color(theme::TEXT_SECONDARY),
                );
                let selected_room_title = project
                    .places
                    .get(&state.place_uuid_draft)
                    .map(|r| r.title.as_str())
                    .unwrap_or("(None)");
                egui::ComboBox::from_id_salt("provision_room_combo")
                    .selected_text(selected_room_title)
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut state.place_uuid_draft,
                            String::new(),
                            "(None)",
                        );
                        let mut rooms: Vec<_> = project.places.values().collect();
                        rooms.sort_by(|a, b| a.title.cmp(&b.title));
                        for room in rooms {
                            ui.selectable_value(
                                &mut state.place_uuid_draft,
                                room.uuid.clone(),
                                &room.title,
                            );
                        }
                    });

                ui.add_space(10.0);
                ui.label(
                    RichText::new("Default Category (optional)")
                        .size(11.5)
                        .color(theme::TEXT_SECONDARY),
                );
                let selected_cat_title = project
                    .categories
                    .get(&state.category_uuid_draft)
                    .map(|c| c.title.as_str())
                    .unwrap_or("(None)");
                egui::ComboBox::from_id_salt("provision_category_combo")
                    .selected_text(selected_cat_title)
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut state.category_uuid_draft,
                            String::new(),
                            "(None)",
                        );
                        let mut cats: Vec<_> = project.categories.values().collect();
                        cats.sort_by(|a, b| a.title.cmp(&b.title));
                        for cat in cats {
                            ui.selectable_value(
                                &mut state.category_uuid_draft,
                                cat.uuid.clone(),
                                &cat.title,
                            );
                        }
                    });

                ui.add_space(16.0);
                let provision_btn = egui::Button::new(
                    RichText::new(format!("➕ Provision {}", tpl.display_name()))
                        .size(13.0)
                        .strong()
                        .color(Color32::BLACK),
                )
                .fill(theme::ACCENT_PRIMARY)
                .corner_radius(CornerRadius::same(6))
                .min_size(egui::vec2(ui.available_width(), 34.0));

                if ui.add(provision_btn).clicked() {
                    let parent_opt = (!state.selected_parent_uuid.is_empty())
                        .then_some(state.selected_parent_uuid.as_str());
                    let room_opt = (!state.place_uuid_draft.is_empty())
                        .then_some(state.place_uuid_draft.as_str());
                    let cat_opt = (!state.category_uuid_draft.is_empty())
                        .then_some(state.category_uuid_draft.as_str());
                    let edit = build_provision_hardware_edit(
                        project,
                        tpl,
                        &state.title_draft,
                        &state.serial_draft,
                        parent_opt,
                        room_opt,
                        cat_opt,
                    );
                    state.status_message = Some(format!(
                        "Provisioned '{}' ({}) with pre-populated I/O channels.",
                        state.title_draft,
                        tpl.node_type()
                    ));
                    emitted_edit = Some(edit);
                }
            });
    });

    emitted_edit
}
fn render_inventory_tab(
    ui: &mut egui::Ui,
    state: &mut PeripheryModalState,
    project: &LoxoneProject,
) -> Option<ProjectEdit> {
    let mut emitted_edit = None;
    let items = collect_provisioned_hardware(project);
    let q = state.inventory_filter.trim().to_ascii_lowercase();

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("Provisioned Hardware ({} devices)", items.len()))
                .size(13.0)
                .strong()
                .color(theme::TEXT_PRIMARY),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut state.inventory_filter)
                    .hint_text("🔎 Filter by name, serial, bus, or type...")
                    .desired_width(260.0),
            );
        });
    });

    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "Swap a physical device's Serial Number in-place while preserving all child I/O channels and schematic wire links.",
        )
        .size(11.0)
        .color(theme::TEXT_SECONDARY),
    );
    ui.add_space(8.0);

    ScrollArea::vertical()
        .id_salt("hardware_inventory_scroll")
        .max_height(430.0)
        .show(ui, |ui| {
            for item in items.iter().filter(|it| {
                q.is_empty()
                    || it.title.to_ascii_lowercase().contains(&q)
                    || it.serial.to_ascii_lowercase().contains(&q)
                    || it.node_type.to_ascii_lowercase().contains(&q)
                    || it.parent_label.to_ascii_lowercase().contains(&q)
            }) {
                let is_editing = state.editing_serial_uuid.as_deref() == Some(item.uuid.as_str());
                let stroke = if is_editing {
                    Stroke::new(1.5_f32, theme::ACCENT_PRIMARY)
                } else {
                    Stroke::new(1.0_f32, theme::BORDER_SUBTLE)
                };

                egui::Frame::NONE
                    .fill(theme::BG_DARK)
                    .stroke(stroke)
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(&item.title)
                                    .size(13.0)
                                    .strong()
                                    .color(theme::TEXT_PRIMARY),
                            );
                            ui.label(
                                RichText::new(format!("[{}]", item.node_type))
                                    .size(11.0)
                                    .monospace()
                                    .color(theme::ACCENT_PRIMARY),
                            );
                            ui.label(
                                RichText::new(format!("• {} • {} I/O", item.parent_label, item.io_count))
                                    .size(11.0)
                                    .color(theme::TEXT_SECONDARY),
                            );

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if is_editing {
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new("Cancel").size(11.0),
                                                )
                                                .fill(theme::BG_WIDGET)
                                                .corner_radius(CornerRadius::same(5)),
                                            )
                                            .clicked()
                                        {
                                            state.editing_serial_uuid = None;
                                        }
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new("✓ Save Serial")
                                                        .size(11.0)
                                                        .strong()
                                                        .color(Color32::BLACK),
                                                )
                                                .fill(theme::ACCENT_PRIMARY)
                                                .corner_radius(CornerRadius::same(5)),
                                            )
                                            .clicked()
                                        {
                                            let edit = build_serial_swap_edit(
                                                &item.uuid,
                                                &state.editing_serial_draft,
                                            );
                                            state.status_message = Some(format!(
                                                "Updated Serial for '{}' to '{}'.",
                                                item.title,
                                                state.editing_serial_draft.trim().to_ascii_uppercase()
                                            ));
                                            state.editing_serial_uuid = None;
                                            emitted_edit = Some(edit);
                                        }
                                        ui.add(
                                            egui::TextEdit::singleline(
                                                &mut state.editing_serial_draft,
                                            )
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(140.0),
                                        );
                                    } else {
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new("⇄ Swap Serial")
                                                        .size(11.0)
                                                        .color(theme::TEXT_PRIMARY),
                                                )
                                                .fill(theme::BG_WIDGET)
                                                .corner_radius(CornerRadius::same(5)),
                                            )
                                            .clicked()
                                        {
                                            state.editing_serial_uuid = Some(item.uuid.clone());
                                            state.editing_serial_draft = item.serial.clone();
                                        }
                                        let serial_text = if item.serial.is_empty() {
                                            "(no serial)".to_string()
                                        } else {
                                            format!("SN: {}", item.serial)
                                        };
                                        ui.label(
                                            RichText::new(serial_text)
                                                .size(11.5)
                                                .monospace()
                                                .color(if item.serial.is_empty() {
                                                    theme::TEXT_MUTED
                                                } else {
                                                    theme::ACCENT_AMBER
                                                }),
                                        );
                                    }
                                },
                            );
                        });
                    });
                ui.add_space(5.0);
            }
        });

    emitted_edit
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_periphery_modal_open_provision_and_inventory() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Klimczaki.Loxone");
        let project =
            loxone_core::parser::load_loxone_file(&path).expect("parse Klimczaki.Loxone");

        let mut state = PeripheryModalState::default();
        state.open_provision(&project, None);
        assert!(state.is_open);
        assert_eq!(state.active_tab, PeripheryModalTab::ProvisionDevice);
        assert!(!state.selected_parent_uuid.is_empty());

        let hw = collect_provisioned_hardware(&project);
        assert!(!hw.is_empty());
        state.open_inventory(Some(hw[0].uuid.clone()), &project);
        assert_eq!(state.active_tab, PeripheryModalTab::HardwareInventory);
        assert_eq!(
            state.editing_serial_uuid.as_deref(),
            Some(hw[0].uuid.as_str())
        );
    }
}
