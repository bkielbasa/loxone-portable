use egui::{Align2, Color32, CornerRadius, Key, Order, RichText, Stroke};
use loxone_core::block_schema::{BlockSchema, page_block_schemas};
use loxone_core::factory::default_category_color;
use loxone_core::model::{LoxoneProject, PinKind, TreeNode, TreeSectionKind};

use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InsertTab {
    #[default]
    FunctionBlocks,
    References,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InsertModalSelection {
    NewFunctionBlock {
        type_name: String,
        title: String,
        px: f32,
        py: f32,
    },
    NewReferenceBlock {
        ref_type: String,
        target_uuid: String,
        px: f32,
        py: f32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceCandidate {
    pub io_uuid: String,
    pub title: String,
    pub control_type: String,
    pub reference_kind: &'static str, // "InputRef", "OutputRef", "Memory", "OutputRefLM"
    pub section_label: String,
}

#[derive(Default)]
pub struct InsertModalState {
    pub is_open: bool,
    pub query: String,
    pub tab: InsertTab,
    pub selected_category: Option<String>,
    pub selected_index: usize,
    pub target_pos: Option<(f32, f32)>,
    pub focus_requested: bool,
}

impl InsertModalState {
    pub fn open_at(&mut self, target_pos: Option<(f32, f32)>) {
        self.is_open = true;
        self.query.clear();
        self.selected_index = 0;
        self.target_pos = target_pos;
        self.focus_requested = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.focus_requested = false;
    }
}

pub fn available_categories() -> Vec<&'static str> {
    let mut cats = Vec::new();
    for schema in page_block_schemas() {
        if !cats.contains(&schema.category_group) {
            cats.push(schema.category_group);
        }
    }
    cats
}

fn category_matches(schema_category: &str, filter: &str) -> bool {
    schema_category == filter
        || (filter.eq_ignore_ascii_case("Climate & Control") && schema_category == "Climate")
}

pub fn matching_block_schemas(
    query: &str,
    category_filter: Option<&str>,
) -> Vec<&'static BlockSchema> {
    let q = query.trim().to_lowercase();
    page_block_schemas()
        .filter(|schema| {
            if let Some(cat) = category_filter {
                if !category_matches(schema.category_group, cat) {
                    return false;
                }
            }
            if q.is_empty() {
                return true;
            }
            schema.display_name_en.to_lowercase().contains(&q)
                || schema.type_name.to_lowercase().contains(&q)
                || schema.category_group.to_lowercase().contains(&q)
                || schema.summary.to_lowercase().contains(&q)
        })
        .collect()
}

pub fn reference_kind_for_tree_node(node_type: &str) -> Option<&'static str> {
    match node_type {
        "Memory" | "Merker" => Some("Memory"),
        "Logger" | "Mailer" | "Caller" | "Notification" | "Tracker" => Some("OutputRefLM"),
        "VirtualIn" | "CallerVirtualIn" | "DigitalIn" | "AnalogIn" | "VoltageIn" | "Sensor"
        | "GenSensor" | "GenAsensor" | "Touch" | "IR" | "OneWire" | "AirBaseDevice"
        | "TreeDevice" | "TreeSensor" | "TreeAsensor" | "LoxAIRsensor" | "LoxAIRAsensor"
        | "ModbusASensor" | "WeatherData" | "Constant" | "SysVar" => Some("InputRef"),
        "VirtualOut" | "VirtualOutCmd" | "ApiActor" | "DigitalOut" | "AnalogOut" | "Relay"
        | "Actor" | "Actuator" | "Dimmer" | "TreeActor" | "TreeAactor" | "TreeTextActor"
        | "LoxAIRactor" | "LoxAIRAactor" => Some("OutputRef"),
        _ => None,
    }
}

fn is_container_or_caption_type(node_type: &str) -> bool {
    node_type.ends_with("Caption")
        || matches!(
            node_type,
            "LoxLIVE"
                | "LoxTree"
                | "LoxAIR"
                | "LoxREL"
                | "LoxDIMM"
                | "LanInt"
                | "SubDevices"
                | "Program"
                | "MessageCenter"
                | "GlobalStates"
                | "ModbusServer"
                | "ModbusDevice"
                | "Mode"
                | "OperatingModes"
        )
}

fn classify_candidate_node(node: &TreeNode) -> Option<&'static str> {
    if node.uuid.is_empty() {
        return None;
    }
    if let Some(kind) = reference_kind_for_tree_node(&node.control_type) {
        return Some(kind);
    }
    if is_container_or_caption_type(&node.control_type) {
        return None;
    }
    if node.pins.iter().any(|p| p.pin_kind == PinKind::Output) {
        return Some("InputRef");
    }
    if node.pins.iter().any(|p| p.pin_kind == PinKind::Input) {
        return Some("OutputRef");
    }
    None
}

fn refine_section_label(node: &TreeNode, default_section: &str) -> String {
    match node.control_type.as_str() {
        "MemoryCaption" | "Memory" | "Merker" => "Memory Flags".to_string(),
        "VirtualInCaption" | "VirtualOutCaption" | "LanInt" | "VirtualIn" | "VirtualOut"
        | "VirtualOutCmd" | "CallerVirtualIn" => "Virtual I/O".to_string(),
        _ => default_section.to_string(),
    }
}

pub fn matching_reference_candidates(
    project: &LoxoneProject,
    query: &str,
) -> Vec<ReferenceCandidate> {
    let q = query.trim().to_lowercase();
    let mut out = Vec::new();

    fn walk(nodes: &[TreeNode], section_label: &str, q: &str, out: &mut Vec<ReferenceCandidate>) {
        for node in nodes {
            let current_section = refine_section_label(node, section_label);
            if let Some(ref_kind) = classify_candidate_node(node) {
                let title = if node.title.trim().is_empty() {
                    node.control_type.clone()
                } else {
                    node.title.clone()
                };
                let matches = q.is_empty()
                    || title.to_lowercase().contains(q)
                    || node.control_type.to_lowercase().contains(q)
                    || current_section.to_lowercase().contains(q)
                    || ref_kind.to_lowercase().contains(q);
                if matches {
                    out.push(ReferenceCandidate {
                        io_uuid: node.uuid.clone(),
                        title,
                        control_type: node.control_type.clone(),
                        reference_kind: ref_kind,
                        section_label: current_section.clone(),
                    });
                }
            }
            let child_section = match node.section {
                TreeSectionKind::Periphery if current_section == "System" => "Periphery",
                _ => current_section.as_str(),
            };
            walk(&node.children, child_section, q, out);
        }
    }

    walk(&project.periphery_tree, "Periphery", &q, &mut out);
    walk(&project.system_tree, "System", &q, &mut out);

    // Also include primary Memory / Merker flags defined on pages if not already in system_tree
    for page in &project.pages {
        for block in &page.blocks {
            if matches!(block.block_type.as_str(), "Memory" | "Merker")
                && block.ref_uuid.is_none()
                && !block.uuid.is_empty()
                && !out.iter().any(|c| c.io_uuid == block.uuid)
            {
                let title = if block.title.trim().is_empty() {
                    block.block_type.clone()
                } else {
                    block.title.clone()
                };
                let section_label = "Memory Flags";
                let ref_kind = "Memory";
                let matches = q.is_empty()
                    || title.to_lowercase().contains(&q)
                    || block.block_type.to_lowercase().contains(&q)
                    || section_label.to_lowercase().contains(&q)
                    || ref_kind.to_lowercase().contains(&q);
                if matches {
                    out.push(ReferenceCandidate {
                        io_uuid: block.uuid.clone(),
                        title,
                        control_type: block.block_type.clone(),
                        reference_kind: ref_kind,
                        section_label: section_label.to_string(),
                    });
                }
            }
        }
    }

    out
}

pub fn show_insert_modal(
    ctx: &egui::Context,
    state: &mut InsertModalState,
    project: Option<&LoxoneProject>,
) -> Option<InsertModalSelection> {
    if !state.is_open {
        return None;
    }

    let mut selection: Option<InsertModalSelection> = None;

    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        state.close();
        return None;
    }

    let move_down = ctx.input(|i| i.key_pressed(Key::ArrowDown));
    let move_up = ctx.input(|i| i.key_pressed(Key::ArrowUp));
    let enter_pressed = ctx.input(|i| i.key_pressed(Key::Enter));
    let (px, py) = state.target_pos.unwrap_or((256.0, 256.0));
    let total_blocks = page_block_schemas().count();

    egui::Window::new("Quick Insert")
        .id(egui::Id::new("quick_insert_modal_window"))
        .order(Order::Tooltip)
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(Align2::CENTER_TOP, [0.0, 68.0])
        .fixed_size([680.0, 500.0])
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(theme::PANEL_BG)
                .stroke(Stroke::new(1.5_f32, theme::CARD_BORDER))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(egui::Margin::same(14)),
        )
        .show(ctx, |ui| {
            // Tab Bar
            ui.horizontal(|ui| {
                let fb_selected = state.tab == InsertTab::FunctionBlocks;
                if ui
                    .selectable_label(
                        fb_selected,
                        RichText::new(format!("🧩 Function Blocks ({total_blocks})")).strong(),
                    )
                    .clicked()
                {
                    state.tab = InsertTab::FunctionBlocks;
                    state.selected_index = 0;
                }
                let ref_selected = state.tab == InsertTab::References;
                if ui
                    .selectable_label(
                        ref_selected,
                        RichText::new("📥 Periphery / Flags / Virtual I/O").strong(),
                    )
                    .clicked()
                {
                    state.tab = InsertTab::References;
                    state.selected_index = 0;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new("Esc to close")
                            .size(11.0)
                            .color(theme::TEXT_MUTED),
                    );
                });
            });

            ui.add_space(8.0);

            // Search Input
            let search_resp = ui.add_sized(
                [ui.available_width(), 32.0],
                egui::TextEdit::singleline(&mut state.query).hint_text(
                    "Search function blocks, periphery I/Os, virtual inputs, or memory flags...",
                ),
            );
            if state.focus_requested {
                search_resp.request_focus();
                state.focus_requested = false;
            }
            if search_resp.changed() {
                state.selected_index = 0;
            }

            ui.add_space(8.0);

            match state.tab {
                InsertTab::FunctionBlocks => {
                    // Category Filter Chips
                    let categories = available_categories();
                    ui.horizontal_wrapped(|ui| {
                        let all_active = state.selected_category.is_none();
                        if ui.selectable_label(all_active, "All").clicked() {
                            state.selected_category = None;
                            state.selected_index = 0;
                        }
                        for cat in categories {
                            let active = state.selected_category.as_deref() == Some(cat);
                            if ui.selectable_label(active, cat).clicked() {
                                state.selected_category = Some(cat.to_string());
                                state.selected_index = 0;
                            }
                        }
                    });

                    ui.separator();

                    let schemas = matching_block_schemas(
                        &state.query,
                        state.selected_category.as_deref(),
                    );
                    if move_down && !schemas.is_empty() {
                        state.selected_index =
                            (state.selected_index + 1).min(schemas.len() - 1);
                    }
                    if move_up && !schemas.is_empty() {
                        state.selected_index = state.selected_index.saturating_sub(1);
                    }
                    if state.selected_index >= schemas.len() {
                        state.selected_index = 0;
                    }

                    if enter_pressed {
                        if let Some(schema) = schemas.get(state.selected_index) {
                            selection = Some(InsertModalSelection::NewFunctionBlock {
                                type_name: schema.type_name.to_string(),
                                title: schema.display_name_en.to_string(),
                                px,
                                py,
                            });
                        }
                    }

                    egui::ScrollArea::vertical()
                        .max_height(350.0)
                        .show(ui, |ui| {
                            for (idx, schema) in schemas.iter().enumerate() {
                                let is_selected = idx == state.selected_index;
                                let bg = if is_selected {
                                    Color32::from_rgb(38, 52, 74)
                                } else {
                                    theme::CARD_BG
                                };
                                let border = if is_selected {
                                    Stroke::new(1.0_f32, theme::ACCENT_BLUE)
                                } else {
                                    Stroke::new(1.0_f32, theme::BORDER_SUBTLE)
                                };

                                let resp = egui::Frame::NONE
                                    .fill(bg)
                                    .stroke(border)
                                    .corner_radius(CornerRadius::same(6))
                                    .inner_margin(egui::Margin::symmetric(10, 7))
                                    .show(ui, |ui| {
                                        ui.set_min_width(ui.available_width());
                                        ui.horizontal(|ui| {
                                            let [r, g, b] =
                                                default_category_color(schema.category_group);
                                            let (bar_rect, _) = ui.allocate_exact_size(
                                                egui::vec2(4.0, 28.0),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().rect_filled(
                                                bar_rect,
                                                CornerRadius::same(2),
                                                Color32::from_rgb(r, g, b),
                                            );

                                            ui.vertical(|ui| {
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        RichText::new(schema.display_name_en)
                                                            .strong()
                                                            .color(theme::TEXT_PRIMARY),
                                                    );
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "[{}]",
                                                            schema.type_name
                                                        ))
                                                        .size(11.0)
                                                        .color(theme::TEXT_MUTED),
                                                    );
                                                    ui.label(
                                                        RichText::new(schema.category_group)
                                                            .size(10.5)
                                                            .color(theme::ACCENT_BLUE),
                                                    );
                                                });
                                                ui.label(
                                                    RichText::new(schema.summary)
                                                        .size(11.0)
                                                        .color(theme::TEXT_MUTED),
                                                );
                                            });

                                            ui.with_layout(
                                                egui::Layout::right_to_left(
                                                    egui::Align::Center,
                                                ),
                                                |ui| {
                                                    let pin_count = schema.pin_docs.len();
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{pin_count} pins"
                                                        ))
                                                        .size(10.5)
                                                        .color(theme::TEXT_MUTED),
                                                    );
                                                },
                                            );
                                        });
                                    })
                                    .response
                                    .interact(egui::Sense::click());

                                if resp.clicked() {
                                    selection =
                                        Some(InsertModalSelection::NewFunctionBlock {
                                            type_name: schema.type_name.to_string(),
                                            title: schema.display_name_en.to_string(),
                                            px,
                                            py,
                                        });
                                }
                                ui.add_space(4.0);
                            }
                        });
                }
                InsertTab::References => {
                    let candidates = project
                        .map(|p| matching_reference_candidates(p, &state.query))
                        .unwrap_or_default();

                    if move_down && !candidates.is_empty() {
                        state.selected_index =
                            (state.selected_index + 1).min(candidates.len() - 1);
                    }
                    if move_up && !candidates.is_empty() {
                        state.selected_index = state.selected_index.saturating_sub(1);
                    }
                    if state.selected_index >= candidates.len() {
                        state.selected_index = 0;
                    }

                    if enter_pressed {
                        if let Some(cand) = candidates.get(state.selected_index) {
                            selection = Some(InsertModalSelection::NewReferenceBlock {
                                ref_type: cand.reference_kind.to_string(),
                                target_uuid: cand.io_uuid.clone(),
                                px,
                                py,
                            });
                        }
                    }

                    egui::ScrollArea::vertical()
                        .max_height(385.0)
                        .show(ui, |ui| {
                            for (idx, cand) in candidates.iter().enumerate() {
                                let is_selected = idx == state.selected_index;
                                let bg = if is_selected {
                                    Color32::from_rgb(38, 52, 74)
                                } else {
                                    theme::CARD_BG
                                };

                                let badge_color = match cand.reference_kind {
                                    "InputRef" => Color32::from_rgb(105, 195, 80),
                                    "OutputRef" => Color32::from_rgb(235, 145, 55),
                                    "Memory" => Color32::from_rgb(90, 175, 245),
                                    _ => Color32::from_rgb(185, 120, 235),
                                };

                                let resp = egui::Frame::NONE
                                    .fill(bg)
                                    .stroke(Stroke::new(
                                        1.0_f32,
                                        if is_selected {
                                            theme::ACCENT_BLUE
                                        } else {
                                            theme::BORDER_SUBTLE
                                        },
                                    ))
                                    .corner_radius(CornerRadius::same(6))
                                    .inner_margin(egui::Margin::symmetric(10, 6))
                                    .show(ui, |ui| {
                                        ui.set_min_width(ui.available_width());
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(cand.reference_kind)
                                                    .size(10.5)
                                                    .strong()
                                                    .color(badge_color),
                                            );
                                            ui.label(
                                                RichText::new(&cand.title)
                                                    .strong()
                                                    .color(theme::TEXT_PRIMARY),
                                            );
                                            ui.label(
                                                RichText::new(format!(
                                                    "[{} · {}]",
                                                    cand.section_label, cand.control_type
                                                ))
                                                .size(11.0)
                                                .color(theme::TEXT_MUTED),
                                            );
                                        });
                                    })
                                    .response
                                    .interact(egui::Sense::click());

                                if resp.clicked() {
                                    selection =
                                        Some(InsertModalSelection::NewReferenceBlock {
                                            ref_type: cand.reference_kind.to_string(),
                                            target_uuid: cand.io_uuid.clone(),
                                            px,
                                            py,
                                        });
                                }
                                ui.add_space(4.0);
                            }
                        });
                }
            }
        });

    if selection.is_some() {
        state.close();
    }

    selection
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_search_stair_includes_stairwell_light() {
        let matches = matching_block_schemas("stair", None);
        assert!(
            matches.iter().any(|s| s.type_name == "StairwellLight"),
            "Expected StairwellLight in results for 'stair', got: {:?}",
            matches.iter().map(|s| s.type_name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_category_filter_returns_only_climate_schemas() {
        let cats = available_categories();
        assert!(!cats.is_empty(), "Expected non-empty category list");
        assert!(cats.contains(&"Climate"), "Expected 'Climate' in categories");

        let climate_exact = matching_block_schemas("", Some("Climate"));
        assert!(!climate_exact.is_empty(), "Expected climate blocks");
        assert!(
            climate_exact.iter().all(|s| s.category_group == "Climate"),
            "All filtered blocks must belong to Climate category"
        );
        assert!(
            climate_exact
                .iter()
                .any(|s| s.type_name == "HeatIRoomController2"),
            "Expected HeatIRoomController2 in Climate category"
        );

        let climate_alias = matching_block_schemas("", Some("Climate & Control"));
        assert_eq!(
            climate_exact.len(),
            climate_alias.len(),
            "Climate & Control alias should match Climate category"
        );
    }

    #[test]
    fn test_reference_candidates_from_sample_project() {
        assert_eq!(reference_kind_for_tree_node("Memory"), Some("Memory"));
        assert_eq!(reference_kind_for_tree_node("Merker"), Some("Memory"));
        assert_eq!(reference_kind_for_tree_node("VirtualIn"), Some("InputRef"));
        assert_eq!(reference_kind_for_tree_node("VirtualOut"), Some("OutputRef"));
        assert_eq!(reference_kind_for_tree_node("Logger"), Some("OutputRefLM"));

        let sample_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Klimczaki.Loxone");
        let project = loxone_core::parser::load_loxone_file(&sample_path)
            .expect("Failed to load Klimczaki.Loxone");

        let all_candidates = matching_reference_candidates(&project, "");
        assert!(
            !all_candidates.is_empty(),
            "Expected reference candidates in Klimczaki.Loxone"
        );

        // Should include InputRef, OutputRef, and Memory candidates
        assert!(
            all_candidates.iter().any(|c| c.reference_kind == "InputRef"),
            "Expected InputRef candidates (periphery inputs / virtual inputs)"
        );
        assert!(
            all_candidates.iter().any(|c| c.reference_kind == "OutputRef"),
            "Expected OutputRef candidates (periphery outputs / virtual outputs)"
        );
        assert!(
            all_candidates.iter().any(|c| c.reference_kind == "Memory"),
            "Expected Memory flag candidates"
        );

        // Query filtering works
        let first_title = &all_candidates[0].title;
        let query_slice: String = first_title.chars().take(4).collect();
        let filtered = matching_reference_candidates(&project, &query_slice);
        assert!(
            filtered.iter().any(|c| c.io_uuid == all_candidates[0].io_uuid),
            "Filtered results should contain candidate matching query"
        );
    }

    #[test]
    fn test_modal_state_open_and_close() {
        let mut state = InsertModalState::default();
        assert!(!state.is_open);

        state.query = "old query".to_string();
        state.selected_index = 5;
        state.open_at(Some((12.0, 8.0)));

        assert!(state.is_open);
        assert_eq!(state.query, "");
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.target_pos, Some((12.0, 8.0)));
        assert!(state.focus_requested);

        // Render in headless egui context and verify Enter selects top match
        let ctx = egui::Context::default();
        state.query = "StairwellLight".to_string();
        let mut raw_input = egui::RawInput::default();
        raw_input.events.push(egui::Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        let mut action = None;
        let _ = ctx.run(raw_input, |ctx| {
            action = show_insert_modal(ctx, &mut state, None);
        });
        assert_eq!(
            action,
            Some(InsertModalSelection::NewFunctionBlock {
                type_name: "StairwellLight".to_string(),
                title: "Stairwell Light Switch".to_string(),
                px: 12.0,
                py: 8.0,
            })
        );
        assert!(!state.is_open);
    }
}
