use egui::{ComboBox, DragValue, RichText, ScrollArea, Stroke, TextEdit, Ui};
use loxone_core::block_schema::{
    lookup_block_schema, AttributeSchema, BlockSchema, ChildConfigKind, PinParamSchema, ValueKind,
};
use loxone_core::model::{
    parse_rgb_color, ConnectorPin, LoxoneProject, PageBlock, PinKind, RawNode, TreeNode,
};
use loxone_core::writer::ProjectEdit;
use loxone_core::xref::{CrossRefKind, ObjectLocation};

use crate::icons::{icon_for_block_type, icon_for_tree_node, ui_icon, LoxoneIcon};
use crate::theme::{ACCENT_BLUE, BORDER_SUBTLE, CARD_BG, TEXT_MUTED, TEXT_PRIMARY};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InspectorAction {
    NavigateToBlock { page_index: usize, block_uuid: String },
    SelectObject(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveCommand {
    pub control_uuid: String,
    pub pin_key: Option<String>,
    pub command: String,
}

#[derive(Debug, Clone, Default)]
pub struct InspectorOutput {
    pub nav_action: Option<InspectorAction>,
    pub edits: Vec<ProjectEdit>,
    pub delete_uuid: Option<String>,
    pub place_on_page_uuid: Option<String>,
    pub live_commands: Vec<LiveCommand>,
}

pub fn render_right_inspector(
    ui: &mut Ui,
    project: &LoxoneProject,
    selected_uuid: Option<&str>,
    live_store: Option<&loxone_core::miniserver::LiveValueStore>,
) -> InspectorOutput {
    let mut output = InspectorOutput::default();

    ui.add_space(6.0);
    ui.label(RichText::new("INSPECTOR").size(11.0).color(TEXT_MUTED).strong());
    ui.add_space(4.0);
    ui.separator();

    let Some(uuid) = selected_uuid else {
        ui.add_space(24.0);
        ui.label(
            RichText::new("Select any function block, peripheral, or page item to inspect and edit properties.")
                .color(TEXT_MUTED),
        );
        return output;
    };

    let Some(summary) = project.xref.objects_by_uuid.get(uuid) else {
        ui.label(RichText::new(format!("Object {} not found", uuid)).color(TEXT_MUTED));
        return output;
    };

    let schema = lookup_block_schema(&summary.object_type);

    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        render_kb_header_card(ui, project, summary, schema, &mut output);
        ui.add_space(8.0);
        if let Some(store) = live_store {
            render_live_monitor_card(ui, project, summary, store, &mut output);
            ui.add_space(8.0);
        }
        render_used_on_pages_section(ui, project, uuid, &mut output);
        ui.add_space(8.0);

        match &summary.location {
            ObjectLocation::PageBlock {
                page_index,
                block_index,
                ..
            } => {
                if let Some(block) = project
                    .pages
                    .get(*page_index)
                    .and_then(|p| p.blocks.get(*block_index))
                {
                    render_page_block_inspector(ui, project, block, schema, &mut output);
                }
            }
            ObjectLocation::TreeNode { .. } => {
                if let Some(node) = find_tree_node(&project.periphery_tree, uuid)
                    .or_else(|| find_tree_node(&project.system_tree, uuid))
                {
                    render_tree_node_inspector(ui, project, node, schema, &mut output);
                }
            }
            ObjectLocation::Category => {
                if let Some(cat) = project.categories.get(uuid) {
                    render_category_inspector(ui, cat, &mut output);
                }
            }
            ObjectLocation::Place => {
                if let Some(place) = project.places.get(uuid) {
                    render_place_inspector(ui, place, &mut output);
                }
            }
            ObjectLocation::Page { page_index } => {
                if let Some(page) = project.pages.get(*page_index) {
                    render_singleline_attr_row(
                        ui,
                        &page.uuid,
                        "Title",
                        "Page Title",
                        &page.title,
                        &mut output.edits,
                    );
                }
            }
            ObjectLocation::DocumentRoot => {}
        }
    });

    output
}

fn render_live_monitor_card(
    ui: &mut Ui,
    project: &LoxoneProject,
    summary: &loxone_core::xref::ObjectSummary,
    store: &loxone_core::miniserver::LiveValueStore,
    output: &mut InspectorOutput,
) {
    let green = egui::Color32::from_rgb(0x59, 0xB8, 0x28);
    let mut target_uuid = summary.uuid.clone();
    let mut primary_sig = None;
    let mut pin_rows: Vec<(String, loxone_core::miniserver::LiveSignal)> = Vec::new();

    match &summary.location {
        ObjectLocation::PageBlock {
            page_index,
            block_index,
            ..
        } => {
            if let Some(page) = project.pages.get(*page_index) {
                if let Some(block) = page.blocks.get(*block_index) {
                    if block.is_ref_bar() {
                        if let Some(ru) = &block.ref_uuid {
                            target_uuid = ru.clone();
                        }
                        primary_sig = Some(store.resolve_ref_bar_signal(block, page, project));
                    } else {
                        for pin in &block.pins {
                            let sig = store.resolve_pin_signal(block, pin, page, project);
                            let short_k = loxone_core::miniserver::loxone_short_pin_label(
                                &block.block_type,
                                &pin.key,
                            );
                            pin_rows.push((format!("{} ({})", short_k, pin.key), sig));
                        }
                    }
                }
            }
        }
        ObjectLocation::TreeNode { .. } => {
            if let Some(node) = find_tree_node(&project.periphery_tree, &summary.uuid)
                .or_else(|| find_tree_node(&project.system_tree, &summary.uuid))
            {
                primary_sig = store.resolve_tree_node_signal(node);
            }
        }
        _ => return,
    }

    egui::Frame::new()
        .fill(CARD_BG)
        .stroke(Stroke::new(1.2_f32, green.gamma_multiply(0.75)))
        .corner_radius(6)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("● LIVE VIEW MONITOR & CONTROL")
                        .size(11.0)
                        .color(green)
                        .strong(),
                );
                if let Some(sig) = &primary_sig {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let badge_bg = if sig.is_active {
                            green
                        } else {
                            egui::Color32::from_rgb(0x36, 0x3E, 0x4E)
                        };
                        egui::Frame::new()
                            .fill(badge_bg)
                            .corner_radius(4)
                            .inner_margin(egui::Margin::symmetric(8, 2))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(&sig.formatted)
                                        .size(12.0)
                                        .strong()
                                        .color(egui::Color32::WHITE),
                                );
                            });
                    });
                }
            });

            if !pin_rows.is_empty() {
                ui.add_space(4.0);
                egui::Grid::new(("live_pins_grid", &summary.uuid))
                    .num_columns(2)
                    .spacing([12.0, 3.0])
                    .show(ui, |ui| {
                        for (label, sig) in pin_rows.iter().take(12) {
                            ui.label(RichText::new(label).monospace().size(11.0).color(TEXT_MUTED));
                            ui.label(
                                RichText::new(&sig.formatted)
                                    .monospace()
                                    .size(11.5)
                                    .strong()
                                    .color(if sig.is_active { green } else { TEXT_PRIMARY }),
                            );
                            ui.end_row();
                        }
                    });
            }

            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Force / Send:").size(11.0).color(TEXT_MUTED));
                for (btn_label, cmd) in [
                    ("Wł. (On)", "On"),
                    ("Wył. (Off)", "Off"),
                    ("Pulse", "Pulse"),
                    ("22.5°", "22.5"),
                    ("50%", "50"),
                ] {
                    if ui.small_button(btn_label).clicked() {
                        output.live_commands.push(LiveCommand {
                            control_uuid: target_uuid.clone(),
                            pin_key: None,
                            command: cmd.to_string(),
                        });
                    }
                }
            });
        });
}

fn resolve_summary_icon(
    project: &LoxoneProject,
    summary: &loxone_core::xref::ObjectSummary,
) -> LoxoneIcon {
    match &summary.location {
        ObjectLocation::TreeNode { .. } => find_tree_node(&project.periphery_tree, &summary.uuid)
            .or_else(|| find_tree_node(&project.system_tree, &summary.uuid))
            .map(icon_for_tree_node)
            .unwrap_or_else(|| icon_for_block_type(&summary.object_type, None)),
        ObjectLocation::PageBlock {
            page_index,
            block_index,
            ..
        } => {
            let ref_type = project
                .pages
                .get(*page_index)
                .and_then(|p| p.blocks.get(*block_index))
                .and_then(|b| b.ref_uuid.as_deref())
                .and_then(|ru| project.xref.objects_by_uuid.get(ru))
                .map(|s| s.object_type.as_str());
            icon_for_block_type(&summary.object_type, ref_type)
        }
        ObjectLocation::Page { .. } => LoxoneIcon::Page,
        ObjectLocation::Place => LoxoneIcon::Room,
        ObjectLocation::Category => {
            LoxoneIcon::Category(project.categories.get(&summary.uuid).and_then(|c| c.bk_color))
        }
        ObjectLocation::DocumentRoot => LoxoneIcon::Miniserver,
    }
}

fn render_kb_header_card(
    ui: &mut Ui,
    project: &LoxoneProject,
    summary: &loxone_core::xref::ObjectSummary,
    schema: Option<&'static BlockSchema>,
    output: &mut InspectorOutput,
) {
    egui::Frame::new()
        .fill(CARD_BG)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .corner_radius(6)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let title = if summary.title.is_empty() {
                &summary.object_type
            } else {
                &summary.title
            };
            let icon = resolve_summary_icon(project, summary);
            ui.horizontal(|ui| {
                ui_icon(ui, icon, 18.0);
                ui.label(RichText::new(title).size(14.5).color(TEXT_PRIMARY).strong());
            });

            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(&summary.object_type)
                        .monospace()
                        .size(11.0)
                        .color(ACCENT_BLUE)
                        .strong(),
                );
                if let Some(s) = schema {
                    ui.label(
                        RichText::new(format!("• {} ({})", s.display_name_en, s.category_group))
                            .size(11.0)
                            .color(TEXT_MUTED),
                    );
                }
            });

            if let Some(s) = schema {
                ui.add_space(4.0);
                ui.label(RichText::new(s.summary).size(11.0).color(TEXT_MUTED));
                ui.add_space(4.0);
                if ui
                    .small_button(RichText::new("📖 Loxone Docs").color(ACCENT_BLUE))
                    .on_hover_text(s.kb_url)
                    .clicked()
                {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(s.kb_url));
                }
            }

            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("UUID: {}", summary.uuid))
                    .monospace()
                    .size(10.0)
                    .color(TEXT_MUTED),
            );
            if let Some(r_uuid) = &summary.room_uuid {
                if let Some(place) = project.places.get(r_uuid) {
                    ui.label(
                        RichText::new(format!("Room: {}", place.title))
                            .size(11.0)
                            .color(TEXT_MUTED),
                    );
                }
            }
            if let Some(c_uuid) = &summary.category_uuid {
                if let Some(cat) = project.categories.get(c_uuid) {
                    let cat_type = cat.cat_group.as_deref().unwrap_or("-");
                    ui.label(
                        RichText::new(format!("Category: {} ({})", cat.title, cat_type))
                            .size(11.0)
                            .color(TEXT_MUTED),
                    );
                }
            }

            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| match &summary.location {
                ObjectLocation::PageBlock { .. } => {
                    if ui
                        .small_button(
                            RichText::new("🗑 Delete Block").color(crate::theme::STATUS_ERROR),
                        )
                        .on_hover_text("Delete this block and its connected wires (Delete)")
                        .clicked()
                    {
                        output.delete_uuid = Some(summary.uuid.clone());
                    }
                }
                ObjectLocation::TreeNode { .. } => {
                    if crate::insert_modal::reference_kind_for_tree_node(&summary.object_type)
                        .is_some()
                        && ui
                            .small_button(
                                RichText::new("📥 Place on Current Page").color(ACCENT_BLUE),
                            )
                            .on_hover_text("Add an Input/Output reference to this item on the active page")
                            .clicked()
                    {
                        output.place_on_page_uuid = Some(summary.uuid.clone());
                    }
                    if matches!(
                        summary.object_type.as_str(),
                        "Merker" | "Memory" | "VirtualIn" | "VirtualOut" | "VirtualOutCmd"
                    ) && ui
                        .small_button(RichText::new("🗑 Delete").color(crate::theme::STATUS_ERROR))
                        .on_hover_text("Delete this item and its schematic references")
                        .clicked()
                    {
                        output.delete_uuid = Some(summary.uuid.clone());
                    }
                }
                ObjectLocation::Category | ObjectLocation::Place | ObjectLocation::Page { .. } => {
                    if ui
                        .small_button(RichText::new("🗑 Delete").color(crate::theme::STATUS_ERROR))
                        .on_hover_text("Delete this item")
                        .clicked()
                    {
                        output.delete_uuid = Some(summary.uuid.clone());
                    }
                }
                ObjectLocation::DocumentRoot => {}
            });
        });
}

fn render_used_on_pages_section(
    ui: &mut Ui,
    project: &LoxoneProject,
    uuid: &str,
    output: &mut InspectorOutput,
) {
    let usages = project.xref.usages_for(uuid);
    if usages.is_empty() {
        return;
    }
    egui::CollapsingHeader::new(
        RichText::new(format!("USED ON PAGES ({})", usages.len()))
            .size(11.0)
            .color(TEXT_MUTED)
            .strong(),
    )
    .default_open(true)
    .show(ui, |ui| {
        for usage in usages {
            let kind_desc = match &usage.kind {
                CrossRefKind::DirectRef => "Direct Ref".to_string(),
                CrossRefKind::ExtraDeviceRef => "Device Link".to_string(),
                CrossRefKind::WireFromPin {
                    source_pin_key,
                    target_pin_key,
                } => format!("Wire {} → {}", source_pin_key, target_pin_key),
            };
            let label = format!(
                "↗ {} • {} ({})",
                usage.page_title,
                if usage.block_title.is_empty() {
                    &usage.block_type
                } else {
                    &usage.block_title
                },
                kind_desc
            );
            if ui
                .button(RichText::new(label).size(11.0).color(ACCENT_BLUE))
                .clicked()
            {
                output.nav_action = Some(InspectorAction::NavigateToBlock {
                    page_index: usage.page_index,
                    block_uuid: usage.block_uuid.clone(),
                });
            }
        }
    });
}

fn get_raw_attr<'a>(attrs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

fn render_general_and_visu_section(
    ui: &mut Ui,
    project: &LoxoneProject,
    control_uuid: &str,
    title: &str,
    room_uuid: Option<&str>,
    category_uuid: Option<&str>,
    raw_attributes: &[(String, String)],
    extra_children: &[RawNode],
    is_note_or_text: bool,
    edits: &mut Vec<ProjectEdit>,
) {
    egui::CollapsingHeader::new(
        RichText::new("GENERAL & VISUALIZATION")
            .size(11.0)
            .color(TEXT_MUTED)
            .strong(),
    )
    .default_open(true)
    .show(ui, |ui| {
        render_singleline_attr_row(ui, control_uuid, "Title", "Title", title, edits);
        let desc = get_raw_attr(raw_attributes, "Desc").unwrap_or("");
        render_singleline_attr_row(ui, control_uuid, "Desc", "Description", desc, edits);

        if is_note_or_text {
            let txt = get_raw_attr(raw_attributes, "Text").unwrap_or("");
            ui.label(RichText::new("Note / Text Content").size(11.0).color(TEXT_MUTED));
            let mut val = txt.to_string();
            if ui
                .add(TextEdit::multiline(&mut val).desired_rows(3).desired_width(f32::INFINITY))
                .changed()
            {
                edits.push(ProjectEdit::SetControlAttr {
                    control_uuid: control_uuid.to_string(),
                    attr_name: "Text".to_string(),
                    value: Some(val),
                });
            }
        }

        // Room selector (<IoData Pr="..."/>)
        let current_room_label = room_uuid
            .and_then(|u| project.places.get(u))
            .map(|p| p.title.as_str())
            .unwrap_or("— Unassigned —");
        ui.horizontal(|ui| {
            ui.label(RichText::new("Room:").size(11.0).color(TEXT_MUTED));
            ComboBox::from_id_salt((control_uuid, "room_combo"))
                .selected_text(current_room_label)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(room_uuid.is_none(), "— Unassigned —")
                        .clicked()
                    {
                        edits.push(ProjectEdit::SetChildTagAttr {
                            control_uuid: control_uuid.to_string(),
                            child_tag: "IoData".to_string(),
                            attr_name: "Pr".to_string(),
                            value: None,
                        });
                    }
                    for (p_uuid, place) in &project.places {
                        if ui
                            .selectable_label(room_uuid == Some(p_uuid.as_str()), &place.title)
                            .clicked()
                        {
                            edits.push(ProjectEdit::SetChildTagAttr {
                                control_uuid: control_uuid.to_string(),
                                child_tag: "IoData".to_string(),
                                attr_name: "Pr".to_string(),
                                value: Some(p_uuid.clone()),
                            });
                        }
                    }
                });
        });

        // Category selector (<IoData Cr="..."/>)
        let current_cat_label = category_uuid
            .and_then(|u| project.categories.get(u))
            .map(|c| c.title.as_str())
            .unwrap_or("— Unassigned —");
        ui.horizontal(|ui| {
            ui.label(RichText::new("Category:").size(11.0).color(TEXT_MUTED));
            ComboBox::from_id_salt((control_uuid, "cat_combo"))
                .selected_text(current_cat_label)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(category_uuid.is_none(), "— Unassigned —")
                        .clicked()
                    {
                        edits.push(ProjectEdit::SetChildTagAttr {
                            control_uuid: control_uuid.to_string(),
                            child_tag: "IoData".to_string(),
                            attr_name: "Cr".to_string(),
                            value: None,
                        });
                    }
                    for (c_uuid, cat) in &project.categories {
                        if ui
                            .selectable_label(category_uuid == Some(c_uuid.as_str()), &cat.title)
                            .clicked()
                        {
                            edits.push(ProjectEdit::SetChildTagAttr {
                                control_uuid: control_uuid.to_string(),
                                child_tag: "IoData".to_string(),
                                attr_name: "Cr".to_string(),
                                value: Some(c_uuid.clone()),
                            });
                        }
                    }
                });
        });

        // Visualization checkboxes & color picker
        ui.horizontal_wrapped(|ui| {
            let mut visu = matches!(
                get_raw_attr(raw_attributes, "Visu"),
                Some("true" | "1")
            );
            if ui.checkbox(&mut visu, "Use in UI (Visu)").changed() {
                edits.push(ProjectEdit::SetControlAttr {
                    control_uuid: control_uuid.to_string(),
                    attr_name: "Visu".to_string(),
                    value: if visu { Some("true".to_string()) } else { None },
                });
            }

            let mut fav = matches!(
                get_raw_attr(raw_attributes, "UseInFav"),
                Some("true" | "1")
            );
            if ui.checkbox(&mut fav, "Favorite").changed() {
                edits.push(ProjectEdit::SetControlAttr {
                    control_uuid: control_uuid.to_string(),
                    attr_name: "UseInFav".to_string(),
                    value: if fav { Some("true".to_string()) } else { None },
                });
            }
        });

        ui.horizontal(|ui| {
            ui.label(RichText::new("Color (Cl):").size(11.0).color(TEXT_MUTED));
            let mut rgb = get_raw_attr(raw_attributes, "Cl")
                .and_then(parse_rgb_color)
                .unwrap_or([0x5E, 0x81, 0xFF]);
            if ui.color_edit_button_srgb(&mut rgb).changed() {
                edits.push(ProjectEdit::SetControlAttr {
                    control_uuid: control_uuid.to_string(),
                    attr_name: "Cl".to_string(),
                    value: Some(format!("{},{},{}", rgb[0], rgb[1], rgb[2])),
                });
            }

            ui.label(RichText::new("Rating:").size(11.0).color(TEXT_MUTED));
            let mut rating: i32 = get_raw_attr(raw_attributes, "Rating")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            if ui.add(DragValue::new(&mut rating).range(0..=5)).changed() {
                edits.push(ProjectEdit::SetControlAttr {
                    control_uuid: control_uuid.to_string(),
                    attr_name: "Rating".to_string(),
                    value: if rating > 0 {
                        Some(rating.to_string())
                    } else {
                        None
                    },
                });
            }
        });

        if let Some(disp) = extra_children.iter().find(|c| c.tag == "Display") {
            let unit = disp.attr("Unit").unwrap_or("");
            let format = disp.attr("Format").unwrap_or("");
            render_child_tag_attr_row(
                ui,
                control_uuid,
                "Display",
                "Unit",
                "Display Unit",
                unit,
                edits,
            );
            render_child_tag_attr_row(
                ui,
                control_uuid,
                "Display",
                "Format",
                "Display Format",
                format,
                edits,
            );
        }
    });
}

fn render_page_block_inspector(
    ui: &mut Ui,
    project: &LoxoneProject,
    block: &PageBlock,
    schema: Option<&'static BlockSchema>,
    output: &mut InspectorOutput,
) {
    render_general_and_visu_section(
        ui,
        project,
        &block.uuid,
        &block.title,
        block.room_uuid.as_deref(),
        block.category_uuid.as_deref(),
        &block.raw_attributes,
        &block.extra_children,
        block.is_note(),
        &mut output.edits,
    );

    ui.add_space(6.0);
    render_schema_parameters_section(
        ui,
        &block.uuid,
        schema,
        &block.raw_attributes,
        &block.pins,
        &mut output.edits,
    );

    ui.add_space(6.0);
    render_specialized_block_editor(ui, block, &mut output.edits);

    ui.add_space(6.0);
    render_child_configs_section(
        ui,
        &block.uuid,
        schema,
        &block.extra_children,
        &mut output.edits,
    );

    if let Some(ref_uuid) = &block.ref_uuid {
        ui.add_space(6.0);
        render_linked_target_section(ui, project, ref_uuid, output);
    }

    ui.add_space(6.0);
    render_documented_pins_section(ui, project, &block.uuid, &block.pins, schema, output);

    ui.add_space(6.0);
    render_advanced_xml_section(
        ui,
        &block.uuid,
        &block.raw_attributes,
        &block.extra_children,
        &mut output.edits,
    );
}

fn render_specialized_block_editor(
    ui: &mut Ui,
    block: &PageBlock,
    edits: &mut Vec<ProjectEdit>,
) {
    match block.block_type.as_str() {
        "LightControllerV2" | "LightController2" | "LightController" => {
            egui::CollapsingHeader::new(
                RichText::new("💡 LIGHTING SCENES & MOOD EDITOR")
                    .size(11.0)
                    .color(ACCENT_BLUE)
                    .strong(),
            )
            .default_open(true)
            .show(ui, |ui| {
                for (attr_key, label) in [
                    ("DefMood", "Default Mood ID"),
                    ("Tfade", "Fade Time (s)"),
                    ("MoveTime", "Presence Timeout (s)"),
                ] {
                    let cur = get_raw_attr(&block.raw_attributes, attr_key).unwrap_or("");
                    render_singleline_attr_row(ui, &block.uuid, attr_key, label, cur, edits);
                }
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Mood / Scene Descriptions (<LightsceneDescription>):")
                        .size(10.5)
                        .color(TEXT_MUTED),
                );
                let lsd = block
                    .extra_children
                    .iter()
                    .find(|c| c.tag == "LightsceneDescription" || c.tag == "Lightscene");
                let tag_name = lsd.map(|n| n.tag.as_str()).unwrap_or("LightsceneDescription");
                for idx in 1..=6 {
                    let attr_name = format!("Scene{idx}");
                    let label = format!("Mood #{idx} Name");
                    let cur = lsd.and_then(|n| n.attr(&attr_name)).unwrap_or("");
                    render_child_tag_attr_row(
                        ui,
                        &block.uuid,
                        tag_name,
                        &attr_name,
                        &label,
                        cur,
                        edits,
                    );
                }
            });
        }
        "IRoomControllerV2" | "HeatIRoomController2" | "IRoomController" => {
            egui::CollapsingHeader::new(
                RichText::new("🌡 COMFORT & ECO TEMPERATURE PRESETS")
                    .size(11.0)
                    .color(ACCENT_BLUE)
                    .strong(),
            )
            .default_open(true)
            .show(ui, |ui| {
                for (pin_key, label, fallback) in [
                    ("Tc", "Comfort Heating (°C)", 21.0_f64),
                    ("Tcc", "Comfort Cooling (°C)", 23.5_f64),
                    ("Teh", "Eco Setback Heating (°C)", 18.0_f64),
                    ("Tec", "Eco Setback Cooling (°C)", 26.0_f64),
                    ("Tf", "Frost / Building Protection (°C)", 8.0_f64),
                ] {
                    let cur_str = block
                        .pins
                        .iter()
                        .find(|p| p.key == pin_key)
                        .and_then(|p| p.default_value.as_deref())
                        .unwrap_or("");
                    let mut val: f64 = cur_str.parse().unwrap_or(fallback);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{label} [{pin_key}]:"))
                                .size(11.0)
                                .color(TEXT_MUTED),
                        );
                        if ui
                            .add(
                                DragValue::new(&mut val)
                                    .speed(0.1)
                                    .range(4.0..=35.0)
                                    .suffix(" °C"),
                            )
                            .changed()
                        {
                            edits.push(ProjectEdit::SetPinDefault {
                                control_uuid: block.uuid.clone(),
                                pin_key: pin_key.to_string(),
                                default_value: Some(format_clean_f64(val)),
                            });
                        }
                    });
                }
            });
        }
        "Radio" | "ValueSelector" => {
            egui::CollapsingHeader::new(
                RichText::new("🔘 RADIO / SELECTOR OUTPUT TABLE (1..8)")
                    .size(11.0)
                    .color(ACCENT_BLUE)
                    .strong(),
            )
            .default_open(true)
            .show(ui, |ui| {
                for idx in 1..=8 {
                    let attr_name = format!("Text{idx}");
                    let label = format!("Button #{idx} Label");
                    let cur = get_raw_attr(&block.raw_attributes, &attr_name).unwrap_or("");
                    render_singleline_attr_row(ui, &block.uuid, &attr_name, &label, cur, edits);
                }
            });
        }
        "Status" => {
            egui::CollapsingHeader::new(
                RichText::new("📊 STATUS CONDITION & TEXT TABLE")
                    .size(11.0)
                    .color(ACCENT_BLUE)
                    .strong(),
            )
            .default_open(true)
            .show(ui, |ui| {
                for (attr_name, label) in [
                    ("Text1", "Condition 1 Status Text"),
                    ("Text2", "Condition 2 Status Text"),
                    ("Text3", "Condition 3 Status Text"),
                    ("Text4", "Condition 4 Status Text"),
                    ("Text", "Default / Fallback Status Text"),
                ] {
                    let cur = get_raw_attr(&block.raw_attributes, attr_name).unwrap_or("");
                    render_singleline_attr_row(ui, &block.uuid, attr_name, label, cur, edits);
                }
            });
        }
        _ => {}
    }
}

fn render_tree_node_inspector(
    ui: &mut Ui,
    project: &LoxoneProject,
    node: &TreeNode,
    schema: Option<&'static BlockSchema>,
    output: &mut InspectorOutput,
) {
    render_general_and_visu_section(
        ui,
        project,
        &node.uuid,
        &node.title,
        node.room_uuid.as_deref(),
        node.category_uuid.as_deref(),
        &node.raw_attributes,
        &node.extra_children,
        false,
        &mut output.edits,
    );

    ui.add_space(6.0);
    render_schema_parameters_section(
        ui,
        &node.uuid,
        schema,
        &node.raw_attributes,
        &node.pins,
        &mut output.edits,
    );

    ui.add_space(6.0);
    render_child_configs_section(
        ui,
        &node.uuid,
        schema,
        &node.extra_children,
        &mut output.edits,
    );

    ui.add_space(6.0);
    render_documented_pins_section(ui, project, &node.uuid, &node.pins, schema, output);

    ui.add_space(6.0);
    render_advanced_xml_section(
        ui,
        &node.uuid,
        &node.raw_attributes,
        &node.extra_children,
        &mut output.edits,
    );
}

fn render_category_inspector(
    ui: &mut Ui,
    cat: &loxone_core::model::Category,
    output: &mut InspectorOutput,
) {
    render_singleline_attr_row(
        ui,
        &cat.uuid,
        "Title",
        "Category Title",
        &cat.title,
        &mut output.edits,
    );
    let grp = cat.cat_group.as_deref().unwrap_or("");
    render_singleline_attr_row(
        ui,
        &cat.uuid,
        "CatGroup",
        "Category Type",
        grp,
        &mut output.edits,
    );
}

fn render_place_inspector(
    ui: &mut Ui,
    place: &loxone_core::model::Place,
    output: &mut InspectorOutput,
) {
    render_singleline_attr_row(
        ui,
        &place.uuid,
        "Title",
        "Room Name",
        &place.title,
        &mut output.edits,
    );
    let sqm = place.sqm.as_deref().unwrap_or("");
    render_singleline_attr_row(
        ui,
        &place.uuid,
        "Sqm",
        "Area (m²)",
        sqm,
        &mut output.edits,
    );
}

fn render_schema_parameters_section(
    ui: &mut Ui,
    control_uuid: &str,
    schema: Option<&'static BlockSchema>,
    raw_attributes: &[(String, String)],
    pins: &[ConnectorPin],
    edits: &mut Vec<ProjectEdit>,
) {
    let schema_attrs: &[AttributeSchema] = schema.map(|s| s.attributes).unwrap_or(&[]);
    let schema_pin_params: &[PinParamSchema] = schema.map(|s| s.pin_params).unwrap_or(&[]);

    let has_param_pins = pins
        .iter()
        .any(|p| p.pin_kind == PinKind::Parameter || p.default_value.is_some());

    if schema_attrs.is_empty() && schema_pin_params.is_empty() && !has_param_pins {
        return;
    }

    egui::CollapsingHeader::new(
        RichText::new("PARAMETERS & OPERATING MODES")
            .size(11.0)
            .color(TEXT_MUTED)
            .strong(),
    )
    .default_open(true)
    .show(ui, |ui| {
        for attr in schema_attrs {
            let current = get_raw_attr(raw_attributes, attr.xml_attr)
                .or(attr.default_value)
                .unwrap_or("");
            if let Some(new_val) = render_value_kind_widget(
                ui,
                (control_uuid, "attr", attr.xml_attr),
                attr.label,
                attr.xml_attr,
                attr.unit,
                attr.kind,
                current,
                attr.description,
            ) {
                edits.push(ProjectEdit::SetControlAttr {
                    control_uuid: control_uuid.to_string(),
                    attr_name: attr.xml_attr.to_string(),
                    value: Some(new_val),
                });
            }
        }

        // Render schema-defined pin parameters + any pin with Def=...
        let mut rendered_pin_keys = Vec::new();
        for p_schema in schema_pin_params {
            let existing_pin = pins.iter().find(|p| p.key == p_schema.pin_key);
            if let Some(pin) = existing_pin {
                if pin.has_visible_incoming_links() && pin.default_value.is_none() {
                    continue;
                }
            }
            let current = existing_pin
                .and_then(|p| p.default_value.as_deref())
                .unwrap_or(p_schema.default_value);
            rendered_pin_keys.push(p_schema.pin_key.to_string());
            if let Some(new_val) = render_value_kind_widget(
                ui,
                (control_uuid, "pin", p_schema.pin_key),
                p_schema.label,
                p_schema.pin_key,
                p_schema.unit,
                p_schema.kind,
                current,
                p_schema.description,
            ) {
                edits.push(ProjectEdit::SetPinDefault {
                    control_uuid: control_uuid.to_string(),
                    pin_key: p_schema.pin_key.to_string(),
                    default_value: Some(new_val),
                });
            }
        }

        for pin in pins {
            if rendered_pin_keys.iter().any(|k| k == &pin.key) {
                continue;
            }
            if pin.pin_kind == PinKind::Parameter || pin.default_value.is_some() {
                let p_meta = schema.and_then(|s| s.find_pin_param(&pin.key));
                let label = p_meta.map(|m| m.label).unwrap_or(&pin.key);
                let unit = p_meta.and_then(|m| m.unit);
                let kind = p_meta.map(|m| m.kind).unwrap_or(ValueKind::Float);
                let desc = p_meta
                    .map(|m| m.description)
                    .unwrap_or("Connector pin default parameter (<Co K=\"...\" Def=\"...\"/>)");
                let current = pin.default_value.as_deref().unwrap_or("0");
                if let Some(new_val) = render_value_kind_widget(
                    ui,
                    (control_uuid, "pin_dyn", pin.key.as_str()),
                    label,
                    &pin.key,
                    unit,
                    kind,
                    current,
                    desc,
                ) {
                    edits.push(ProjectEdit::SetPinDefault {
                        control_uuid: control_uuid.to_string(),
                        pin_key: pin.key.clone(),
                        default_value: Some(new_val),
                    });
                }
            }
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn render_value_kind_widget(
    ui: &mut Ui,
    id_salt: (&str, &str, &str),
    label: &str,
    code_key: &str,
    unit: Option<&str>,
    kind: ValueKind,
    current: &str,
    tooltip: &str,
) -> Option<String> {
    let mut changed_val = None;
    ui.horizontal(|ui| {
        let lbl_resp = ui.label(
            RichText::new(format!("{} ({}):", label, code_key))
                .size(11.0)
                .color(TEXT_MUTED),
        );
        if !tooltip.is_empty() {
            lbl_resp.on_hover_text(tooltip);
        }

        match kind {
            ValueKind::Bool => {
                let mut b = matches!(current, "true" | "1");
                if ui.checkbox(&mut b, "").on_hover_text(tooltip).changed() {
                    changed_val = Some(if b { "true".to_string() } else { "false".to_string() });
                }
            }
            ValueKind::Int => {
                let mut v: i64 = current.parse().unwrap_or(0);
                let mut dv = DragValue::new(&mut v).speed(1.0);
                if let Some(u) = unit {
                    dv = dv.suffix(format!(" {u}"));
                }
                if ui.add(dv).on_hover_text(tooltip).changed() {
                    changed_val = Some(v.to_string());
                }
            }
            ValueKind::Float
            | ValueKind::DurationSeconds
            | ValueKind::DurationMinutes
            | ValueKind::Percent
            | ValueKind::TemperatureCelsius
            | ValueKind::Lux => {
                let mut v: f64 = current.parse().unwrap_or(0.0);
                let suffix = unit.unwrap_or(match kind {
                    ValueKind::DurationSeconds => "s",
                    ValueKind::DurationMinutes => "min",
                    ValueKind::Percent => "%",
                    ValueKind::TemperatureCelsius => "°C",
                    ValueKind::Lux => "lx",
                    _ => "",
                });
                let mut dv = DragValue::new(&mut v).speed(0.1);
                if !suffix.is_empty() {
                    dv = dv.suffix(format!(" {suffix}"));
                }
                if ui.add(dv).on_hover_text(tooltip).changed() {
                    changed_val = Some(format_clean_f64(v));
                }
            }
            ValueKind::Enum(options) => {
                let current_desc = options
                    .iter()
                    .find(|(val, _)| *val == current)
                    .map(|(_, desc)| *desc)
                    .unwrap_or(current);
                ComboBox::from_id_salt(id_salt)
                    .selected_text(current_desc)
                    .show_ui(ui, |ui| {
                        for (val, desc) in options {
                            if ui
                                .selectable_label(*val == current, format!("{desc} ({val})"))
                                .clicked()
                            {
                                changed_val = Some((*val).to_string());
                            }
                        }
                    });
            }
            ValueKind::Bitmask(flags) => {
                let mut mask: u32 = current.parse().unwrap_or(0);
                ui.vertical(|ui| {
                    for (bit, desc) in flags {
                        let mut set = (mask & *bit) != 0;
                        if ui.checkbox(&mut set, *desc).changed() {
                            if set {
                                mask |= *bit;
                            } else {
                                mask &= !*bit;
                            }
                            changed_val = Some(mask.to_string());
                        }
                    }
                });
            }
            ValueKind::Text | ValueKind::UuidRef => {
                let mut txt = current.to_string();
                if ui
                    .add(TextEdit::singleline(&mut txt).desired_width(130.0))
                    .on_hover_text(tooltip)
                    .changed()
                {
                    changed_val = Some(txt);
                }
            }
        }
    });
    changed_val
}

fn format_clean_f64(v: f64) -> String {
    if (v - v.round()).abs() < 1e-6 {
        format!("{}", v.round() as i64)
    } else {
        format!("{:.2}", v)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn render_singleline_attr_row(
    ui: &mut Ui,
    control_uuid: &str,
    attr_name: &str,
    label: &str,
    current: &str,
    edits: &mut Vec<ProjectEdit>,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{label}:")).size(11.0).color(TEXT_MUTED));
        let mut val = current.to_string();
        if ui
            .add(TextEdit::singleline(&mut val).desired_width(f32::INFINITY))
            .changed()
        {
            edits.push(ProjectEdit::SetControlAttr {
                control_uuid: control_uuid.to_string(),
                attr_name: attr_name.to_string(),
                value: Some(val),
            });
        }
    });
}

fn render_child_tag_attr_row(
    ui: &mut Ui,
    control_uuid: &str,
    child_tag: &str,
    attr_name: &str,
    label: &str,
    current: &str,
    edits: &mut Vec<ProjectEdit>,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("{label}:")).size(11.0).color(TEXT_MUTED));
        let mut val = current.to_string();
        if ui
            .add(TextEdit::singleline(&mut val).desired_width(f32::INFINITY))
            .changed()
        {
            edits.push(ProjectEdit::SetChildTagAttr {
                control_uuid: control_uuid.to_string(),
                child_tag: child_tag.to_string(),
                attr_name: attr_name.to_string(),
                value: Some(val),
            });
        }
    });
}

fn render_child_configs_section(
    ui: &mut Ui,
    control_uuid: &str,
    schema: Option<&'static BlockSchema>,
    extra_children: &[RawNode],
    edits: &mut Vec<ProjectEdit>,
) {
    let child_configs = schema.map(|s| s.child_configs).unwrap_or(&[]);

    let has_lm = child_configs.contains(&ChildConfigKind::LoggerMailer)
        || extra_children
            .iter()
            .any(|c| matches!(c.tag.as_str(), "LoggerMailer" | "Logger" | "Mailer"));
    if has_lm {
        let lm_node = extra_children
            .iter()
            .find(|c| matches!(c.tag.as_str(), "LoggerMailer" | "Logger" | "Mailer"));
        let tag_name = lm_node.map(|n| n.tag.as_str()).unwrap_or("LoggerMailer");
        egui::CollapsingHeader::new(
            RichText::new(format!("LOGGER / MAILER / TRACKER (<{tag_name}>)"))
                .size(11.0)
                .color(TEXT_MUTED)
                .strong(),
        )
        .default_open(true)
        .show(ui, |ui| {
            for (attr, label) in [
                ("Type", "Mode / Type"),
                ("Address", "Recipient / Address"),
                ("Subject", "Subject"),
                ("TextON", "Message ON (TextON)"),
                ("TextOFF", "Message OFF (TextOFF)"),
                ("Value", "Value Format"),
                ("RefCaller", "Caller / Service Ref"),
                ("RefNotification", "Notification Ref"),
            ] {
                let cur = lm_node.and_then(|n| n.attr(attr)).unwrap_or("");
                render_child_tag_attr_row(ui, control_uuid, tag_name, attr, label, cur, edits);
            }
        });
    }

    let has_hcl = child_configs.contains(&ChildConfigKind::Hcl)
        || extra_children.iter().any(|c| c.tag == "HCL");
    if has_hcl {
        let hcl_node = extra_children.iter().find(|c| c.tag == "HCL");
        egui::CollapsingHeader::new(
            RichText::new("HUMAN CENTRIC LIGHTING (<HCL>)")
                .size(11.0)
                .color(TEXT_MUTED)
                .strong(),
        )
        .default_open(false)
        .show(ui, |ui| {
            for (attr, label) in [
                ("Mode", "HCL Curve Mode"),
                ("Min", "Min Color Temp (K)"),
                ("Max", "Max Color Temp (K)"),
                ("Offset", "Time Offset (min)"),
                ("Hs", "Start Hour (Hs)"),
                ("He", "End Hour (He)"),
                ("Tfade", "Fade Time (s)"),
            ] {
                let cur = hcl_node.and_then(|n| n.attr(attr)).unwrap_or("");
                render_child_tag_attr_row(ui, control_uuid, "HCL", attr, label, cur, edits);
            }
        });
    }

    let has_psd = child_configs.contains(&ChildConfigKind::Psd)
        || extra_children.iter().any(|c| c.tag == "PSD");
    if has_psd {
        let psd_node = extra_children.iter().find(|c| c.tag == "PSD");
        egui::CollapsingHeader::new(
            RichText::new("SUN DIRECTION / SHADING (<PSD>)")
                .size(11.0)
                .color(TEXT_MUTED)
                .strong(),
        )
        .default_open(false)
        .show(ui, |ui| {
            for (attr, label) in [
                ("Type", "Polygon Type"),
                ("Az", "Azimuth (Az °)"),
                ("El", "Elevation (El °)"),
                ("AzS", "Start Azimuth (°)"),
                ("AzE", "End Azimuth (°)"),
                ("ElMin", "Min Elevation (°)"),
                ("ElMax", "Max Elevation (°)"),
            ] {
                let cur = psd_node.and_then(|n| n.attr(attr)).unwrap_or("");
                render_child_tag_attr_row(ui, control_uuid, "PSD", attr, label, cur, edits);
            }
        });
    }
}

fn render_linked_target_section(
    ui: &mut Ui,
    project: &LoxoneProject,
    ref_uuid: &str,
    output: &mut InspectorOutput,
) {
    let Some(target) = project.xref.objects_by_uuid.get(ref_uuid) else {
        return;
    };
    let target_schema = lookup_block_schema(&target.object_type);

    egui::CollapsingHeader::new(
        RichText::new(format!("LINKED TARGET ({})", target.object_type))
            .size(11.0)
            .color(ACCENT_BLUE)
            .strong(),
    )
    .default_open(true)
    .show(ui, |ui| {
        if ui
            .button(
                RichText::new(format!(
                    "🎯 Select Target in Tree: {} ({})",
                    if target.title.is_empty() {
                        &target.object_type
                    } else {
                        &target.title
                    },
                    target.object_type
                ))
                .size(11.0)
                .color(ACCENT_BLUE),
            )
            .clicked()
        {
            output.nav_action = Some(InspectorAction::SelectObject(ref_uuid.to_string()));
        }

        if let Some(ts) = target_schema {
            ui.label(RichText::new(ts.summary).size(10.5).color(TEXT_MUTED));
        }

        if let Some(tree_node) = find_tree_node(&project.periphery_tree, ref_uuid)
            .or_else(|| find_tree_node(&project.system_tree, ref_uuid))
        {
            render_singleline_attr_row(
                ui,
                &tree_node.uuid,
                "Title",
                "Target Title",
                &tree_node.title,
                &mut output.edits,
            );
            render_schema_parameters_section(
                ui,
                &tree_node.uuid,
                target_schema,
                &tree_node.raw_attributes,
                &tree_node.pins,
                &mut output.edits,
            );
        } else if let ObjectLocation::PageBlock {
            page_index,
            block_index,
            ..
        } = &target.location
        {
            if let Some(target_block) = project
                .pages
                .get(*page_index)
                .and_then(|p| p.blocks.get(*block_index))
            {
                render_singleline_attr_row(
                    ui,
                    &target_block.uuid,
                    "Title",
                    "Target Title",
                    &target_block.title,
                    &mut output.edits,
                );
                render_schema_parameters_section(
                    ui,
                    &target_block.uuid,
                    target_schema,
                    &target_block.raw_attributes,
                    &target_block.pins,
                    &mut output.edits,
                );
            }
        }
    });
}

fn render_documented_pins_section(
    ui: &mut Ui,
    project: &LoxoneProject,
    control_uuid: &str,
    pins: &[ConnectorPin],
    schema: Option<&'static BlockSchema>,
    output: &mut InspectorOutput,
) {
    if pins.is_empty() {
        return;
    }

    egui::CollapsingHeader::new(
        RichText::new(format!("INPUTS & OUTPUTS ({})", pins.len()))
            .size(11.0)
            .color(TEXT_MUTED)
            .strong(),
    )
    .default_open(true)
    .show(ui, |ui| {
        for pin in pins {
            let doc = schema.and_then(|s| s.find_pin_doc(&pin.key));
            let dir_badge = match pin.pin_kind {
                PinKind::Output => "OUT",
                PinKind::Parameter => "PAR",
                PinKind::Input => "IN",
            };
            let title_str = match doc {
                Some(d) => format!("[{dir_badge}] {} — {}", pin.key, d.label),
                None => format!("[{dir_badge}] {}", pin.key),
            };
            let resp = ui.label(
                RichText::new(title_str)
                    .monospace()
                    .size(11.0)
                    .color(TEXT_PRIMARY),
            );
            if let Some(d) = doc {
                resp.on_hover_text(d.description);
                ui.label(RichText::new(d.description).size(10.0).color(TEXT_MUTED));
            }

            // Incoming wire navigation links + disconnect button
            for link in &pin.incoming_links {
                if link.is_hidden() {
                    continue;
                }
                ui.horizontal(|ui| {
                    if let Some(src) = project.xref.pins_by_uuid.get(&link.source_pin_uuid) {
                        if let Some(p_idx) = src.page_index {
                            let page_name = src.page_title.as_deref().unwrap_or("Page");
                            let btn_label = format!(
                                "  ⬅ {}.{}: {} ({})",
                                src.owner_title, src.pin_key, src.owner_type, page_name
                            );
                            if ui
                                .small_button(RichText::new(btn_label).color(ACCENT_BLUE))
                                .clicked()
                            {
                                output.nav_action = Some(InspectorAction::NavigateToBlock {
                                    page_index: p_idx,
                                    block_uuid: src.owner_uuid.clone(),
                                });
                            }
                        } else {
                            ui.label(
                                RichText::new(format!(
                                    "  ⬅ {}.{} ({})",
                                    src.owner_title, src.pin_key, src.owner_type
                                ))
                                .size(11.0)
                                .color(TEXT_MUTED),
                            );
                        }
                    } else {
                        ui.label(
                            RichText::new(format!("  ⬅ {}", link.source_pin_uuid))
                                .size(10.5)
                                .monospace()
                                .color(TEXT_MUTED),
                        );
                    }
                    if ui
                        .small_button(RichText::new("✕").color(crate::theme::STATUS_ERROR))
                        .on_hover_text("Disconnect wire")
                        .clicked()
                    {
                        output.edits.push(ProjectEdit::RemoveWireLink {
                            target_block_uuid: control_uuid.to_string(),
                            target_pin_key: pin.key.clone(),
                            source_pin_uuid: link.source_pin_uuid.clone(),
                        });
                    }
                });
            }

            // Outgoing wire navigation links
            if let Some(targets) = project.xref.outgoing_wires_by_pin_uuid.get(&pin.uuid) {
                for t in targets {
                    if t.is_hidden {
                        continue;
                    }
                    let btn_label = format!(
                        "  ➡ {}.{} ({})",
                        t.target_block_title, t.target_pin_key, t.target_page_title
                    );
                    if ui
                        .small_button(RichText::new(btn_label).color(ACCENT_BLUE))
                        .clicked()
                    {
                        output.nav_action = Some(InspectorAction::NavigateToBlock {
                            page_index: t.target_page_index,
                            block_uuid: t.target_block_uuid.clone(),
                        });
                    }
                }
            }
        }
    });
}

fn render_advanced_xml_section(
    ui: &mut Ui,
    control_uuid: &str,
    raw_attributes: &[(String, String)],
    extra_children: &[RawNode],
    edits: &mut Vec<ProjectEdit>,
) {
    egui::CollapsingHeader::new(
        RichText::new(format!(
            "ADVANCED / RAW XML ATTRIBUTES ({})",
            raw_attributes.len()
        ))
        .size(11.0)
        .color(TEXT_MUTED),
    )
    .default_open(false)
    .show(ui, |ui| {
        for (k, v) in raw_attributes {
            if matches!(k.as_str(), "U" | "Type" | "V") {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{k}:"))
                            .monospace()
                            .size(10.5)
                            .color(TEXT_MUTED),
                    );
                    ui.label(RichText::new(v).monospace().size(10.5).color(TEXT_PRIMARY));
                });
            } else {
                render_singleline_attr_row(ui, control_uuid, k, k, v, edits);
            }
        }

        if !extra_children.is_empty() {
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("Child XML Elements ({})", extra_children.len()))
                    .size(10.5)
                    .color(TEXT_MUTED)
                    .strong(),
            );
            render_raw_nodes(ui, extra_children);
        }
    });
}

fn render_raw_nodes(ui: &mut Ui, nodes: &[RawNode]) {
    for (idx, raw) in nodes.iter().enumerate() {
        let mut header_str = format!("<{}", raw.tag);
        for (k, v) in &raw.attributes {
            header_str.push_str(&format!(" {}=\"{}\"", k, v));
        }
        header_str.push('>');
        ui.label(
            RichText::new(header_str)
                .monospace()
                .size(10.0)
                .color(TEXT_PRIMARY),
        );
        if let Some(txt) = &raw.text {
            ui.label(
                RichText::new(txt)
                    .monospace()
                    .size(10.0)
                    .color(TEXT_MUTED),
            );
        }
        if !raw.children.is_empty() {
            ui.indent((idx, &raw.tag), |ui| {
                render_raw_nodes(ui, &raw.children);
            });
        }
    }
}

fn find_tree_node<'a>(nodes: &'a [TreeNode], uuid: &str) -> Option<&'a TreeNode> {
    for node in nodes {
        if node.uuid == uuid {
            return Some(node);
        }
        if let Some(found) = find_tree_node(&node.children, uuid) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use loxone_core::parser::load_loxone_file;

    #[test]
    fn test_find_tree_node_in_klimczaki() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should parse");
        let root_uuid = &project.periphery_tree[0].uuid;
        let found = find_tree_node(&project.periphery_tree, root_uuid);
        assert!(found.is_some());
        assert_eq!(found.unwrap().uuid, *root_uuid);
    }

    #[test]
    fn test_inspector_renders_complex_blocks_and_refs_and_periphery() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should parse");

        let find_block_by_type = |ty: &str| -> String {
            project
                .pages
                .iter()
                .flat_map(|p| p.blocks.iter())
                .find(|b| b.block_type == ty)
                .unwrap_or_else(|| panic!("Should find block of type {ty}"))
                .uuid
                .clone()
        };

        fn find_tree_by_type(nodes: &[TreeNode], ty: &str) -> Option<String> {
            for n in nodes {
                if n.control_type == ty {
                    return Some(n.uuid.clone());
                }
                if let Some(found) = find_tree_by_type(&n.children, ty) {
                    return Some(found);
                }
            }
            None
        }

        let target_uuids = vec![
            find_block_by_type("LightController2"),
            find_block_by_type("AutoJalousie"),
            find_block_by_type("HeatIRoomController2"),
            find_block_by_type("OutputRefLM"),
            find_block_by_type("InputRef"),
            find_block_by_type("OutputRef"),
            find_block_by_type("Text"),
            find_tree_by_type(&project.periphery_tree, "TreeAsensor")
                .expect("Should find TreeAsensor"),
            find_tree_by_type(&project.system_tree, "VirtualOutCmd")
                .or_else(|| find_tree_by_type(&project.periphery_tree, "VirtualOutCmd"))
                .expect("Should find VirtualOutCmd"),
            project.categories.keys().next().unwrap().clone(),
            project.places.keys().next().unwrap().clone(),
        ];

        let mut store = loxone_core::miniserver::LiveValueStore::new();
        store.seed_project_defaults(&project);

        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let empty_out: InspectorOutput = render_right_inspector(ui, &project, None, None);
                assert!(empty_out.nav_action.is_none());
                assert!(empty_out.edits.is_empty());

                for uuid in &target_uuids {
                    let out: InspectorOutput =
                        render_right_inspector(ui, &project, Some(uuid), Some(&store));
                    assert!(out.nav_action.is_none());
                    assert!(
                        out.edits.is_empty(),
                        "Non-interacted frame should not emit edits for {uuid}"
                    );
                }
            });
        });
    }

    #[test]
    fn test_inspector_renders_every_block_on_all_28_pages_without_panic() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should parse");
        assert_eq!(project.pages.len(), 28);

        let ctx = egui::Context::default();
        for (page_idx, page) in project.pages.iter().enumerate() {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    for block in &page.blocks {
                        let out: InspectorOutput =
                            render_right_inspector(ui, &project, Some(&block.uuid), None);
                        assert!(
                            out.edits.is_empty(),
                            "Page {page_idx} block {} ({}) emitted unexpected edits on idle render",
                            block.uuid,
                            block.block_type
                        );
                    }
                });
            });
        }
    }
}
