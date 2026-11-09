use egui::{
    Align, Color32, ComboBox, Context, CornerRadius, DragValue, Id, Key, Layout, Order, RichText,
    ScrollArea, Stroke, TextEdit, Vec2,
};
use loxone_core::advanced_blocks_config::{
    collect_audio_out_channels, collect_audio_zone_candidates, collect_central_candidates,
    collect_irc_candidates, collect_presence_sensor_candidates, AccessBlockConfig, AudioZoneConfig,
    CentralBlockConfig, ClimateControllerConfig, PresenceBlockConfig, SequenceConfig,
    ShadingBlockConfig,
};
use loxone_core::model::LoxoneProject;
use loxone_core::writer::ProjectEdit;

use crate::i18n::{tr, tr_fmt};
use crate::theme::{
    draw_modal_backdrop, should_close_modal_on_outside_click, BORDER_SUBTLE, CARD_BG, ELEVATED_BG,
    LOXONE_GREEN, PANEL_BG, TEXT_MUTED, TEXT_PRIMARY,
};

#[derive(Debug, Clone)]
pub enum ActiveAdvancedModal {
    Central(CentralBlockConfig),
    AudioZone(AudioZoneConfig),
    Climate(ClimateControllerConfig),
    Sequence(SequenceConfig),
    Shading(ShadingBlockConfig),
    Access(AccessBlockConfig),
    Presence(PresenceBlockConfig),
}

#[derive(Debug, Clone, Default)]
pub struct AdvancedBlocksModalState {
    pub is_open: bool,
    pub page_uuid: String,
    pub block_uuid: String,
    pub block_title: String,
    pub block_type: String,
    pub search_filter: String,
    pub active: Option<ActiveAdvancedModal>,
}

impl AdvancedBlocksModalState {
    pub fn open_for_block(&mut self, project: &LoxoneProject, block_uuid: &str) -> bool {
        let Some((page_uuid, block)) = project.pages.iter().find_map(|p| {
            p.blocks
                .iter()
                .find(|b| b.uuid == block_uuid)
                .map(|b| (p.uuid.clone(), b))
        }) else {
            return false;
        };

        let active = if CentralBlockConfig::is_central_block(&block.block_type) {
            Some(ActiveAdvancedModal::Central(CentralBlockConfig::from_block(
                block,
            )))
        } else if AudioZoneConfig::is_audio_zone_block(&block.block_type) {
            Some(ActiveAdvancedModal::AudioZone(AudioZoneConfig::from_block(
                block, project,
            )))
        } else if ClimateControllerConfig::is_climate_controller_block(&block.block_type) {
            Some(ActiveAdvancedModal::Climate(
                ClimateControllerConfig::from_block_with_project(block, Some(project)),
            ))
        } else if SequenceConfig::is_sequence_block(&block.block_type) {
            Some(ActiveAdvancedModal::Sequence(SequenceConfig::from_block(
                block,
            )))
        } else if ShadingBlockConfig::is_shading_block(&block.block_type) {
            Some(ActiveAdvancedModal::Shading(ShadingBlockConfig::from_block(
                block,
            )))
        } else if AccessBlockConfig::is_access_block(&block.block_type) {
            Some(ActiveAdvancedModal::Access(AccessBlockConfig::from_block(
                block,
            )))
        } else if PresenceBlockConfig::is_presence_block(&block.block_type) {
            Some(ActiveAdvancedModal::Presence(
                PresenceBlockConfig::from_block(block),
            ))
        } else {
            None
        };

        let Some(active) = active else {
            return false;
        };

        self.is_open = true;
        self.page_uuid = page_uuid;
        self.block_uuid = block.uuid.clone();
        self.block_title = if block.title.is_empty() {
            block.block_type.clone()
        } else {
            block.title.clone()
        };
        self.block_type = block.block_type.clone();
        self.search_filter.clear();
        self.active = Some(active);
        true
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.active = None;
    }
}

pub fn render_advanced_blocks_modal(
    ctx: &Context,
    state: &mut AdvancedBlocksModalState,
    project: Option<&LoxoneProject>,
) -> Option<ProjectEdit> {
    if !state.is_open {
        return None;
    }
    let Some(proj) = project else {
        state.close();
        return None;
    };

    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        state.close();
        return None;
    }

    let (backdrop_clicked, popup_was_open) = draw_modal_backdrop(
        ctx,
        "advanced_blocks_modal_backdrop",
        Order::Foreground,
        175,
    );

    let mut pending_edit: Option<ProjectEdit> = None;
    let mut should_close = false;
    let mut modal_rect = None;

    let subtitle = match &state.active {
        Some(ActiveAdvancedModal::Central(cfg)) => tr(cfg.kind.label()),
        Some(ActiveAdvancedModal::AudioZone(_)) => tr("Audio Zone & Speaker Routing"),
        Some(ActiveAdvancedModal::Climate(cfg)) => {
            if cfg.is_heatmixer {
                tr("Intelligent Temperature Controller (Heatmixer2)")
            } else {
                tr("HVAC Climate Controller")
            }
        }
        Some(ActiveAdvancedModal::Sequence(_)) => tr("Sequence Controller Step Table"),
        Some(ActiveAdvancedModal::Shading(_)) => {
            tr("Automatic Blinds & Solar Shading Configuration")
        }
        Some(ActiveAdvancedModal::Access(_)) => tr("Access Control, Intercom & Door Parameters"),
        Some(ActiveAdvancedModal::Presence(_)) => {
            tr("Presence Detector Timing & Assigned Sensors")
        }
        None => tr("Advanced Block Configuration"),
    };

    if let Some(inner) = egui::Window::new("advanced_blocks_config_window")
        .id(Id::new("advanced_blocks_config_window"))
        .order(Order::Foreground)
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size([840.0, 560.0])
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(PANEL_BG)
                .stroke(Stroke::new(1.5_f32, LOXONE_GREEN))
                .corner_radius(CornerRadius::same(10))
                .inner_margin(egui::Margin::same(16)),
        )
        .show(ctx, |ui| {
            // Header
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        egui::Frame::NONE
                            .fill(Color32::from_rgba_unmultiplied(0x69, 0xC3, 0x50, 34))
                            .stroke(Stroke::new(1.0_f32, LOXONE_GREEN))
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(egui::Margin::symmetric(8, 2))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(&state.block_type)
                                        .size(11.0)
                                        .strong()
                                        .color(LOXONE_GREEN),
                                );
                            });
                        ui.label(
                            RichText::new(&state.block_title)
                                .size(16.5)
                                .strong()
                                .color(TEXT_PRIMARY),
                        );
                    });
                    ui.label(RichText::new(subtitle).size(11.5).color(TEXT_MUTED));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(RichText::new("✕").size(13.0).color(TEXT_MUTED))
                                .fill(CARD_BG)
                                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                                .corner_radius(CornerRadius::same(6)),
                        )
                        .clicked()
                    {
                        should_close = true;
                    }
                });
            });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            let page_uuid = state.page_uuid.clone();
            let block_uuid = state.block_uuid.clone();
            let block_type = state.block_type.clone();
            let filter_ref = &mut state.search_filter;

            if let Some(active) = &mut state.active {
                match active {
                    ActiveAdvancedModal::Central(cfg) => {
                        render_central_body(ui, cfg, proj, &block_type, filter_ref);
                    }
                    ActiveAdvancedModal::AudioZone(cfg) => {
                        render_audio_zone_body(ui, cfg, proj, &block_uuid);
                    }
                    ActiveAdvancedModal::Climate(cfg) => {
                        render_climate_body(ui, cfg, proj, filter_ref);
                    }
                    ActiveAdvancedModal::Sequence(cfg) => {
                        render_sequence_body(ui, cfg);
                    }
                    ActiveAdvancedModal::Shading(cfg) => {
                        render_shading_body(ui, cfg);
                    }
                    ActiveAdvancedModal::Access(cfg) => {
                        render_access_body(ui, cfg);
                    }
                    ActiveAdvancedModal::Presence(cfg) => {
                        render_presence_body(ui, cfg, proj, filter_ref);
                    }
                }
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(8.0);

            // Footer
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(tr(
                        "Changes support full Undo/Redo (Cmd+Z) and XML roundtrip.",
                    ))
                    .size(11.0)
                    .color(TEXT_MUTED),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(tr("✔ Apply & Save"))
                                    .size(12.5)
                                    .strong()
                                    .color(Color32::from_rgb(0x0B, 0x12, 0x09)),
                            )
                            .fill(LOXONE_GREEN)
                            .corner_radius(CornerRadius::same(6))
                            .min_size(Vec2::new(124.0, 30.0)),
                        )
                        .clicked()
                    {
                        if let Some(active) = &state.active {
                            pending_edit = Some(match active {
                                ActiveAdvancedModal::Central(cfg) => {
                                    cfg.to_project_edit(&page_uuid, &block_uuid)
                                }
                                ActiveAdvancedModal::AudioZone(cfg) => {
                                    let channels = collect_audio_out_channels(proj);
                                    cfg.to_project_edit(&page_uuid, &block_uuid, &channels)
                                }
                                ActiveAdvancedModal::Climate(cfg) => {
                                    cfg.to_project_edit_with_project(
                                        &page_uuid,
                                        &block_uuid,
                                        Some(proj),
                                    )
                                }
                                ActiveAdvancedModal::Sequence(cfg) => {
                                    cfg.to_project_edit(&page_uuid, &block_uuid)
                                }
                                ActiveAdvancedModal::Shading(cfg) => {
                                    cfg.to_project_edit(&page_uuid, &block_uuid)
                                }
                                ActiveAdvancedModal::Access(cfg) => {
                                    cfg.to_project_edit(&page_uuid, &block_uuid)
                                }
                                ActiveAdvancedModal::Presence(cfg) => {
                                    cfg.to_project_edit(&page_uuid, &block_uuid)
                                }
                            });
                        }
                        should_close = true;
                    }

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(tr("Cancel")).size(12.0).color(TEXT_PRIMARY),
                            )
                            .fill(CARD_BG)
                            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                            .corner_radius(CornerRadius::same(6))
                            .min_size(Vec2::new(84.0, 30.0)),
                        )
                        .clicked()
                    {
                        should_close = true;
                    }
                });
            });
        })
    {
        modal_rect = Some(inner.response.rect);
    }

    if should_close_modal_on_outside_click(
        ctx,
        "advanced_blocks_config_window",
        backdrop_clicked,
        popup_was_open,
        modal_rect,
    ) {
        should_close = true;
    }

    if should_close {
        state.close();
    }

    pending_edit
}

fn render_central_body(
    ui: &mut egui::Ui,
    cfg: &mut CentralBlockConfig,
    project: &LoxoneProject,
    block_type: &str,
    search_filter: &mut String,
) {
    let candidates = collect_central_candidates(project, block_type);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(tr_fmt(
                "Assigned member blocks: {selected} / {total}",
                &[
                    ("selected", &cfg.selected_uuids.len().to_string()),
                    ("total", &candidates.len().to_string()),
                ],
            ))
            .size(12.0)
            .strong()
            .color(LOXONE_GREEN),
        );
        ui.add_space(12.0);
        ui.add(
            TextEdit::singleline(search_filter)
                .hint_text(tr("🔍 Filter by block title, room, or page..."))
                .desired_width(280.0),
        );
        if ui.button(tr("Select All")).clicked() {
            cfg.select_all(&candidates);
        }
        if ui.button(tr("Clear All")).clicked() {
            cfg.clear_all();
        }
    });

    ui.add_space(8.0);
    let q = search_filter.trim().to_lowercase();
    ScrollArea::vertical()
        .id_salt("central_candidates_scroll")
        .max_height(380.0)
        .show(ui, |ui| {
            for cand in &candidates {
                if !q.is_empty()
                    && !cand.title.to_lowercase().contains(&q)
                    && !cand.room_name.to_lowercase().contains(&q)
                    && !cand.page_title.to_lowercase().contains(&q)
                {
                    continue;
                }
                let mut selected = cfg.is_selected(&cand.uuid);
                egui::Frame::NONE
                    .fill(if selected { ELEVATED_BG } else { CARD_BG })
                    .stroke(Stroke::new(
                        1.0_f32,
                        if selected { LOXONE_GREEN } else { BORDER_SUBTLE },
                    ))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui.checkbox(&mut selected, "").changed() {
                                cfg.toggle_uuid(&cand.uuid);
                            }
                            ui.label(
                                RichText::new(&cand.title)
                                    .size(12.5)
                                    .strong()
                                    .color(TEXT_PRIMARY),
                            );
                            ui.label(
                                RichText::new(format!("[{}]", cand.block_type))
                                    .size(11.0)
                                    .monospace()
                                    .color(LOXONE_GREEN),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "{} • {}",
                                        cand.room_name, cand.page_title
                                    ))
                                    .size(11.0)
                                    .color(TEXT_MUTED),
                                );
                            });
                        });
                    });
                ui.add_space(4.0);
            }
        });
}

fn render_audio_zone_body(
    ui: &mut egui::Ui,
    cfg: &mut AudioZoneConfig,
    project: &LoxoneProject,
    block_uuid: &str,
) {
    let channels = collect_audio_out_channels(project);
    let zone_candidates = collect_audio_zone_candidates(project, block_uuid);

    ui.columns(2, |cols| {
        // Left column: Streaming, Speaker Config, Volume Presets & Favorites
        egui::Frame::NONE
            .fill(CARD_BG)
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::same(12))
            .show(&mut cols[0], |ui| {
                ScrollArea::vertical()
                    .id_salt("audio_left_scroll")
                    .max_height(395.0)
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(tr("ZONE & STREAMING SERVICES"))
                                .size(11.0)
                                .strong()
                                .color(LOXONE_GREEN),
                        );
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(tr("Speaker Mode (SpCfg):"));
                            ComboBox::from_id_salt("audio_spcfg_combo")
                                .selected_text(match cfg.speaker_cfg {
                                    1 => tr("1 — Mono"),
                                    2 => tr("2 — Stereo"),
                                    4 => tr("4 — Quad / Multi-Channel"),
                                    _ => tr("Custom"),
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut cfg.speaker_cfg, 1, tr("1 — Mono"));
                                    ui.selectable_value(&mut cfg.speaker_cfg, 2, tr("2 — Stereo"));
                                    ui.selectable_value(
                                        &mut cfg.speaker_cfg,
                                        4,
                                        tr("4 — Quad / Multi-Channel"),
                                    );
                                });
                        });
                        ui.checkbox(&mut cfg.presence_enabled, tr("Presence Auto-Activation (PR)"));
                        ui.checkbox(&mut cfg.airplay_enabled, tr("Apple AirPlay 2 Receiver (APL)"));
                        ui.checkbox(&mut cfg.spotify_enabled, tr("Spotify Connect Receiver (SPC)"));
                        ui.checkbox(&mut cfg.high_pass_filter, tr("Speaker High-Pass Filter (HPF)"));

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(tr("VOLUME PRESETS (%)"))
                                .size(11.0)
                                .strong()
                                .color(LOXONE_GREEN),
                        );
                        ui.add_space(4.0);

                        egui::Grid::new("audio_vol_grid")
                            .num_columns(2)
                            .spacing([16.0, 5.0])
                            .show(ui, |ui| {
                                ui.label(tr("Default Volume (DefVol):"));
                                ui.add(
                                    DragValue::new(&mut cfg.default_vol)
                                        .range(0..=100)
                                        .suffix("%"),
                                );
                                ui.end_row();

                                ui.label(tr("Max Volume Limit (MaxVol):"));
                                ui.add(
                                    DragValue::new(&mut cfg.max_vol).range(1..=100).suffix("%"),
                                );
                                ui.end_row();

                                ui.label(tr("Volume Step (Step):"));
                                ui.add(
                                    DragValue::new(&mut cfg.step_vol).range(1..=20).suffix("%"),
                                );
                                ui.end_row();

                                ui.label(tr("Alarm Volume (AlarmVol):"));
                                ui.add(
                                    DragValue::new(&mut cfg.alarm_vol)
                                        .range(0..=100)
                                        .suffix("%"),
                                );
                                ui.end_row();

                                ui.label(tr("Doorbell Volume (BellVol):"));
                                ui.add(
                                    DragValue::new(&mut cfg.bell_vol).range(0..=100).suffix("%"),
                                );
                                ui.end_row();

                                ui.label(tr("Buzzer Volume (BuzzVol):"));
                                ui.add(
                                    DragValue::new(&mut cfg.buzzer_vol)
                                        .range(0..=100)
                                        .suffix("%"),
                                );
                                ui.end_row();

                                ui.label(tr("Text-To-Speech (TTSVol):"));
                                ui.add(
                                    DragValue::new(&mut cfg.tts_vol).range(0..=100).suffix("%"),
                                );
                                ui.end_row();
                            });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(tr("ROOM AUDIO FAVORITES / PRESETS"))
                                    .size(11.0)
                                    .strong()
                                    .color(LOXONE_GREEN),
                            );
                            if ui.small_button(tr("+ Add Preset")).clicked() {
                                let next_idx = cfg.favorites.len() + 1;
                                cfg.add_favorite(
                                    format!("Favorite {next_idx}"),
                                    "tunein://station".to_string(),
                                );
                            }
                        });
                        ui.add_space(4.0);
                        let mut remove_fav_idx = None;
                        for (idx, fav) in cfg.favorites.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("#{}", fav.slot))
                                        .size(11.0)
                                        .monospace()
                                        .color(LOXONE_GREEN),
                                );
                                ui.add(
                                    TextEdit::singleline(&mut fav.title)
                                        .hint_text("Preset name")
                                        .desired_width(110.0),
                                );
                                ui.add(
                                    TextEdit::singleline(&mut fav.source_uri)
                                        .hint_text("URI / Source")
                                        .desired_width(135.0),
                                );
                                if ui.small_button("✕").clicked() {
                                    remove_fav_idx = Some(idx);
                                }
                            });
                        }
                        if let Some(idx) = remove_fav_idx {
                            cfg.remove_favorite(idx);
                        }
                    });
            });

        // Right column: Audioserver Output Channels (`AudioOut`) + Player Group Sync
        egui::Frame::NONE
            .fill(CARD_BG)
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::same(12))
            .show(&mut cols[1], |ui| {
                ui.label(
                    RichText::new(format!(
                        "{} ({})",
                        tr("ASSIGNED AUDIO OUTPUT CHANNELS"),
                        cfg.assigned_channel_uuids.len()
                    ))
                    .size(11.0)
                    .strong()
                    .color(LOXONE_GREEN),
                );
                ui.add_space(4.0);
                ScrollArea::vertical()
                    .id_salt("audio_channels_scroll")
                    .max_height(215.0)
                    .show(ui, |ui| {
                        for ch in &channels {
                            let mut assigned = cfg.is_channel_assigned(&ch.uuid);
                            egui::Frame::NONE
                                .fill(if assigned { ELEVATED_BG } else { PANEL_BG })
                                .stroke(Stroke::new(
                                    1.0_f32,
                                    if assigned {
                                        LOXONE_GREEN
                                    } else {
                                        BORDER_SUBTLE
                                    },
                                ))
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(egui::Margin::symmetric(8, 6))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        if ui.checkbox(&mut assigned, "").changed() {
                                            cfg.toggle_channel(&ch.uuid);
                                        }
                                        ui.vertical(|ui| {
                                            ui.label(
                                                RichText::new(&ch.title)
                                                    .size(12.0)
                                                    .strong()
                                                    .color(TEXT_PRIMARY),
                                            );
                                            ui.label(
                                                RichText::new(&ch.parent_title)
                                                    .size(10.5)
                                                    .color(TEXT_MUTED),
                                            );
                                        });
                                    });
                                    if assigned {
                                        let mut mxv = cfg.channel_max_vol(ch);
                                        let mut voll = cfg.channel_line_out_vol(ch);
                                        let mut vg = cfg.channel_gain(ch);
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new("Max (MxV):")
                                                    .size(10.5)
                                                    .color(TEXT_MUTED),
                                            );
                                            if ui
                                                .add(
                                                    DragValue::new(&mut mxv)
                                                        .range(1..=100)
                                                        .suffix("%"),
                                                )
                                                .changed()
                                            {
                                                cfg.set_channel_max_vol(&ch.uuid, mxv);
                                            }
                                            ui.label(
                                                RichText::new("Line (VOLL):")
                                                    .size(10.5)
                                                    .color(TEXT_MUTED),
                                            );
                                            if ui
                                                .add(
                                                    DragValue::new(&mut voll)
                                                        .range(0..=100)
                                                        .suffix("%"),
                                                )
                                                .changed()
                                            {
                                                cfg.set_channel_line_out_vol(&ch.uuid, voll);
                                            }
                                            ui.label(
                                                RichText::new("Gain (Vg):")
                                                    .size(10.5)
                                                    .color(TEXT_MUTED),
                                            );
                                            if ui
                                                .add(
                                                    DragValue::new(&mut vg)
                                                        .range(-12..=12)
                                                        .suffix("dB"),
                                                )
                                                .changed()
                                            {
                                                cfg.set_channel_gain(&ch.uuid, vg);
                                            }
                                        });
                                    }
                                });
                            ui.add_space(4.0);
                        }
                    });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!(
                        "{} ({})",
                        tr("PLAYER GROUP & MULTI-ROOM SYNC"),
                        cfg.grouped_player_uuids.len()
                    ))
                    .size(11.0)
                    .strong()
                    .color(LOXONE_GREEN),
                );
                ui.add_space(4.0);
                let selected_master_label = cfg
                    .master_player_uuid
                    .as_deref()
                    .and_then(|u| zone_candidates.iter().find(|c| c.uuid == u))
                    .map(|c| format!("{} ({})", c.title, c.room_name))
                    .unwrap_or_else(|| tr("Independent / Group Master").to_string());
                ui.horizontal(|ui| {
                    ui.label(tr("Master Zone (Mst):"));
                    ComboBox::from_id_salt("audio_master_zone_combo")
                        .selected_text(selected_master_label)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut cfg.master_player_uuid,
                                None,
                                tr("Independent / Group Master"),
                            );
                            for cand in &zone_candidates {
                                ui.selectable_value(
                                    &mut cfg.master_player_uuid,
                                    Some(cand.uuid.clone()),
                                    format!("{} ({})", cand.title, cand.room_name),
                                );
                            }
                        });
                });
                ui.add_space(4.0);
                ScrollArea::vertical()
                    .id_salt("audio_group_members_scroll")
                    .max_height(110.0)
                    .show(ui, |ui| {
                        for cand in &zone_candidates {
                            let mut grouped = cfg.is_player_grouped(&cand.uuid);
                            ui.horizontal(|ui| {
                                if ui.checkbox(&mut grouped, "").changed() {
                                    cfg.toggle_grouped_player(&cand.uuid);
                                }
                                ui.label(
                                    RichText::new(&cand.title)
                                        .size(11.5)
                                        .color(TEXT_PRIMARY),
                                );
                                ui.label(
                                    RichText::new(format!("• {}", cand.room_name))
                                        .size(10.5)
                                        .color(TEXT_MUTED),
                                );
                            });
                        }
                    });
            });
    });
}

fn render_climate_body(
    ui: &mut egui::Ui,
    cfg: &mut ClimateControllerConfig,
    project: &LoxoneProject,
    search_filter: &mut String,
) {
    if cfg.is_heatmixer {
        let ircs = collect_irc_candidates(project);
        ui.columns(2, |cols| {
            egui::Frame::NONE
                .fill(CARD_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::same(12))
                .show(&mut cols[0], |ui| {
                    ui.label(
                        RichText::new(tr("HEATING CURVE & MIXER PARAMETERS"))
                            .size(11.0)
                            .strong()
                            .color(LOXONE_GREEN),
                    );
                    ui.add_space(8.0);
                    egui::Grid::new("heatmixer_grid")
                        .num_columns(2)
                        .spacing([16.0, 8.0])
                        .show(ui, |ui| {
                            ui.label(tr("Controller Mode:"));
                            ui.add(DragValue::new(&mut cfg.controller_mode).range(0..=4));
                            ui.end_row();

                            ui.label(tr("Cooling Mode (Cool):"));
                            ui.add(DragValue::new(&mut cfg.cooling_mode).range(0..=3));
                            ui.end_row();

                            ui.label(tr("Min Flow Temp (Min):"));
                            ui.add(
                                DragValue::new(&mut cfg.min_flow_temp)
                                    .speed(0.5)
                                    .range(0.0..=60.0)
                                    .suffix(" °C"),
                            );
                            ui.end_row();

                            ui.label(tr("Max Flow Temp (Max):"));
                            ui.add(
                                DragValue::new(&mut cfg.max_flow_temp)
                                    .speed(0.5)
                                    .range(15.0..=90.0)
                                    .suffix(" °C"),
                            );
                            ui.end_row();

                            ui.label(tr("Buffer Offset (BufferInc):"));
                            ui.add(
                                DragValue::new(&mut cfg.buffer_inc)
                                    .speed(0.5)
                                    .range(0.0..=30.0)
                                    .suffix(" K"),
                            );
                            ui.end_row();

                            ui.label(tr("Flow Offset (PrevInc):"));
                            ui.add(
                                DragValue::new(&mut cfg.prev_inc)
                                    .speed(0.5)
                                    .range(0.0..=20.0)
                                    .suffix(" K"),
                            );
                            ui.end_row();

                            ui.label(tr("Curve Slope (Transconductance):"));
                            ui.add(
                                DragValue::new(&mut cfg.transconductance)
                                    .speed(0.05)
                                    .range(0.1..=4.0),
                            );
                            ui.end_row();

                            ui.label(tr("Threshold (Treshold):"));
                            ui.add(
                                DragValue::new(&mut cfg.threshold)
                                    .speed(0.5)
                                    .range(0.0..=90.0)
                                    .suffix(" °C"),
                            );
                            ui.end_row();

                            ui.label(tr("Valve Gain (Gain):"));
                            ui.add(
                                DragValue::new(&mut cfg.gain).speed(0.1).range(0.1..=10.0),
                            );
                            ui.end_row();
                        });
                    ui.add_space(8.0);
                    ui.checkbox(&mut cfg.use_hvac, tr("Link to HVAC Controller (UseHVAc)"));
                });

            egui::Frame::NONE
                .fill(CARD_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::same(12))
                .show(&mut cols[1], |ui| {
                    ui.label(
                        RichText::new(format!(
                            "{} ({}/{})",
                            tr("ASSIGNED ROOM CONTROLLERS"),
                            cfg.assigned_irc_uuids.len(),
                            ircs.len()
                        ))
                        .size(11.0)
                        .strong()
                        .color(LOXONE_GREEN),
                    );
                    ui.add_space(6.0);
                    ui.add(
                        TextEdit::singleline(search_filter)
                            .hint_text(tr("🔍 Filter room controllers..."))
                            .desired_width(240.0),
                    );
                    ui.add_space(6.0);
                    let q = search_filter.trim().to_lowercase();
                    ScrollArea::vertical()
                        .id_salt("heatmixer_irc_scroll")
                        .max_height(310.0)
                        .show(ui, |ui| {
                            for irc in &ircs {
                                if !q.is_empty()
                                    && !irc.title.to_lowercase().contains(&q)
                                    && !irc.room_name.to_lowercase().contains(&q)
                                {
                                    continue;
                                }
                                let mut assigned = cfg.is_irc_assigned(&irc.uuid);
                                ui.horizontal(|ui| {
                                    if ui.checkbox(&mut assigned, "").changed() {
                                        cfg.toggle_irc(&irc.uuid);
                                    }
                                    ui.label(
                                        RichText::new(&irc.title)
                                            .size(12.0)
                                            .strong()
                                            .color(TEXT_PRIMARY),
                                    );
                                    ui.label(
                                        RichText::new(format!("({})", irc.room_name))
                                            .size(11.0)
                                            .color(TEXT_MUTED),
                                    );
                                });
                            }
                        });
                });
        });
    } else {
        let ircs = collect_irc_candidates(project);
        ui.columns(2, |cols| {
            egui::Frame::NONE
                .fill(CARD_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::same(14))
                .show(&mut cols[0], |ui| {
                    ui.label(
                        RichText::new(tr("HVAC CLIMATE CONTROLLER SETTINGS"))
                            .size(11.5)
                            .strong()
                            .color(LOXONE_GREEN),
                    );
                    ui.add_space(8.0);
                    egui::Grid::new("climate_us_grid")
                        .num_columns(2)
                        .spacing([24.0, 10.0])
                        .show(ui, |ui| {
                            ui.label(tr("Capabilities Bitmask (cCap):"));
                            ui.add(DragValue::new(&mut cfg.capabilities).range(1..=15));
                            ui.end_row();

                            ui.label(tr("Device Type (DType):"));
                            ui.add(DragValue::new(&mut cfg.device_type).range(0..=10));
                            ui.end_row();

                            ui.label(tr("Cooling Outdoor Limit (TempLimitC):"));
                            ui.add(
                                DragValue::new(&mut cfg.temp_limit_c)
                                    .speed(0.5)
                                    .suffix(" °C"),
                            );
                            ui.end_row();

                            ui.label(tr("Heating Outdoor Limit (TempLimitH):"));
                            ui.add(
                                DragValue::new(&mut cfg.temp_limit_h)
                                    .speed(0.5)
                                    .suffix(" °C"),
                            );
                            ui.end_row();

                            ui.label(tr("Activation Threshold (OnThreshold):"));
                            ui.add(
                                DragValue::new(&mut cfg.on_threshold)
                                    .speed(1.0)
                                    .suffix(" %"),
                            );
                            ui.end_row();

                            ui.label(tr("Target Humidity (HTarget):"));
                            ui.add(
                                DragValue::new(&mut cfg.humid_target)
                                    .speed(1.0)
                                    .suffix(" %"),
                            );
                            ui.end_row();

                            ui.label(tr("Pump Off Delay (TimeOffPump):"));
                            ui.add(DragValue::new(&mut cfg.time_off_pump).suffix(" s"));
                            ui.end_row();

                            ui.label(tr("Fan Delay (FanDelay):"));
                            ui.add(DragValue::new(&mut cfg.fan_delay).suffix(" s"));
                            ui.end_row();

                            ui.label(tr("Min Setpoint (TPMin):"));
                            ui.add(DragValue::new(&mut cfg.tp_min).speed(0.5).suffix(" °C"));
                            ui.end_row();

                            ui.label(tr("Max Setpoint (TPMax):"));
                            ui.add(DragValue::new(&mut cfg.tp_max).speed(0.5).suffix(" °C"));
                            ui.end_row();
                        });
                });

            egui::Frame::NONE
                .fill(CARD_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::same(12))
                .show(&mut cols[1], |ui| {
                    ui.label(
                        RichText::new(format!(
                            "{} ({}/{})",
                            tr("ASSIGNED ROOM CONTROLLERS"),
                            cfg.assigned_irc_uuids.len(),
                            ircs.len()
                        ))
                        .size(11.0)
                        .strong()
                        .color(LOXONE_GREEN),
                    );
                    ui.add_space(6.0);
                    ui.add(
                        TextEdit::singleline(search_filter)
                            .hint_text(tr("🔍 Filter room controllers..."))
                            .desired_width(240.0),
                    );
                    ui.add_space(6.0);
                    let q = search_filter.trim().to_lowercase();
                    ScrollArea::vertical()
                        .id_salt("hvac_irc_scroll")
                        .max_height(310.0)
                        .show(ui, |ui| {
                            for irc in &ircs {
                                if !q.is_empty()
                                    && !irc.title.to_lowercase().contains(&q)
                                    && !irc.room_name.to_lowercase().contains(&q)
                                {
                                    continue;
                                }
                                let mut assigned = cfg.is_irc_assigned(&irc.uuid);
                                ui.horizontal(|ui| {
                                    if ui.checkbox(&mut assigned, "").changed() {
                                        cfg.toggle_irc(&irc.uuid);
                                    }
                                    ui.label(
                                        RichText::new(&irc.title)
                                            .size(12.0)
                                            .strong()
                                            .color(TEXT_PRIMARY),
                                    );
                                    ui.label(
                                        RichText::new(format!("({})", irc.room_name))
                                            .size(11.0)
                                            .color(TEXT_MUTED),
                                    );
                                });
                            }
                        });
                });
        });
    }
}

fn render_sequence_body(ui: &mut egui::Ui, cfg: &mut SequenceConfig) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{} ({})", tr("Sequence Steps"), cfg.steps.len()))
                .size(12.0)
                .strong()
                .color(LOXONE_GREEN),
        );
        if ui
            .add(
                egui::Button::new(RichText::new(tr("+ Add Step")).size(11.5).color(LOXONE_GREEN))
                    .fill(CARD_BG)
                    .stroke(Stroke::new(1.0_f32, LOXONE_GREEN))
                    .corner_radius(CornerRadius::same(6)),
            )
            .clicked()
        {
            cfg.add_step();
        }
    });
    ui.add_space(8.0);

    let mut to_remove = None;
    let mut to_up = None;
    let mut to_down = None;
    let step_len = cfg.steps.len();

    ScrollArea::vertical()
        .id_salt("sequence_steps_scroll")
        .max_height(360.0)
        .show(ui, |ui| {
            for (idx, step) in cfg.steps.iter_mut().enumerate() {
                egui::Frame::NONE
                    .fill(CARD_BG)
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("#{}", idx + 1))
                                    .size(12.0)
                                    .strong()
                                    .color(LOXONE_GREEN),
                            );
                            ui.add(
                                TextEdit::singleline(&mut step.label)
                                    .hint_text(tr("Step Name"))
                                    .desired_width(120.0),
                            );
                            ui.label(RichText::new(tr("IF")).size(11.0).color(TEXT_MUTED));
                            ui.add(
                                TextEdit::singleline(&mut step.condition)
                                    .hint_text(tr("Condition (e.g. AI1 > 0)"))
                                    .desired_width(130.0),
                            );
                            ui.label("AQ1:");
                            ui.add(DragValue::new(&mut step.aq1).speed(0.5));
                            ui.label("AQ2:");
                            ui.add(DragValue::new(&mut step.aq2).speed(0.5));
                            ui.label("AQ3:");
                            ui.add(DragValue::new(&mut step.aq3).speed(0.5));
                            ui.label("AQ4:");
                            ui.add(DragValue::new(&mut step.aq4).speed(0.5));
                            ui.label(tr("Wait:"));
                            ui.add(
                                DragValue::new(&mut step.wait_sec)
                                    .speed(0.5)
                                    .range(0.0..=3600.0)
                                    .suffix("s"),
                            );

                            if ui.add_enabled(idx > 0, egui::Button::new("▲")).clicked() {
                                to_up = Some(idx);
                            }
                            if ui
                                .add_enabled(idx + 1 < step_len, egui::Button::new("▼"))
                                .clicked()
                            {
                                to_down = Some(idx);
                            }
                            if ui.button("🗑").clicked() {
                                to_remove = Some(idx);
                            }
                        });
                    });
                ui.add_space(4.0);
            }
        });

    if let Some(i) = to_up {
        cfg.move_up(i);
    }
    if let Some(i) = to_down {
        cfg.move_down(i);
    }
    if let Some(i) = to_remove {
        cfg.remove_step(i);
    }
}

fn render_shading_body(ui: &mut egui::Ui, cfg: &mut ShadingBlockConfig) {
    ui.columns(2, |cols| {
        egui::Frame::NONE
            .fill(CARD_BG)
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::same(12))
            .show(&mut cols[0], |ui| {
                ui.label(
                    RichText::new(tr("BLIND TYPE & TRAVEL TIMING"))
                        .size(11.0)
                        .strong()
                        .color(LOXONE_GREEN),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(tr("Shading Type (Type):"));
                    ComboBox::from_id_salt("shading_type_combo")
                        .selected_text(match cfg.blind_type {
                            0 => tr("0 — Venetian Blind (Jalousie)"),
                            1 => tr("1 — Roller Shutter (Roleta)"),
                            2 => tr("2 — Awning / Sunblind"),
                            3 => tr("3 — Roof Window Blind"),
                            4 => tr("4 — Curtain"),
                            _ => tr("Custom"),
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut cfg.blind_type,
                                0,
                                tr("0 — Venetian Blind (Jalousie)"),
                            );
                            ui.selectable_value(
                                &mut cfg.blind_type,
                                1,
                                tr("1 — Roller Shutter (Roleta)"),
                            );
                            ui.selectable_value(
                                &mut cfg.blind_type,
                                2,
                                tr("2 — Awning / Sunblind"),
                            );
                            ui.selectable_value(
                                &mut cfg.blind_type,
                                3,
                                tr("3 — Roof Window Blind"),
                            );
                            ui.selectable_value(&mut cfg.blind_type, 4, tr("4 — Curtain"));
                        });
                });
                ui.add_space(6.0);
                egui::Grid::new("shading_timing_grid")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(tr("Full Up Time (TimeEnd):"));
                        ui.add(
                            DragValue::new(&mut cfg.time_up)
                                .speed(0.1)
                                .range(1.0..=300.0)
                                .suffix(" s"),
                        );
                        ui.end_row();

                        ui.label(tr("Full Down Time (TimeEndDown):"));
                        ui.add(
                            DragValue::new(&mut cfg.time_down)
                                .speed(0.1)
                                .range(1.0..=300.0)
                                .suffix(" s"),
                        );
                        ui.end_row();

                        ui.label(tr("Slat Turn Offset (TurnOffset):"));
                        ui.add(
                            DragValue::new(&mut cfg.turn_offset)
                                .speed(0.01)
                                .range(0.0..=5.0)
                                .suffix(" s"),
                        );
                        ui.end_row();

                        ui.label(tr("Min Move Duration (MinMove):"));
                        ui.add(
                            DragValue::new(&mut cfg.min_move)
                                .speed(0.05)
                                .range(0.0..=10.0)
                                .suffix(" s"),
                        );
                        ui.end_row();

                        ui.label(tr("Reverse Slat Time (Back):"));
                        ui.add(
                            DragValue::new(&mut cfg.back_turn)
                                .speed(0.05)
                                .range(0.0..=10.0)
                                .suffix(" s"),
                        );
                        ui.end_row();

                        ui.label(tr("Slat Width (Width):"));
                        ui.add(
                            DragValue::new(&mut cfg.slat_width_mm)
                                .range(10..=200)
                                .suffix(" mm"),
                        );
                        ui.end_row();

                        ui.label(tr("Slat Spacing (Space):"));
                        ui.add(
                            DragValue::new(&mut cfg.slat_space_mm)
                                .range(10..=200)
                                .suffix(" mm"),
                        );
                        ui.end_row();
                    });
            });

        egui::Frame::NONE
            .fill(CARD_BG)
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::same(12))
            .show(&mut cols[1], |ui| {
                ui.label(
                    RichText::new(tr("SOLAR SHADING & COMPASS ORIENTATION"))
                        .size(11.0)
                        .strong()
                        .color(LOXONE_GREEN),
                );
                ui.add_space(6.0);
                ui.checkbox(
                    &mut cfg.sun_automation,
                    tr("Enable Solar Shading Automation (Sun)"),
                );
                ui.add_space(6.0);
                egui::Grid::new("shading_sun_grid")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(tr("Compass Direction (Dir 0..15):"));
                        ui.add(DragValue::new(&mut cfg.compass_dir).range(0..=360));
                        ui.end_row();

                        ui.label(tr("Sun Entry Tolerance (DirTol):"));
                        ui.add(
                            DragValue::new(&mut cfg.dir_tol_start)
                                .speed(1.0)
                                .range(0.0..=180.0)
                                .suffix(" °"),
                        );
                        ui.end_row();

                        ui.label(tr("Sun Exit Tolerance (DirTol2):"));
                        ui.add(
                            DragValue::new(&mut cfg.dir_tol_end)
                                .speed(1.0)
                                .range(0.0..=180.0)
                                .suffix(" °"),
                        );
                        ui.end_row();

                        ui.label(tr("Auto Mode (AutMode):"));
                        ui.add(DragValue::new(&mut cfg.auto_mode).range(0..=3));
                        ui.end_row();

                        ui.label(tr("End of Shading Action (AutoShadeEnd):"));
                        ui.add(DragValue::new(&mut cfg.auto_shade_end).range(0..=3));
                        ui.end_row();

                        ui.label(tr("Shading Delay Time (AutoShadeTime):"));
                        ui.add(
                            DragValue::new(&mut cfg.auto_shade_time_min)
                                .range(0..=600)
                                .suffix(" min"),
                        );
                        ui.end_row();

                        ui.label(tr("Sunrise Offset (SRoff):"));
                        ui.add(
                            DragValue::new(&mut cfg.sunrise_offset_min)
                                .range(-180..=180)
                                .suffix(" min"),
                        );
                        ui.end_row();

                        ui.label(tr("Sunset Offset (SSoff):"));
                        ui.add(
                            DragValue::new(&mut cfg.sunset_offset_min)
                                .range(-180..=180)
                                .suffix(" min"),
                        );
                        ui.end_row();
                    });
            });
    });
}

fn render_access_body(ui: &mut egui::Ui, cfg: &mut AccessBlockConfig) {
    ui.columns(2, |cols| {
        egui::Frame::NONE
            .fill(CARD_BG)
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::same(12))
            .show(&mut cols[0], |ui| {
                ui.label(
                    RichText::new(tr("OUTPUT LABELS & QUICK ACTIONS"))
                        .size(11.0)
                        .strong()
                        .color(LOXONE_GREEN),
                );
                ui.add_space(6.0);
                if cfg.block_type == "NfcCodeTouch" {
                    egui::Grid::new("nfc_labels_grid")
                        .num_columns(2)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            for i in 0..6 {
                                ui.label(format!("Output Q{}:", i + 1));
                                ui.add(
                                    TextEdit::singleline(&mut cfg.nfc_output_labels[i])
                                        .desired_width(220.0),
                                );
                                ui.end_row();
                            }
                        });
                } else {
                    egui::Grid::new("door_out_names_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label(tr("Output 1 Label (OutName1):"));
                            ui.add(
                                TextEdit::singleline(&mut cfg.out_name_1).desired_width(220.0),
                            );
                            ui.end_row();

                            ui.label(tr("Output 2 Label (OutName2):"));
                            ui.add(
                                TextEdit::singleline(&mut cfg.out_name_2).desired_width(220.0),
                            );
                            ui.end_row();

                            ui.label(tr("Output 3 Label (OutName3):"));
                            ui.add(
                                TextEdit::singleline(&mut cfg.out_name_3).desired_width(220.0),
                            );
                            ui.end_row();
                        });
                    if cfg.block_type == "Intercom" {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(tr("Intercom Voice Answers (ans, semicolon-separated):"))
                                .size(11.0)
                                .color(TEXT_MUTED),
                        );
                        ui.add(
                            TextEdit::multiline(&mut cfg.intercom_answers)
                                .desired_rows(3)
                                .desired_width(340.0),
                        );
                    }
                }
            });

        egui::Frame::NONE
            .fill(CARD_BG)
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::same(12))
            .show(&mut cols[1], |ui| {
                ui.label(
                    RichText::new(tr("PULSE TIMING & HARDWARE PARAMETERS"))
                        .size(11.0)
                        .strong()
                        .color(LOXONE_GREEN),
                );
                ui.add_space(6.0);
                egui::Grid::new("access_params_grid")
                    .num_columns(2)
                    .spacing([16.0, 8.0])
                    .show(ui, |ui| {
                        match cfg.block_type.as_str() {
                            "NfcCodeTouch" => {
                                ui.label(tr("Pulse Duration (PulseTime):"));
                                ui.add(
                                    DragValue::new(&mut cfg.pulse_time_sec)
                                        .speed(0.5)
                                        .range(0.5..=120.0)
                                        .suffix(" s"),
                                );
                                ui.end_row();

                                ui.label(tr("LED Brightness (Lbr):"));
                                ui.add(
                                    DragValue::new(&mut cfg.led_brightness)
                                        .range(0..=100)
                                        .suffix(" %"),
                                );
                                ui.end_row();

                                ui.label(tr("Acoustic Beep (Au):"));
                                ui.add(DragValue::new(&mut cfg.acoustic_feedback).range(0..=1));
                                ui.end_row();

                                ui.label(tr("History Size (HSize):"));
                                ui.add(DragValue::new(&mut cfg.history_size).range(1..=100));
                                ui.end_row();
                            }
                            "Door" | "Gate" => {
                                ui.label(tr("Controller Type (CtrlType):"));
                                ui.add(DragValue::new(&mut cfg.ctrl_type).range(0..=5));
                                ui.end_row();

                                ui.label(tr("Open Pulse Duration (PulseOpen):"));
                                ui.add(
                                    DragValue::new(&mut cfg.pulse_open_sec)
                                        .speed(0.5)
                                        .range(0.1..=300.0)
                                        .suffix(" s"),
                                );
                                ui.end_row();

                                ui.label(tr("Close Pulse Duration (PulseClose):"));
                                ui.add(
                                    DragValue::new(&mut cfg.pulse_close_sec)
                                        .speed(0.5)
                                        .range(0.1..=300.0)
                                        .suffix(" s"),
                                );
                                ui.end_row();
                            }
                            _ => {
                                ui.label(tr("Door Number (DoorNumber):"));
                                ui.add(DragValue::new(&mut cfg.door_number).range(1..=16));
                                ui.end_row();

                                ui.label(tr("Call / Ring Timeout (Timeout):"));
                                ui.add(
                                    DragValue::new(&mut cfg.timeout_sec)
                                        .speed(1.0)
                                        .range(5.0..=300.0)
                                        .suffix(" s"),
                                );
                                ui.end_row();

                                if cfg.block_type == "Intercom" {
                                    ui.label(tr("LED Brightness (Lbr):"));
                                    ui.add(
                                        DragValue::new(&mut cfg.led_brightness)
                                            .range(0..=100)
                                            .suffix(" %"),
                                    );
                                    ui.end_row();

                                    ui.label(tr("Proximity Sensor (Tp):"));
                                    ui.add(DragValue::new(&mut cfg.acoustic_feedback).range(0..=1));
                                    ui.end_row();
                                }
                            }
                        }
                    });
            });
    });
}

fn render_presence_body(
    ui: &mut egui::Ui,
    cfg: &mut PresenceBlockConfig,
    project: &LoxoneProject,
    search_filter: &mut String,
) {
    let sensors = collect_presence_sensor_candidates(project);
    ui.columns(2, |cols| {
        egui::Frame::NONE
            .fill(CARD_BG)
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::same(12))
            .show(&mut cols[0], |ui| {
                ui.label(
                    RichText::new(tr("PRESENCE TIMING & TRACKING"))
                        .size(11.0)
                        .strong()
                        .color(LOXONE_GREEN),
                );
                ui.add_space(8.0);
                egui::Grid::new("presence_timing_grid")
                    .num_columns(2)
                    .spacing([16.0, 10.0])
                    .show(ui, |ui| {
                        ui.label(tr("Hold / Overrun Time (ParamTOn):"));
                        ui.add(
                            DragValue::new(&mut cfg.hold_time_sec)
                                .speed(5.0)
                                .range(1.0..=7200.0)
                                .suffix(" s"),
                        );
                        ui.end_row();

                        ui.label(tr("Pre-Off Warning Time (ParamTWarn):"));
                        ui.add(
                            DragValue::new(&mut cfg.warn_time_sec)
                                .speed(1.0)
                                .range(0.0..=600.0)
                                .suffix(" s"),
                        );
                        ui.end_row();

                        ui.label(tr("History Events (TRK_NE):"));
                        ui.add(DragValue::new(&mut cfg.track_ne).range(0..=500));
                        ui.end_row();

                        ui.label(tr("Max Tracked Entries (TRK_MAX):"));
                        ui.add(DragValue::new(&mut cfg.track_max).range(0..=500));
                        ui.end_row();
                    });
            });

        egui::Frame::NONE
            .fill(CARD_BG)
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::same(12))
            .show(&mut cols[1], |ui| {
                ui.label(
                    RichText::new(format!(
                        "{} ({}/{})",
                        tr("ASSIGNED PRESENCE SENSORS (DEVS)"),
                        cfg.assigned_device_uuids.len(),
                        sensors.len()
                    ))
                    .size(11.0)
                    .strong()
                    .color(LOXONE_GREEN),
                );
                ui.add_space(6.0);
                ui.add(
                    TextEdit::singleline(search_filter)
                        .hint_text(tr("🔍 Filter presence sensors..."))
                        .desired_width(240.0),
                );
                ui.add_space(6.0);
                let q = search_filter.trim().to_lowercase();
                ScrollArea::vertical()
                    .id_salt("presence_sensors_scroll")
                    .max_height(310.0)
                    .show(ui, |ui| {
                        for sensor in &sensors {
                            if !q.is_empty()
                                && !sensor.title.to_lowercase().contains(&q)
                                && !sensor.parent_title.to_lowercase().contains(&q)
                                && !sensor.serial.to_lowercase().contains(&q)
                            {
                                continue;
                            }
                            let mut assigned = cfg.is_device_assigned(&sensor.uuid);
                            ui.horizontal(|ui| {
                                if ui.checkbox(&mut assigned, "").changed() {
                                    cfg.toggle_device(&sensor.uuid);
                                }
                                ui.label(
                                    RichText::new(&sensor.title)
                                        .size(12.0)
                                        .strong()
                                        .color(TEXT_PRIMARY),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "({} • {})",
                                        sensor.parent_title, sensor.serial
                                    ))
                                    .size(10.5)
                                    .color(TEXT_MUTED),
                                );
                            });
                        }
                    });
            });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use loxone_core::parser::load_loxone_file;
    use std::path::PathBuf;

    #[test]
    fn test_advanced_blocks_modal_opens_renders_and_closes_on_outside_click() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Sample.Loxone");
        let proj = load_loxone_file(&path).expect("load Sample.Loxone");

        let mut state = AdvancedBlocksModalState::default();
        assert!(state.open_for_block(&proj, "19eb19aa-03b9-3ff5-ffff51535bcccc04"));
        assert!(state.is_open);

        let ctx = Context::default();
        let mut raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1440.0, 900.0),
            )),
            ..Default::default()
        };

        let _ = ctx.run(raw.clone(), |ctx| {
            let _ = render_advanced_blocks_modal(ctx, &mut state, Some(&proj));
        });
        assert!(state.is_open);

        // Click outside at (20, 20)
        raw.events = vec![
            egui::Event::PointerMoved(egui::pos2(20.0, 20.0)),
            egui::Event::PointerButton {
                pos: egui::pos2(20.0, 20.0),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ];
        let _ = ctx.run(raw.clone(), |ctx| {
            let _ = render_advanced_blocks_modal(ctx, &mut state, Some(&proj));
        });
        raw.events = vec![egui::Event::PointerButton {
            pos: egui::pos2(20.0, 20.0),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }];
        let _ = ctx.run(raw, |ctx| {
            let _ = render_advanced_blocks_modal(ctx, &mut state, Some(&proj));
        });
        assert!(
            !state.is_open,
            "Clicking outside the advanced blocks modal must close it"
        );

        // Verify Shading (AutoJalousie), Access (Intercom, NfcCodeTouch), and Presence (PresenceDetector) modals open and render
        for btype in ["AutoJalousie", "Intercom", "NfcCodeTouch", "PresenceDetector"] {
            let uuid = proj
                .pages
                .iter()
                .flat_map(|p| &p.blocks)
                .find(|b| b.block_type == btype)
                .map(|b| b.uuid.clone())
                .expect("find block type in Sample.Loxone");
            assert!(state.open_for_block(&proj, &uuid), "should open for {btype}");
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                let _ = render_advanced_blocks_modal(ctx, &mut state, Some(&proj));
            });
            state.close();
        }
    }
}
