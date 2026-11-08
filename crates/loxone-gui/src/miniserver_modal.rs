use egui::{Align2, Color32, Context, Frame, Id, Key, Margin, Order, RichText, ScrollArea, Stroke, Vec2};
use loxone_core::merge::ServerVersionConflict;
use loxone_core::miniserver::{
    default_cloud_external_address, format_serial_colon, BackupSnapshot, ConnectionProfile,
    ConnectionStatus, MiniserverCredentials, MiniserverKeychain, MiniserverSystemDiagnostics,
    SavedMiniserverEntry,
};
use loxone_core::model::LoxoneProject;

use crate::theme::{
    ACCENT_BLUE, BORDER_SUBTLE, CARD_BG, ELEVATED_BG, LOXONE_GREEN, PANEL_BG, TEXT_MUTED,
    TEXT_PRIMARY,
};
use crate::top_bar::{paint_status_indicator, status_badge_visuals_localized};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MiniserverModalTab {
    #[default]
    ConnectManually,
    MyMiniservers,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MiniserverModalAction {
    TestConnection(MiniserverCredentials),
    RefreshBackups(MiniserverCredentials),
    FetchDiagnostics(MiniserverCredentials),
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
    pub active_tab: MiniserverModalTab,
    pub keychain: MiniserverKeychain,
    pub selected_keychain_idx: Option<usize>,
    pub host: String,
    pub username: String,
    pub password: String,
    pub remember_password: bool,
    pub show_backups_browser: bool,
    pub show_upload_panel: bool,
    pub show_diagnostics_panel: bool,
    pub confirm_upload: bool,
    pub restart_sps_after_upload: bool,
    pub snapshots: Vec<BackupSnapshot>,
    pub diagnostics: Option<MiniserverSystemDiagnostics>,
    pub diagnostics_filter: String,
    pub status_message: Option<String>,
    pub pending_conflict: Option<ServerVersionConflict>,
}

impl Default for MiniserverModalState {
    fn default() -> Self {
        let keychain = MiniserverKeychain::default_config_path()
            .and_then(|p| MiniserverKeychain::load_from_path(&p).ok())
            .unwrap_or_default();
        let selected_keychain_idx = (!keychain.entries.is_empty()).then_some(0);
        Self {
            is_open: false,
            active_tab: MiniserverModalTab::ConnectManually,
            keychain,
            selected_keychain_idx,
            host: String::new(),
            username: "admin".to_string(),
            password: String::new(),
            remember_password: false,
            show_backups_browser: false,
            show_upload_panel: false,
            show_diagnostics_panel: false,
            confirm_upload: false,
            restart_sps_after_upload: true,
            snapshots: Vec::new(),
            diagnostics: None,
            diagnostics_filter: String::new(),
            status_message: None,
            pending_conflict: None,
        }
    }
}

impl MiniserverModalState {
    pub fn from_profile(profile: &ConnectionProfile) -> Self {
        let mut s = Self {
            host: profile.host.clone(),
            username: if profile.username.trim().is_empty() {
                "admin".to_string()
            } else {
                profile.username.clone()
            },
            password: profile.password.clone().unwrap_or_default(),
            remember_password: profile.remember_password,
            ..Self::default()
        };
        if !s.host.trim().is_empty() && s.keychain.entries.is_empty() {
            s.keychain.upsert_entry(SavedMiniserverEntry {
                name: "Miniserver".to_string(),
                project_name: String::new(),
                user: s.username.clone(),
                serial: String::new(),
                password: profile.password.clone(),
                internal_address: s.host.clone(),
                external_address: String::new(),
                save_locally: s.remember_password,
            });
            s.selected_keychain_idx = Some(0);
        }
        s
    }

    pub fn persist_keychain_best_effort(&self) {
        if let Some(path) = MiniserverKeychain::default_config_path() {
            let _ = self.keychain.save_to_path(&path);
        }
    }

    pub fn apply_project_fallback_host(&mut self, project: Option<&LoxoneProject>) {
        if let Some(proj) = project {
            if self.keychain.upsert_from_project(proj) {
                if self.selected_keychain_idx.is_none() && !self.keychain.entries.is_empty() {
                    self.selected_keychain_idx = Some(self.keychain.entries.len() - 1);
                }
                self.persist_keychain_best_effort();
            }
            if self.host.trim().is_empty() {
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
    }

    pub fn open_my_miniservers(&mut self, project: Option<&LoxoneProject>) {
        self.apply_project_fallback_host(project);
        self.active_tab = MiniserverModalTab::MyMiniservers;
        self.is_open = true;
    }

    pub fn open_manual_connect(&mut self, project: Option<&LoxoneProject>) {
        self.apply_project_fallback_host(project);
        self.active_tab = MiniserverModalTab::ConnectManually;
        self.is_open = true;
    }

    pub fn select_keychain_entry(&mut self, index: usize) {
        if let Some(entry) = self.keychain.entries.get(index) {
            self.selected_keychain_idx = Some(index);
            let pref = entry.preferred_host().to_string();
            if !pref.is_empty() {
                self.host = pref;
            }
            if !entry.user.trim().is_empty() {
                self.username = entry.user.trim().to_string();
            }
            if let Some(pw) = &entry.password {
                self.password = pw.clone();
            }
            self.remember_password = entry.save_locally;
        }
    }

    pub fn sync_active_credentials_to_keychain(&mut self, project: Option<&LoxoneProject>) {
        if self.host.trim().is_empty() {
            return;
        }
        let proj_title = project
            .map(|p| p.metadata.title.trim())
            .filter(|t| !t.is_empty())
            .unwrap_or("Miniserver")
            .to_string();
        let raw_serial = project
            .and_then(|p| p.metadata.miniserver_serial.as_deref())
            .unwrap_or_default();
        self.keychain.upsert_entry(SavedMiniserverEntry {
            name: proj_title.clone(),
            project_name: proj_title,
            user: self.username.trim().to_string(),
            serial: format_serial_colon(raw_serial),
            password: if self.remember_password && !self.password.is_empty() {
                Some(self.password.clone())
            } else {
                None
            },
            internal_address: self.host.trim().to_string(),
            external_address: default_cloud_external_address(raw_serial),
            save_locally: self.remember_password,
        });
        if self.selected_keychain_idx.is_none() && !self.keychain.entries.is_empty() {
            self.selected_keychain_idx = Some(0);
        }
        self.persist_keychain_best_effort();
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
    let badge = status_badge_visuals_localized(connection_status, Some(&state.host));

    let (backdrop_clicked, popup_was_open) =
        crate::theme::draw_modal_backdrop(ctx, "miniserver_modal_backdrop", Order::Middle, 55);

    let win_resp = egui::Area::new(Id::new("miniserver_modal_window"))
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
                    ui.set_width(580.0);

                    // Header row: Title + Status Badge + Close button
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(match state.active_tab {
                                MiniserverModalTab::ConnectManually => {
                                    crate::i18n::tr("Miniserver Connection & Project Download")
                                }
                                MiniserverModalTab::MyMiniservers => {
                                    crate::i18n::tr("My Miniservers")
                                }
                            })
                            .size(14.0)
                            .color(TEXT_PRIMARY)
                            .strong(),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .button(RichText::new("✕").color(TEXT_MUTED))
                                .on_hover_text(crate::i18n::tr("Close (Esc)"))
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
                    // Mode switcher pill bar: Connect to Miniserver | My Miniservers
                    Frame::new()
                        .fill(Color32::from_rgb(0x14, 0x16, 0x1A))
                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                        .corner_radius(6)
                        .inner_margin(Margin::symmetric(4, 3))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                for (tab, label) in [
                                    (
                                        MiniserverModalTab::ConnectManually,
                                        crate::i18n::tr("Connect to Miniserver"),
                                    ),
                                    (
                                        MiniserverModalTab::MyMiniservers,
                                        crate::i18n::tr("My Miniservers"),
                                    ),
                                ] {
                                    let sel = state.active_tab == tab;
                                    let btn = egui::Button::new(
                                        RichText::new(label)
                                            .size(11.8)
                                            .color(if sel {
                                                Color32::WHITE
                                            } else {
                                                TEXT_MUTED
                                            })
                                            .strong(),
                                    )
                                    .fill(if sel {
                                        ELEVATED_BG
                                    } else {
                                        Color32::TRANSPARENT
                                    })
                                    .stroke(if sel {
                                        Stroke::new(1.0_f32, LOXONE_GREEN.gamma_multiply(0.75))
                                    } else {
                                        Stroke::NONE
                                    })
                                    .corner_radius(4);
                                    if ui.add(btn).clicked() {
                                        state.active_tab = tab;
                                    }
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "{}: {}",
                                                crate::i18n::tr("Saved"),
                                                state.keychain.entries.len()
                                            ))
                                            .size(10.8)
                                            .color(TEXT_MUTED),
                                        );
                                    },
                                );
                            });
                        });

                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(8.0);

                    if state.active_tab == MiniserverModalTab::MyMiniservers {
                        if let Some(kc_act) = render_my_miniservers_panel(ui, state, can_submit) {
                            action = Some(kc_act);
                        }
                        return;
                    }

                    // Credentials grid
                    egui::Grid::new("miniserver_creds_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label(RichText::new(crate::i18n::tr("Host / IP")).color(TEXT_MUTED));
                            ui.add(
                                egui::TextEdit::singleline(&mut state.host)
                                    .hint_text(crate::i18n::tr(
                                        "192.168.1.100 or http://miniserwer:80",
                                    ))
                                    .desired_width(440.0),
                            );
                            ui.end_row();

                            ui.label(RichText::new(crate::i18n::tr("Username")).color(TEXT_MUTED));
                            ui.add(
                                egui::TextEdit::singleline(&mut state.username)
                                    .hint_text("admin")
                                    .desired_width(440.0),
                            );
                            ui.end_row();

                            ui.label(RichText::new(crate::i18n::tr("Password")).color(TEXT_MUTED));
                            ui.add(
                                egui::TextEdit::singleline(&mut state.password)
                                    .password(true)
                                    .hint_text(crate::i18n::tr("Miniserver password"))
                                    .desired_width(440.0),
                            );
                            ui.end_row();
                        });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.checkbox(
                            &mut state.remember_password,
                            RichText::new(crate::i18n::tr("Remember password on this computer"))
                                .size(11.5)
                                .color(TEXT_MUTED),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .small_button(
                                    RichText::new(crate::i18n::tr("💾 Save to My Miniservers"))
                                        .color(LOXONE_GREEN),
                                )
                                .clicked()
                                && !state.host.trim().is_empty()
                            {
                                state.sync_active_credentials_to_keychain(None);
                                state.status_message = Some(
                                    crate::i18n::tr("Saved to My Miniservers.").to_string(),
                                );
                            }
                        });
                    });

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
                            RichText::new(crate::i18n::tr("🔌 Test Connection"))
                                .size(12.0)
                                .color(TEXT_PRIMARY),
                        );
                        if ui
                            .add_enabled(can_submit, test_btn)
                            .on_hover_text(crate::i18n::tr(
                                "Verify credentials and check /prog/ backups",
                            ))
                            .clicked()
                        {
                            action = Some(MiniserverModalAction::TestConnection(
                                state.credentials(),
                            ));
                        }

                        let dl_latest_btn = egui::Button::new(
                            RichText::new(crate::i18n::tr("⬇ Download Latest Project"))
                                .size(12.0)
                                .color(ACCENT_BLUE)
                                .strong(),
                        );
                        if ui
                            .add_enabled(can_submit, dl_latest_btn)
                            .on_hover_text(crate::i18n::tr(
                                "Download newest sps_<ver>_<timestamp>.zip from /prog/ and open in viewer",
                            ))
                            .clicked()
                        {
                            action = Some(MiniserverModalAction::DownloadLatest(
                                state.credentials(),
                            ));
                        }

                        let browse_label = if state.show_backups_browser {
                            crate::i18n::tr("▾ Hide Backups")
                        } else {
                            crate::i18n::tr("▸ Browse Backups…")
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
                            crate::i18n::tr("▾ Hide Upload")
                        } else {
                            crate::i18n::tr("⬆ Save to Miniserver…")
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
                            .on_hover_text(crate::i18n::tr(
                                "Safely back up active Miniserver program locally, then upload current project (.LoxCC + sps_new.zip)",
                            ))
                            .clicked()
                        {
                            state.show_upload_panel = !state.show_upload_panel;
                            if !state.show_upload_panel {
                                state.confirm_upload = false;
                            }
                        }

                        let diag_label = if state.show_diagnostics_panel {
                            crate::i18n::tr("▾ Hide Diagnostics")
                        } else {
                            crate::i18n::tr("🩺 Diagnostics & Logs…")
                        };
                        if ui
                            .add_enabled(
                                can_submit,
                                egui::Button::new(
                                    RichText::new(diag_label).size(11.5).color(LOXONE_GREEN),
                                ),
                            )
                            .on_hover_text(crate::i18n::tr(
                                "Inspect live Miniserver CPU, heap, SPS cycle rate, online hardware monitors, and def.log",
                            ))
                            .clicked()
                        {
                            state.show_diagnostics_panel = !state.show_diagnostics_panel;
                            if state.show_diagnostics_panel && state.diagnostics.is_none() {
                                action = Some(MiniserverModalAction::FetchDiagnostics(
                                    state.credentials(),
                                ));
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
                                    RichText::new(crate::i18n::tr(
                                        "⚠ Newer Version Detected on Miniserver — Upload Blocked",
                                    ))
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
                                    RichText::new(crate::i18n::tr(
                                        "Please pull the difference from the Miniserver into your project before uploading. What is on the server takes higher priority on any conflict, while preserving your non-conflicting local additions.",
                                    ))
                                    .size(11.0)
                                    .color(TEXT_MUTED),
                                );
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    if ui
                                        .button(
                                            RichText::new(crate::i18n::tr(
                                                "⬇ Pull & Merge into My Project (Server Priority)",
                                            ))
                                            .size(12.0)
                                            .color(ACCENT_BLUE)
                                            .strong(),
                                        )
                                        .on_hover_text(crate::i18n::tr(
                                            "Merge the newer Miniserver version into your current project with Server Priority",
                                        ))
                                        .clicked()
                                    {
                                        action = Some(MiniserverModalAction::PullAndMergeServerConflict);
                                    }
                                    if ui
                                        .button(
                                            RichText::new(crate::i18n::tr("Dismiss"))
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
                                    RichText::new(crate::i18n::tr(
                                        "🛡 Safe Hardware Deployment (Save to Miniserver)",
                                    ))
                                    .size(12.0)
                                    .color(Color32::from_rgb(232, 168, 56))
                                    .strong(),
                                );
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(crate::i18n::tr(
                                        "1. Checks if the Miniserver already has a newer version and safeguards against accidental overwrite.\n2. Automatically saves a local safety backup of the running Miniserver program to ~/.config/loxone-portable/backups/.\n3. Compresses the active project into .LoxCC and uploads sps_new.zip to /prog/.",
                                    ))
                                    .size(11.0)
                                    .color(TEXT_MUTED),
                                );
                                ui.add_space(6.0);
                                ui.checkbox(
                                    &mut state.restart_sps_after_upload,
                                    RichText::new(crate::i18n::tr(
                                        "Restart Miniserver SPS (/dev/sps/restart) after upload",
                                    ))
                                    .size(11.5)
                                    .color(TEXT_PRIMARY),
                                );
                                ui.checkbox(
                                    &mut state.confirm_upload,
                                    RichText::new(crate::i18n::tr(
                                        "I understand this will update the live Miniserver program",
                                    ))
                                    .size(11.5)
                                    .color(TEXT_PRIMARY),
                                );
                                ui.add_space(6.0);
                                if ui
                                    .add_enabled(
                                        can_submit && state.confirm_upload,
                                        egui::Button::new(
                                            RichText::new(crate::i18n::tr(
                                                "⬆ Backup & Upload Project Now",
                                            ))
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
                                    "{} ({})",
                                    crate::i18n::tr("Backups in /prog/"),
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
                                                RichText::new(crate::i18n::tr("↻ Refresh"))
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
                                    RichText::new(crate::i18n::tr(
                                        "No backups loaded yet. Click 'Refresh' or 'Test Connection' to list /prog/.",
                                    ))
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
                                                                RichText::new(crate::i18n::tr(
                                                                    "⬇ Download",
                                                                ))
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

                    // Expandable Live Diagnostics & Logs Panel
                    if state.show_diagnostics_panel {
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(crate::i18n::tr(
                                    "🩺 Live Miniserver Diagnostics & Online Monitors",
                                ))
                                .size(12.0)
                                .color(LOXONE_GREEN)
                                .strong(),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .add_enabled(
                                            can_submit,
                                            egui::Button::new(
                                                RichText::new(crate::i18n::tr(
                                                    "↻ Refresh Diagnostics",
                                                ))
                                                .size(11.0)
                                                .color(TEXT_PRIMARY),
                                            ),
                                        )
                                        .clicked()
                                    {
                                        action = Some(MiniserverModalAction::FetchDiagnostics(
                                            state.credentials(),
                                        ));
                                    }
                                },
                            );
                        });

                        ui.add_space(6.0);
                        if let Some(diag) = &state.diagnostics {
                            Frame::new()
                                .fill(CARD_BG)
                                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                                .corner_radius(6)
                                .inner_margin(Margin::symmetric(10, 8))
                                .show(ui, |ui| {
                                    ui.set_width(512.0);
                                    egui::Grid::new("miniserver_diag_metrics_grid")
                                        .num_columns(4)
                                        .spacing([14.0, 6.0])
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new(crate::i18n::tr("Firmware:"))
                                                    .size(11.0)
                                                    .color(TEXT_MUTED),
                                            );
                                            ui.label(
                                                RichText::new(&diag.firmware_version)
                                                    .monospace()
                                                    .size(11.5)
                                                    .strong()
                                                    .color(TEXT_PRIMARY),
                                            );
                                            ui.label(
                                                RichText::new(crate::i18n::tr("MAC Address:"))
                                                    .size(11.0)
                                                    .color(TEXT_MUTED),
                                            );
                                            ui.label(
                                                RichText::new(&diag.mac_address)
                                                    .monospace()
                                                    .size(11.5)
                                                    .strong()
                                                    .color(TEXT_PRIMARY),
                                            );
                                            ui.end_row();

                                            ui.label(
                                                RichText::new(crate::i18n::tr("SPS Status:"))
                                                    .size(11.0)
                                                    .color(TEXT_MUTED),
                                            );
                                            ui.label(
                                                RichText::new(&diag.sps_status)
                                                    .monospace()
                                                    .size(11.5)
                                                    .strong()
                                                    .color(LOXONE_GREEN),
                                            );
                                            ui.label(
                                                RichText::new(crate::i18n::tr("CPU Load:"))
                                                    .size(11.0)
                                                    .color(TEXT_MUTED),
                                            );
                                            ui.label(
                                                RichText::new(&diag.cpu_load)
                                                    .monospace()
                                                    .size(11.5)
                                                    .strong()
                                                    .color(TEXT_PRIMARY),
                                            );
                                            ui.end_row();

                                            ui.label(
                                                RichText::new(crate::i18n::tr("Heap Memory:"))
                                                    .size(11.0)
                                                    .color(TEXT_MUTED),
                                            );
                                            ui.label(
                                                RichText::new(&diag.heap_usage)
                                                    .monospace()
                                                    .size(11.5)
                                                    .color(TEXT_PRIMARY),
                                            );
                                            ui.label(
                                                RichText::new(crate::i18n::tr("Active Tasks:"))
                                                    .size(11.0)
                                                    .color(TEXT_MUTED),
                                            );
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} • TX {}",
                                                    diag.num_tasks, diag.lan_tx_packets
                                                ))
                                                .monospace()
                                                .size(11.5)
                                                .color(TEXT_PRIMARY),
                                            );
                                            ui.end_row();
                                        });
                                });

                            if !diag.online_devices.is_empty() {
                                ui.add_space(6.0);
                                let online_count = diag
                                    .online_devices
                                    .iter()
                                    .filter(|d| d.is_online)
                                    .count();
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "{}: {} / {}",
                                            crate::i18n::tr("Online Hardware Monitors"),
                                            online_count,
                                            diag.online_devices.len()
                                        ))
                                        .size(11.5)
                                        .strong()
                                        .color(LOXONE_GREEN),
                                    );
                                    ui.add(
                                        egui::TextEdit::singleline(&mut state.diagnostics_filter)
                                            .hint_text(crate::i18n::tr("🔍 Filter devices…"))
                                            .desired_width(220.0),
                                    );
                                });
                                ui.add_space(4.0);
                                let q = state.diagnostics_filter.trim().to_lowercase();
                                ScrollArea::vertical()
                                    .id_salt("miniserver_online_monitors_scroll")
                                    .max_height(130.0)
                                    .show(ui, |ui| {
                                        for item in &diag.online_devices {
                                            if !q.is_empty()
                                                && !item.monitor.title.to_lowercase().contains(&q)
                                                && !item
                                                    .monitor
                                                    .parent_title
                                                    .to_lowercase()
                                                    .contains(&q)
                                                && !item
                                                    .monitor
                                                    .serial_number
                                                    .to_lowercase()
                                                    .contains(&q)
                                            {
                                                continue;
                                            }
                                            ui.horizontal(|ui| {
                                                let badge_text = if item.is_online {
                                                    "🟢 Online"
                                                } else {
                                                    "🔴 Offline"
                                                };
                                                let badge_col = if item.is_online {
                                                    LOXONE_GREEN
                                                } else {
                                                    Color32::from_rgb(235, 87, 87)
                                                };
                                                ui.label(
                                                    RichText::new(badge_text)
                                                        .size(11.0)
                                                        .strong()
                                                        .color(badge_col),
                                                );
                                                ui.label(
                                                    RichText::new(&item.monitor.parent_title)
                                                        .size(11.5)
                                                        .strong()
                                                        .color(TEXT_PRIMARY),
                                                );
                                                if !item.monitor.serial_number.is_empty() {
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "[{}]",
                                                            item.monitor.serial_number
                                                        ))
                                                        .monospace()
                                                        .size(10.5)
                                                        .color(TEXT_MUTED),
                                                    );
                                                }
                                            });
                                        }
                                    });
                            }

                            if !diag.recent_log_lines.is_empty() {
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new(format!(
                                        "{} ({})",
                                        crate::i18n::tr("System Log (/log/def.log)"),
                                        diag.recent_log_lines.len()
                                    ))
                                    .size(11.0)
                                    .strong()
                                    .color(TEXT_MUTED),
                                );
                                Frame::new()
                                    .fill(CARD_BG)
                                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                                    .corner_radius(6)
                                    .inner_margin(Margin::symmetric(8, 6))
                                    .show(ui, |ui| {
                                        ui.set_width(512.0);
                                        ScrollArea::vertical()
                                            .id_salt("miniserver_def_log_scroll")
                                            .max_height(110.0)
                                            .show(ui, |ui| {
                                                for line in &diag.recent_log_lines {
                                                    ui.label(
                                                        RichText::new(line)
                                                            .monospace()
                                                            .size(10.0)
                                                            .color(TEXT_PRIMARY),
                                                    );
                                                }
                                            });
                                    });
                            }
                        } else {
                            ui.label(
                                RichText::new(crate::i18n::tr(
                                    "Click 'Refresh Diagnostics' to query Miniserver CPU, heap, online devices, and def.log.",
                                ))
                                .size(11.0)
                                .color(TEXT_MUTED),
                            );
                        }
                    }
                });
        });

    if crate::theme::should_close_modal_on_outside_click(
        ctx,
        "miniserver_modal_window",
        backdrop_clicked,
        popup_was_open,
        Some(win_resp.response.rect),
    ) {
        action = Some(MiniserverModalAction::Close);
    }

    action
}

fn render_my_miniservers_panel(
    ui: &mut egui::Ui,
    state: &mut MiniserverModalState,
    can_submit: bool,
) -> Option<MiniserverModalAction> {
    let mut action = None;
    let mut select_idx = None;
    let mut remove_idx = None;

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(crate::i18n::tr(
                "Saved Miniservers (auto-imported from .Loxone projects & local keychain)",
            ))
            .size(11.5)
            .color(TEXT_MUTED),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .small_button(
                    RichText::new(format!("+ {}", crate::i18n::tr("Add Miniserver")))
                        .color(LOXONE_GREEN)
                        .strong(),
                )
                .clicked()
            {
                state.keychain.entries.push(SavedMiniserverEntry {
                    name: format!("Miniserver {}", state.keychain.entries.len() + 1),
                    project_name: String::new(),
                    user: "admin".to_string(),
                    serial: String::new(),
                    password: None,
                    internal_address: "192.168.1.100".to_string(),
                    external_address: String::new(),
                    save_locally: true,
                });
                let new_idx = state.keychain.entries.len() - 1;
                state.select_keychain_entry(new_idx);
                state.persist_keychain_best_effort();
            }
        });
    });

    ui.add_space(6.0);

    // Table of Saved Miniservers (matching Loxone Config 17's CMiniserverKeychainDialog)
    Frame::new()
        .fill(CARD_BG)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .corner_radius(6)
        .inner_margin(Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_width(556.0);
            ScrollArea::vertical()
                .id_salt("my_miniservers_table_scroll")
                .max_height(155.0)
                .show(ui, |ui| {
                    egui::Grid::new("my_miniservers_table_grid")
                        .num_columns(6)
                        .striped(true)
                        .spacing([12.0, 5.0])
                        .show(ui, |ui| {
                            for hdr in [
                                crate::i18n::tr("Name"),
                                crate::i18n::tr("Project Name"),
                                crate::i18n::tr("Username"),
                                crate::i18n::tr("S/N"),
                                crate::i18n::tr("Password"),
                                "",
                            ] {
                                ui.label(
                                    RichText::new(hdr)
                                        .size(11.0)
                                        .color(TEXT_MUTED)
                                        .strong(),
                                );
                            }
                            ui.end_row();

                            for (idx, entry) in state.keychain.entries.iter().enumerate() {
                                let is_sel = state.selected_keychain_idx == Some(idx);
                                let row_col = if is_sel {
                                    LOXONE_GREEN
                                } else {
                                    TEXT_PRIMARY
                                };
                                if ui
                                    .selectable_label(
                                        is_sel,
                                        RichText::new(&entry.name).color(row_col).strong(),
                                    )
                                    .clicked()
                                {
                                    select_idx = Some(idx);
                                }
                                if ui
                                    .selectable_label(
                                        is_sel,
                                        RichText::new(&entry.project_name).color(TEXT_PRIMARY),
                                    )
                                    .clicked()
                                {
                                    select_idx = Some(idx);
                                }
                                ui.label(
                                    RichText::new(&entry.user)
                                        .size(11.2)
                                        .color(TEXT_PRIMARY),
                                );
                                ui.label(
                                    RichText::new(if entry.serial.is_empty() {
                                        "-"
                                    } else {
                                        &entry.serial
                                    })
                                    .monospace()
                                    .size(11.0)
                                    .color(ACCENT_BLUE),
                                );
                                let pw_mask = if entry
                                    .password
                                    .as_deref()
                                    .is_some_and(|p| !p.is_empty())
                                {
                                    "••••••"
                                } else {
                                    "-"
                                };
                                ui.label(RichText::new(pw_mask).size(11.0).color(TEXT_MUTED));
                                if ui
                                    .small_button(RichText::new("🗑").color(TEXT_MUTED))
                                    .on_hover_text(crate::i18n::tr("Delete"))
                                    .clicked()
                                {
                                    remove_idx = Some(idx);
                                }
                                ui.end_row();
                            }
                        });
                });
        });

    if let Some(idx) = select_idx {
        state.select_keychain_entry(idx);
    }
    if let Some(idx) = remove_idx {
        if idx < state.keychain.entries.len() {
            state.keychain.entries.remove(idx);
            state.selected_keychain_idx = if state.keychain.entries.is_empty() {
                None
            } else {
                Some(idx.min(state.keychain.entries.len() - 1))
            };
            state.persist_keychain_best_effort();
        }
    }

    // Bottom detail editor for the selected entry (matching Loxone Config 17's bottom form)
    if let Some(sel_idx) = state.selected_keychain_idx {
        let mut changed = false;
        if let Some(entry) = state.keychain.entries.get_mut(sel_idx) {
            ui.add_space(8.0);
            Frame::new()
                .fill(CARD_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .corner_radius(6)
                .inner_margin(Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.set_width(552.0);
                    egui::Grid::new("my_miniserver_detail_grid")
                        .num_columns(2)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            ui.label(RichText::new(crate::i18n::tr("Name")).color(TEXT_MUTED));
                            changed |= ui
                                .add(
                                    egui::TextEdit::singleline(&mut entry.name)
                                        .desired_width(400.0),
                                )
                                .changed();
                            ui.end_row();

                            ui.label(
                                RichText::new(crate::i18n::tr("Project Name")).color(TEXT_MUTED),
                            );
                            changed |= ui
                                .add(
                                    egui::TextEdit::singleline(&mut entry.project_name)
                                        .desired_width(400.0),
                                )
                                .changed();
                            ui.end_row();

                            ui.label(RichText::new(crate::i18n::tr("S/N")).color(TEXT_MUTED));
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut entry.serial)
                                        .hint_text("50:4F:94:A0:EC:AD")
                                        .desired_width(400.0),
                                )
                                .lost_focus()
                            {
                                entry.serial = format_serial_colon(&entry.serial);
                                if entry.external_address.trim().is_empty() {
                                    entry.external_address =
                                        default_cloud_external_address(&entry.serial);
                                }
                                changed = true;
                            }
                            ui.end_row();

                            ui.label(
                                RichText::new(crate::i18n::tr("Internal Address"))
                                    .color(TEXT_MUTED),
                            );
                            changed |= ui
                                .add(
                                    egui::TextEdit::singleline(&mut entry.internal_address)
                                        .hint_text("192.168.1.100")
                                        .desired_width(400.0),
                                )
                                .changed();
                            ui.end_row();

                            ui.label(
                                RichText::new(crate::i18n::tr("External Address"))
                                    .color(TEXT_MUTED),
                            );
                            changed |= ui
                                .add(
                                    egui::TextEdit::singleline(&mut entry.external_address)
                                        .hint_text("connect.loxonecloud.com/504F94A0ECAD")
                                        .desired_width(400.0),
                                )
                                .changed();
                            ui.end_row();

                            ui.label(RichText::new(crate::i18n::tr("Username")).color(TEXT_MUTED));
                            changed |= ui
                                .add(
                                    egui::TextEdit::singleline(&mut entry.user)
                                        .desired_width(400.0),
                                )
                                .changed();
                            ui.end_row();

                            ui.label(RichText::new(crate::i18n::tr("Password")).color(TEXT_MUTED));
                            let mut pw = entry.password.clone().unwrap_or_default();
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut pw)
                                        .password(true)
                                        .desired_width(400.0),
                                )
                                .changed()
                            {
                                entry.password = if pw.is_empty() { None } else { Some(pw) };
                                changed = true;
                            }
                            ui.end_row();
                        });

                    ui.add_space(4.0);
                    changed |= ui
                        .checkbox(
                            &mut entry.save_locally,
                            RichText::new(crate::i18n::tr("Save credentials locally"))
                                .size(11.5)
                                .color(TEXT_MUTED),
                        )
                        .changed();
                });
        }
        if changed {
            state.select_keychain_entry(sel_idx);
            state.persist_keychain_best_effort();
        }
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                can_submit,
                egui::Button::new(
                    RichText::new(crate::i18n::tr("🔌 Connect & Test"))
                        .size(12.0)
                        .color(LOXONE_GREEN)
                        .strong(),
                ),
            )
            .clicked()
        {
            state.active_tab = MiniserverModalTab::ConnectManually;
            action = Some(MiniserverModalAction::TestConnection(state.credentials()));
        }
        if ui
            .add_enabled(
                can_submit,
                egui::Button::new(
                    RichText::new(crate::i18n::tr("⬇ Download Latest Project"))
                        .size(12.0)
                        .color(ACCENT_BLUE)
                        .strong(),
                ),
            )
            .clicked()
        {
            state.active_tab = MiniserverModalTab::ConnectManually;
            action = Some(MiniserverModalAction::DownloadLatest(state.credentials()));
        }
        if ui
            .button(
                RichText::new(crate::i18n::tr("Switch to Manual Connection"))
                    .size(11.5)
                    .color(TEXT_MUTED),
            )
            .clicked()
        {
            state.active_tab = MiniserverModalTab::ConnectManually;
        }
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
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Sample.Loxone");
        let project = load_loxone_file(&path).expect("Sample.Loxone should parse");

        let mut modal = MiniserverModalState::default();
        modal.host.clear();

        modal.apply_project_fallback_host(Some(&project));
        assert_eq!(modal.host, "127.0.0.1");
        assert!(modal
            .keychain
            .entries
            .iter()
            .any(|e| e.name == "Sample" && e.internal_address == "127.0.0.1"));

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

        // Also render the MyMiniservers tab headlessly
        modal.active_tab = MiniserverModalTab::MyMiniservers;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let _ = render_miniserver_modal(ctx, &mut modal, &status);
        });
    }
}
