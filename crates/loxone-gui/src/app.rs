use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use egui::{Context, Key, Stroke};
use loxone_core::miniserver::{
    BackupSnapshot, ConnectionProfile, ConnectionStatus, MiniserverClient, MiniserverCredentials,
};
use loxone_core::model::LoxoneProject;
use loxone_core::parser::{load_loxone_file, parse_loxone_xml};

use crate::canvas::{render_page_canvas, CanvasAction, CanvasViewport};
use crate::command_palette::{CommandPaletteState, PaletteAction};
use crate::inspector::{render_right_inspector, InspectorAction};
use crate::miniserver_modal::{
    render_miniserver_modal, MiniserverModalAction, MiniserverModalState,
};
use crate::theme::{apply_minimalist_theme, BORDER_SUBTLE, PANEL_BG};
use crate::top_bar::{render_top_bar, TopBarAction};
use crate::tree_panel::{render_left_explorer, LeftTab, TreeAction};

#[derive(Debug)]
pub enum MiniserverWorkerEvent {
    ConnectionTested {
        host: String,
        result: Result<(String, Vec<BackupSnapshot>), String>,
    },
    BackupsListed {
        host: String,
        result: Result<Vec<BackupSnapshot>, String>,
    },
    ProjectDownloaded {
        host: String,
        result: Result<(String, String), String>,
    },
}

pub struct LoxonePortableApp {
    pub project: Option<LoxoneProject>,
    pub loaded_path: Option<PathBuf>,
    pub error_banner: Option<String>,
    pub active_page_index: usize,
    pub selected_uuid: Option<String>,
    pub left_tab: LeftTab,
    pub tree_filter: String,
    pub show_left_panel: bool,
    pub show_right_panel: bool,
    pub viewport: CanvasViewport,
    pub palette: CommandPaletteState,
    pub needs_initial_fit: bool,
    pub pending_center_block_uuid: Option<String>,
    pub connection_status: ConnectionStatus,
    pub miniserver_modal: MiniserverModalState,
    pub worker_tx: Sender<MiniserverWorkerEvent>,
    pub worker_rx: Receiver<MiniserverWorkerEvent>,
}

impl Default for LoxonePortableApp {
    fn default() -> Self {
        let (worker_tx, worker_rx) = mpsc::channel();
        let initial_modal = ConnectionProfile::default_config_path()
            .and_then(|p| ConnectionProfile::load_from_path(&p).ok())
            .map(|prof| MiniserverModalState::from_profile(&prof))
            .unwrap_or_default();

        Self {
            project: None,
            loaded_path: None,
            error_banner: None,
            active_page_index: 0,
            selected_uuid: None,
            left_tab: LeftTab::Pages,
            tree_filter: String::new(),
            show_left_panel: true,
            show_right_panel: true,
            viewport: CanvasViewport::default(),
            palette: CommandPaletteState::default(),
            needs_initial_fit: true,
            pending_center_block_uuid: None,
            connection_status: ConnectionStatus::Disconnected,
            miniserver_modal: initial_modal,
            worker_tx,
            worker_rx,
        }
    }
}

impl LoxonePortableApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial_path: Option<PathBuf>) -> Self {
        apply_minimalist_theme(&cc.egui_ctx);
        let mut app = Self::default();
        if let Some(path) = initial_path {
            app.load_file(&path);
        }
        app
    }

    pub fn load_file(&mut self, path: &Path) {
        match load_loxone_file(path) {
            Ok(proj) => {
                self.miniserver_modal.apply_project_fallback_host(Some(&proj));
                self.project = Some(proj);
                self.loaded_path = Some(path.to_path_buf());
                self.error_banner = None;
                self.active_page_index = 0;
                self.selected_uuid = None;
                self.needs_initial_fit = true;
            }
            Err(err) => {
                self.error_banner = Some(format!("Failed to load {}: {}", path.display(), err));
            }
        }
    }

    pub fn load_downloaded_xml(
        &mut self,
        host: &str,
        archive_filename: &str,
        xml: &str,
    ) -> Result<(), String> {
        let proj = parse_loxone_xml(xml).map_err(|e| format!("Failed to parse XML: {}", e))?;
        self.miniserver_modal.apply_project_fallback_host(Some(&proj));
        self.project = Some(proj);
        self.loaded_path = None;
        self.error_banner = None;
        self.active_page_index = 0;
        self.selected_uuid = None;
        self.needs_initial_fit = true;
        self.connection_status = ConnectionStatus::Connected {
            host: host.to_string(),
            server_info: format!("Loaded {}", archive_filename),
        };
        self.miniserver_modal.status_message =
            Some(format!("Downloaded and opened {}", archive_filename));
        self.miniserver_modal.is_open = false;
        Ok(())
    }

    pub fn save_xml_to_path(&mut self, path: &Path) -> Result<(), String> {
        let raw_xml = self
            .project
            .as_ref()
            .and_then(|p| p.raw_xml.as_deref())
            .ok_or_else(|| "No project XML loaded to save".to_string())?;
        std::fs::write(path, raw_xml)
            .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
        self.loaded_path = Some(path.to_path_buf());
        self.error_banner = None;
        Ok(())
    }

    pub fn open_miniserver_modal(&mut self) {
        self.miniserver_modal
            .apply_project_fallback_host(self.project.as_ref());
        self.miniserver_modal.is_open = true;
    }

    fn persist_profile_best_effort(&self) {
        if let Some(config_path) = ConnectionProfile::default_config_path() {
            let _ = self.miniserver_modal.to_profile().save_to_path(&config_path);
        }
    }

    pub fn execute_modal_action(&mut self, action: MiniserverModalAction) {
        match action {
            MiniserverModalAction::Close => {
                self.miniserver_modal.is_open = false;
            }
            MiniserverModalAction::TestConnection(creds) => {
                self.persist_profile_best_effort();
                let host = creds.host.clone();
                self.connection_status = ConnectionStatus::Connecting {
                    host: host.clone(),
                    message: "Testing connection…".to_string(),
                };
                self.miniserver_modal.status_message =
                    Some(format!("Testing connection to {}…", host));
                let tx = self.worker_tx.clone();
                std::thread::spawn(move || {
                    let result = MiniserverClient::list_snapshots(&creds).map(|snaps| {
                        let summary = if let Some(latest) = snaps.first() {
                            format!(
                                "{} backup(s) available · latest: {}",
                                snaps.len(),
                                latest.filename
                            )
                        } else {
                            "Connected (0 backups in /prog)".to_string()
                        };
                        (summary, snaps)
                    });
                    let _ = tx.send(MiniserverWorkerEvent::ConnectionTested {
                        host,
                        result: result.map_err(|e| e.to_string()),
                    });
                });
            }
            MiniserverModalAction::RefreshBackups(creds) => {
                self.persist_profile_best_effort();
                let host = creds.host.clone();
                self.connection_status = ConnectionStatus::Connecting {
                    host: host.clone(),
                    message: "Listing /prog/ backups…".to_string(),
                };
                self.miniserver_modal.status_message =
                    Some(format!("Fetching /prog/ backup list from {}…", host));
                let tx = self.worker_tx.clone();
                std::thread::spawn(move || {
                    let result =
                        MiniserverClient::list_snapshots(&creds).map_err(|e| e.to_string());
                    let _ = tx.send(MiniserverWorkerEvent::BackupsListed { host, result });
                });
            }
            MiniserverModalAction::DownloadLatest(creds) => {
                self.start_download_worker(creds, None);
            }
            MiniserverModalAction::DownloadSnapshot { creds, filename } => {
                self.start_download_worker(creds, Some(filename));
            }
        }
    }

    fn start_download_worker(&mut self, creds: MiniserverCredentials, filename: Option<String>) {
        self.persist_profile_best_effort();
        let host = creds.host.clone();
        let target_desc = filename.as_deref().unwrap_or("latest backup");
        self.connection_status = ConnectionStatus::Connecting {
            host: host.clone(),
            message: format!("Downloading {}…", target_desc),
        };
        self.miniserver_modal.status_message =
            Some(format!("Downloading {} from {}…", target_desc, host));
        let tx = self.worker_tx.clone();
        std::thread::spawn(move || {
            let result = MiniserverClient::download_snapshot_xml(&creds, filename.as_deref())
                .map_err(|e| e.to_string());
            let _ = tx.send(MiniserverWorkerEvent::ProjectDownloaded { host, result });
        });
    }

    pub fn poll_miniserver_worker(&mut self) {
        while let Ok(event) = self.worker_rx.try_recv() {
            match event {
                MiniserverWorkerEvent::ConnectionTested { host, result } => match result {
                    Ok((summary, snapshots)) => {
                        self.connection_status = ConnectionStatus::Connected {
                            host,
                            server_info: summary.clone(),
                        };
                        self.miniserver_modal.snapshots = snapshots;
                        self.miniserver_modal.status_message = Some(summary);
                    }
                    Err(err) => {
                        self.connection_status = ConnectionStatus::Error {
                            host,
                            error: err.clone(),
                        };
                        self.miniserver_modal.status_message = Some(err);
                    }
                },
                MiniserverWorkerEvent::BackupsListed { host, result } => match result {
                    Ok(snapshots) => {
                        let summary = format!("{} backup(s) found in /prog/", snapshots.len());
                        self.connection_status = ConnectionStatus::Connected {
                            host,
                            server_info: summary.clone(),
                        };
                        self.miniserver_modal.snapshots = snapshots;
                        self.miniserver_modal.status_message = Some(summary);
                    }
                    Err(err) => {
                        self.connection_status = ConnectionStatus::Error {
                            host,
                            error: err.clone(),
                        };
                        self.miniserver_modal.status_message = Some(err);
                    }
                },
                MiniserverWorkerEvent::ProjectDownloaded { host, result } => match result {
                    Ok((filename, xml)) => {
                        if let Err(err) = self.load_downloaded_xml(&host, &filename, &xml) {
                            self.connection_status = ConnectionStatus::Error {
                                host,
                                error: err.clone(),
                            };
                            self.miniserver_modal.status_message = Some(err.clone());
                            self.error_banner = Some(err);
                        }
                    }
                    Err(err) => {
                        self.connection_status = ConnectionStatus::Error {
                            host,
                            error: err.clone(),
                        };
                        self.miniserver_modal.status_message = Some(err);
                    }
                },
            }
        }
    }

    pub fn execute_top_bar_action(&mut self, action: TopBarAction) {
        match action {
            TopBarAction::ToggleLeftPanel => {
                self.show_left_panel = !self.show_left_panel;
            }
            TopBarAction::ToggleRightPanel => {
                self.show_right_panel = !self.show_right_panel;
            }
            TopBarAction::OpenFile => {
                self.execute_palette_action(PaletteAction::OpenFile);
            }
            TopBarAction::SaveFile => {
                self.execute_palette_action(PaletteAction::SaveFile);
            }
            TopBarAction::OpenMiniserverModal => {
                self.open_miniserver_modal();
            }
            TopBarAction::OpenCommandPalette => {
                self.palette.open();
            }
            TopBarAction::ToggleCompactPins => {
                self.viewport.compact_pins = !self.viewport.compact_pins;
            }
            TopBarAction::FitPage => {
                self.needs_initial_fit = true;
            }
        }
    }

    pub fn execute_palette_action(&mut self, action: PaletteAction) {
        match action {
            PaletteAction::OpenFile => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Loxone Project", &["Loxone", "loxone", "xml"])
                    .pick_file()
                {
                    self.load_file(&path);
                }
            }
            PaletteAction::SaveFile => {
                if self.project.as_ref().and_then(|p| p.raw_xml.as_ref()).is_none() {
                    return;
                }
                let default_name = self
                    .project
                    .as_ref()
                    .map(|p| {
                        let base = p.metadata.title.trim();
                        if base.is_empty() {
                            "DownloadedProject.Loxone".to_string()
                        } else if base.ends_with(".Loxone") || base.ends_with(".loxone") {
                            base.to_string()
                        } else {
                            format!("{}.Loxone", base)
                        }
                    })
                    .unwrap_or_else(|| "Project.Loxone".to_string());

                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name(&default_name)
                    .add_filter("Loxone Project", &["Loxone", "loxone", "xml"])
                    .save_file()
                {
                    if let Err(err) = self.save_xml_to_path(&path) {
                        self.error_banner = Some(err);
                    }
                }
            }
            PaletteAction::ConnectMiniserver => {
                self.open_miniserver_modal();
            }
            PaletteAction::FitPage => {
                self.needs_initial_fit = true;
            }
            PaletteAction::ResetZoom => {
                self.viewport.zoom = 1.0;
                self.viewport.pan = egui::Vec2::new(40.0, 40.0);
            }
            PaletteAction::ToggleCompactPins => {
                self.viewport.compact_pins = !self.viewport.compact_pins;
            }
            PaletteAction::ToggleLeftPanel => {
                self.show_left_panel = !self.show_left_panel;
            }
            PaletteAction::ToggleRightPanel => {
                self.show_right_panel = !self.show_right_panel;
            }
            PaletteAction::NavigateToPage(idx) => {
                self.active_page_index = idx;
                self.needs_initial_fit = true;
            }
            PaletteAction::NavigateToBlock {
                page_index,
                block_uuid,
            } => {
                self.active_page_index = page_index;
                self.selected_uuid = Some(block_uuid.clone());
                self.pending_center_block_uuid = Some(block_uuid);
                self.show_right_panel = true;
            }
            PaletteAction::SelectObject(uuid) => {
                self.selected_uuid = Some(uuid);
                self.show_right_panel = true;
            }
        }
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        let cmd_or_ctrl = ctx.input(|i| i.modifiers.command || i.modifiers.ctrl);
        let shift = ctx.input(|i| i.modifiers.shift);

        if cmd_or_ctrl && ctx.input(|i| i.key_pressed(Key::K)) {
            if self.palette.is_open {
                self.palette.close();
            } else {
                self.miniserver_modal.is_open = false;
                self.palette.open();
            }
        }
        if self.palette.is_open {
            return;
        }

        if cmd_or_ctrl && shift && ctx.input(|i| i.key_pressed(Key::D)) {
            self.open_miniserver_modal();
        }
        if self.miniserver_modal.is_open {
            return;
        }

        if cmd_or_ctrl && ctx.input(|i| i.key_pressed(Key::B)) {
            self.show_left_panel = !self.show_left_panel;
        }
        if cmd_or_ctrl && ctx.input(|i| i.key_pressed(Key::I)) {
            self.show_right_panel = !self.show_right_panel;
        }
        if cmd_or_ctrl && ctx.input(|i| i.key_pressed(Key::O)) {
            self.execute_palette_action(PaletteAction::OpenFile);
        }
        if cmd_or_ctrl && ctx.input(|i| i.key_pressed(Key::S)) {
            self.execute_palette_action(PaletteAction::SaveFile);
        }

        if !ctx.wants_keyboard_input() {
            if ctx.input(|i| i.key_pressed(Key::F)) {
                self.needs_initial_fit = true;
            }
            if ctx.input(|i| i.key_pressed(Key::P)) {
                self.viewport.compact_pins = !self.viewport.compact_pins;
            }
            if ctx.input(|i| i.key_pressed(Key::Num0)) {
                self.execute_palette_action(PaletteAction::ResetZoom);
            }
        }
    }
}

impl eframe::App for LoxonePortableApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.poll_miniserver_worker();
        if self.connection_status.is_connecting() {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }

        self.handle_shortcuts(ctx);

        // Extensible Top Header Bar with Connection Status Indicator
        if let Some(top_act) = render_top_bar(
            ctx,
            self.project.as_ref(),
            self.loaded_path.as_deref(),
            self.active_page_index,
            self.viewport.compact_pins,
            self.viewport.zoom,
            &self.connection_status,
            Some(&self.miniserver_modal.host),
        ) {
            self.execute_top_bar_action(top_act);
        }

        if let Some(err) = &self.error_banner {
            egui::TopBottomPanel::top("error_banner").show(ctx, |ui| {
                ui.colored_label(egui::Color32::LIGHT_RED, err);
            });
        }

        // Left Explorer Panel
        if self.show_left_panel {
            if let Some(proj) = &self.project {
                let mut tree_act = None;
                egui::SidePanel::left("left_explorer")
                    .default_width(250.0)
                    .frame(
                        egui::Frame::new()
                            .fill(PANEL_BG)
                            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                            .inner_margin(egui::Margin::symmetric(10, 6)),
                    )
                    .show(ctx, |ui| {
                        tree_act = render_left_explorer(
                            ui,
                            proj,
                            &mut self.left_tab,
                            &mut self.tree_filter,
                            self.active_page_index,
                            self.selected_uuid.as_deref(),
                        );
                    });

                if let Some(act) = tree_act {
                    match act {
                        TreeAction::SelectPage(idx) => {
                            self.active_page_index = idx;
                            self.needs_initial_fit = true;
                        }
                        TreeAction::SelectObject(uuid) => {
                            self.selected_uuid = Some(uuid);
                            self.show_right_panel = true;
                        }
                        TreeAction::NavigateToBlock {
                            page_index,
                            block_uuid,
                        } => {
                            self.execute_palette_action(PaletteAction::NavigateToBlock {
                                page_index,
                                block_uuid,
                            });
                        }
                    }
                }
            }
        }

        // Right Inspector Panel
        if self.show_right_panel {
            if let Some(proj) = &self.project {
                let mut insp_act = None;
                egui::SidePanel::right("right_inspector")
                    .default_width(300.0)
                    .frame(
                        egui::Frame::new()
                            .fill(PANEL_BG)
                            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                            .inner_margin(egui::Margin::symmetric(10, 6)),
                    )
                    .show(ctx, |ui| {
                        insp_act =
                            render_right_inspector(ui, proj, self.selected_uuid.as_deref());
                    });

                if let Some(act) = insp_act {
                    match act {
                        InspectorAction::NavigateToBlock {
                            page_index,
                            block_uuid,
                        } => {
                            self.execute_palette_action(PaletteAction::NavigateToBlock {
                                page_index,
                                block_uuid,
                            });
                        }
                        InspectorAction::SelectObject(uuid) => {
                            self.selected_uuid = Some(uuid);
                        }
                    }
                }
            }
        }

        // Central Schematic Page Canvas
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(crate::theme::CANVAS_BG))
            .show(ctx, |ui| {
                let view_size = ui.available_size();
                if let Some(proj) = &self.project {
                    if self.needs_initial_fit {
                        if let Some(page) = proj.pages.get(self.active_page_index) {
                            self.viewport.fit_to_page(page, view_size);
                        }
                        self.needs_initial_fit = false;
                    }
                    if let Some(target_uuid) = self.pending_center_block_uuid.take() {
                        if let Some(page) = proj.pages.get(self.active_page_index) {
                            if let Some(block) =
                                page.blocks.iter().find(|b| b.uuid == target_uuid)
                            {
                                if block.has_position {
                                    self.viewport.center_on_block(block, view_size);
                                } else {
                                    self.viewport.fit_to_page(page, view_size);
                                }
                            }
                        }
                    }

                    if let Some(canvas_act) = render_page_canvas(
                        ui,
                        proj,
                        self.active_page_index,
                        &mut self.viewport,
                        self.selected_uuid.as_deref(),
                    ) {
                        match canvas_act {
                            CanvasAction::SelectObject(uuid) => {
                                self.selected_uuid = Some(uuid);
                                self.show_right_panel = true;
                            }
                            CanvasAction::ClearSelection => {
                                self.selected_uuid = None;
                            }
                            CanvasAction::NavigateToBlock {
                                page_index,
                                block_uuid,
                            } => {
                                self.execute_palette_action(PaletteAction::NavigateToBlock {
                                    page_index,
                                    block_uuid,
                                });
                            }
                        }
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.horizontal(|ui| {
                            if ui
                                .button("📂 Open .Loxone Project File (Ctrl+O)…")
                                .clicked()
                            {
                                self.execute_palette_action(PaletteAction::OpenFile);
                            }
                            if ui
                                .button("⬇ Connect to Miniserver & Download (Ctrl+Shift+D)…")
                                .clicked()
                            {
                                self.execute_palette_action(PaletteAction::ConnectMiniserver);
                            }
                        });
                    });
                }
            });

        // Miniserver Connection & Backup Browser Modal Overlay
        if let Some(modal_act) = render_miniserver_modal(
            ctx,
            &mut self.miniserver_modal,
            &self.connection_status,
        ) {
            self.execute_modal_action(modal_act);
        }

        // Ctrl+K / Cmd+K Command Palette Modal Overlay
        if let Some(palette_act) = self.palette.show(ctx, self.project.as_ref()) {
            self.execute_palette_action(palette_act);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_load_and_navigate_klimczaki() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let mut app = LoxonePortableApp::default();
        app.load_file(&path);

        assert!(app.project.is_some());
        assert!(app.error_banner.is_none());

        // Navigate via Command Palette action to Page 5
        app.execute_palette_action(PaletteAction::NavigateToPage(5));
        assert_eq!(app.active_page_index, 5);

        // Toggle compact pins
        assert!(app.viewport.compact_pins);
        app.execute_palette_action(PaletteAction::ToggleCompactPins);
        assert!(!app.viewport.compact_pins);

        // Navigate to a block and select an object
        let first_block_uuid = app.project.as_ref().unwrap().pages[0].blocks[0].uuid.clone();
        app.execute_palette_action(PaletteAction::NavigateToBlock {
            page_index: 0,
            block_uuid: first_block_uuid.clone(),
        });
        assert_eq!(app.active_page_index, 0);
        assert_eq!(app.selected_uuid.as_deref(), Some(first_block_uuid.as_str()));
        assert_eq!(
            app.pending_center_block_uuid.as_deref(),
            Some(first_block_uuid.as_str())
        );

        // Headless egui pass exercising tree_panel and inspector across PageBlock, TreeNode, and Category
        let ctx = egui::Context::default();
        let proj = app.project.as_ref().unwrap();
        let tree_uuid = proj.periphery_tree[0].uuid.clone();
        let cat_uuid = proj.categories.keys().next().unwrap().clone();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                for mut tab in [
                    LeftTab::Pages,
                    LeftTab::Periphery,
                    LeftTab::Rooms,
                    LeftTab::System,
                ] {
                    let mut filter = String::new();
                    let _ = render_left_explorer(
                        ui,
                        proj,
                        &mut tab,
                        &mut filter,
                        0,
                        Some(&first_block_uuid),
                    );
                }
                let _ = render_right_inspector(ui, proj, Some(&first_block_uuid));
                let _ = render_right_inspector(ui, proj, Some(&tree_uuid));
                let _ = render_right_inspector(ui, proj, Some(&cat_uuid));
            });
        });
    }

    #[test]
    fn test_app_miniserver_worker_and_save_xml_roundtrip() {
        let mut app = LoxonePortableApp::default();
        app.miniserver_modal.host.clear();

        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        app.load_file(&path);
        assert_eq!(app.miniserver_modal.host, "192.168.1.100");
        assert_eq!(app.connection_status, ConnectionStatus::Disconnected);

        // Open Miniserver modal via TopBarAction
        app.execute_top_bar_action(TopBarAction::OpenMiniserverModal);
        assert!(app.miniserver_modal.is_open);

        // Simulate worker completing ConnectionTested
        app.worker_tx
            .send(MiniserverWorkerEvent::ConnectionTested {
                host: "192.168.1.100".to_string(),
                result: Ok(("2 backups available".to_string(), Vec::new())),
            })
            .unwrap();
        app.poll_miniserver_worker();
        assert!(app.connection_status.is_connected());

        // Simulate worker completing ProjectDownloaded
        let downloaded_xml = "\u{FEFF}<?xml version=\"1.0\" encoding=\"utf-8\"?>\
            <ControlList Version=\"274\">\
                <C Type=\"Document\" U=\"doc-dl\" Title=\"DownloadedFromMiniserver\" ConfigVersion=\"17020828\"/>\
            </ControlList>";
        app.worker_tx
            .send(MiniserverWorkerEvent::ProjectDownloaded {
                host: "192.168.1.100".to_string(),
                result: Ok((
                    "sps_194_20260308182256.zip".to_string(),
                    downloaded_xml.to_string(),
                )),
            })
            .unwrap();
        app.poll_miniserver_worker();

        assert!(!app.miniserver_modal.is_open);
        assert_eq!(
            app.project.as_ref().unwrap().metadata.title,
            "DownloadedFromMiniserver"
        );
        assert!(app.loaded_path.is_none());

        // Save downloaded XML to disk and verify contents
        let temp_file = std::env::temp_dir().join(format!(
            "loxone_portable_saved_{}.Loxone",
            std::process::id()
        ));
        app.save_xml_to_path(&temp_file)
            .expect("save_xml_to_path should succeed");
        let saved_contents = std::fs::read_to_string(&temp_file).unwrap();
        assert_eq!(saved_contents, downloaded_xml);
        assert_eq!(app.loaded_path.as_deref(), Some(temp_file.as_path()));
        let _ = std::fs::remove_file(&temp_file);
    }
}
