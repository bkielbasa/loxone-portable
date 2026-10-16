use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use egui::{Context, Key, Stroke};
use loxone_core::miniserver::{
    BackupSnapshot, ConnectionProfile, ConnectionStatus, LiveControlSnapshot, LiveValueStore,
    MiniserverClient, MiniserverCredentials,
};
use loxone_core::model::{LoxoneProject, TreeNode, TreeSectionKind};
use loxone_core::parser::{load_loxone_file, parse_loxone_xml};
use loxone_core::writer::ProjectEdit;
use loxone_core::xref::ObjectLocation;

use crate::canvas::{render_page_canvas, CanvasAction, CanvasViewport};
use crate::command_palette::{CommandPaletteState, PaletteAction};
use crate::insert_modal::{show_insert_modal, InsertModalSelection, InsertModalState};
use crate::inspector::{render_right_inspector, InspectorAction, LiveCommand};
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
    LiveValuesPolled(Vec<(String, LiveControlSnapshot)>),
    LiveCommandSent(Result<(String, LiveControlSnapshot), String>),
}

pub struct LoxonePortableApp {
    pub project: Option<LoxoneProject>,
    pub loaded_path: Option<PathBuf>,
    pub is_dirty: bool,
    pub undo_stack: Vec<ProjectEdit>,
    pub redo_stack: Vec<ProjectEdit>,
    pub insert_modal: InsertModalState,
    pub next_created_counter: usize,
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
    pub live_view_enabled: bool,
    pub live_store: LiveValueStore,
    pub last_live_poll: Option<std::time::Instant>,
    pub live_poll_in_flight: bool,
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
            is_dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            insert_modal: InsertModalState::default(),
            next_created_counter: 1,
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
            live_view_enabled: false,
            live_store: LiveValueStore::default(),
            last_live_poll: None,
            live_poll_in_flight: false,
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
                self.live_store = LiveValueStore::default();
                self.live_store.seed_project_defaults(&proj);
                self.last_live_poll = None;
                self.project = Some(proj);
                self.loaded_path = Some(path.to_path_buf());
                self.is_dirty = false;
                self.undo_stack.clear();
                self.redo_stack.clear();
                self.insert_modal.close();
                self.viewport.interaction = Default::default();
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
        self.live_store = LiveValueStore::default();
        self.live_store.seed_project_defaults(&proj);
        self.last_live_poll = None;
        self.project = Some(proj);
        self.loaded_path = None;
        self.is_dirty = false;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.insert_modal.close();
        self.viewport.interaction = Default::default();
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

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn execute_edit(&mut self, edit: ProjectEdit) {
        if let Some(proj) = &mut self.project {
            match loxone_core::writer::apply_edit_with_inverse(proj, &edit) {
                Ok(inverse) => {
                    self.undo_stack.push(inverse);
                    self.redo_stack.clear();
                    self.is_dirty = true;
                    self.error_banner = None;
                }
                Err(e) => {
                    self.error_banner = Some(format!("Edit error: {}", e));
                }
            }
        }
    }

    pub fn undo(&mut self) {
        let Some(inv) = self.undo_stack.pop() else {
            return;
        };
        if let Some(proj) = &mut self.project {
            match loxone_core::writer::apply_edit_with_inverse(proj, &inv) {
                Ok(redo_edit) => {
                    self.redo_stack.push(redo_edit);
                    self.is_dirty = true;
                    self.error_banner = None;
                }
                Err(e) => {
                    self.error_banner = Some(format!("Undo error: {}", e));
                }
            }
        }
    }

    pub fn redo(&mut self) {
        let Some(redo_edit) = self.redo_stack.pop() else {
            return;
        };
        if let Some(proj) = &mut self.project {
            match loxone_core::writer::apply_edit_with_inverse(proj, &redo_edit) {
                Ok(undo_edit) => {
                    self.undo_stack.push(undo_edit);
                    self.is_dirty = true;
                    self.error_banner = None;
                }
                Err(e) => {
                    self.error_banner = Some(format!("Redo error: {}", e));
                }
            }
        }
    }

    pub fn apply_project_edit(&mut self, edit: ProjectEdit) {
        self.execute_edit(edit);
    }

    pub fn save_xml_to_path(&mut self, path: &Path) -> Result<(), String> {
        if self.is_dirty {
            let proj = self
                .project
                .as_mut()
                .ok_or_else(|| "No project XML loaded to save".to_string())?;
            loxone_core::writer::save_loxone_file(path, proj)
                .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
        } else {
            let raw_xml = self
                .project
                .as_ref()
                .and_then(|p| p.raw_xml.as_deref())
                .ok_or_else(|| "No project XML loaded to save".to_string())?;
            std::fs::write(path, raw_xml)
                .map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
        }
        self.loaded_path = Some(path.to_path_buf());
        self.is_dirty = false;
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
                MiniserverWorkerEvent::LiveValuesPolled(updates) => {
                    self.live_poll_in_flight = false;
                    for (id, snap) in updates {
                        self.live_store.apply_polled_snapshot(id, snap);
                    }
                }
                MiniserverWorkerEvent::LiveCommandSent(res) => {
                    if let Ok((id, snap)) = res {
                        self.live_store.apply_polled_snapshot(id, snap);
                    }
                }
            }
        }
    }

    pub fn collect_active_page_poll_identifiers(&self) -> Vec<String> {
        let mut ids = Vec::new();
        let mut seen = std::collections::HashSet::new();
        if let Some(sel) = &self.selected_uuid {
            if !sel.is_empty() && seen.insert(sel.clone()) {
                ids.push(sel.clone());
            }
        }
        if let Some(proj) = &self.project {
            if let Some(page) = proj.pages.get(self.active_page_index) {
                for block in &page.blocks {
                    if let Some(ref_u) = &block.ref_uuid {
                        if !ref_u.is_empty() && seen.insert(ref_u.clone()) {
                            ids.push(ref_u.clone());
                        }
                    }
                    if !block.uuid.is_empty() && seen.insert(block.uuid.clone()) {
                        ids.push(block.uuid.clone());
                    }
                    if ids.len() >= 48 {
                        break;
                    }
                }
            }
        }
        ids
    }

    pub fn maybe_schedule_live_poll(&mut self, ctx: &Context) {
        if !self.live_view_enabled {
            return;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
        if !self.connection_status.is_connected() || self.live_poll_in_flight {
            return;
        }
        let should_poll = self
            .last_live_poll
            .map_or(true, |t| t.elapsed() >= std::time::Duration::from_millis(1200));
        if !should_poll {
            return;
        }
        let ids = self.collect_active_page_poll_identifiers();
        if ids.is_empty() || self.miniserver_modal.host.trim().is_empty() {
            return;
        }
        let creds = MiniserverCredentials {
            host: self.miniserver_modal.host.clone(),
            username: self.miniserver_modal.username.clone(),
            password: self.miniserver_modal.password.clone(),
        };
        self.live_poll_in_flight = true;
        self.last_live_poll = Some(std::time::Instant::now());
        let tx = self.worker_tx.clone();
        std::thread::spawn(move || {
            let updates = MiniserverClient::fetch_batch_io_states(&creds, &ids)
                .into_iter()
                .filter_map(|(id, res)| res.ok().map(|snap| (id, snap)))
                .collect();
            let _ = tx.send(MiniserverWorkerEvent::LiveValuesPolled(updates));
        });
    }

    pub fn handle_live_command(&mut self, cmd: LiveCommand) {
        let optimistic_val = match cmd.command.as_str() {
            "On" | "Pulse" | "on" | "pulse" => "1",
            "Off" | "off" => "0",
            other => other,
        };
        if let Some(pin_key) = &cmd.pin_key {
            self.live_store
                .set_pin_override(&cmd.control_uuid, pin_key, optimistic_val);
        } else {
            self.live_store
                .set_control_value(&cmd.control_uuid, optimistic_val);
            self.live_store
                .set_pin_override(&cmd.control_uuid, "O", optimistic_val);
            self.live_store
                .set_pin_override(&cmd.control_uuid, "Q", optimistic_val);
            self.live_store
                .set_pin_override(&cmd.control_uuid, "AQ", optimistic_val);
        }

        let mut target_io_id = cmd.control_uuid.clone();
        if let Some(proj) = &self.project {
            // Find if cmd.control_uuid is a PageBlock with a ref_uuid and badge
            let mut matched_ref_and_badge: Option<(String, String)> = None;
            for page in &proj.pages {
                if let Some(blk) = page.blocks.iter().find(|b| b.uuid == cmd.control_uuid) {
                    if let Some(ref_u) = &blk.ref_uuid {
                        let badge = loxone_core::miniserver::ref_bar_capsule_badge(blk, page, proj);
                        matched_ref_and_badge = Some((ref_u.clone(), badge));
                        target_io_id = ref_u.clone();
                    }
                    break;
                }
            }

            if let Some((ref_u, badge)) = matched_ref_and_badge {
                // Count how many distinct badges share `ref_u` across the project
                let mut shares_multi_channel_device = false;
                for page in &proj.pages {
                    for blk in &page.blocks {
                        if blk.ref_uuid.as_deref() == Some(ref_u.as_str()) {
                            let other_badge =
                                loxone_core::miniserver::ref_bar_capsule_badge(blk, page, proj);
                            if other_badge != badge {
                                shares_multi_channel_device = true;
                            } else if blk.uuid != cmd.control_uuid {
                                self.live_store.set_control_value(&blk.uuid, optimistic_val);
                            }
                        }
                    }
                }
                if !shares_multi_channel_device {
                    self.live_store.set_control_value(&ref_u, optimistic_val);
                    self.live_store
                        .set_pin_override(&ref_u, "O", optimistic_val);
                    self.live_store
                        .set_pin_override(&ref_u, "Q", optimistic_val);
                }
            } else {
                // cmd.control_uuid may be a TreeNode UUID directly
                for page in &proj.pages {
                    for blk in &page.blocks {
                        if blk.ref_uuid.as_deref() == Some(cmd.control_uuid.as_str()) {
                            self.live_store.set_control_value(&blk.uuid, optimistic_val);
                        }
                    }
                }
            }
        }

        if self.connection_status.is_connected() && !self.miniserver_modal.host.trim().is_empty() {
            let creds = MiniserverCredentials {
                host: self.miniserver_modal.host.clone(),
                username: self.miniserver_modal.username.clone(),
                password: self.miniserver_modal.password.clone(),
            };
            let store_key = cmd.control_uuid.clone();
            let command = cmd.command.clone();
            let tx = self.worker_tx.clone();
            std::thread::spawn(move || {
                let res = MiniserverClient::send_io_command(&creds, &target_io_id, &command)
                    .map(|snap| (store_key, snap))
                    .map_err(|e| e.to_string());
                let _ = tx.send(MiniserverWorkerEvent::LiveCommandSent(res));
            });
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
            TopBarAction::AddBlock => {
                if self.project.is_some() {
                    self.insert_modal.open_at(None);
                }
            }
            TopBarAction::Undo => {
                self.undo();
            }
            TopBarAction::Redo => {
                self.redo();
            }
            TopBarAction::OpenMiniserverModal => {
                self.open_miniserver_modal();
            }
            TopBarAction::ToggleLiveView => {
                self.live_view_enabled = !self.live_view_enabled;
                if self.live_view_enabled {
                    if let Some(proj) = &self.project {
                        if self.live_store.controls.is_empty() {
                            self.live_store.seed_project_defaults(proj);
                        }
                    }
                    self.last_live_poll = None;
                }
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
            TopBarAction::ZoomIn => {
                self.viewport.zoom = (self.viewport.zoom * 1.25).clamp(0.15, 3.0);
            }
            TopBarAction::ZoomOut => {
                self.viewport.zoom = (self.viewport.zoom / 1.25).clamp(0.15, 3.0);
            }
            TopBarAction::ResetZoom => {
                self.execute_palette_action(PaletteAction::ResetZoom);
            }
        }
    }

    pub fn delete_object_by_uuid(&mut self, uuid: &str) {
        let Some(proj) = &self.project else {
            return;
        };
        let Some(summary) = proj.xref.objects_by_uuid.get(uuid).cloned() else {
            return;
        };
        match summary.location {
            ObjectLocation::PageBlock { page_index, .. } => {
                if let Some(page) = proj.pages.get(page_index) {
                    let page_uuid = page.uuid.clone();
                    self.execute_edit(ProjectEdit::RemovePageBlock {
                        page_uuid,
                        block_uuid: uuid.to_string(),
                    });
                }
            }
            ObjectLocation::Page { .. } => {
                if proj.pages.len() <= 1 {
                    self.error_banner =
                        Some("Cannot delete the last remaining page.".to_string());
                    return;
                }
                self.execute_edit(ProjectEdit::RemovePage {
                    page_uuid: uuid.to_string(),
                });
                if let Some(p) = &self.project {
                    if self.active_page_index >= p.pages.len() && !p.pages.is_empty() {
                        self.active_page_index = p.pages.len() - 1;
                    }
                }
            }
            ObjectLocation::Place => {
                self.execute_edit(ProjectEdit::RemoveRoom {
                    room_uuid: uuid.to_string(),
                });
            }
            ObjectLocation::Category => {
                self.execute_edit(ProjectEdit::RemoveCategory {
                    category_uuid: uuid.to_string(),
                });
            }
            ObjectLocation::TreeNode { .. } => {
                self.execute_edit(ProjectEdit::RemoveTreeNode {
                    node_uuid: uuid.to_string(),
                });
            }
            ObjectLocation::DocumentRoot => return,
        }
        if self.selected_uuid.as_deref() == Some(uuid) {
            self.selected_uuid = None;
        }
    }

    pub fn place_tree_node_on_active_page(
        &mut self,
        target_uuid: &str,
        pos: Option<(f32, f32)>,
    ) {
        let Some(proj) = &self.project else {
            return;
        };
        let Some(node) = find_tree_node_recursive(&proj.periphery_tree, target_uuid)
            .or_else(|| find_tree_node_recursive(&proj.system_tree, target_uuid))
            .cloned()
        else {
            return;
        };
        let Some(ref_type) = crate::insert_modal::reference_kind_for_tree_node(&node.control_type)
        else {
            return;
        };
        let Some(page) = proj.pages.get(self.active_page_index) else {
            return;
        };
        let page_uuid = page.uuid.clone();
        let (px, py) = pos.unwrap_or((192.0, 192.0));
        let block = loxone_core::factory::new_reference_block(ref_type, &node, px, py);
        let new_uuid = block.uuid.clone();
        self.execute_edit(ProjectEdit::AddPageBlock { page_uuid, block });
        self.selected_uuid = Some(new_uuid);
        self.show_right_panel = true;
    }

    pub fn handle_insert_selection(&mut self, sel: InsertModalSelection) {
        let Some(proj) = &self.project else {
            return;
        };
        let Some(page) = proj.pages.get(self.active_page_index) else {
            return;
        };
        let page_uuid = page.uuid.clone();
        match sel {
            InsertModalSelection::NewFunctionBlock {
                type_name,
                title,
                px,
                py,
            } => {
                let block = loxone_core::factory::new_page_block(&type_name, &title, px, py);
                let new_uuid = block.uuid.clone();
                self.execute_edit(ProjectEdit::AddPageBlock { page_uuid, block });
                self.selected_uuid = Some(new_uuid);
                self.show_right_panel = true;
            }
            InsertModalSelection::NewReferenceBlock {
                ref_type,
                target_uuid,
                px,
                py,
            } => {
                if let Some(node) = find_tree_node_recursive(&proj.periphery_tree, &target_uuid)
                    .or_else(|| find_tree_node_recursive(&proj.system_tree, &target_uuid))
                    .cloned()
                {
                    let block =
                        loxone_core::factory::new_reference_block(&ref_type, &node, px, py);
                    let new_uuid = block.uuid.clone();
                    self.execute_edit(ProjectEdit::AddPageBlock { page_uuid, block });
                    self.selected_uuid = Some(new_uuid);
                    self.show_right_panel = true;
                }
            }
        }
    }

    pub fn handle_tree_action(&mut self, act: TreeAction) {
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
            TreeAction::CreatePage => {
                let n = self.next_created_counter;
                self.next_created_counter += 1;
                let page = loxone_core::factory::new_page(&format!("New Page {}", n));
                let page_uuid = page.uuid.clone();
                self.execute_edit(ProjectEdit::AddPage { page });
                if let Some(proj) = &self.project {
                    if let Some(idx) = proj.pages.iter().position(|p| p.uuid == page_uuid) {
                        self.active_page_index = idx;
                    }
                }
                self.selected_uuid = Some(page_uuid);
                self.show_right_panel = true;
                self.needs_initial_fit = true;
            }
            TreeAction::CreateRoom => {
                let n = self.next_created_counter;
                self.next_created_counter += 1;
                let room = loxone_core::factory::new_room(&format!("New Room {}", n));
                let uuid = room.uuid.clone();
                self.execute_edit(ProjectEdit::AddRoom { room });
                self.selected_uuid = Some(uuid);
                self.show_right_panel = true;
            }
            TreeAction::CreateCategory => {
                let n = self.next_created_counter;
                self.next_created_counter += 1;
                let category =
                    loxone_core::factory::new_category(&format!("New Category {}", n), "lights");
                let uuid = category.uuid.clone();
                self.execute_edit(ProjectEdit::AddCategory { category });
                self.selected_uuid = Some(uuid);
                self.show_right_panel = true;
            }
            TreeAction::CreateMemoryFlag => {
                let n = self.next_created_counter;
                self.next_created_counter += 1;
                let node =
                    loxone_core::factory::new_tree_node("Memory", &format!("Memory Flag {}", n));
                let uuid = node.uuid.clone();
                let parent_uuid = self
                    .project
                    .as_ref()
                    .and_then(|p| find_tree_node_by_type_recursive(&p.system_tree, "MemoryCaption"))
                    .map(|n| n.uuid.clone());
                self.execute_edit(ProjectEdit::AddTreeNode {
                    section: TreeSectionKind::System,
                    parent_uuid,
                    node,
                });
                self.selected_uuid = Some(uuid);
                self.show_right_panel = true;
            }
            TreeAction::CreateVirtualIn => {
                let n = self.next_created_counter;
                self.next_created_counter += 1;
                let node = loxone_core::factory::new_tree_node(
                    "VirtualIn",
                    &format!("Virtual Input {}", n),
                );
                let uuid = node.uuid.clone();
                let parent_uuid = self.project.as_ref().and_then(|p| {
                    find_tree_node_by_type_recursive(&p.periphery_tree, "VirtualInCaption")
                        .or_else(|| find_tree_node_by_type_recursive(&p.periphery_tree, "LoxLIVE"))
                        .map(|n| n.uuid.clone())
                });
                self.execute_edit(ProjectEdit::AddTreeNode {
                    section: TreeSectionKind::Periphery,
                    parent_uuid,
                    node,
                });
                self.selected_uuid = Some(uuid);
                self.show_right_panel = true;
            }
            TreeAction::CreateVirtualOut => {
                let n = self.next_created_counter;
                self.next_created_counter += 1;
                let node = loxone_core::factory::new_tree_node(
                    "VirtualOut",
                    &format!("Virtual Output {}", n),
                );
                let uuid = node.uuid.clone();
                let parent_uuid = self.project.as_ref().and_then(|p| {
                    find_tree_node_by_type_recursive(&p.periphery_tree, "VirtualOutCaption")
                        .or_else(|| find_tree_node_by_type_recursive(&p.periphery_tree, "LoxLIVE"))
                        .map(|n| n.uuid.clone())
                });
                self.execute_edit(ProjectEdit::AddTreeNode {
                    section: TreeSectionKind::Periphery,
                    parent_uuid,
                    node,
                });
                self.selected_uuid = Some(uuid);
                self.show_right_panel = true;
            }
            TreeAction::PlaceOnPage(target_uuid) => {
                self.place_tree_node_on_active_page(&target_uuid, None);
            }
        }
    }

    pub fn handle_canvas_action(&mut self, canvas_act: CanvasAction) {
        match canvas_act {
            CanvasAction::SelectObject(uuid) => {
                self.selected_uuid = Some(uuid);
                self.show_right_panel = true;
            }
            CanvasAction::ClearSelection => {
                self.selected_uuid = None;
                self.viewport.interaction.selected_blocks.clear();
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
            CanvasAction::MoveBlock {
                page_uuid,
                block_uuid,
                px,
                py,
            } => {
                self.execute_edit(ProjectEdit::MovePageBlock {
                    page_uuid,
                    block_uuid,
                    px,
                    py,
                });
            }
            CanvasAction::MoveBlocks { page_uuid, moves } => {
                let edits: Vec<ProjectEdit> = moves
                    .into_iter()
                    .map(|(block_uuid, px, py)| ProjectEdit::MovePageBlock {
                        page_uuid: page_uuid.clone(),
                        block_uuid,
                        px,
                        py,
                    })
                    .collect();
                if !edits.is_empty() {
                    self.execute_edit(ProjectEdit::Batch(edits));
                }
            }
            CanvasAction::DeleteBlock { block_uuid, .. } => {
                self.delete_object_by_uuid(&block_uuid);
                self.viewport
                    .interaction
                    .selected_blocks
                    .retain(|u| u != &block_uuid);
            }
            CanvasAction::DeleteBlocks {
                page_uuid,
                block_uuids,
            } => {
                let edits: Vec<ProjectEdit> = block_uuids
                    .iter()
                    .map(|block_uuid| ProjectEdit::RemovePageBlock {
                        page_uuid: page_uuid.clone(),
                        block_uuid: block_uuid.clone(),
                    })
                    .collect();
                if !edits.is_empty() {
                    self.execute_edit(ProjectEdit::Batch(edits));
                }
                if let Some(sel) = &self.selected_uuid {
                    if block_uuids.contains(sel) {
                        self.selected_uuid = None;
                    }
                }
                self.viewport.interaction.selected_blocks.clear();
            }
            CanvasAction::ConnectPins {
                target_block_uuid,
                target_pin_key,
                source_pin_uuid,
            } => {
                let mut resolved_src_uuid = source_pin_uuid.clone();
                if let Some((src_block_uuid, src_pin_key)) = source_pin_uuid.split_once(':') {
                    if let Some(proj) = &mut self.project {
                        let mut found = None;
                        for page in &mut proj.pages {
                            if let Some(blk) =
                                page.blocks.iter_mut().find(|b| b.uuid == src_block_uuid)
                            {
                                loxone_core::factory::ensure_schema_pins_on_block(blk);
                                if let Some(pin) = blk.pins.iter().find(|p| p.key == src_pin_key) {
                                    found = Some(pin.uuid.clone());
                                }
                                break;
                            }
                        }
                        if let Some(real_uuid) = found {
                            proj.xref = loxone_core::xref::XrefIndex::build(proj);
                            resolved_src_uuid = real_uuid;
                        }
                    }
                }
                self.execute_edit(ProjectEdit::AddWireLink {
                    target_block_uuid,
                    target_pin_key,
                    source_pin_uuid: resolved_src_uuid,
                });
            }
            CanvasAction::DisconnectWire {
                target_block_uuid,
                target_pin_key,
                source_pin_uuid,
            } => {
                self.execute_edit(ProjectEdit::RemoveWireLink {
                    target_block_uuid,
                    target_pin_key,
                    source_pin_uuid,
                });
            }
            CanvasAction::OpenInsertModalAt { px, py } => {
                self.insert_modal.open_at(Some((px, py)));
            }
            CanvasAction::TriggerLiveCommand {
                control_uuid,
                pin_key,
                command,
            } => {
                self.handle_live_command(LiveCommand {
                    control_uuid,
                    pin_key,
                    command,
                });
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
                if let Some(existing_path) = self.loaded_path.clone() {
                    if let Err(err) = self.save_xml_to_path(&existing_path) {
                        self.error_banner = Some(err);
                    }
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
                self.viewport.interaction.selected_blocks.clear();
                self.needs_initial_fit = true;
            }
            PaletteAction::NavigateToBlock {
                page_index,
                block_uuid,
            } => {
                self.active_page_index = page_index;
                self.selected_uuid = Some(block_uuid.clone());
                self.viewport.interaction.selected_blocks = vec![block_uuid.clone()];
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
                self.insert_modal.close();
                self.palette.open();
            }
        }
        if self.palette.is_open {
            return;
        }

        if cmd_or_ctrl && shift && ctx.input(|i| i.key_pressed(Key::D)) {
            self.open_miniserver_modal();
        }
        if self.miniserver_modal.is_open || self.insert_modal.is_open {
            return;
        }

        if cmd_or_ctrl && ctx.input(|i| i.key_pressed(Key::Z)) {
            if shift {
                self.redo();
            } else {
                self.undo();
            }
        }
        if cmd_or_ctrl && ctx.input(|i| i.key_pressed(Key::Y)) {
            self.redo();
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
            if cmd_or_ctrl && ctx.input(|i| i.key_pressed(Key::A)) {
                if let Some(proj) = &self.project {
                    if let Some(page) = proj.pages.get(self.active_page_index) {
                        let all_uuids: Vec<String> = page
                            .blocks
                            .iter()
                            .filter(|b| b.has_position)
                            .map(|b| b.uuid.clone())
                            .collect();
                        if let Some(first) = all_uuids.first() {
                            self.selected_uuid = Some(first.clone());
                        }
                        self.viewport.interaction.selected_blocks = all_uuids;
                    }
                }
            }
            if !cmd_or_ctrl && ctx.input(|i| i.key_pressed(Key::A)) && self.project.is_some() {
                self.insert_modal.open_at(None);
            }
            if ctx.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace)) {
                if let Some(wire) = self.viewport.interaction.selected_wire.take() {
                    self.execute_edit(ProjectEdit::RemoveWireLink {
                        target_block_uuid: wire.target_block_uuid,
                        target_pin_key: wire.target_pin_key,
                        source_pin_uuid: wire.source_pin_uuid,
                    });
                } else if self.viewport.interaction.selected_blocks.len() > 1 {
                    let page_uuid = self
                        .project
                        .as_ref()
                        .and_then(|p| p.pages.get(self.active_page_index))
                        .map(|p| p.uuid.clone());
                    if let Some(page_uuid) = page_uuid {
                        let block_uuids = self.viewport.interaction.selected_blocks.clone();
                        self.handle_canvas_action(CanvasAction::DeleteBlocks {
                            page_uuid,
                            block_uuids,
                        });
                    }
                } else if let Some(sel_uuid) = self.selected_uuid.clone() {
                    self.delete_object_by_uuid(&sel_uuid);
                    self.viewport.interaction.selected_blocks.clear();
                }
            }
            if ctx.input(|i| i.key_pressed(Key::F)) {
                self.needs_initial_fit = true;
            }
            if ctx.input(|i| i.key_pressed(Key::P)) {
                self.viewport.compact_pins = !self.viewport.compact_pins;
            }
            if !cmd_or_ctrl && ctx.input(|i| i.key_pressed(Key::L)) && self.project.is_some() {
                self.execute_top_bar_action(TopBarAction::ToggleLiveView);
            }
            if ctx.input(|i| i.key_pressed(Key::Num0)) {
                self.execute_palette_action(PaletteAction::ResetZoom);
            }
            if ctx.input(|i| i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals)) {
                self.execute_top_bar_action(TopBarAction::ZoomIn);
            }
            if ctx.input(|i| i.key_pressed(Key::Minus)) {
                self.execute_top_bar_action(TopBarAction::ZoomOut);
            }
        }
    }
}

fn find_tree_node_recursive<'a>(nodes: &'a [TreeNode], uuid: &str) -> Option<&'a TreeNode> {
    for node in nodes {
        if node.uuid == uuid {
            return Some(node);
        }
        if let Some(found) = find_tree_node_recursive(&node.children, uuid) {
            return Some(found);
        }
    }
    None
}

fn find_tree_node_by_type_recursive<'a>(
    nodes: &'a [TreeNode],
    control_type: &str,
) -> Option<&'a TreeNode> {
    for node in nodes {
        if node.control_type == control_type && !node.uuid.is_empty() {
            return Some(node);
        }
        if let Some(found) = find_tree_node_by_type_recursive(&node.children, control_type) {
            return Some(found);
        }
    }
    None
}

impl eframe::App for LoxonePortableApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.poll_miniserver_worker();
        if self.connection_status.is_connecting() {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }
        self.maybe_schedule_live_poll(ctx);

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
            self.is_dirty,
            self.can_undo(),
            self.can_redo(),
            self.live_view_enabled,
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
            let mut tree_act = None;
            let live_opt = self.live_view_enabled.then_some(&self.live_store);
            if let Some(proj) = &self.project {
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
                            live_opt,
                        );
                    });
            }

            if let Some(act) = tree_act {
                self.handle_tree_action(act);
            }
        }

        // Right Inspector Panel
        if self.show_right_panel {
            let mut insp_out = crate::inspector::InspectorOutput::default();
            let live_opt = self.live_view_enabled.then_some(&self.live_store);
            if let Some(proj) = &self.project {
                egui::SidePanel::right("right_inspector")
                    .default_width(300.0)
                    .frame(
                        egui::Frame::new()
                            .fill(PANEL_BG)
                            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                            .inner_margin(egui::Margin::symmetric(10, 6)),
                    )
                    .show(ctx, |ui| {
                        insp_out = render_right_inspector(
                            ui,
                            proj,
                            self.selected_uuid.as_deref(),
                            live_opt,
                        );
                    });
            }

            for edit in insp_out.edits {
                self.apply_project_edit(edit);
            }

            for cmd in insp_out.live_commands {
                self.handle_live_command(cmd);
            }

            if let Some(del_uuid) = insp_out.delete_uuid {
                self.delete_object_by_uuid(&del_uuid);
            }

            if let Some(place_uuid) = insp_out.place_on_page_uuid {
                self.place_tree_node_on_active_page(&place_uuid, None);
            }

            if let Some(act) = insp_out.nav_action {
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

        // Central Schematic Page Canvas
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(crate::theme::CANVAS_BG))
            .show(ctx, |ui| {
                let view_size = ui.available_size();
                let mut pending_canvas_act = None;
                let live_opt = self.live_view_enabled.then_some(&self.live_store);
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

                    pending_canvas_act = render_page_canvas(
                        ui,
                        proj,
                        self.active_page_index,
                        &mut self.viewport,
                        self.selected_uuid.as_deref(),
                        live_opt,
                    );
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

                if let Some(canvas_act) = pending_canvas_act {
                    self.handle_canvas_action(canvas_act);
                }
            });

        // Insert Block / Reference Modal Overlay
        if self.insert_modal.is_open {
            if let Some(sel) =
                show_insert_modal(ctx, &mut self.insert_modal, self.project.as_ref())
            {
                self.handle_insert_selection(sel);
            }
        }

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
        let live_opt = Some(&app.live_store);
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
                        live_opt,
                    );
                }
                let _ = render_right_inspector(ui, proj, Some(&first_block_uuid), live_opt);
                let _ = render_right_inspector(ui, proj, Some(&tree_uuid), live_opt);
                let _ = render_right_inspector(ui, proj, Some(&cat_uuid), live_opt);
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

    #[test]
    fn test_app_apply_project_edit_dirty_state_and_save_roundtrip() {
        use loxone_core::writer::ProjectEdit;

        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let mut app = LoxonePortableApp::default();
        assert!(!app.is_dirty);

        app.load_file(&path);
        assert!(!app.is_dirty);

        let target_block_uuid = app.project.as_ref().unwrap().pages[0].blocks[0].uuid.clone();
        let target_pin_key = app.project.as_ref().unwrap().pages[0].blocks[0].pins[0]
            .key
            .clone();
        let first_room_uuid = app
            .project
            .as_ref()
            .unwrap()
            .places
            .keys()
            .next()
            .unwrap()
            .clone();

        app.apply_project_edit(ProjectEdit::SetControlAttr {
            control_uuid: target_block_uuid.clone(),
            attr_name: "Title".to_string(),
            value: Some("App Edited Block".to_string()),
        });
        assert!(app.is_dirty);

        app.apply_project_edit(ProjectEdit::SetPinDefault {
            control_uuid: target_block_uuid.clone(),
            pin_key: target_pin_key.clone(),
            default_value: Some("77.5".to_string()),
        });
        app.apply_project_edit(ProjectEdit::SetChildTagAttr {
            control_uuid: target_block_uuid.clone(),
            child_tag: "IoData".to_string(),
            attr_name: "Pr".to_string(),
            value: Some(first_room_uuid.clone()),
        });

        let temp_file = std::env::temp_dir().join(format!(
            "loxone_app_dirty_roundtrip_{}.Loxone",
            std::process::id()
        ));
        app.save_xml_to_path(&temp_file)
            .expect("save_xml_to_path should succeed on dirty project");
        assert!(!app.is_dirty);
        assert_eq!(app.loaded_path.as_deref(), Some(temp_file.as_path()));

        // Also verify PaletteAction::SaveFile saves directly to existing loaded_path without dialog
        app.apply_project_edit(ProjectEdit::SetControlAttr {
            control_uuid: target_block_uuid.clone(),
            attr_name: "Desc".to_string(),
            value: Some("Saved via PaletteAction".to_string()),
        });
        assert!(app.is_dirty);
        app.execute_palette_action(PaletteAction::SaveFile);
        assert!(!app.is_dirty);

        let mut reloaded_app = LoxonePortableApp::default();
        reloaded_app.load_file(&temp_file);
        let _ = std::fs::remove_file(&temp_file);

        assert!(!reloaded_app.is_dirty);
        let blk = &reloaded_app.project.as_ref().unwrap().pages[0].blocks[0];
        assert_eq!(blk.uuid, target_block_uuid);
        assert_eq!(blk.title, "App Edited Block");
        assert_eq!(
            blk.raw_attributes
                .iter()
                .find(|(k, _)| k == "Desc")
                .map(|(_, v)| v.as_str()),
            Some("Saved via PaletteAction")
        );
        assert_eq!(blk.pins[0].default_value.as_deref(), Some("77.5"));
        assert_eq!(blk.room_uuid.as_deref(), Some(first_room_uuid.as_str()));
    }

    #[test]
    fn test_app_execute_edit_and_undo_redo_updates_dirty_and_restores_state() {
        use loxone_core::factory::new_page_block;
        use loxone_core::writer::ProjectEdit;

        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let mut app = LoxonePortableApp::default();
        app.load_file(&path);
        assert!(!app.is_dirty);
        assert!(!app.can_undo());
        assert!(!app.can_redo());

        let page_uuid = app.project.as_ref().unwrap().pages[0].uuid.clone();
        let initial_count = app.project.as_ref().unwrap().pages[0].blocks.len();
        let new_blk = new_page_block("StairwellLight", "Hallway Stair Light", 256.0, 256.0);
        let new_blk_uuid = new_blk.uuid.clone();

        app.execute_edit(ProjectEdit::AddPageBlock {
            page_uuid: page_uuid.clone(),
            block: new_blk,
        });

        assert!(app.is_dirty);
        assert!(app.can_undo());
        assert!(!app.can_redo());
        assert_eq!(
            app.project.as_ref().unwrap().pages[0].blocks.len(),
            initial_count + 1
        );
        assert!(app.project.as_ref().unwrap().pages[0]
            .blocks
            .iter()
            .any(|b| b.uuid == new_blk_uuid));

        app.undo();
        assert!(app.is_dirty);
        assert!(!app.can_undo());
        assert!(app.can_redo());
        assert_eq!(
            app.project.as_ref().unwrap().pages[0].blocks.len(),
            initial_count
        );
        assert!(!app.project.as_ref().unwrap().pages[0]
            .blocks
            .iter()
            .any(|b| b.uuid == new_blk_uuid));

        app.redo();
        assert!(app.is_dirty);
        assert!(app.can_undo());
        assert!(!app.can_redo());
        assert_eq!(
            app.project.as_ref().unwrap().pages[0].blocks.len(),
            initial_count + 1
        );
        assert!(app.project.as_ref().unwrap().pages[0]
            .blocks
            .iter()
            .any(|b| b.uuid == new_blk_uuid));
    }

    #[test]
    fn test_app_authoring_tree_canvas_insert_and_xml_roundtrip() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let mut app = LoxonePortableApp::default();
        app.load_file(&path);

        let initial_pages = app.project.as_ref().unwrap().pages.len();
        let initial_places = app.project.as_ref().unwrap().places.len();
        let initial_categories = app.project.as_ref().unwrap().categories.len();

        // 1. Create a new page via TreeAction::CreatePage
        app.handle_tree_action(TreeAction::CreatePage);
        assert_eq!(app.project.as_ref().unwrap().pages.len(), initial_pages + 1);
        assert_eq!(app.active_page_index, initial_pages);
        let created_page_uuid = app.project.as_ref().unwrap().pages[initial_pages].uuid.clone();

        // 2. Create a room, category, memory flag, virtual in, and virtual out via TreeAction
        app.handle_tree_action(TreeAction::CreateRoom);
        let created_room_uuid = app.selected_uuid.clone().unwrap();
        assert_eq!(app.project.as_ref().unwrap().places.len(), initial_places + 1);

        app.handle_tree_action(TreeAction::CreateCategory);
        let created_cat_uuid = app.selected_uuid.clone().unwrap();
        assert_eq!(
            app.project.as_ref().unwrap().categories.len(),
            initial_categories + 1
        );

        app.handle_tree_action(TreeAction::CreateMemoryFlag);
        let created_flag_uuid = app.selected_uuid.clone().unwrap();

        app.handle_tree_action(TreeAction::CreateVirtualIn);
        let created_vi_uuid = app.selected_uuid.clone().unwrap();

        app.handle_tree_action(TreeAction::CreateVirtualOut);
        let created_vo_uuid = app.selected_uuid.clone().unwrap();

        // 3. Place the created Memory Flag on the new page via TreeAction::PlaceOnPage
        app.handle_tree_action(TreeAction::PlaceOnPage(created_flag_uuid.clone()));
        let placed_ref_uuid = app.selected_uuid.clone().unwrap();
        assert_eq!(
            app.project.as_ref().unwrap().pages[initial_pages].blocks.len(),
            1
        );

        // 4. Open insert modal at canvas coords and insert a StairwellLight function block
        app.handle_canvas_action(CanvasAction::OpenInsertModalAt {
            px: 480.0,
            py: 240.0,
        });
        assert!(app.insert_modal.is_open);
        app.handle_insert_selection(InsertModalSelection::NewFunctionBlock {
            type_name: "StairwellLight".to_string(),
            title: "Stairwell Light".to_string(),
            px: 480.0,
            py: 240.0,
        });
        let stair_uuid = app.selected_uuid.clone().unwrap();
        assert_eq!(
            app.project.as_ref().unwrap().pages[initial_pages].blocks.len(),
            2
        );

        // 5. Connect wire from placed MemoryFlag ("Q") to StairwellLight ("Dis")
        let source_pin_uuid = format!("{}:Q", placed_ref_uuid);
        app.handle_canvas_action(CanvasAction::ConnectPins {
            target_block_uuid: stair_uuid.clone(),
            target_pin_key: "Dis".to_string(),
            source_pin_uuid,
        });
        let dis_pin = app.project.as_ref().unwrap().pages[initial_pages]
            .blocks
            .iter()
            .find(|b| b.uuid == stair_uuid)
            .unwrap()
            .pins
            .iter()
            .find(|p| p.key == "Dis")
            .unwrap();
        assert_eq!(dis_pin.incoming_links.len(), 1);

        // 6. Move StairwellLight block via CanvasAction::MoveBlock
        app.handle_canvas_action(CanvasAction::MoveBlock {
            page_uuid: created_page_uuid.clone(),
            block_uuid: stair_uuid.clone(),
            px: 640.0,
            py: 320.0,
        });
        let moved_blk = app.project.as_ref().unwrap().pages[initial_pages]
            .blocks
            .iter()
            .find(|b| b.uuid == stair_uuid)
            .unwrap();
        assert_eq!((moved_blk.px, moved_blk.py), (640.0, 320.0));

        // 7. Undo move and redo move via TopBarAction
        app.execute_top_bar_action(TopBarAction::Undo);
        let undone_blk = app.project.as_ref().unwrap().pages[initial_pages]
            .blocks
            .iter()
            .find(|b| b.uuid == stair_uuid)
            .unwrap();
        assert_eq!((undone_blk.px, undone_blk.py), (480.0, 240.0));

        app.execute_top_bar_action(TopBarAction::Redo);
        let redone_blk = app.project.as_ref().unwrap().pages[initial_pages]
            .blocks
            .iter()
            .find(|b| b.uuid == stair_uuid)
            .unwrap();
        assert_eq!((redone_blk.px, redone_blk.py), (640.0, 320.0));

        // 8. Save modified project to disk and reload in a fresh LoxonePortableApp
        let temp_file = std::env::temp_dir().join(format!(
            "loxone_app_full_authoring_roundtrip_{}.Loxone",
            std::process::id()
        ));
        app.save_xml_to_path(&temp_file)
            .expect("save_xml_to_path should succeed after full authoring");

        let mut reloaded = LoxonePortableApp::default();
        reloaded.load_file(&temp_file);
        let _ = std::fs::remove_file(&temp_file);

        let rproj = reloaded.project.as_ref().unwrap();
        assert!(rproj.pages.iter().any(|p| p.uuid == created_page_uuid));
        assert!(rproj.places.contains_key(&created_room_uuid));
        assert!(rproj.categories.contains_key(&created_cat_uuid));
        assert!(find_tree_node_recursive(&rproj.system_tree, &created_flag_uuid).is_some());
        assert!(find_tree_node_recursive(&rproj.periphery_tree, &created_vi_uuid).is_some());
        assert!(find_tree_node_recursive(&rproj.periphery_tree, &created_vo_uuid).is_some());

        let rpage = rproj
            .pages
            .iter()
            .find(|p| p.uuid == created_page_uuid)
            .unwrap();
        assert_eq!(rpage.blocks.len(), 2);
        let r_stair = rpage.blocks.iter().find(|b| b.uuid == stair_uuid).unwrap();
        assert_eq!((r_stair.px, r_stair.py), (640.0, 320.0));
        let r_dis = r_stair.pins.iter().find(|p| p.key == "Dis").unwrap();
        assert_eq!(r_dis.incoming_links.len(), 1);
    }

    #[test]
    fn test_app_multi_block_move_and_delete_batch_undo_redo() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.pop();
        path.pop();
        path.push("Klimczaki.Loxone");
        if !path.exists() {
            return;
        }

        let mut app = LoxonePortableApp::default();
        app.load_file(&path);
        app.handle_tree_action(TreeAction::CreatePage);
        let page_idx = app.active_page_index;
        let page_uuid = app.project.as_ref().unwrap().pages[page_idx].uuid.clone();

        app.handle_insert_selection(InsertModalSelection::NewFunctionBlock {
            type_name: "Pushbutton".to_string(),
            title: "Btn 1".to_string(),
            px: 128.0,
            py: 128.0,
        });
        let b1_uuid = app.selected_uuid.clone().unwrap();

        app.handle_insert_selection(InsertModalSelection::NewFunctionBlock {
            type_name: "StairwellLight".to_string(),
            title: "Light 1".to_string(),
            px: 512.0,
            py: 128.0,
        });
        let b2_uuid = app.selected_uuid.clone().unwrap();

        // Group move both blocks together in a single batch
        app.handle_canvas_action(CanvasAction::MoveBlocks {
            page_uuid: page_uuid.clone(),
            moves: vec![
                (b1_uuid.clone(), 256.0, 320.0),
                (b2_uuid.clone(), 640.0, 320.0),
            ],
        });

        {
            let page = &app.project.as_ref().unwrap().pages[page_idx];
            let b1 = page.blocks.iter().find(|b| b.uuid == b1_uuid).unwrap();
            let b2 = page.blocks.iter().find(|b| b.uuid == b2_uuid).unwrap();
            assert_eq!((b1.px, b1.py), (256.0, 320.0));
            assert_eq!((b2.px, b2.py), (640.0, 320.0));
        }

        // Single Undo restores both blocks to their original positions
        app.undo();
        {
            let page = &app.project.as_ref().unwrap().pages[page_idx];
            let b1 = page.blocks.iter().find(|b| b.uuid == b1_uuid).unwrap();
            let b2 = page.blocks.iter().find(|b| b.uuid == b2_uuid).unwrap();
            assert_eq!((b1.px, b1.py), (128.0, 128.0));
            assert_eq!((b2.px, b2.py), (512.0, 128.0));
        }

        // Single Redo re-applies the group move
        app.redo();
        {
            let page = &app.project.as_ref().unwrap().pages[page_idx];
            let b1 = page.blocks.iter().find(|b| b.uuid == b1_uuid).unwrap();
            let b2 = page.blocks.iter().find(|b| b.uuid == b2_uuid).unwrap();
            assert_eq!((b1.px, b1.py), (256.0, 320.0));
            assert_eq!((b2.px, b2.py), (640.0, 320.0));
        }

        // Group delete both blocks in a single batch
        app.viewport.interaction.selected_blocks = vec![b1_uuid.clone(), b2_uuid.clone()];
        app.handle_canvas_action(CanvasAction::DeleteBlocks {
            page_uuid: page_uuid.clone(),
            block_uuids: vec![b1_uuid.clone(), b2_uuid.clone()],
        });
        assert!(app.viewport.interaction.selected_blocks.is_empty());
        assert!(app.project.as_ref().unwrap().pages[page_idx].blocks.is_empty());

        // Single Undo restores both deleted blocks
        app.undo();
        assert_eq!(app.project.as_ref().unwrap().pages[page_idx].blocks.len(), 2);
    }

    #[test]
    fn test_app_live_view_toggle_and_live_command_updates_store() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let mut app = LoxonePortableApp::default();
        app.load_file(&path);

        assert!(!app.live_view_enabled);
        assert!(!app.live_store.controls.is_empty());

        app.execute_top_bar_action(TopBarAction::ToggleLiveView);
        assert!(app.live_view_enabled);

        let ids = app.collect_active_page_poll_identifiers();
        assert!(!ids.is_empty());

        let first_id = ids[0].clone();
        app.handle_live_command(LiveCommand {
            control_uuid: first_id.clone(),
            pin_key: None,
            command: "On".to_string(),
        });
        assert_eq!(
            app.live_store
                .pin_overrides
                .get(&(first_id.clone(), "O".to_string()))
                .map(|s| s.as_str()),
            Some("1")
        );

        app.handle_live_command(LiveCommand {
            control_uuid: first_id.clone(),
            pin_key: None,
            command: "Off".to_string(),
        });
        assert_eq!(
            app.live_store
                .pin_overrides
                .get(&(first_id.clone(), "O".to_string()))
                .map(|s| s.as_str()),
            Some("0")
        );

        let mut outputs = std::collections::BTreeMap::new();
        outputs.insert("AQ".to_string(), "23.4".to_string());
        let snap = LiveControlSnapshot {
            control_id: first_id.clone(),
            primary_value: Some("23.4".to_string()),
            code: 200,
            outputs,
        };
        app.worker_tx
            .send(MiniserverWorkerEvent::LiveValuesPolled(vec![(
                first_id.clone(),
                snap,
            )]))
            .unwrap();
        app.poll_miniserver_worker();

        assert_eq!(
            app.live_store
                .controls
                .get(&first_id)
                .and_then(|c| c.outputs.get("AQ"))
                .map(|s| s.as_str()),
            Some("23.4")
        );

        // Verify on-canvas TriggerLiveCommand isolates sibling channels on a shared Touch Tree device
        let proj = app.project.as_ref().unwrap().clone();
        let agatka = proj
            .pages
            .iter()
            .find(|p| p.title == "Pokój Agatki")
            .unwrap();
        let sufit_p = agatka
            .blocks
            .iter()
            .find(|b| {
                b.is_input_ref_bar()
                    && loxone_core::miniserver::ref_bar_capsule_badge(b, agatka, &proj) == "P"
            })
            .unwrap();
        let temp_blk = agatka
            .blocks
            .iter()
            .find(|b| {
                b.is_input_ref_bar()
                    && loxone_core::miniserver::ref_bar_capsule_badge(b, agatka, &proj) == "ϑc"
            })
            .unwrap();

        app.handle_canvas_action(CanvasAction::TriggerLiveCommand {
            control_uuid: sufit_p.uuid.clone(),
            pin_key: None,
            command: "On".to_string(),
        });
        app.handle_canvas_action(CanvasAction::TriggerLiveCommand {
            control_uuid: temp_blk.uuid.clone(),
            pin_key: None,
            command: "21.6".to_string(),
        });

        let sig_p = app.live_store.resolve_ref_bar_signal(sufit_p, agatka, &proj);
        assert!(sig_p.is_active);
        assert_eq!(sig_p.formatted, "Wł.");

        let sig_temp = app
            .live_store
            .resolve_ref_bar_signal(temp_blk, agatka, &proj);
        assert!(sig_temp.is_active);
        assert_eq!(sig_temp.formatted, "21.6°");
    }
}

