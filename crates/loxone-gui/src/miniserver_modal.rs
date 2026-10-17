use egui::{Align2, Color32, Context, Frame, Id, Key, Margin, Order, RichText, ScrollArea, Stroke, Vec2};
use loxone_core::merge::ServerVersionConflict;
use loxone_core::miniserver::{
    BackupSnapshot, ConnectionProfile, ConnectionStatus, MiniserverCredentials,
};
use loxone_core::model::LoxoneProject;

use crate::theme::{ACCENT_BLUE, BORDER_SUBTLE, CARD_BG, PANEL_BG, TEXT_MUTED, TEXT_PRIMARY};
use crate::top_bar::{paint_status_indicator, status_badge_visuals};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MiniserverModalAction {
    TestConnection(MiniserverCredentials),
    RefreshBackups(MiniserverCredentials),
    DownloadLatest(MiniserverCredentials),
    DownloadSnapshot {
        creds: MiniserverCredentials,
        filename: String,
    },
    UploadProject {
        creds: MiniserverCredentials,
        restart_sps: bool,
    },
    PullAndMergeServerConflict,
    Close,
}

#[derive(Debug, Clone)]
pub struct MiniserverModalState {
    pub is_open: bool,
    pub host: String,
    pub username: String,
    pub password: String,
    pub remember_password: bool,
    pub show_backups_browser: bool,
    pub show_upload_panel: bool,
    pub confirm_upload: bool,
    pub restart_sps_after_upload: bool,
    pub snapshots: Vec<BackupSnapshot>,
    pub status_message: Option<String>,
    pub pending_conflict: Option<ServerVersionConflict>,
}

impl Default for MiniserverModalState {
    fn default() -> Self {
        Self {
            is_open: false,
            host: String::new(),
            username: "admin".to_string(),
            password: String::new(),
            remember_password: false,
            show_backups_browser: false,
            show_upload_panel: false,
            confirm_upload: false,
            restart_sps_after_upload: true,
            snapshots: Vec::new(),
            status_message: None,
            pending_conflict: None,
        }
    }
}

impl MiniserverModalState {
    pub fn from_profile(profile: &ConnectionProfile) -> Self {
        Self {
            is_open: false,
            host: profile.host.clone(),
            username: if profile.username.trim().is_empty() {
                "admin".to_string()
            } else {
                profile.username.clone()
            },
            password: profile.password.clone().unwrap_or_default(),
            remember_password: profile.remember_password,
            show_backups_browser: false,
            show_upload_panel: false,
            confirm_upload: false,
            restart_sps_after_upload: true,
            snapshots: Vec::new(),
            status_message: None,
            pending_conflict: None,
        }
    }

    pub fn apply_project_fallback_host(&mut self, project: Option<&LoxoneProject>) {
        if !self.host.trim().is_empty() {
            return;
        }
        if let Some(proj) = project {
            if let Some(ip_or_host) = proj
                .metadata
                .miniserver_ip
                .as_deref()
                .or(proj.metadata.miniserver_hostname.as_deref())
            {
                self.host = ip_or_host.to_string();
            }
        }
    }

    pub fn credentials(&self) -> MiniserverCredentials {
        MiniserverCredentials {
            host: self.host.trim().to_string(),
            username: self.username.trim().to_string(),
            password: self.password.clone(),
        }
    }

    pub fn to_profile(&self) -> ConnectionProfile {
        ConnectionProfile {
            host: self.host.trim().to_string(),
            username: self.username.trim().to_string(),
            password: if self.remember_password {
                Some(self.password.clone())
            } else {
                None
            },
            remember_password: self.remember_password,
        }
    }
}

pub fn render_miniserver_modal(
    ctx: &Context,
    state: &mut MiniserverModalState,
    connection_status: &ConnectionStatus,
) -> Option<MiniserverModalAction> {
    if !state.is_open {
        return None;
    }

    if ctx.input(|i| i.key_pressed(Key::Escape)) {
        return Some(MiniserverModalAction::Close);
    }

    let mut action = None;
    let is_busy = connection_status.is_connecting();
    let can_submit = !state.host.trim().is_empty() && !is_busy;
    let badge = status_badge_visuals(connection_status, Some(&state.host));

    // Subtle backdrop on Order::Middle (strictly behind Order::Foreground modal)
    let screen_rect = ctx.screen_rect();
    egui::Area::new(Id::new("miniserver_modal_backdrop"))
        .order(Order::Middle)
        .fixed_pos(screen_rect.min)
        .show(ctx, |ui| {
            let _ = ui.allocate_rect(screen_rect, egui::Sense::click());
            ui.painter().rect_filled(
                screen_rect,
                0.0,
                Color32::from_black_alpha(55),
            );
        });

    egui::Area::new(Id::new("miniserver_modal_window"))
        .order(Order::Foreground)
        .anchor(Align2::CENTER_TOP, Vec2::new(0.0, 68.0))
        .show(ctx, |ui| {
            Frame::new()
                .fill(PANEL_BG)
                .stroke(Stroke::new(1.5_f32, ACCENT_BLUE))
                .shadow(egui::Shadow {
                    offset: [0, 8],
                    blur: 24,
                    spread: 0,
                    color: Color32::from_black_alpha(100),
                })
                .corner_radius(10)
                .inner_margin(Margin::same(16))
                .show(ui, |ui| {
                    ui.set_width(540.0);

                    // Header row: Title + Status Badge + Close button
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Miniserver Connection & Project Download")
                                .size(14.0)
                                .color(TEXT_PRIMARY)
                                .strong(),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .button(RichText::new("✕").color(TEXT_MUTED))
                                .on_hover_text("Close (Esc)")
                                .clicked()
                            {
                                action = Some(MiniserverModalAction::Close);
                            }

                            Frame::new()
                                .fill(CARD_BG)
                                .stroke(Stroke::new(
                                    1.0_f32,
                                    badge.dot_color.gamma_multiply(0.65),
                                ))
                                .corner_radius(6)
                                .inner_margin(Margin::symmetric(8, 3))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 5.0;
                                        paint_status_indicator(ui, &badge);
                                        ui.label(
                                            RichText::new(&badge.label)
                                                .size(11.0)
                                                .color(TEXT_PRIMARY),
                                        );
                                    });
                                })
                                .response
                                .on_hover_text(&badge.tooltip);
                        });
                    });

                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Credentials grid
                    egui::Grid::new("miniserver_creds_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label(RichText::new("Host / IP").color(TEXT_MUTED));
                            ui.add(
                                egui::TextEdit::singleline(&mut state.host)
                                    .hint_text("192.168.1.100 or http://miniserwer:80")
                                    .desired_width(400.0),
                            );
                            ui.end_row();

                            ui.label(RichText::new("Username").color(TEXT_MUTED));
                            ui.add(
                                egui::TextEdit::singleline(&mut state.username)
                                    .hint_text("admin")
                                    .desired_width(400.0),
                            );
                            ui.end_row();

                            ui.label(RichText::new("Password").color(TEXT_MUTED));
                            ui.add(
                                egui::TextEdit::singleline(&mut state.password)
                                    .password(true)
                                    .hint_text("Miniserver password")
                                    .desired_width(400.0),
                            );
                            ui.end_row();
                        });

                    ui.add_space(6.0);
                    ui.checkbox(
                        &mut state.remember_password,
                        RichText::new("Remember password on this computer").size(11.5).color(TEXT_MUTED),
                    );

                    if let Some(msg) = &state.status_message {
                        ui.add_space(6.0);
                        Frame::new()
                            .fill(CARD_BG)
                            .stroke(Stroke::new(1.0_f32, badge.dot_color.gamma_multiply(0.55)))
                            .corner_radius(6)
                            .inner_margin(Margin::symmetric(10, 6))
                            .show(ui, |ui| {
                                ui.set_width(512.0);
                                ui.label(RichText::new(msg).size(11.5).color(TEXT_PRIMARY));
                            });
                    }

                    ui.add_space(10.0);

                    // Primary Actions Row
                    ui.horizontal(|ui| {
                        let test_btn = egui::Button::new(
                            RichText::new("🔌 Test Connection")
                                .size(12.0)
                                .color(TEXT_PRIMARY),
                        );
                        if ui
                            .add_enabled(can_submit, test_btn)
                            .on_hover_text("Verify credentials and check /prog/ backups")
                            .clicked()
                        {
                            action = Some(MiniserverModalAction::TestConnection(
                                state.credentials(),
                            ));
                        }

                        let dl_latest_btn = egui::Button::new(
                            RichText::new("⬇ Download Latest Project")
                                .size(12.0)
                                .color(ACCENT_BLUE)
                                .strong(),
                        );
                        if ui
                            .add_enabled(can_submit, dl_latest_btn)
                            .on_hover_text(
                                "Download newest sps_<ver>_<timestamp>.zip from /prog/ and open in viewer",
                            )
                            .clicked()
                        {
                            action = Some(MiniserverModalAction::DownloadLatest(
                                state.credentials(),
                            ));
                        }

                        let browse_label = if state.show_backups_browser {
                            "▾ Hide Backups"
                        } else {
                            "▸ Browse Backups…"
                        };
                        if ui
                            .add_enabled(
                                can_submit,
                                egui::Button::new(
                                    RichText::new(browse_label).size(11.5).color(TEXT_MUTED),
                                ),
                            )
                            .clicked()
                        {
                            state.show_backups_browser = !state.show_backups_browser;
                            if state.show_backups_browser && state.snapshots.is_empty() {
                                action = Some(MiniserverModalAction::RefreshBackups(
                                    state.credentials(),
                                ));
                            }
                        }

                        let upload_label = if state.show_upload_panel {
                            "▾ Hide Upload"
                        } else {
                            "⬆ Save to Miniserver…"
                        };
                        if ui
                            .add_enabled(
                                can_submit,
                                egui::Button::new(
                                    RichText::new(upload_label)
                                        .size(11.5)
                                        .color(Color32::from_rgb(232, 168, 56))
                                        .strong(),
                                ),
                            )
                            .on_hover_text(
                                "Safely back up active Miniserver program locally, then upload current project (.LoxCC + sps_new.zip)",
                            )
                            .clicked()
                        {
                            state.show_upload_panel = !state.show_upload_panel;
                            if !state.show_upload_panel {
                                state.confirm_upload = false;
                            }
                        }
                    });

                    // Newer Server Version Safeguard Card
                    if let Some(conflict) = state.pending_conflict.clone() {
                        ui.add_space(10.0);
                        let warn_amber = Color32::from_rgb(240, 128, 60);
                        Frame::new()
                            .fill(Color32::from_rgb(44, 28, 22))
                            .stroke(Stroke::new(1.5_f32, warn_amber))
                            .corner_radius(6)
                            .inner_margin(Margin::symmetric(10, 8))
                            .show(ui, |ui| {
                                ui.set_width(512.0);
                                ui.label(
                                    RichText::new("⚠ Newer Version Detected on Miniserver — Upload Blocked")
                                        .size(12.5)
                                        .color(warn_amber)
                                        .strong(),
                                );
                                ui.add_space(3.0);
                                ui.label(
                                    RichText::new(&conflict.diff.reason)
                                        .size(11.5)
                                        .color(TEXT_PRIMARY)
                                        .strong(),
                                );
                                ui.add_space(3.0);
                                let remote_sl = conflict
                                    .diff
                                    .remote_save_level
                                    .map(|v| format!("SL={}", v))
                                    .unwrap_or_else(|| "SL=n/a".to_string());
                                let local_sl = conflict
                                    .diff
                                    .local_save_level
                                    .map(|v| format!("SL={}", v))
                                    .unwrap_or_else(|| "SL=n/a".to_string());
                                ui.label(
                                    RichText::new(format!(
                                        "Server: {} • {} ({})   |   Local: {} • {}",
                                        remote_sl,
                                        if conflict.diff.remote_c_date.is_empty() {
                                            "no CDate"
                                        } else {
                                            &conflict.diff.remote_c_date
                                        },
                                        conflict.remote_filename,
                                        local_sl,
                                        if conflict.diff.local_c_date.is_empty() {
                                            "no CDate"
                                        } else {
                                            &conflict.diff.local_c_date
                                        },
                                    ))
                                    .size(11.0)
                                    .color(TEXT_MUTED),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "Diff summary: {} server block(s) added/modified, {} removed, {} tree item(s) changed, {} page(s) changed • {} local edit(s)",
                                        conflict.diff.remote_modified_or_added_blocks,
                                        conflict.diff.remote_removed_blocks,
                                        conflict.diff.remote_modified_or_added_tree_nodes,
                                        conflict.diff.remote_modified_or_added_pages,
                                        conflict.diff.local_unmerged_edits,
                                    ))
                                    .size(11.0)
                                    .color(TEXT_MUTED),
                                );
                                if !conflict.diff.change_highlights.is_empty() {
                                    ui.add_space(3.0);
                                    for item in conflict.diff.change_highlights.iter().take(5) {
                                        ui.label(
                                            RichText::new(format!(" • {}", item))
                                                .size(10.5)
                                                .color(TEXT_PRIMARY),
                                        );
                                    }
                                }
                                ui.add_space(5.0);
                                ui.label(
                                    RichText::new(
                                        "Please pull the difference from the Miniserver into your project before uploading. What is on the server takes higher priority on any conflict, while preserving your non-conflicting local additions.",
                                    )
                                    .size(11.0)
                                    .color(TEXT_MUTED),
                                );
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    if ui
                                        .button(
                                            RichText::new("⬇ Pull & Merge into My Project (Server Priority)")
                                                .size(12.0)
                                                .color(ACCENT_BLUE)
                                                .strong(),
                                        )
                                        .on_hover_text(
                                            "Merge the newer Miniserver version into your current project with Server Priority",
                                        )
                                        .clicked()
                                    {
                                        action = Some(MiniserverModalAction::PullAndMergeServerConflict);
                                    }
                                    if ui
                                        .button(
                                            RichText::new("Dismiss")
                                                .size(11.0)
                                                .color(TEXT_MUTED),
                                        )
                                        .clicked()
                                    {
                                        state.pending_conflict = None;
                                    }
                                });
                            });
                    }

                    // Safe Hardware Deployment (Upload to Miniserver) Section
                    if state.show_upload_panel {
                        ui.add_space(10.0);
                        Frame::new()
                            .fill(CARD_BG)
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(232, 168, 56)))
                            .corner_radius(6)
                            .inner_margin(Margin::symmetric(10, 8))
                            .show(ui, |ui| {
                                ui.set_width(512.0);
                                ui.label(
                                    RichText::new("🛡 Safe Hardware Deployment (Save to Miniserver)")
                                        .size(12.0)
                                        .color(Color32::from_rgb(232, 168, 56))
                                        .strong(),
                                );
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(
                                        "1. Checks if the Miniserver already has a newer version and safeguards against accidental overwrite.\n2. Automatically saves a local safety backup of the running Miniserver program to ~/.config/loxone-portable/backups/.\n3. Compresses the active project into .LoxCC and uploads sps_new.zip to /prog/.",
                                    )
                                    .size(11.0)
                                    .color(TEXT_MUTED),
                                );
                                ui.add_space(6.0);
                                ui.checkbox(
                                    &mut state.restart_sps_after_upload,
                                    RichText::new("Restart Miniserver SPS (/dev/sps/restart) after upload")
                                        .size(11.5)
                                        .color(TEXT_PRIMARY),
                                );
                                ui.checkbox(
                                    &mut state.confirm_upload,
                                    RichText::new("I understand this will update the live Miniserver program")
                                        .size(11.5)
                                        .color(TEXT_PRIMARY),
                                );
                                ui.add_space(6.0);
                                if ui
                                    .add_enabled(
                                        can_submit && state.confirm_upload,
                                        egui::Button::new(
                                            RichText::new("⬆ Backup & Upload Project Now")
                                                .size(12.0)
                                                .color(Color32::from_rgb(232, 168, 56))
                                                .strong(),
                                        ),
                                    )
                                    .clicked()
                                {
                                    state.confirm_upload = false;
                                    action = Some(MiniserverModalAction::UploadProject {
                                        creds: state.credentials(),
                                        restart_sps: state.restart_sps_after_upload,
                                    });
                                }
                            });
                    }

                    // Expandable Backups Browser
                    if state.show_backups_browser {
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "Backups in /prog/ ({})",
                                    state.snapshots.len()
                                ))
                                .size(11.5)
                                .color(TEXT_MUTED)
                                .strong(),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .add_enabled(
                                            can_submit,
                                            egui::Button::new(
                                                RichText::new("↻ Refresh")
                                                    .size(11.0)
                                                    .color(TEXT_PRIMARY),
                                            ),
                                        )
                                        .clicked()
                                    {
                                        action = Some(MiniserverModalAction::RefreshBackups(
                                            state.credentials(),
                                        ));
                                    }
                                },
                            );
                        });

                        ui.add_space(4.0);
                        ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                            if state.snapshots.is_empty() {
                                ui.label(
                                    RichText::new(
                                        "No backups loaded yet. Click 'Refresh' or 'Test Connection' to list /prog/.",
                                    )
                                    .size(11.5)
                                    .color(TEXT_MUTED),
                                );
                            }
                            for snap in &state.snapshots {
                                Frame::new()
                                    .fill(CARD_BG)
                                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                                    .corner_radius(6)
                                    .inner_margin(Margin::symmetric(10, 6))
                                    .show(ui, |ui| {
                                        ui.set_width(508.0);
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.label(
                                                    RichText::new(&snap.filename)
                                                        .size(12.0)
                                                        .color(TEXT_PRIMARY)
                                                        .strong(),
                                                );
                                                let size_kb = snap
                                                    .size_bytes
                                                    .map(|b| format!(" • {} KB", b / 1024))
                                                    .unwrap_or_default();
                                                let ver_str = snap
                                                    .config_version
                                                    .map(|v| format!("v{} • ", v))
                                                    .unwrap_or_default();
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{}{}{}",
                                                        ver_str, snap.formatted_date, size_kb
                                                    ))
                                                    .size(11.0)
                                                    .color(TEXT_MUTED),
                                                );
                                            });
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui
                                                        .add_enabled(
                                                            can_submit,
                                                            egui::Button::new(
                                                                RichText::new("⬇ Download")
                                                                    .size(11.5)
                                                                    .color(ACCENT_BLUE),
                                                            ),
                                                        )
                                                        .clicked()
                                                    {
                                                        action = Some(
                                                            MiniserverModalAction::DownloadSnapshot {
                                                                creds: state.credentials(),
                                                                filename: snap.filename.clone(),
                                                            },
                                                        );
                                                    }
                                                },
                                            );
                                        });
                                    });
                                ui.add_space(4.0);
                            }
                        });
                    }
                });
        });

    action
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use loxone_core::parser::load_loxone_file;

    #[test]
    fn test_miniserver_modal_prefills_host_from_project_loxlive_and_renders() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should parse");

        let mut modal = MiniserverModalState::default();
        assert!(modal.host.is_empty());

        modal.apply_project_fallback_host(Some(&project));
        assert_eq!(modal.host, "192.168.1.100");

        // Does not overwrite an explicitly entered host
        modal.host = "10.0.0.50".to_string();
        modal.apply_project_fallback_host(Some(&project));
        assert_eq!(modal.host, "10.0.0.50");

        // Profile round-trip
        let prof = modal.to_profile();
        let restored = MiniserverModalState::from_profile(&prof);
        assert_eq!(restored.host, "10.0.0.50");

        modal.is_open = true;
        modal.show_backups_browser = true;
        modal.snapshots.push(BackupSnapshot {
            filename: "sps_194_20260308182256.zip".to_string(),
            config_version: Some(194),
            timestamp_raw: Some("20260308182256".to_string()),
            formatted_date: "2026-03-08 18:22:56".to_string(),
            size_bytes: Some(158_262),
        });

        let ctx = egui::Context::default();
        let status = ConnectionStatus::Connected {
            host: "10.0.0.50".to_string(),
            server_info: "1 backup(s) available".to_string(),
        };
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let _ = render_miniserver_modal(ctx, &mut modal, &status);
        });
    }
}
