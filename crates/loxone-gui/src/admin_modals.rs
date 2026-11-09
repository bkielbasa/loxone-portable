//! Project administration dialogs: project properties, users & rights,
//! operating modes & calendar, and the Miniserver network search.

use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use egui::{CornerRadius, Margin, RichText, Stroke};
use loxone_core::discovery::{discover_miniservers, DiscoveredMiniserver};
use loxone_core::model::{LoxoneProject, TreeNode};
use loxone_core::project_admin::{
    active_modes_on, build_add_todo_edit, build_autopilot_rule_edits, build_resolve_todo_edit,
    calendar_entries, collect_autopilot_rules, collect_project_todos, delete_user_edits,
    find_node_by_uuid, find_nodes_by_type, granted_permission_refs, group_member_uuids,
    new_calendar_entry_edit, new_operating_mode_edit, new_user_edit, new_user_group_edit,
    operating_modes, recurrence_label, set_group_membership_edit, set_permission_edit,
    AutopilotRuleSpec, USER_FIELDS,
};
use loxone_core::writer::ProjectEdit;

use crate::i18n::{tr, tr_fmt};
use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AdminModal {
    #[default]
    None,
    Properties,
    Users,
    Modes,
    Network,
    EditByType,
    FindAndReplace,
    ProjectBackups,
    AutoConfiguration,
    TodoList,
    AutopilotDesigner,
    ProjectDiff,
    TechReportWizard,
    MultiplierManager,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum UsersTab {
    #[default]
    Users,
    Groups,
}

pub enum AdminAction {
    Edit(ProjectEdit),
    UseMiniserver(String),
    RestoreBackupFile(std::path::PathBuf),
    SelectObject(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SameTypeObjectRow {
    pub uuid: String,
    pub is_page_block: bool,
    pub location_label: String,
    pub title: String,
    pub orig_title: String,
    pub desc: String,
    pub orig_desc: String,
    pub room_uuid: String,
    pub orig_room_uuid: String,
    pub category_uuid: String,
    pub orig_category_uuid: String,
    pub rating: String,
    pub orig_rating: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindReplaceMatch {
    pub uuid: String,
    pub object_type: String,
    pub location_label: String,
    pub field_name: String,
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone)]
pub struct ProjectBackupEntry {
    pub path: std::path::PathBuf,
    pub file_name: String,
    pub timestamp_display: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoConfigRoomRow {
    pub room_uuid: String,
    pub room_title: String,
    pub selected: bool,
    pub has_lighting: bool,
    pub has_shading: bool,
    pub has_climate: bool,
    pub has_audio: bool,
    pub gen_lighting: bool,
    pub gen_shading: bool,
    pub gen_climate: bool,
    pub gen_audio: bool,
}

pub struct AdminModalsState {
    pub open: AdminModal,
    pub status: Option<String>,
    props_draft: Vec<(String, String)>,
    users_tab: UsersTab,
    selected_user: Option<String>,
    selected_group: Option<String>,
    user_draft: Vec<(String, String)>,
    user_draft_for: Option<String>,
    new_name: String,
    new_cal_title: String,
    new_cal_mode: i32,
    preview: Option<(u32, u32)>,
    net_rx: Option<Receiver<Result<Vec<DiscoveredMiniserver>, String>>>,
    net_results: Vec<DiscoveredMiniserver>,
    net_status: Option<String>,
    // Edit Objects of Same Type state
    pub selected_edit_type: String,
    pub edit_type_filter: String,
    pub edit_by_type_rows: Vec<SameTypeObjectRow>,
    // Find & Replace state
    pub find_query: String,
    pub replace_with: String,
    pub find_case_sensitive: bool,
    pub find_in_titles: bool,
    pub find_in_descriptions: bool,
    pub find_in_attributes: bool,
    // Project Backups state
    pub backup_note: String,
    pub backups_list: Vec<ProjectBackupEntry>,
    // Auto Configuration Wizard state
    pub auto_config_rows: Vec<AutoConfigRoomRow>,
    // To-Do / Memo Manager state (Stage 9)
    pub todo_filter: String,
    pub todo_page_uuid_draft: String,
    pub todo_title_draft: String,
    pub todo_text_draft: String,
    // Autopilot Rule Designer state (Stage 9)
    pub ap_title_draft: String,
    pub ap_page_uuid_draft: String,
    pub ap_logic_op: String,
    pub ap_selected_conditions: Vec<String>,
    pub ap_target_block_uuid: String,
    pub ap_target_pin_key: String,
    // Project Diff / Compare state (Stage 10)
    pub diff_compare_path: String,
    pub diff_filter: String,
    pub diff_cached_report: Option<loxone_core::ProjectDiffReport>,
    // Technical Report & BOM / Terminal Schedule state (Stage 11)
    pub tech_report_tab: usize,
    pub tech_report_filter: String,
    pub tech_report_wired_only: bool,
    // Multiplier Project Manager state (Stage 13)
    pub mult_selected_uuid: Option<String>,
    pub mult_title_draft: String,
    pub mult_serial_draft: String,
    pub mult_address_draft: String,
    pub mult_desc_draft: String,
    pub mult_overrides_draft: Vec<(String, String)>,
}

impl Default for AdminModalsState {
    fn default() -> Self {
        Self {
            open: AdminModal::None,
            status: None,
            props_draft: Vec::new(),
            users_tab: UsersTab::Users,
            selected_user: None,
            selected_group: None,
            user_draft: Vec::new(),
            user_draft_for: None,
            new_name: String::new(),
            new_cal_title: String::new(),
            new_cal_mode: 0,
            preview: None,
            net_rx: None,
            net_results: Vec::new(),
            net_status: None,
            selected_edit_type: String::new(),
            edit_type_filter: String::new(),
            edit_by_type_rows: Vec::new(),
            find_query: String::new(),
            replace_with: String::new(),
            find_case_sensitive: false,
            find_in_titles: true,
            find_in_descriptions: true,
            find_in_attributes: false,
            backup_note: String::new(),
            backups_list: Vec::new(),
            auto_config_rows: Vec::new(),
            todo_filter: String::new(),
            todo_page_uuid_draft: String::new(),
            todo_title_draft: String::new(),
            todo_text_draft: String::new(),
            ap_title_draft: String::new(),
            ap_page_uuid_draft: String::new(),
            ap_logic_op: "AND".to_string(),
            ap_selected_conditions: Vec::new(),
            ap_target_block_uuid: String::new(),
            ap_target_pin_key: "Tr".to_string(),
            diff_compare_path: String::new(),
            diff_filter: String::new(),
            diff_cached_report: None,
            tech_report_tab: 0,
            tech_report_filter: String::new(),
            tech_report_wired_only: false,
            mult_selected_uuid: None,
            mult_title_draft: String::new(),
            mult_serial_draft: String::new(),
            mult_address_draft: String::new(),
            mult_desc_draft: String::new(),
            mult_overrides_draft: Vec::new(),
        }
    }
}

/// Editable `Document` attributes: (section, attribute, label).
const PROPERTY_FIELDS: &[(&str, &str, &str)] = &[
    ("General", "Title", "Project name"),
    ("General", "Cust", "Customer"),
    ("General", "Creator", "Created by"),
    ("Contact", "Tel", "Phone"),
    ("Contact", "EMail", "E-mail"),
    ("Address", "Street", "Street"),
    ("Address", "PostCode", "Post code"),
    ("Address", "Town", "Town"),
    ("Address", "Ctry", "Country"),
    ("Location", "Latitude", "Latitude"),
    ("Location", "Longitude", "Longitude"),
    ("Location", "Timezone", "Time zone"),
    ("Location", "Timeserver", "Time server"),
    ("Regional", "Currency", "Currency"),
];

const READONLY_FIELDS: &[(&str, &str)] = &[
    ("ConfigVersion", "Config version"),
    ("CDate", "Created"),
    ("Date", "Last saved"),
    ("NumO", "Objects"),
];

const TIME_ZONES: &[&str] = &[
    "Europe/Vienna",
    "Europe/Berlin",
    "Europe/Warsaw",
    "Europe/Zurich",
    "Europe/London",
    "Europe/Paris",
    "Europe/Prague",
    "Europe/Rome",
    "Europe/Madrid",
    "Europe/Amsterdam",
    "America/New_York",
    "America/Chicago",
    "America/Los_Angeles",
    "Asia/Dubai",
    "Australia/Sydney",
];

const MONTHS: &[&str] = &[
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

impl AdminModalsState {
    pub fn open_properties(&mut self, project: &LoxoneProject) {
        self.props_draft = PROPERTY_FIELDS
            .iter()
            .map(|(_, k, _)| {
                (
                    k.to_string(),
                    project.metadata.doc_attr(k).unwrap_or_default().to_string(),
                )
            })
            .collect();
        self.status = None;
        self.open = AdminModal::Properties;
    }

    pub fn open_users(&mut self) {
        self.status = None;
        self.user_draft_for = None;
        self.new_name.clear();
        self.open = AdminModal::Users;
    }

    pub fn open_modes(&mut self) {
        self.status = None;
        self.new_name.clear();
        if self.preview.is_none() {
            let (_, m, d) = today();
            self.preview = Some((m, d));
        }
        self.open = AdminModal::Modes;
    }

    pub fn open_network(&mut self) {
        self.open = AdminModal::Network;
        if self.net_rx.is_none() && self.net_results.is_empty() {
            self.start_search();
        }
    }

    pub fn open_edit_by_type(&mut self, project: &LoxoneProject, initial_type: Option<&str>) {
        self.status = None;
        let types = collect_editable_object_types(project);
        let chosen = initial_type
            .filter(|t| types.iter().any(|(ty, _)| ty == *t))
            .map(str::to_string)
            .or_else(|| {
                if !self.selected_edit_type.is_empty()
                    && types.iter().any(|(ty, _)| ty == &self.selected_edit_type)
                {
                    Some(self.selected_edit_type.clone())
                } else {
                    types.first().map(|(ty, _)| ty.clone())
                }
            })
            .unwrap_or_default();
        self.selected_edit_type = chosen.clone();
        self.edit_by_type_rows = build_same_type_object_rows(project, &chosen);
        self.open = AdminModal::EditByType;
    }

    pub fn open_find_and_replace(&mut self) {
        self.status = None;
        self.open = AdminModal::FindAndReplace;
    }

    pub fn open_project_backups(&mut self) {
        self.status = None;
        self.backups_list = list_project_backups_in_dir(&project_backups_dir());
        self.open = AdminModal::ProjectBackups;
    }

    pub fn open_auto_configuration(&mut self, project: &LoxoneProject) {
        self.status = None;
        self.auto_config_rows = build_auto_config_room_rows(project);
        self.open = AdminModal::AutoConfiguration;
    }

    pub fn open_todo_list(&mut self, project: &LoxoneProject) {
        self.status = None;
        if self.todo_page_uuid_draft.is_empty() {
            if let Some(first_page) = project.pages.first() {
                self.todo_page_uuid_draft = first_page.uuid.clone();
            }
        }
        self.open = AdminModal::TodoList;
    }

    pub fn open_autopilot_designer(&mut self, project: &LoxoneProject) {
        self.status = None;
        if self.ap_page_uuid_draft.is_empty() {
            if let Some(first_page) = project.pages.first() {
                self.ap_page_uuid_draft = first_page.uuid.clone();
            }
        }
        if self.ap_title_draft.is_empty() {
            self.ap_title_draft = "Evening Comfort Rule".to_string();
        }
        self.open = AdminModal::AutopilotDesigner;
    }

    pub fn open_project_diff(
        &mut self,
        base_project: Option<&LoxoneProject>,
        active_project: &LoxoneProject,
    ) {
        self.status = None;
        if let Some(base) = base_project {
            self.diff_cached_report = Some(loxone_core::compare_projects(base, active_project));
        } else {
            self.diff_cached_report =
                Some(loxone_core::compare_projects(active_project, active_project));
        }
        self.open = AdminModal::ProjectDiff;
    }

    pub fn open_tech_report_wizard(&mut self) {
        self.status = None;
        self.open = AdminModal::TechReportWizard;
    }

    pub fn open_multiplier_manager(&mut self, project: &LoxoneProject) {
        self.status = None;
        let units = loxone_core::collect_multiplier_units(project);
        if let Some(first) = units.first() {
            self.mult_selected_uuid = Some(first.uuid.clone());
            self.mult_title_draft = first.title.clone();
            self.mult_serial_draft = first.miniserver_serial.clone();
            self.mult_address_draft = first.address.clone();
            self.mult_desc_draft = first.description.clone();
            self.mult_overrides_draft = first.device_serial_overrides.clone();
        } else {
            self.mult_selected_uuid = None;
            if self.mult_title_draft.is_empty() {
                self.mult_title_draft = "Unit 101".to_string();
                self.mult_serial_draft = "504F94FF0101".to_string();
                self.mult_address_draft = "192.168.1.101".to_string();
            }
        }
        self.open = AdminModal::MultiplierManager;
    }

    pub fn close(&mut self) {
        self.open = AdminModal::None;
    }

    fn start_search(&mut self) {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(discover_miniservers(Duration::from_secs(3)));
        });
        self.net_rx = Some(rx);
        self.net_status = Some("Searching the local network…".to_string());
    }
}

// ---------------------------------------------------------------------------
// Date helpers (no chrono dependency)
// ---------------------------------------------------------------------------

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn today() -> (i64, u32, u32) {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    civil_from_days(secs.div_euclid(86_400))
}

/// Weekday with Monday = 0.
fn weekday_mon0(y: i64, m: u32, d: u32) -> u32 {
    // 1970-01-01 was a Thursday (= 3).
    ((days_from_civil(y, m, d) + 3).rem_euclid(7)) as u32
}

const WEEKDAYS: &[&str] = &["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

// ---------------------------------------------------------------------------
// Shared widgets
// ---------------------------------------------------------------------------

fn button(ui: &mut egui::Ui, text: &str, accent: bool) -> egui::Response {
    let (fill, color) = if accent {
        (theme::ACCENT_GREEN_DIM, theme::ACCENT_PRIMARY)
    } else {
        (theme::BG_WIDGET, theme::TEXT_PRIMARY)
    };
    ui.add(
        egui::Button::new(RichText::new(tr(text)).size(12.0).color(color))
            .fill(fill)
            .stroke(Stroke::new(
                1.0_f32,
                if accent {
                    theme::ACCENT_PRIMARY
                } else {
                    theme::BORDER_SUBTLE
                },
            ))
            .corner_radius(CornerRadius::same(6)),
    )
}

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(
        RichText::new(tr(text))
            .size(12.5)
            .strong()
            .color(theme::ACCENT_PRIMARY),
    );
    ui.add_space(2.0);
}

fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::NONE
        .fill(theme::BG_DARK)
        .stroke(Stroke::new(1.0_f32, theme::CARD_BORDER))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, add)
        .inner
}

/// Draws a centred modal window. Returns false when it should close.
fn modal_window(
    ctx: &egui::Context,
    id: &str,
    title: &str,
    size: [f32; 2],
    status: Option<&str>,
    add: impl FnOnce(&mut egui::Ui),
) -> bool {
    let mut keep_open = true;
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        return false;
    }
    let backdrop_id = format!("{id}_backdrop");
    let (backdrop_clicked, popup_was_open) =
        theme::draw_modal_backdrop(ctx, &backdrop_id, egui::Order::Foreground, 150);
    let screen = ctx.screen_rect().size();
    let size = [
        size[0].min(screen.x - 60.0).max(320.0),
        size[1].min(screen.y - 60.0).max(240.0),
    ];
    let resp = egui::Window::new(id)
        .id(egui::Id::new(id))
        .order(egui::Order::Tooltip)
        .title_bar(false)
        .collapsible(false)
        .resizable(true)
        .default_size(size)
        .min_size([size[0] * 0.8, size[1] * 0.6])
        .max_size([screen.x - 52.0, screen.y - 52.0])
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
                    RichText::new(tr(title))
                        .size(16.0)
                        .strong()
                        .color(theme::TEXT_PRIMARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if button(ui, "✕ Close", false).clicked() {
                        keep_open = false;
                    }
                });
            });
            if let Some(msg) = status {
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
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);
            add(ui);
        });
    let rect = resp.map(|r| r.response.rect);
    if std::env::var("LOXONE_DEBUG_SHOT").is_ok() {
        eprintln!("modal {id} rect={rect:?} size={size:?}");
    }
    if theme::should_close_modal_on_outside_click(ctx, id, backdrop_clicked, popup_was_open, rect) {
        keep_open = false;
    }
    keep_open
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn render_admin_modals(
    ctx: &egui::Context,
    state: &mut AdminModalsState,
    project: Option<&LoxoneProject>,
) -> Vec<AdminAction> {
    let mut out = Vec::new();
    // Collect finished network searches even while the dialog is closed.
    if let Some(rx) = &state.net_rx {
        match rx.try_recv() {
            Ok(Ok(found)) => {
                state.net_status = Some(if found.is_empty() {
                    "No Miniserver answered. Check that this computer is on the same network."
                        .to_string()
                } else {
                    format!("Found {} Miniserver(s).", found.len())
                });
                state.net_results = found;
                state.net_rx = None;
            }
            Ok(Err(e)) => {
                state.net_status = Some(format!("Search failed: {e}"));
                state.net_rx = None;
            }
            Err(mpsc::TryRecvError::Empty) => ctx.request_repaint_after(Duration::from_millis(100)),
            Err(mpsc::TryRecvError::Disconnected) => state.net_rx = None,
        }
    }

    let keep = match state.open {
        AdminModal::None => true,
        AdminModal::Network => render_network(ctx, state, &mut out),
        modal => {
            let Some(project) = project else {
                state.close();
                return out;
            };
            match modal {
                AdminModal::Properties => render_properties(ctx, state, project, &mut out),
                AdminModal::Users => render_users(ctx, state, project, &mut out),
                AdminModal::Modes => render_modes(ctx, state, project, &mut out),
                AdminModal::EditByType => render_edit_by_type(ctx, state, project, &mut out),
                AdminModal::FindAndReplace => render_find_and_replace(ctx, state, project, &mut out),
                AdminModal::ProjectBackups => render_project_backups(ctx, state, project, &mut out),
                AdminModal::AutoConfiguration => {
                    render_auto_configuration(ctx, state, project, &mut out)
                }
                AdminModal::TodoList => render_todo_list(ctx, state, project, &mut out),
                AdminModal::AutopilotDesigner => {
                    render_autopilot_designer(ctx, state, project, &mut out)
                }
                AdminModal::ProjectDiff => render_project_diff(ctx, state, project, &mut out),
                AdminModal::TechReportWizard => {
                    render_tech_report_wizard(ctx, state, project, &mut out)
                }
                AdminModal::MultiplierManager => {
                    render_multiplier_manager(ctx, state, project, &mut out)
                }
                _ => true,
            }
        }
    };
    if !keep {
        state.close();
    }
    out
}

// ---------------------------------------------------------------------------
// Project properties
// ---------------------------------------------------------------------------

fn render_properties(
    ctx: &egui::Context,
    state: &mut AdminModalsState,
    project: &LoxoneProject,
    out: &mut Vec<AdminAction>,
) -> bool {
    let status = state.status.clone();
    let mut close_after = false;
    let keep = modal_window(
        ctx,
        "admin_project_properties",
        "🏠 Project Properties",
        [620.0, 600.0],
        status.as_deref(),
        |ui| {
            egui::ScrollArea::vertical()
                .id_salt("props_scroll")
                .max_height(ui.available_height() - 44.0)
                .show(ui, |ui| {
                    let mut current_section = "";
                    for (i, (section, key, label)) in PROPERTY_FIELDS.iter().enumerate() {
                        if *section != current_section {
                            current_section = section;
                            section_label(ui, section);
                        }
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [120.0, 20.0],
                                egui::Label::new(
                                    RichText::new(tr(label)).size(12.0).color(theme::TEXT_SECONDARY),
                                ),
                            );
                            let value = &mut state.props_draft[i].1;
                            ui.add(egui::TextEdit::singleline(value).desired_width(300.0));
                            if *key == "Timezone" {
                                egui::ComboBox::from_id_salt("tz_pick")
                                    .selected_text(tr("Pick…"))
                                    .width(110.0)
                                    .show_ui(ui, |ui| {
                                        for tz in TIME_ZONES {
                                            if ui.selectable_label(value == tz, *tz).clicked() {
                                                *value = tz.to_string();
                                            }
                                        }
                                    });
                            }
                        });
                    }
                    section_label(ui, "File");
                    for (key, label) in READONLY_FIELDS {
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [120.0, 20.0],
                                egui::Label::new(
                                    RichText::new(tr(label)).size(12.0).color(theme::TEXT_SECONDARY),
                                ),
                            );
                            ui.label(
                                RichText::new(project.metadata.doc_attr(key).unwrap_or("—"))
                                    .size(12.0)
                                    .color(theme::TEXT_PRIMARY),
                            );
                        });
                    }
                });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let changed: Vec<(String, String)> = state
                    .props_draft
                    .iter()
                    .filter(|(k, v)| project.metadata.doc_attr(k).unwrap_or_default() != v)
                    .cloned()
                    .collect();
                let label = if changed.is_empty() {
                    tr("No changes").to_string()
                } else {
                    tr_fmt(
                        "✔ Apply {count} change(s)",
                        &[("count", &changed.len().to_string())],
                    )
                };
                if button(ui, &label, !changed.is_empty()).clicked() && !changed.is_empty() {
                    let edits = changed
                        .into_iter()
                        .map(|(k, v)| ProjectEdit::SetControlAttr {
                            control_uuid: project.metadata.uuid.clone(),
                            attr_name: k,
                            value: Some(v),
                        })
                        .collect();
                    out.push(AdminAction::Edit(ProjectEdit::Batch(edits)));
                    close_after = true;
                }
                if button(ui, "Revert", false).clicked() {
                    for (k, v) in state.props_draft.iter_mut() {
                        *v = project.metadata.doc_attr(k).unwrap_or_default().to_string();
                    }
                }
            });
        },
    );
    keep && !close_after
}

// ---------------------------------------------------------------------------
// Users & rights
// ---------------------------------------------------------------------------

fn render_users(
    ctx: &egui::Context,
    state: &mut AdminModalsState,
    project: &LoxoneProject,
    out: &mut Vec<AdminAction>,
) -> bool {
    let users: Vec<&TreeNode> = find_nodes_by_type(project, "User");
    let groups: Vec<&TreeNode> = find_nodes_by_type(project, "UserGroup");
    let permissions: Vec<&TreeNode> = find_nodes_by_type(project, "Permission");
    let status = state.status.clone();
    if !users.iter().any(|u| Some(u.uuid.as_str()) == state.selected_user.as_deref()) {
        state.selected_user = users.first().map(|u| u.uuid.clone());
    }
    if !groups.iter().any(|g| Some(g.uuid.as_str()) == state.selected_group.as_deref()) {
        state.selected_group = groups.first().map(|g| g.uuid.clone());
    }

    modal_window(
        ctx,
        "admin_users_rights",
        "👤 Users & Rights",
        [900.0, 620.0],
        status.as_deref(),
        |ui| {
            ui.horizontal(|ui| {
                for (tab, label) in [
                    (
                        UsersTab::Users,
                        format!("{} ({})", tr("Users"), users.len()),
                    ),
                    (
                        UsersTab::Groups,
                        format!("{} ({})", tr("User groups"), groups.len()),
                    ),
                ] {
                    if button(ui, &label, state.users_tab == tab).clicked() {
                        state.users_tab = tab;
                        state.new_name.clear();
                    }
                }
            });
            ui.add_space(8.0);
            let list: &[&TreeNode] = match state.users_tab {
                UsersTab::Users => &users,
                UsersTab::Groups => &groups,
            };
            let narrow = ui.available_width() < 760.0;
            let height = if narrow { 440.0 } else { ui.available_height() };
            let layout = if narrow { egui::Layout::top_down(egui::Align::Min) } else { egui::Layout::left_to_right(egui::Align::Min) };
            egui::ScrollArea::vertical().id_salt("users_outer").show(ui, |ui| { ui.with_layout(layout, |ui| {
                // ---- left: list + add
                ui.vertical(|ui| {
                    ui.set_width(250.0);
                    card(ui, |ui| {
                        ui.set_min_height(height - 90.0);
                        egui::ScrollArea::vertical()
                            .id_salt("users_list")
                            .max_height(height - 100.0)
                            .show(ui, |ui| {
                                ui.set_width(228.0);
                                if list.is_empty() {
                                    ui.label(
                                        RichText::new(tr("None in this project"))
                                            .color(theme::TEXT_SECONDARY),
                                    );
                                }
                                for n in list {
                                    let selected_slot = match state.users_tab {
                                        UsersTab::Users => &mut state.selected_user,
                                        UsersTab::Groups => &mut state.selected_group,
                                    };
                                    let sel = selected_slot.as_deref() == Some(n.uuid.as_str());
                                    let icon = if state.users_tab == UsersTab::Users {
                                        "👤"
                                    } else {
                                        "👥"
                                    };
                                    if ui
                                        .selectable_label(sel, format!("{icon}  {}", n.title))
                                        .clicked()
                                    {
                                        *selected_slot = Some(n.uuid.clone());
                                    }
                                }
                            });
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut state.new_name)
                                .hint_text(match state.users_tab {
                                    UsersTab::Users => tr("New user name"),
                                    UsersTab::Groups => tr("New group name"),
                                })
                                .desired_width(170.0),
                        );
                        let name = state.new_name.trim().to_string();
                        if button(ui, "➕ Add", true).clicked() && !name.is_empty() {
                            let res = match state.users_tab {
                                UsersTab::Users => new_user_edit(project, &name),
                                UsersTab::Groups => new_user_group_edit(project, &name),
                            };
                            match res {
                                Some((edit, uuid)) => {
                                    out.push(AdminAction::Edit(edit));
                                    match state.users_tab {
                                        UsersTab::Users => state.selected_user = Some(uuid),
                                        UsersTab::Groups => state.selected_group = Some(uuid),
                                    }
                                    state.status = Some(format!("Added \"{name}\"."));
                                    state.new_name.clear();
                                }
                                None => {
                                    state.status = Some(
                                        "This project has no users section to add to.".to_string(),
                                    )
                                }
                            }
                        }
                    });
                });
                ui.add_space(8.0);
                // ---- right: details
                ui.vertical(|ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("users_detail")
                        .max_height(height - 10.0)
                        .show(ui, |ui| match state.users_tab {
                            UsersTab::Users => {
                                user_detail(ui, state, project, &groups, &permissions, out)
                            }
                            UsersTab::Groups => {
                                group_detail(ui, state, project, &users, &permissions, out)
                            }
                        });
                });
            }); });
        },
    )
}

fn user_detail(
    ui: &mut egui::Ui,
    state: &mut AdminModalsState,
    project: &LoxoneProject,
    groups: &[&TreeNode],
    permissions: &[&TreeNode],
    out: &mut Vec<AdminAction>,
) {
    let Some(user) = state
        .selected_user
        .as_deref()
        .and_then(|u| find_node_by_uuid(project, u))
    else {
        ui.label(RichText::new(tr("Select a user on the left.")).color(theme::TEXT_SECONDARY));
        return;
    };
    if state.user_draft_for.as_deref() != Some(user.uuid.as_str()) {
        state.user_draft = USER_FIELDS
            .iter()
            .map(|(k, _)| (k.to_string(), user.attr(k).unwrap_or_default().to_string()))
            .collect();
        state.user_draft_for = Some(user.uuid.clone());
    }
    section_label(ui, "User details");
    for (i, (_, label)) in USER_FIELDS.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.add_sized(
                [110.0, 20.0],
                egui::Label::new(
                    RichText::new(tr(label))
                        .size(12.0)
                        .color(theme::TEXT_SECONDARY),
                ),
            );
            ui.add(egui::TextEdit::singleline(&mut state.user_draft[i].1).desired_width(280.0));
        });
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let changed: Vec<(String, String)> = state
            .user_draft
            .iter()
            .filter(|(k, v)| user.attr(k).unwrap_or_default() != v)
            .cloned()
            .collect();
        if button(ui, "✔ Apply details", !changed.is_empty()).clicked() && !changed.is_empty() {
            let edits = changed
                .into_iter()
                .map(|(k, v)| ProjectEdit::SetControlAttr {
                    control_uuid: user.uuid.clone(),
                    attr_name: k,
                    value: Some(v),
                })
                .collect();
            out.push(AdminAction::Edit(ProjectEdit::Batch(edits)));
            state.status = Some(format!("Updated user \"{}\".", user.title));
        }
        if button(ui, "🗑 Delete user", false).clicked() {
            out.push(AdminAction::Edit(delete_user_edits(project, &user.uuid)));
            state.status = Some(format!("Deleted user \"{}\".", user.title));
            state.selected_user = None;
            state.user_draft_for = None;
        }
    });

    section_label(ui, "Member of groups");
    if groups.is_empty() {
        ui.label(RichText::new(tr("No user groups")).color(theme::TEXT_SECONDARY));
    }
    for g in groups {
        let mut member = group_member_uuids(g).iter().any(|u| u == &user.uuid);
        if ui.checkbox(&mut member, g.title.as_str()).changed() {
            out.push(AdminAction::Edit(set_group_membership_edit(g, &user.uuid, member)));
        }
    }

    section_label(ui, "Effective rights");
    let mut effective: Vec<String> = granted_permission_refs(user);
    for g in groups {
        if group_member_uuids(g).iter().any(|u| u == &user.uuid) {
            effective.extend(granted_permission_refs(g));
        }
    }
    for p in permissions {
        let has = effective.contains(&p.uuid);
        ui.label(
            RichText::new(format!("{}  {}", if has { "✔" } else { "✖" }, p.title))
                .size(12.0)
                .color(if has {
                    theme::ACCENT_PRIMARY
                } else {
                    theme::TEXT_SECONDARY
                }),
        );
    }
    ui.label(
        RichText::new(tr(
            "Rights come from the user's groups; edit them on the User groups tab.",
        ))
        .size(11.0)
        .italics()
        .color(theme::TEXT_SECONDARY),
    );
}

fn group_detail(
    ui: &mut egui::Ui,
    state: &mut AdminModalsState,
    project: &LoxoneProject,
    users: &[&TreeNode],
    permissions: &[&TreeNode],
    out: &mut Vec<AdminAction>,
) {
    let Some(group) = state
        .selected_group
        .as_deref()
        .and_then(|u| find_node_by_uuid(project, u))
    else {
        ui.label(
            RichText::new(tr("Select a user group on the left.")).color(theme::TEXT_SECONDARY),
        );
        return;
    };
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("👥 {}", group.title))
                .size(14.0)
                .strong()
                .color(theme::TEXT_PRIMARY),
        );
        if button(ui, "🗑 Delete group", false).clicked() {
            out.push(AdminAction::Edit(ProjectEdit::RemoveTreeNode {
                node_uuid: group.uuid.clone(),
            }));
            state.status = Some(format!("Deleted group \"{}\".", group.title));
            state.selected_group = None;
        }
    });
    let members = group_member_uuids(group);
    section_label(ui, &format!("{} ({})", tr("Members"), members.len()));
    if users.is_empty() {
        ui.label(RichText::new(tr("No users")).color(theme::TEXT_SECONDARY));
    }
    for u in users {
        let mut member = members.contains(&u.uuid);
        if ui.checkbox(&mut member, u.title.as_str()).changed() {
            out.push(AdminAction::Edit(set_group_membership_edit(group, &u.uuid, member)));
        }
    }
    let granted = granted_permission_refs(group);
    section_label(
        ui,
        &tr_fmt(
            "Rights ({count} granted)",
            &[("count", &granted.len().to_string())],
        ),
    );
    if permissions.is_empty() {
        ui.label(
            RichText::new(tr("This project defines no rights")).color(theme::TEXT_SECONDARY),
        );
    }
    egui::Grid::new("perm_grid")
        .num_columns(2)
        .spacing([24.0, 4.0])
        .show(ui, |ui| {
            for (i, p) in permissions.iter().enumerate() {
                let mut has = granted.contains(&p.uuid);
                if ui.checkbox(&mut has, p.title.as_str()).changed() {
                    out.push(AdminAction::Edit(set_permission_edit(group, &p.uuid, has)));
                }
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
}

// ---------------------------------------------------------------------------
// Operating modes & calendar
// ---------------------------------------------------------------------------

fn month_day_picker(
    ui: &mut egui::Ui,
    id: &str,
    month: u32,
    day: u32,
) -> Option<(u32, u32)> {
    let mut result = None;
    egui::ComboBox::from_id_salt(format!("{id}_m"))
        .selected_text(MONTHS[(month.clamp(1, 12) - 1) as usize])
        .width(56.0)
        .show_ui(ui, |ui| {
            for (i, m) in MONTHS.iter().enumerate() {
                let mm = i as u32 + 1;
                if ui.selectable_label(mm == month, *m).clicked() {
                    result = Some((mm, day));
                }
            }
        });
    egui::ComboBox::from_id_salt(format!("{id}_d"))
        .selected_text(day.to_string())
        .width(40.0)
        .show_ui(ui, |ui| {
            for d in 1..=31u32 {
                if ui.selectable_label(d == day, d.to_string()).clicked() {
                    result = Some((month, d));
                }
            }
        });
    result
}

fn render_modes(
    ctx: &egui::Context,
    state: &mut AdminModalsState,
    project: &LoxoneProject,
    out: &mut Vec<AdminAction>,
) -> bool {
    let modes = operating_modes(project);
    let entries = calendar_entries(project);
    let status = state.status.clone();
    let mode_name = |n: i32| {
        modes
            .iter()
            .find(|m| m.number == n)
            .map(|m| m.title.clone())
            .unwrap_or_else(|| format!("Mode {n}"))
    };

    modal_window(
        ctx,
        "admin_operating_modes",
        "📅 Operating Modes & Calendar",
        [1040.0, 660.0],
        status.as_deref(),
        |ui| {
            let narrow = ui.available_width() < 960.0;
            let height = if narrow { 440.0 } else { ui.available_height() };
            let layout = if narrow { egui::Layout::top_down(egui::Align::Min) } else { egui::Layout::left_to_right(egui::Align::Min) };
            egui::ScrollArea::vertical().id_salt("modes_outer").show(ui, |ui| { ui.with_layout(layout, |ui| {
                // ---- operating modes
                ui.vertical(|ui| {
                    ui.set_width(330.0);
                    section_label(
                        ui,
                        &format!("{} ({})", tr("Operating modes"), modes.len()),
                    );
                    card(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("modes_list")
                            .max_height(height - 150.0)
                            .show(ui, |ui| {
                                ui.set_width(300.0);
                                egui::Grid::new("modes_grid")
                                    .num_columns(3)
                                    .striped(true)
                                    .spacing([10.0, 4.0])
                                    .show(ui, |ui| {
                                        for m in &modes {
                                            ui.label(
                                                RichText::new(m.number.to_string())
                                                    .monospace()
                                                    .color(theme::TEXT_SECONDARY),
                                            );
                                            ui.label(
                                                RichText::new(&m.title).color(theme::TEXT_PRIMARY),
                                            );
                                            if m.user_defined {
                                                if ui
                                                    .small_button("🗑")
                                                    .on_hover_text(tr(
                                                        "Delete this user-defined mode",
                                                    ))
                                                    .clicked()
                                                {
                                                    out.push(AdminAction::Edit(
                                                        ProjectEdit::RemoveTreeNode {
                                                            node_uuid: m.uuid.clone(),
                                                        },
                                                    ));
                                                    state.status =
                                                        Some(format!("Deleted mode \"{}\".", m.title));
                                                }
                                            } else {
                                                ui.label(
                                                    RichText::new(tr("built-in"))
                                                        .size(10.5)
                                                        .color(theme::TEXT_SECONDARY),
                                                );
                                            }
                                            ui.end_row();
                                        }
                                    });
                            });
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut state.new_name)
                                .hint_text(tr("New operating mode"))
                                .desired_width(200.0),
                        );
                        let name = state.new_name.trim().to_string();
                        if button(ui, "➕ Add mode", true).clicked() && !name.is_empty() {
                            match new_operating_mode_edit(project, &name) {
                                Some((edit, _)) => {
                                    out.push(AdminAction::Edit(edit));
                                    state.status = Some(format!("Added mode \"{name}\"."));
                                    state.new_name.clear();
                                }
                                None => {
                                    state.status =
                                        Some("This project has no operating modes section.".into())
                                }
                            }
                        }
                    });
                });
                ui.add_space(10.0);
                // ---- calendar
                ui.vertical(|ui| {
                    section_label(
                        ui,
                        &format!("{} ({})", tr("Calendar entries"), entries.len()),
                    );
                    card(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("cal_list")
                            .max_height(height - 230.0)
                            .show(ui, |ui| {
                                if entries.is_empty() {
                                    ui.label(
                                        RichText::new(tr("No calendar entries"))
                                            .color(theme::TEXT_SECONDARY),
                                    );
                                }
                                egui::Grid::new("cal_grid")
                                    .num_columns(5)
                                    .striped(true)
                                    .spacing([10.0, 6.0])
                                    .show(ui, |ui| {
                                        for h in ["Entry", "Activates", "From", "To", ""] {
                                            ui.label(
                                                RichText::new(tr(h))
                                                    .size(11.0)
                                                    .strong()
                                                    .color(theme::TEXT_SECONDARY),
                                            );
                                        }
                                        ui.end_row();
                                        for e in &entries {
                                            ui.label(RichText::new(&e.title).color(theme::TEXT_PRIMARY))
                                                .on_hover_text(tr(&recurrence_label(e.recurrence)));
                                            egui::ComboBox::from_id_salt(format!("cal_mode_{}", e.uuid))
                                                .selected_text(mode_name(e.mode_number))
                                                .width(140.0)
                                                .show_ui(ui, |ui| {
                                                    for m in &modes {
                                                        if ui
                                                            .selectable_label(m.number == e.mode_number, &m.title)
                                                            .clicked()
                                                        {
                                                            out.push(AdminAction::Edit(
                                                                ProjectEdit::SetControlAttr {
                                                                    control_uuid: e.uuid.clone(),
                                                                    attr_name: "CalType".into(),
                                                                    value: Some(m.number.to_string()),
                                                                },
                                                            ));
                                                        }
                                                    }
                                                });
                                            ui.horizontal(|ui| {
                                                if let Some((m, d)) = month_day_picker(
                                                    ui,
                                                    &format!("cal_from_{}", e.uuid),
                                                    e.month,
                                                    e.day,
                                                ) {
                                                    out.push(set_md(&e.uuid, "Month", "Day", m, d));
                                                }
                                            });
                                            ui.horizontal(|ui| {
                                                if let Some((m, d)) = month_day_picker(
                                                    ui,
                                                    &format!("cal_to_{}", e.uuid),
                                                    e.month2,
                                                    e.day2,
                                                ) {
                                                    out.push(set_md(&e.uuid, "Month2", "Day2", m, d));
                                                }
                                            });
                                            if ui
                                                .small_button("🗑")
                                                .on_hover_text(tr("Delete entry"))
                                                .clicked()
                                            {
                                                out.push(AdminAction::Edit(ProjectEdit::RemoveTreeNode {
                                                    node_uuid: e.uuid.clone(),
                                                }));
                                                state.status = Some(format!("Deleted entry \"{}\".", e.title));
                                            }
                                            ui.end_row();
                                        }
                                    });
                            });
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut state.new_cal_title)
                                .hint_text(tr("New entry, e.g. Summer holidays"))
                                .desired_width(200.0),
                        );
                        if !modes.iter().any(|m| m.number == state.new_cal_mode) {
                            state.new_cal_mode = modes
                                .iter()
                                .find(|m| m.number == 1)
                                .or(modes.first())
                                .map(|m| m.number)
                                .unwrap_or(0);
                        }
                        egui::ComboBox::from_id_salt("new_cal_mode")
                            .selected_text(mode_name(state.new_cal_mode))
                            .width(140.0)
                            .show_ui(ui, |ui| {
                                for m in &modes {
                                    ui.selectable_value(&mut state.new_cal_mode, m.number, &m.title);
                                }
                            });
                        let title = state.new_cal_title.trim().to_string();
                        if button(ui, "➕ Add entry", true).clicked() && !title.is_empty() {
                            match new_calendar_entry_edit(project, &title, state.new_cal_mode) {
                                Some((edit, _)) => {
                                    out.push(AdminAction::Edit(edit));
                                    state.status = Some(format!("Added calendar entry \"{title}\"."));
                                    state.new_cal_title.clear();
                                }
                                None => {
                                    state.status =
                                        Some("This project has no calendar section.".into())
                                }
                            }
                        }
                    });

                    section_label(ui, "Which modes are active on…");
                    let (year, _, _) = today();
                    let (pm, pd) = state.preview.unwrap_or((1, 1));
                    ui.horizontal(|ui| {
                        if let Some(sel) = month_day_picker(ui, "preview_date", pm, pd) {
                            state.preview = Some(sel);
                        }
                        let wd = weekday_mon0(year, pm, pd);
                        ui.label(
                            RichText::new(format!("{} {}", tr(WEEKDAYS[wd as usize]), year))
                                .color(theme::TEXT_SECONDARY),
                        );
                        if ui.small_button(tr("Today")).clicked() {
                            let (_, m, d) = today();
                            state.preview = Some((m, d));
                        }
                    });
                    let wd = weekday_mon0(year, pm, pd);
                    let active = active_modes_on(project, pm, pd, wd);
                    ui.horizontal_wrapped(|ui| {
                        if active.is_empty() {
                            ui.label(
                                RichText::new(tr("No mode active")).color(theme::TEXT_SECONDARY),
                            );
                        }
                        for a in active {
                            egui::Frame::NONE
                                .fill(theme::ACCENT_GREEN_DIM)
                                .corner_radius(CornerRadius::same(10))
                                .inner_margin(Margin::symmetric(8, 2))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(a).size(11.5).color(theme::ACCENT_PRIMARY));
                                });
                        }
                    });
                });
            }); });
        },
    )
}

fn set_md(uuid: &str, mk: &str, dk: &str, m: u32, d: u32) -> AdminAction {
    AdminAction::Edit(ProjectEdit::Batch(vec![
        ProjectEdit::SetControlAttr {
            control_uuid: uuid.to_string(),
            attr_name: mk.to_string(),
            value: Some(m.to_string()),
        },
        ProjectEdit::SetControlAttr {
            control_uuid: uuid.to_string(),
            attr_name: dk.to_string(),
            value: Some(d.to_string()),
        },
    ]))
}

// ---------------------------------------------------------------------------
// Network search
// ---------------------------------------------------------------------------

fn render_network(
    ctx: &egui::Context,
    state: &mut AdminModalsState,
    out: &mut Vec<AdminAction>,
) -> bool {
    let status = state.net_status.clone();
    let mut close_after = false;
    let keep = modal_window(
        ctx,
        "admin_network_search",
        "📡 Search Network for Miniservers",
        [680.0, 420.0],
        status.as_deref(),
        |ui| {
            ui.horizontal(|ui| {
                let busy = state.net_rx.is_some();
                if busy {
                    ui.spinner();
                }
                if ui
                    .add_enabled(!busy, egui::Button::new(tr("🔄 Search again")))
                    .clicked()
                {
                    state.net_results.clear();
                    state.start_search();
                }
                ui.label(
                    RichText::new(tr(
                        "Sends a broadcast on the local network and lists every Miniserver that answers.",
                    ))
                    .size(11.0)
                    .color(theme::TEXT_SECONDARY),
                );
            });
            ui.add_space(8.0);
            card(ui, |ui| {
                ui.set_min_height(200.0);
                ui.set_width(ui.available_width());
                egui::Grid::new("net_grid")
                    .num_columns(4)
                    .striped(true)
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        for h in ["Name", "Address", "Serial number", ""] {
                            ui.label(
                                RichText::new(tr(h))
                                    .size(11.0)
                                    .strong()
                                    .color(theme::TEXT_SECONDARY),
                            );
                        }
                        ui.end_row();
                        for r in &state.net_results {
                            ui.label(RichText::new(&r.name).color(theme::TEXT_PRIMARY))
                                .on_hover_text(&r.raw);
                            ui.label(RichText::new(&r.address).monospace().color(theme::TEXT_PRIMARY));
                            ui.label(
                                RichText::new(if r.serial.is_empty() { "—" } else { &r.serial })
                                    .monospace()
                                    .color(theme::TEXT_SECONDARY),
                            );
                            if button(ui, "Connect…", true).clicked() {
                                out.push(AdminAction::UseMiniserver(r.address.clone()));
                                close_after = true;
                            }
                            ui.end_row();
                        }
                    });
            });
        },
    );
    if state.net_rx.is_some() {
        ctx.request_repaint_after(Duration::from_millis(100));
    }
    keep && !close_after
}

// ---------------------------------------------------------------------------
// Edit Objects of Same Type (`CObjectTypeConfigurationDialog`)
// ---------------------------------------------------------------------------

pub fn collect_editable_object_types(project: &LoxoneProject) -> Vec<(String, usize)> {
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for page in &project.pages {
        for block in &page.blocks {
            if !block.is_ref_bar() && !block.is_note() {
                *counts.entry(block.block_type.clone()).or_insert(0) += 1;
            }
        }
    }
    fn walk_tree(node: &TreeNode, counts: &mut std::collections::BTreeMap<String, usize>) {
        if !matches!(
            node.control_type.as_str(),
            "PlaceCaption"
                | "CategoryCaption"
                | "PageCaption"
                | "PeripheryCaption"
                | "MemoryCaption"
                | "UserCaption"
                | "UsergroupCaption"
                | "OperatingModeCaption"
                | "CalendarCaption"
        ) && !node.uuid.is_empty()
        {
            *counts.entry(node.control_type.clone()).or_insert(0) += 1;
        }
        for child in &node.children {
            walk_tree(child, counts);
        }
    }
    for root in &project.periphery_tree {
        walk_tree(root, &mut counts);
    }
    for root in &project.system_tree {
        walk_tree(root, &mut counts);
    }
    let mut list: Vec<(String, usize)> = counts.into_iter().collect();
    list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    list
}

pub fn build_same_type_object_rows(
    project: &LoxoneProject,
    object_type: &str,
) -> Vec<SameTypeObjectRow> {
    let mut rows = Vec::new();
    for page in &project.pages {
        for block in &page.blocks {
            if block.block_type == object_type && !block.is_ref_bar() && !block.is_note() {
                let desc = block.attr("Desc").unwrap_or_default().to_string();
                let room = block.room_uuid.clone().unwrap_or_default();
                let cat = block.category_uuid.clone().unwrap_or_default();
                let rate = block.attr("Rate").unwrap_or("0").to_string();
                rows.push(SameTypeObjectRow {
                    uuid: block.uuid.clone(),
                    is_page_block: true,
                    location_label: page.title.clone(),
                    title: block.title.clone(),
                    orig_title: block.title.clone(),
                    desc: desc.clone(),
                    orig_desc: desc,
                    room_uuid: room.clone(),
                    orig_room_uuid: room,
                    category_uuid: cat.clone(),
                    orig_category_uuid: cat,
                    rating: rate.clone(),
                    orig_rating: rate,
                });
            }
        }
    }
    fn walk_nodes(
        node: &TreeNode,
        parent_label: &str,
        object_type: &str,
        rows: &mut Vec<SameTypeObjectRow>,
    ) {
        let loc = if parent_label.is_empty() {
            node.title.clone()
        } else {
            format!("{parent_label} / {}", node.title)
        };
        if node.control_type == object_type && !node.uuid.is_empty() {
            let desc = node.attr("Desc").unwrap_or_default().to_string();
            let room = node
                .room_uuid
                .clone()
                .or_else(|| node.attr("Place").map(str::to_string))
                .unwrap_or_default();
            let cat = node
                .category_uuid
                .clone()
                .or_else(|| node.attr("Cat").map(str::to_string))
                .unwrap_or_default();
            let rate = node.attr("Rate").unwrap_or("0").to_string();
            rows.push(SameTypeObjectRow {
                uuid: node.uuid.clone(),
                is_page_block: false,
                location_label: parent_label.to_string(),
                title: node.title.clone(),
                orig_title: node.title.clone(),
                desc: desc.clone(),
                orig_desc: desc,
                room_uuid: room.clone(),
                orig_room_uuid: room,
                category_uuid: cat.clone(),
                orig_category_uuid: cat,
                rating: rate.clone(),
                orig_rating: rate,
            });
        }
        for child in &node.children {
            walk_nodes(child, &loc, object_type, rows);
        }
    }
    for root in &project.periphery_tree {
        walk_nodes(root, "", object_type, &mut rows);
    }
    for root in &project.system_tree {
        walk_nodes(root, "", object_type, &mut rows);
    }
    rows
}

pub fn build_same_type_batch_edit(rows: &[SameTypeObjectRow]) -> Option<ProjectEdit> {
    let mut edits = Vec::new();
    for r in rows {
        if r.title != r.orig_title {
            edits.push(ProjectEdit::SetControlAttr {
                control_uuid: r.uuid.clone(),
                attr_name: "Title".to_string(),
                value: Some(r.title.clone()),
            });
        }
        if r.desc != r.orig_desc {
            edits.push(ProjectEdit::SetControlAttr {
                control_uuid: r.uuid.clone(),
                attr_name: "Desc".to_string(),
                value: if r.desc.is_empty() {
                    None
                } else {
                    Some(r.desc.clone())
                },
            });
        }
        if r.room_uuid != r.orig_room_uuid {
            edits.push(ProjectEdit::SetControlAttr {
                control_uuid: r.uuid.clone(),
                attr_name: "Place".to_string(),
                value: if r.room_uuid.is_empty() {
                    None
                } else {
                    Some(r.room_uuid.clone())
                },
            });
        }
        if r.category_uuid != r.orig_category_uuid {
            edits.push(ProjectEdit::SetControlAttr {
                control_uuid: r.uuid.clone(),
                attr_name: "Cat".to_string(),
                value: if r.category_uuid.is_empty() {
                    None
                } else {
                    Some(r.category_uuid.clone())
                },
            });
        }
        if r.rating != r.orig_rating {
            edits.push(ProjectEdit::SetControlAttr {
                control_uuid: r.uuid.clone(),
                attr_name: "Rate".to_string(),
                value: Some(r.rating.clone()),
            });
        }
    }
    if edits.is_empty() {
        None
    } else {
        Some(ProjectEdit::Batch(edits))
    }
}

fn render_edit_by_type(
    ctx: &egui::Context,
    state: &mut AdminModalsState,
    project: &LoxoneProject,
    out: &mut Vec<AdminAction>,
) -> bool {
    let status = state.status.clone();
    let types = collect_editable_object_types(project);
    modal_window(
        ctx,
        "admin_edit_by_type",
        "📋 Edit Objects of Same Type",
        [1080.0, 680.0],
        status.as_deref(),
        |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(tr("Object Type:"))
                        .strong()
                        .color(theme::TEXT_SECONDARY),
                );
                let current_label = if state.selected_edit_type.is_empty() {
                    tr("Select type…").to_string()
                } else {
                    format!(
                        "{} ({})",
                        crate::i18n::block_display_name(&state.selected_edit_type),
                        state.selected_edit_type
                    )
                };
                egui::ComboBox::from_id_salt("edit_by_type_selector")
                    .selected_text(current_label)
                    .width(290.0)
                    .show_ui(ui, |ui| {
                        for (ty, cnt) in &types {
                            let lbl = format!(
                                "{} ({}) — {cnt}",
                                crate::i18n::block_display_name(ty),
                                ty
                            );
                            if ui
                                .selectable_label(&state.selected_edit_type == ty, lbl)
                                .clicked()
                            {
                                state.selected_edit_type = ty.clone();
                                state.edit_by_type_rows =
                                    build_same_type_object_rows(project, ty);
                            }
                        }
                    });

                ui.add_space(10.0);
                ui.add(
                    egui::TextEdit::singleline(&mut state.edit_type_filter)
                        .hint_text(tr("Filter rows by title or page…"))
                        .desired_width(210.0),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let pending = build_same_type_batch_edit(&state.edit_by_type_rows);
                    if button(ui, "✔ Apply Batch Changes", pending.is_some()).clicked() {
                        if let Some(batch) = pending {
                            out.push(AdminAction::Edit(batch));
                            for r in &mut state.edit_by_type_rows {
                                r.orig_title = r.title.clone();
                                r.orig_desc = r.desc.clone();
                                r.orig_room_uuid = r.room_uuid.clone();
                                r.orig_category_uuid = r.category_uuid.clone();
                                r.orig_rating = r.rating.clone();
                            }
                            state.status = Some(
                                tr("Applied batch changes across objects of same type.")
                                    .to_string(),
                            );
                        }
                    }
                });
            });

            ui.add_space(6.0);
            let filter_lc = state.edit_type_filter.trim().to_ascii_lowercase();
            card(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("edit_by_type_scroll")
                    .max_height(ui.available_height() - 16.0)
                    .show(ui, |ui| {
                        egui::Grid::new("edit_by_type_grid")
                            .num_columns(6)
                            .striped(true)
                            .spacing([10.0, 6.0])
                            .show(ui, |ui| {
                                for h in [
                                    "Location",
                                    "Title",
                                    "Description",
                                    "Room",
                                    "Category",
                                    "Rating",
                                ] {
                                    ui.label(
                                        RichText::new(tr(h))
                                            .size(11.0)
                                            .strong()
                                            .color(theme::TEXT_SECONDARY),
                                    );
                                }
                                ui.end_row();

                                for row in &mut state.edit_by_type_rows {
                                    if !filter_lc.is_empty()
                                        && !row.title.to_ascii_lowercase().contains(&filter_lc)
                                        && !row
                                            .location_label
                                            .to_ascii_lowercase()
                                            .contains(&filter_lc)
                                    {
                                        continue;
                                    }
                                    if ui
                                        .link(
                                            RichText::new(&row.location_label)
                                                .size(11.5)
                                                .color(theme::ACCENT_PRIMARY),
                                        )
                                        .on_hover_text(tr("Select object in editor"))
                                        .clicked()
                                    {
                                        out.push(AdminAction::SelectObject(row.uuid.clone()));
                                    }
                                    ui.add(
                                        egui::TextEdit::singleline(&mut row.title)
                                            .desired_width(200.0),
                                    );
                                    ui.add(
                                        egui::TextEdit::singleline(&mut row.desc)
                                            .desired_width(180.0),
                                    );
                                    let room_lbl = project
                                        .places
                                        .get(&row.room_uuid)
                                        .map(|p| p.title.as_str())
                                        .unwrap_or("—");
                                    egui::ComboBox::from_id_salt(format!("ebt_room_{}", row.uuid))
                                        .selected_text(room_lbl)
                                        .width(135.0)
                                        .show_ui(ui, |ui| {
                                            if ui
                                                .selectable_label(row.room_uuid.is_empty(), "—")
                                                .clicked()
                                            {
                                                row.room_uuid.clear();
                                            }
                                            for (u, place) in &project.places {
                                                if ui
                                                    .selectable_label(
                                                        &row.room_uuid == u,
                                                        &place.title,
                                                    )
                                                    .clicked()
                                                {
                                                    row.room_uuid = u.clone();
                                                }
                                            }
                                        });
                                    let cat_lbl = project
                                        .categories
                                        .get(&row.category_uuid)
                                        .map(|c| c.title.as_str())
                                        .unwrap_or("—");
                                    egui::ComboBox::from_id_salt(format!("ebt_cat_{}", row.uuid))
                                        .selected_text(cat_lbl)
                                        .width(135.0)
                                        .show_ui(ui, |ui| {
                                            if ui
                                                .selectable_label(row.category_uuid.is_empty(), "—")
                                                .clicked()
                                            {
                                                row.category_uuid.clear();
                                            }
                                            for (u, cat) in &project.categories {
                                                if ui
                                                    .selectable_label(
                                                        &row.category_uuid == u,
                                                        &cat.title,
                                                    )
                                                    .clicked()
                                                {
                                                    row.category_uuid = u.clone();
                                                }
                                            }
                                        });
                                    egui::ComboBox::from_id_salt(format!("ebt_rate_{}", row.uuid))
                                        .selected_text(format!("★ {}", row.rating))
                                        .width(65.0)
                                        .show_ui(ui, |ui| {
                                            for r_val in ["0", "1", "2", "3", "4", "5"] {
                                                if ui
                                                    .selectable_label(
                                                        row.rating == r_val,
                                                        format!("★ {r_val}"),
                                                    )
                                                    .clicked()
                                                {
                                                    row.rating = r_val.to_string();
                                                }
                                            }
                                        });
                                    ui.end_row();
                                }
                            });
                    });
            });
        },
    )
}

// ---------------------------------------------------------------------------
// Find & Replace (`Ctrl+H` / `⌘H`)
// ---------------------------------------------------------------------------

fn replace_substring(src: &str, needle: &str, replacement: &str, case_sensitive: bool) -> Option<String> {
    if needle.is_empty() {
        return None;
    }
    if case_sensitive {
        if src.contains(needle) {
            Some(src.replace(needle, replacement))
        } else {
            None
        }
    } else {
        let src_lc = src.to_lowercase();
        let needle_lc = needle.to_lowercase();
        if !src_lc.contains(&needle_lc) {
            return None;
        }
        let mut out = String::with_capacity(src.len());
        let mut last_idx = 0;
        for (idx, _) in src_lc.match_indices(&needle_lc) {
            if idx >= last_idx && src.is_char_boundary(idx) && src.is_char_boundary(idx + needle.len()) {
                out.push_str(&src[last_idx..idx]);
                out.push_str(replacement);
                last_idx = idx + needle.len();
            }
        }
        if last_idx == 0 {
            return None;
        }
        out.push_str(&src[last_idx..]);
        Some(out)
    }
}

pub fn preview_find_and_replace(
    project: &LoxoneProject,
    query: &str,
    replacement: &str,
    case_sensitive: bool,
    in_titles: bool,
    in_descriptions: bool,
    in_attributes: bool,
) -> Vec<FindReplaceMatch> {
    let mut matches = Vec::new();
    if query.trim().is_empty() {
        return matches;
    }
    for page in &project.pages {
        for block in &page.blocks {
            if block.is_ref_bar() || block.is_note() {
                continue;
            }
            if in_titles {
                if let Some(after) =
                    replace_substring(&block.title, query, replacement, case_sensitive)
                {
                    matches.push(FindReplaceMatch {
                        uuid: block.uuid.clone(),
                        object_type: block.block_type.clone(),
                        location_label: page.title.clone(),
                        field_name: "Title".to_string(),
                        before: block.title.clone(),
                        after,
                    });
                }
            }
            if in_descriptions {
                if let Some(desc) = block.attr("Desc") {
                    if let Some(after) = replace_substring(desc, query, replacement, case_sensitive)
                    {
                        matches.push(FindReplaceMatch {
                            uuid: block.uuid.clone(),
                            object_type: block.block_type.clone(),
                            location_label: page.title.clone(),
                            field_name: "Desc".to_string(),
                            before: desc.to_string(),
                            after,
                        });
                    }
                }
            }
            if in_attributes {
                for (k, v) in &block.raw_attributes {
                    if matches!(k.as_str(), "U" | "Type" | "Title" | "Desc" | "Place" | "Cat") {
                        continue;
                    }
                    if let Some(after) = replace_substring(v, query, replacement, case_sensitive) {
                        matches.push(FindReplaceMatch {
                            uuid: block.uuid.clone(),
                            object_type: block.block_type.clone(),
                            location_label: page.title.clone(),
                            field_name: k.clone(),
                            before: v.clone(),
                            after,
                        });
                    }
                }
            }
        }
    }

    fn walk_tree_matches(
        node: &TreeNode,
        parent: &str,
        query: &str,
        replacement: &str,
        case_sensitive: bool,
        in_titles: bool,
        in_descriptions: bool,
        in_attributes: bool,
        matches: &mut Vec<FindReplaceMatch>,
    ) {
        if !node.uuid.is_empty() {
            if in_titles {
                if let Some(after) =
                    replace_substring(&node.title, query, replacement, case_sensitive)
                {
                    matches.push(FindReplaceMatch {
                        uuid: node.uuid.clone(),
                        object_type: node.control_type.clone(),
                        location_label: parent.to_string(),
                        field_name: "Title".to_string(),
                        before: node.title.clone(),
                        after,
                    });
                }
            }
            if in_descriptions {
                if let Some(desc) = node.attr("Desc") {
                    if let Some(after) = replace_substring(desc, query, replacement, case_sensitive)
                    {
                        matches.push(FindReplaceMatch {
                            uuid: node.uuid.clone(),
                            object_type: node.control_type.clone(),
                            location_label: parent.to_string(),
                            field_name: "Desc".to_string(),
                            before: desc.to_string(),
                            after,
                        });
                    }
                }
            }
            if in_attributes {
                for (k, v) in &node.raw_attributes {
                    if matches!(k.as_str(), "U" | "Type" | "Title" | "Desc" | "Place" | "Cat") {
                        continue;
                    }
                    if let Some(after) = replace_substring(v, query, replacement, case_sensitive) {
                        matches.push(FindReplaceMatch {
                            uuid: node.uuid.clone(),
                            object_type: node.control_type.clone(),
                            location_label: parent.to_string(),
                            field_name: k.clone(),
                            before: v.clone(),
                            after,
                        });
                    }
                }
            }
        }
        let next_parent = if parent.is_empty() {
            node.title.clone()
        } else {
            format!("{parent} / {}", node.title)
        };
        for child in &node.children {
            walk_tree_matches(
                child,
                &next_parent,
                query,
                replacement,
                case_sensitive,
                in_titles,
                in_descriptions,
                in_attributes,
                matches,
            );
        }
    }

    for root in &project.periphery_tree {
        walk_tree_matches(
            root,
            "",
            query,
            replacement,
            case_sensitive,
            in_titles,
            in_descriptions,
            in_attributes,
            &mut matches,
        );
    }
    for root in &project.system_tree {
        walk_tree_matches(
            root,
            "",
            query,
            replacement,
            case_sensitive,
            in_titles,
            in_descriptions,
            in_attributes,
            &mut matches,
        );
    }
    matches
}

pub fn build_find_and_replace_batch_edit(matches: &[FindReplaceMatch]) -> Option<ProjectEdit> {
    if matches.is_empty() {
        return None;
    }
    let edits: Vec<ProjectEdit> = matches
        .iter()
        .map(|m| ProjectEdit::SetControlAttr {
            control_uuid: m.uuid.clone(),
            attr_name: m.field_name.clone(),
            value: Some(m.after.clone()),
        })
        .collect();
    Some(ProjectEdit::Batch(edits))
}

fn render_find_and_replace(
    ctx: &egui::Context,
    state: &mut AdminModalsState,
    project: &LoxoneProject,
    out: &mut Vec<AdminAction>,
) -> bool {
    let status = state.status.clone();
    let matches = preview_find_and_replace(
        project,
        &state.find_query,
        &state.replace_with,
        state.find_case_sensitive,
        state.find_in_titles,
        state.find_in_descriptions,
        state.find_in_attributes,
    );
    modal_window(
        ctx,
        "admin_find_and_replace",
        "🔍 Find & Replace (Ctrl+H)",
        [960.0, 620.0],
        status.as_deref(),
        |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("Find:")).strong().color(theme::TEXT_SECONDARY));
                ui.add(
                    egui::TextEdit::singleline(&mut state.find_query)
                        .hint_text(tr("Search text…"))
                        .desired_width(210.0),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(tr("Replace with:"))
                        .strong()
                        .color(theme::TEXT_SECONDARY),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut state.replace_with)
                        .hint_text(tr("Replacement text…"))
                        .desired_width(210.0),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let btn_label = format!("✔ {} ({})", tr("Replace All"), matches.len());
                    if button(ui, &btn_label, !matches.is_empty()).clicked() && !matches.is_empty()
                    {
                        if let Some(batch) = build_find_and_replace_batch_edit(&matches) {
                            let count = matches.len();
                            out.push(AdminAction::Edit(batch));
                            state.status = Some(format!("Replaced {count} match(es)."));
                        }
                    }
                });
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.checkbox(&mut state.find_case_sensitive, tr("Case sensitive"));
                ui.checkbox(&mut state.find_in_titles, tr("Titles"));
                ui.checkbox(&mut state.find_in_descriptions, tr("Descriptions"));
                ui.checkbox(&mut state.find_in_attributes, tr("Attributes"));
            });
            ui.add_space(6.0);
            card(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("find_replace_scroll")
                    .max_height(ui.available_height() - 16.0)
                    .show(ui, |ui| {
                        egui::Grid::new("find_replace_grid")
                            .num_columns(5)
                            .striped(true)
                            .spacing([12.0, 6.0])
                            .show(ui, |ui| {
                                for h in ["Object Type", "Location", "Field", "Before", "After"] {
                                    ui.label(
                                        RichText::new(tr(h))
                                            .size(11.0)
                                            .strong()
                                            .color(theme::TEXT_SECONDARY),
                                    );
                                }
                                ui.end_row();
                                for m in &matches {
                                    ui.label(
                                        RichText::new(&m.object_type)
                                            .monospace()
                                            .size(11.0)
                                            .color(theme::TEXT_SECONDARY),
                                    );
                                    if ui
                                        .link(
                                            RichText::new(&m.location_label)
                                                .size(11.5)
                                                .color(theme::ACCENT_PRIMARY),
                                        )
                                        .clicked()
                                    {
                                        out.push(AdminAction::SelectObject(m.uuid.clone()));
                                    }
                                    ui.label(
                                        RichText::new(&m.field_name)
                                            .monospace()
                                            .size(11.0)
                                            .color(theme::TEXT_SECONDARY),
                                    );
                                    ui.label(
                                        RichText::new(&m.before)
                                            .size(11.5)
                                            .color(theme::TEXT_PRIMARY),
                                    );
                                    ui.label(
                                        RichText::new(&m.after)
                                            .size(11.5)
                                            .strong()
                                            .color(theme::ACCENT_PRIMARY),
                                    );
                                    ui.end_row();
                                }
                            });
                    });
            });
        },
    )
}

// ---------------------------------------------------------------------------
// Project Backups (`CProjectBackupsDialog`)
// ---------------------------------------------------------------------------

pub fn project_backups_dir() -> std::path::PathBuf {
    let base = std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    base.join(".config")
        .join("loxone-portable")
        .join("backups")
}

pub fn save_project_backup_to_dir(
    project: &LoxoneProject,
    dir: &std::path::Path,
    note: &str,
) -> Result<std::path::PathBuf, String> {
    let xml = project
        .raw_xml
        .as_deref()
        .ok_or_else(|| "Project has no raw XML to back up.".to_string())?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let dt = loxone_core::SimulatedDateTime::now_utc();
    let slug: String = project
        .metadata
        .title
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let note_slug: String = note
        .trim()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let suffix = if note_slug.is_empty() {
        String::new()
    } else {
        format!("_{note_slug}")
    };
    let file_name = format!(
        "{}_{:04}{:02}{:02}_{:02}{:02}{:02}{}.Loxone",
        if slug.is_empty() { "Project" } else { &slug },
        dt.year,
        dt.month,
        dt.day,
        dt.hour,
        dt.minute,
        dt.second,
        suffix
    );
    let path = dir.join(file_name);
    std::fs::write(&path, xml).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn list_project_backups_in_dir(dir: &std::path::Path) -> Vec<ProjectBackupEntry> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    for entry in rd.flatten() {
        let path = entry.path();
        if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("Loxone"))
        {
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let secs = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let dt = loxone_core::SimulatedDateTime::from_epoch_seconds(secs);
            entries.push(ProjectBackupEntry {
                file_name: path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Backup.Loxone")
                    .to_string(),
                path,
                timestamp_display: dt.format_iso(),
                size_bytes: meta.len(),
            });
        }
    }
    entries.sort_by(|a, b| b.timestamp_display.cmp(&a.timestamp_display));
    entries
}

fn render_project_backups(
    ctx: &egui::Context,
    state: &mut AdminModalsState,
    project: &LoxoneProject,
    out: &mut Vec<AdminAction>,
) -> bool {
    let status = state.status.clone();
    let mut close_after = false;
    let keep = modal_window(
        ctx,
        "admin_project_backups",
        "🗄 Project Backups",
        [860.0, 540.0],
        status.as_deref(),
        |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(tr("Snapshot label:"))
                        .strong()
                        .color(theme::TEXT_SECONDARY),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut state.backup_note)
                        .hint_text(tr("Optional note, e.g. Before HVAC changes"))
                        .desired_width(260.0),
                );
                if button(ui, "💾 Create Snapshot Now", true).clicked() {
                    let dir = project_backups_dir();
                    match save_project_backup_to_dir(project, &dir, &state.backup_note) {
                        Ok(saved) => {
                            state.status =
                                Some(format!("Saved backup snapshot: {}", saved.display()));
                            state.backup_note.clear();
                            state.backups_list = list_project_backups_in_dir(&dir);
                        }
                        Err(err) => {
                            state.status = Some(format!("Backup failed: {err}"));
                        }
                    }
                }
            });
            ui.add_space(8.0);
            card(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("project_backups_scroll")
                    .max_height(ui.available_height() - 16.0)
                    .show(ui, |ui| {
                        if state.backups_list.is_empty() {
                            ui.label(
                                RichText::new(tr("No local project backups created yet."))
                                    .color(theme::TEXT_SECONDARY),
                            );
                        }
                        egui::Grid::new("project_backups_grid")
                            .num_columns(4)
                            .striped(true)
                            .spacing([14.0, 6.0])
                            .show(ui, |ui| {
                                for h in ["Backup File", "Timestamp", "Size", ""] {
                                    ui.label(
                                        RichText::new(tr(h))
                                            .size(11.0)
                                            .strong()
                                            .color(theme::TEXT_SECONDARY),
                                    );
                                }
                                ui.end_row();
                                let mut to_delete = None;
                                for b in &state.backups_list {
                                    ui.label(
                                        RichText::new(&b.file_name)
                                            .monospace()
                                            .color(theme::TEXT_PRIMARY),
                                    );
                                    ui.label(
                                        RichText::new(&b.timestamp_display)
                                            .monospace()
                                            .color(theme::TEXT_SECONDARY),
                                    );
                                    ui.label(
                                        RichText::new(format!("{:.1} KB", b.size_bytes as f64 / 1024.0))
                                            .monospace()
                                            .color(theme::TEXT_SECONDARY),
                                    );
                                    ui.horizontal(|ui| {
                                        if button(ui, "📂 Restore", true).clicked() {
                                            out.push(AdminAction::RestoreBackupFile(b.path.clone()));
                                            close_after = true;
                                        }
                                        if ui.small_button("🗑").clicked() {
                                            to_delete = Some(b.path.clone());
                                        }
                                    });
                                    ui.end_row();
                                }
                                if let Some(del_path) = to_delete {
                                    let _ = std::fs::remove_file(&del_path);
                                    state.backups_list =
                                        list_project_backups_in_dir(&project_backups_dir());
                                }
                            });
                    });
            });
        },
    );
    keep && !close_after
}

// ---------------------------------------------------------------------------
// Auto Configuration Wizard (`CWizardDialog`)
// ---------------------------------------------------------------------------

pub fn build_auto_config_room_rows(project: &LoxoneProject) -> Vec<AutoConfigRoomRow> {
    let mut places: Vec<(&String, &loxone_core::model::Place)> = project.places.iter().collect();
    places.sort_by(|a, b| a.1.title.cmp(&b.1.title));

    let mut rows = Vec::new();
    for (room_uuid, place) in places {
        let mut has_lighting = false;
        let mut has_shading = false;
        let mut has_climate = false;
        let mut has_audio = false;

        for page in &project.pages {
            let page_matches_room = page.title.eq_ignore_ascii_case(&place.title);
            for block in &page.blocks {
                let in_room = block.room_uuid.as_deref() == Some(room_uuid.as_str())
                    || (block.room_uuid.is_none() && page_matches_room);
                if !in_room {
                    continue;
                }
                match block.block_type.as_str() {
                    "LightController2" | "LightControllerV2" | "LightController" => {
                        has_lighting = true;
                    }
                    "AutoJalousie" | "Jalousie" => {
                        has_shading = true;
                    }
                    "HeatIRoomController2" | "IRoomControllerV2" | "IRoomController" => {
                        has_climate = true;
                    }
                    "AudioZone" | "MusicPlayer" | "MusicServerZone" => {
                        has_audio = true;
                    }
                    _ => {}
                }
            }
        }

        let missing_any = !has_lighting || !has_shading || !has_climate;
        rows.push(AutoConfigRoomRow {
            room_uuid: room_uuid.clone(),
            room_title: place.title.clone(),
            selected: missing_any,
            has_lighting,
            has_shading,
            has_climate,
            has_audio,
            gen_lighting: !has_lighting,
            gen_shading: !has_shading,
            gen_climate: !has_climate,
            gen_audio: false,
        });
    }
    rows
}

pub fn build_auto_configuration_edits(
    project: &LoxoneProject,
    rows: &[AutoConfigRoomRow],
) -> Option<(ProjectEdit, usize)> {
    let mut edits = Vec::new();
    let mut generated_blocks = 0usize;

    let find_category_uuid = |group_hint: &str| -> Option<String> {
        project
            .categories
            .values()
            .find(|c| {
                c.cat_group
                    .as_deref()
                    .is_some_and(|g| g.eq_ignore_ascii_case(group_hint))
                    || c.title.to_lowercase().contains(&group_hint.to_lowercase())
            })
            .map(|c| c.uuid.clone())
    };
    let cat_lights = find_category_uuid("lights").or_else(|| find_category_uuid("oświetlenie"));
    let cat_shading = find_category_uuid("shading").or_else(|| find_category_uuid("zacienienie"));
    let cat_climate = find_category_uuid("climate")
        .or_else(|| find_category_uuid("temperatura"))
        .or_else(|| find_category_uuid("klimat"));
    let cat_audio = find_category_uuid("multimedia").or_else(|| find_category_uuid("audio"));

    for row in rows.iter().filter(|r| r.selected) {
        let mut specs: Vec<(&str, String, Option<String>)> = Vec::new();
        if row.gen_lighting {
            specs.push((
                "LightController2",
                format!("{} — Lighting", row.room_title),
                cat_lights.clone(),
            ));
        }
        if row.gen_shading {
            specs.push((
                "AutoJalousie",
                format!("{} — Shading", row.room_title),
                cat_shading.clone(),
            ));
        }
        if row.gen_climate {
            specs.push((
                "HeatIRoomController2",
                format!("{} — Room Climate", row.room_title),
                cat_climate.clone(),
            ));
        }
        if row.gen_audio {
            specs.push((
                "AudioZone",
                format!("{} — Audio", row.room_title),
                cat_audio.clone(),
            ));
        }
        if specs.is_empty() {
            continue;
        }

        let (page_uuid, mut next_py) = if let Some(existing_page) = project
            .pages
            .iter()
            .find(|p| p.title.eq_ignore_ascii_case(&row.room_title))
        {
            let max_py = existing_page
                .blocks
                .iter()
                .filter(|b| b.has_position)
                .map(|b| b.py2.max(b.py + 640.0))
                .fold(192.0_f32, f32::max);
            (existing_page.uuid.clone(), max_py + 256.0)
        } else {
            let new_page = loxone_core::factory::new_page(&row.room_title);
            let pu = new_page.uuid.clone();
            edits.push(ProjectEdit::AddPage { page: new_page });
            (pu, 256.0_f32)
        };

        for (block_type, title, cat_uuid) in specs {
            let mut blk = loxone_core::factory::new_page_block(block_type, &title, 1920.0, next_py);
            blk.room_uuid = Some(row.room_uuid.clone());
            blk.category_uuid = cat_uuid.clone();
            let mut iodata_attrs = vec![("Pr".to_string(), row.room_uuid.clone())];
            if let Some(cu) = cat_uuid {
                iodata_attrs.push(("Cr".to_string(), cu));
            }
            blk.extra_children.push(loxone_core::model::RawNode {
                tag: "IoData".to_string(),
                attributes: iodata_attrs,
                text: None,
                children: Vec::new(),
            });
            next_py += 896.0;
            generated_blocks += 1;
            edits.push(ProjectEdit::AddPageBlock {
                page_uuid: page_uuid.clone(),
                block: blk,
            });
        }
    }

    if edits.is_empty() {
        None
    } else {
        Some((ProjectEdit::Batch(edits), generated_blocks))
    }
}

fn render_auto_configuration(
    ctx: &egui::Context,
    state: &mut AdminModalsState,
    project: &LoxoneProject,
    out: &mut Vec<AdminAction>,
) -> bool {
    let status = state.status.clone();
    modal_window(
        ctx,
        "admin_auto_configuration",
        "🪄 Auto Configuration Wizard (Room Logic Generator)",
        [940.0, 580.0],
        status.as_deref(),
        |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(tr(
                        "Select rooms and standard Loxone function blocks to generate automatically:",
                    ))
                    .size(12.0)
                    .color(theme::TEXT_SECONDARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let selected_rooms =
                        state.auto_config_rows.iter().filter(|r| r.selected).count();
                    let btn_lbl = format!(
                        "🪄 {} ({})",
                        tr("Generate Room Logic"),
                        selected_rooms
                    );
                    if button(ui, &btn_lbl, selected_rooms > 0).clicked() && selected_rooms > 0 {
                        if let Some((batch, count)) =
                            build_auto_configuration_edits(project, &state.auto_config_rows)
                        {
                            out.push(AdminAction::Edit(batch));
                            state.status = Some(format!(
                                "Generated {count} standard room logic block(s) across {selected_rooms} room(s)."
                            ));
                        } else {
                            state.status = Some(
                                "No subsystems checked for the selected rooms.".to_string(),
                            );
                        }
                    }
                    if button(ui, "Select All Rooms", false).clicked() {
                        for r in &mut state.auto_config_rows {
                            r.selected = true;
                        }
                    }
                    if button(ui, "Clear Selection", false).clicked() {
                        for r in &mut state.auto_config_rows {
                            r.selected = false;
                        }
                    }
                });
            });
            ui.add_space(6.0);
            card(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("auto_config_rooms_scroll")
                    .max_height(ui.available_height() - 16.0)
                    .show(ui, |ui| {
                        egui::Grid::new("auto_config_rooms_grid")
                            .num_columns(6)
                            .striped(true)
                            .spacing([16.0, 6.0])
                            .show(ui, |ui| {
                                for h in [
                                    "Room",
                                    "Existing Blocks",
                                    "💡 Lighting (LightController2)",
                                    "🪟 Shading (AutoJalousie)",
                                    "🌡 Climate (HeatIRoomController2)",
                                    "🎵 Audio (AudioZone)",
                                ] {
                                    ui.label(
                                        RichText::new(tr(h))
                                            .size(11.0)
                                            .strong()
                                            .color(theme::TEXT_SECONDARY),
                                    );
                                }
                                ui.end_row();
                                for row in &mut state.auto_config_rows {
                                    ui.checkbox(
                                        &mut row.selected,
                                        RichText::new(&row.room_title)
                                            .strong()
                                            .color(theme::TEXT_PRIMARY),
                                    );
                                    let mut badges = Vec::new();
                                    if row.has_lighting {
                                        badges.push("💡");
                                    }
                                    if row.has_shading {
                                        badges.push("🪟");
                                    }
                                    if row.has_climate {
                                        badges.push("🌡");
                                    }
                                    if row.has_audio {
                                        badges.push("🎵");
                                    }
                                    let existing_str = if badges.is_empty() {
                                        "—".to_string()
                                    } else {
                                        badges.join(" ")
                                    };
                                    ui.label(
                                        RichText::new(existing_str)
                                            .size(11.5)
                                            .color(theme::TEXT_SECONDARY),
                                    );
                                    ui.checkbox(&mut row.gen_lighting, "");
                                    ui.checkbox(&mut row.gen_shading, "");
                                    ui.checkbox(&mut row.gen_climate, "");
                                    ui.checkbox(&mut row.gen_audio, "");
                                    ui.end_row();
                                }
                            });
                    });
            });
        },
    )
}

fn render_todo_list(
    ctx: &egui::Context,
    state: &mut AdminModalsState,
    project: &LoxoneProject,
    out: &mut Vec<AdminAction>,
) -> bool {
    let status = state.status.clone();
    let todos = collect_project_todos(project);
    modal_window(
        ctx,
        "admin_todo_list",
        "📌 Project To-Do & Memo Manager",
        [860.0, 560.0],
        status.as_deref(),
        |ui| {
            card(ui, |ui| {
                section_label(ui, "Add New To-Do / Memo Note");
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Page:").size(11.5).color(theme::TEXT_SECONDARY));
                    let selected_page_title = project
                        .pages
                        .iter()
                        .find(|p| p.uuid == state.todo_page_uuid_draft)
                        .map(|p| p.title.as_str())
                        .unwrap_or("Select Page");
                    egui::ComboBox::from_id_salt("todo_page_combo")
                        .selected_text(selected_page_title)
                        .show_ui(ui, |ui| {
                            for p in &project.pages {
                                ui.selectable_value(
                                    &mut state.todo_page_uuid_draft,
                                    p.uuid.clone(),
                                    &p.title,
                                );
                            }
                        });
                    ui.label(RichText::new("Title:").size(11.5).color(theme::TEXT_SECONDARY));
                    ui.add(
                        egui::TextEdit::singleline(&mut state.todo_title_draft)
                            .hint_text("To-Do summary")
                            .desired_width(170.0),
                    );
                    ui.label(RichText::new("Note:").size(11.5).color(theme::TEXT_SECONDARY));
                    ui.add(
                        egui::TextEdit::singleline(&mut state.todo_text_draft)
                            .hint_text("Details...")
                            .desired_width(210.0),
                    );
                    let can_add = !state.todo_page_uuid_draft.is_empty()
                        && !state.todo_title_draft.trim().is_empty();
                    if button(ui, "➕ Add To-Do", can_add).clicked() && can_add {
                        let (_, edit) = build_add_todo_edit(
                            &state.todo_page_uuid_draft,
                            state.todo_title_draft.trim(),
                            state.todo_text_draft.trim(),
                            192.0,
                            128.0,
                        );
                        out.push(AdminAction::Edit(edit));
                        state.status = Some(format!(
                            "Added To-Do '{}' to page.",
                            state.todo_title_draft.trim()
                        ));
                        state.todo_title_draft.clear();
                        state.todo_text_draft.clear();
                    }
                });
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("To-Do / Memo Items ({})", todos.len()))
                        .size(12.5)
                        .strong()
                        .color(theme::TEXT_PRIMARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut state.todo_filter)
                            .hint_text("🔎 Filter To-Do notes...")
                            .desired_width(240.0),
                    );
                });
            });
            ui.add_space(4.0);
            let q = state.todo_filter.trim().to_ascii_lowercase();
            card(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("todo_items_scroll")
                    .max_height(ui.available_height() - 16.0)
                    .show(ui, |ui| {
                        for item in todos.iter().filter(|t| {
                            q.is_empty()
                                || t.title.to_ascii_lowercase().contains(&q)
                                || t.text.to_ascii_lowercase().contains(&q)
                                || t.page_title.to_ascii_lowercase().contains(&q)
                        }) {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(if item.is_resolved { "✅" } else { "📌" })
                                        .size(12.5),
                                );
                                ui.label(
                                    RichText::new(&item.title)
                                        .size(12.0)
                                        .strong()
                                        .color(theme::TEXT_PRIMARY),
                                );
                                ui.label(
                                    RichText::new(format!("[{}]", item.page_title))
                                        .size(11.0)
                                        .color(theme::ACCENT_PRIMARY),
                                );
                                if !item.text.is_empty() {
                                    ui.label(
                                        RichText::new(format!("— {}", item.text))
                                            .size(11.0)
                                            .color(theme::TEXT_SECONDARY),
                                    );
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.small_button("🗑 Delete").clicked() {
                                            if let Some(edit) =
                                                build_resolve_todo_edit(project, &item.uuid, true)
                                            {
                                                out.push(AdminAction::Edit(edit));
                                            }
                                        }
                                        if !item.is_resolved && ui.small_button("✓ Done").clicked()
                                        {
                                            if let Some(edit) =
                                                build_resolve_todo_edit(project, &item.uuid, false)
                                            {
                                                out.push(AdminAction::Edit(edit));
                                            }
                                        }
                                        if ui.small_button("↗ Jump").clicked() {
                                            out.push(AdminAction::SelectObject(item.uuid.clone()));
                                        }
                                    },
                                );
                            });
                            ui.separator();
                        }
                    });
            });
        },
    )
}

fn render_autopilot_designer(
    ctx: &egui::Context,
    state: &mut AdminModalsState,
    project: &LoxoneProject,
    out: &mut Vec<AdminAction>,
) -> bool {
    let status = state.status.clone();
    let existing_rules = collect_autopilot_rules(project);
    modal_window(
        ctx,
        "admin_autopilot_designer",
        "🤖 Loxone Autopilot Rule Designer (IF / AND / OR → Action)",
        [880.0, 580.0],
        status.as_deref(),
        |ui| {
            card(ui, |ui| {
                section_label(ui, "Create New Autopilot Rule");
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Rule Name:").size(11.5).color(theme::TEXT_SECONDARY));
                    ui.add(
                        egui::TextEdit::singleline(&mut state.ap_title_draft).desired_width(180.0),
                    );
                    ui.label(RichText::new("Page:").size(11.5).color(theme::TEXT_SECONDARY));
                    let page_title = project
                        .pages
                        .iter()
                        .find(|p| p.uuid == state.ap_page_uuid_draft)
                        .map(|p| p.title.as_str())
                        .unwrap_or("Select Page");
                    egui::ComboBox::from_id_salt("ap_page_combo")
                        .selected_text(page_title)
                        .show_ui(ui, |ui| {
                            for p in &project.pages {
                                ui.selectable_value(
                                    &mut state.ap_page_uuid_draft,
                                    p.uuid.clone(),
                                    &p.title,
                                );
                            }
                        });
                    ui.label(RichText::new("Operator:").size(11.5).color(theme::TEXT_SECONDARY));
                    egui::ComboBox::from_id_salt("ap_op_combo")
                        .selected_text(&state.ap_logic_op)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.ap_logic_op, "AND".to_string(), "AND");
                            ui.selectable_value(&mut state.ap_logic_op, "OR".to_string(), "OR");
                        });
                });
                ui.add_space(6.0);
                if let Some(page) = project
                    .pages
                    .iter()
                    .find(|p| p.uuid == state.ap_page_uuid_draft)
                {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Target Block:")
                                .size(11.5)
                                .color(theme::TEXT_SECONDARY),
                        );
                        let tgt_label = page
                            .blocks
                            .iter()
                            .find(|b| b.uuid == state.ap_target_block_uuid)
                            .map(|b| format!("{} [{}]", b.title, b.block_type))
                            .unwrap_or_else(|| "(None)".to_string());
                        egui::ComboBox::from_id_salt("ap_target_block_combo")
                            .selected_text(tgt_label)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut state.ap_target_block_uuid,
                                    String::new(),
                                    "(None)",
                                );
                                for b in &page.blocks {
                                    ui.selectable_value(
                                        &mut state.ap_target_block_uuid,
                                        b.uuid.clone(),
                                        format!("{} [{}]", b.title, b.block_type),
                                    );
                                }
                            });
                        ui.label(RichText::new("Pin:").size(11.5).color(theme::TEXT_SECONDARY));
                        ui.add(
                            egui::TextEdit::singleline(&mut state.ap_target_pin_key)
                                .desired_width(60.0),
                        );
                        let can_create = !state.ap_title_draft.trim().is_empty();
                        if button(ui, "⚡ Generate Autopilot Rule", can_create).clicked()
                            && can_create
                        {
                            let spec = AutopilotRuleSpec {
                                title: state.ap_title_draft.clone(),
                                page_uuid: state.ap_page_uuid_draft.clone(),
                                logic_op: state.ap_logic_op.clone(),
                                condition_source_uuids: state.ap_selected_conditions.clone(),
                                target_block_uuid: state.ap_target_block_uuid.clone(),
                                target_pin_key: state.ap_target_pin_key.clone(),
                                px: 1024.0,
                                py: 256.0,
                            };
                            if let Some((_, edit)) = build_autopilot_rule_edits(project, &spec) {
                                out.push(AdminAction::Edit(edit));
                                state.status = Some(format!(
                                    "Created Autopilot rule '{}' on page '{}'.",
                                    spec.title, page.title
                                ));
                            }
                        }
                    });
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Condition Sources on Page:")
                            .size(11.0)
                            .color(theme::TEXT_SECONDARY),
                    );
                    egui::ScrollArea::vertical()
                        .id_salt("ap_conds_scroll")
                        .max_height(140.0)
                        .show(ui, |ui| {
                            for b in &page.blocks {
                                let mut checked =
                                    state.ap_selected_conditions.iter().any(|u| u == &b.uuid);
                                if ui
                                    .checkbox(
                                        &mut checked,
                                        format!("{} [{}]", b.title, b.block_type),
                                    )
                                    .changed()
                                {
                                    if let Some(pos) = state
                                        .ap_selected_conditions
                                        .iter()
                                        .position(|u| u == &b.uuid)
                                    {
                                        state.ap_selected_conditions.remove(pos);
                                    } else {
                                        state.ap_selected_conditions.push(b.uuid.clone());
                                    }
                                }
                            }
                        });
                }
            });
            ui.add_space(6.0);
            card(ui, |ui| {
                section_label(
                    ui,
                    &format!("Existing Autopilot Rules ({})", existing_rules.len()),
                );
                egui::ScrollArea::vertical()
                    .id_salt("ap_existing_scroll")
                    .max_height(150.0)
                    .show(ui, |ui| {
                        for r in &existing_rules {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(&r.title)
                                        .size(12.0)
                                        .strong()
                                        .color(theme::TEXT_PRIMARY),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "[{}] • {} • {} cond(s)",
                                        r.logic_op, r.page_title, r.condition_count
                                    ))
                                    .size(11.0)
                                    .color(theme::TEXT_SECONDARY),
                                );
                                if ui.small_button("↗ Jump").clicked() {
                                    out.push(AdminAction::SelectObject(r.uuid.clone()));
                                }
                            });
                        }
                    });
            });
        },
    )
}

fn render_project_diff(
    ctx: &egui::Context,
    state: &mut AdminModalsState,
    project: &LoxoneProject,
    out: &mut Vec<AdminAction>,
) -> bool {
    let status = state.status.clone();
    modal_window(
        ctx,
        "admin_project_diff",
        "⚖ Compare Projects (Structural Diff)",
        [920.0, 580.0],
        status.as_deref(),
        |ui| {
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Compare against .Loxone file path (optional):")
                            .size(11.5)
                            .color(theme::TEXT_SECONDARY),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut state.diff_compare_path)
                            .hint_text("Leave blank to compare against saved baseline")
                            .desired_width(340.0),
                    );
                    if button(ui, "🔄 Compare File", !state.diff_compare_path.trim().is_empty())
                        .clicked()
                    {
                        let p = std::path::Path::new(state.diff_compare_path.trim());
                        match loxone_core::parser::load_loxone_file(p) {
                            Ok(other) => {
                                state.diff_cached_report =
                                    Some(loxone_core::compare_projects(&other, project));
                                state.status = Some(format!(
                                    "Compared active project against '{}'.",
                                    p.display()
                                ));
                            }
                            Err(e) => {
                                state.status = Some(format!("Failed to load comparison file: {e}"));
                            }
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut state.diff_filter)
                                .hint_text("🔎 Filter changes...")
                                .desired_width(180.0),
                        );
                    });
                });
            });
            ui.add_space(6.0);
            if let Some(report) = &state.diff_cached_report {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "Pages: +{} / ~{} / -{}   •   Blocks: +{} / ~{} / -{}   •   Tree Nodes: +{} / ~{} / -{}",
                            report.added_pages,
                            report.modified_pages,
                            report.removed_pages,
                            report.added_blocks,
                            report.modified_blocks,
                            report.removed_blocks,
                            report.added_tree_nodes,
                            report.modified_tree_nodes,
                            report.removed_tree_nodes
                        ))
                        .size(12.0)
                        .strong()
                        .color(theme::ACCENT_PRIMARY),
                    );
                });
                ui.add_space(4.0);
                let q = state.diff_filter.trim().to_ascii_lowercase();
                card(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("project_diff_entries_scroll")
                        .max_height(ui.available_height() - 16.0)
                        .show(ui, |ui| {
                            if report.entries.is_empty() {
                                ui.label(
                                    RichText::new("✓ No structural differences detected.")
                                        .size(12.0)
                                        .color(theme::TEXT_SECONDARY),
                                );
                            }
                            for entry in report.entries.iter().filter(|e| {
                                q.is_empty()
                                    || e.title.to_ascii_lowercase().contains(&q)
                                    || e.object_type.to_ascii_lowercase().contains(&q)
                                    || e.location.to_ascii_lowercase().contains(&q)
                                    || e.detail.to_ascii_lowercase().contains(&q)
                            }) {
                                ui.horizontal(|ui| {
                                    let badge_color = match entry.change_kind.as_str() {
                                        "Added" => theme::ACCENT_PRIMARY,
                                        "Removed" => egui::Color32::from_rgb(0xE8, 0x5D, 0x5D),
                                        _ => theme::ACCENT_AMBER,
                                    };
                                    ui.label(
                                        RichText::new(format!("[{}]", entry.change_kind))
                                            .size(11.0)
                                            .monospace()
                                            .strong()
                                            .color(badge_color),
                                    );
                                    ui.label(
                                        RichText::new(format!("{}: {}", entry.object_kind, entry.title))
                                            .size(12.0)
                                            .strong()
                                            .color(theme::TEXT_PRIMARY),
                                    );
                                    ui.label(
                                        RichText::new(format!("({}) • {}", entry.object_type, entry.location))
                                            .size(11.0)
                                            .color(theme::TEXT_SECONDARY),
                                    );
                                    ui.label(
                                        RichText::new(format!("— {}", entry.detail))
                                            .size(11.0)
                                            .color(theme::TEXT_MUTED),
                                    );
                                    if entry.change_kind != "Removed" {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui.small_button("↗ Jump").clicked() {
                                                    out.push(AdminAction::SelectObject(
                                                        entry.uuid.clone(),
                                                    ));
                                                }
                                            },
                                        );
                                    }
                                });
                                ui.separator();
                            }
                        });
                });
            }
        },
    )
}

fn render_tech_report_wizard(
    ctx: &egui::Context,
    state: &mut AdminModalsState,
    project: &LoxoneProject,
    out: &mut Vec<AdminAction>,
) -> bool {
    let status = state.status.clone();
    modal_window(
        ctx,
        "admin_tech_report_wizard",
        "BOM & Technical Report Wizard (CTechReportWizard)",
        [980.0, 660.0],
        status.as_deref(),
        |ui| {
            let bom = loxone_core::build_project_bom(project);
            let schedule = loxone_core::collect_terminal_schedule(project);
            let total_wired = schedule.iter().filter(|t| t.is_wired).count();

            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut state.tech_report_tab,
                    0,
                    format!("📦 Bill of Materials ({} devices)", bom.devices.len()),
                );
                ui.selectable_value(
                    &mut state.tech_report_tab,
                    1,
                    format!(
                        "🔌 DIN-Rail & Terminal Schedule ({total_wired}/{} wired)",
                        schedule.len()
                    ),
                );
                ui.separator();
                ui.label(RichText::new("Filter:").size(12.0).color(theme::TEXT_SECONDARY));
                ui.add(
                    egui::TextEdit::singleline(&mut state.tech_report_filter)
                        .desired_width(170.0)
                        .hint_text("Board, room, serial, block…"),
                );
                if state.tech_report_tab == 1 {
                    ui.checkbox(&mut state.tech_report_wired_only, "Wired only");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if button(ui, "📄 Export HTML Report", true).clicked() {
                        let html = loxone_core::technical_report_html(project);
                        let out_path = std::env::temp_dir().join("loxone_technical_report.html");
                        if std::fs::write(&out_path, html).is_ok() {
                            state.status =
                                Some(format!("Saved HTML report to {}", out_path.display()));
                        }
                    }
                    if button(ui, "🔌 Export Terminal CSV", false).clicked() {
                        let csv = loxone_core::export_terminal_schedule_csv(project);
                        let out_path = std::env::temp_dir().join("loxone_terminal_schedule.csv");
                        if std::fs::write(&out_path, csv).is_ok() {
                            state.status =
                                Some(format!("Saved Terminal CSV to {}", out_path.display()));
                        }
                    }
                    if button(ui, "📦 Export BOM CSV", false).clicked() {
                        let csv = loxone_core::export_component_bom_csv(project);
                        let out_path = std::env::temp_dir().join("loxone_component_bom.csv");
                        if std::fs::write(&out_path, csv).is_ok() {
                            state.status =
                                Some(format!("Saved BOM CSV to {}", out_path.display()));
                        }
                    }
                });
            });

            ui.add_space(6.0);
            let q = state.tech_report_filter.trim().to_ascii_lowercase();

            if state.tech_report_tab == 0 {
                section_label(ui, "Hardware Part Totals");
                ui.horizontal_wrapped(|ui| {
                    for part in &bom.hardware_summaries {
                        egui::Frame::NONE
                            .fill(theme::BG_WIDGET)
                            .stroke(Stroke::new(1.0_f32, theme::BORDER_SUBTLE))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(Margin::symmetric(10, 6))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "{} × {} ({}/{} I/O wired)",
                                        part.quantity,
                                        part.part_type,
                                        part.wired_channels,
                                        part.total_channels
                                    ))
                                    .size(11.5)
                                    .strong()
                                    .color(theme::TEXT_PRIMARY),
                                );
                            });
                    }
                });

                section_label(ui, "Provisioned Hardware & DIN Panel Placement");
                card(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("tech_report_bom_scroll")
                        .max_height(ui.available_height() - 16.0)
                        .show(ui, |ui| {
                            for dev in bom.devices.iter().filter(|d| {
                                q.is_empty()
                                    || d.title.to_ascii_lowercase().contains(&q)
                                    || d.node_type.to_ascii_lowercase().contains(&q)
                                    || d.serial.to_ascii_lowercase().contains(&q)
                                    || d.distribution_board.to_ascii_lowercase().contains(&q)
                                    || d.row_position.to_ascii_lowercase().contains(&q)
                            }) {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(&dev.title)
                                            .size(12.5)
                                            .strong()
                                            .color(theme::TEXT_PRIMARY),
                                    );
                                    ui.label(
                                        RichText::new(format!("[{}]", dev.node_type))
                                            .size(11.0)
                                            .monospace()
                                            .color(theme::ACCENT_PRIMARY),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "S/N: {} • Bus: {}",
                                            if dev.serial.is_empty() {
                                                "Unset"
                                            } else {
                                                &dev.serial
                                            },
                                            dev.parent_label
                                        ))
                                        .size(11.0)
                                        .color(theme::TEXT_SECONDARY),
                                    );
                                    if !dev.distribution_board.is_empty()
                                        || !dev.row_position.is_empty()
                                    {
                                        ui.label(
                                            RichText::new(format!(
                                                "• Panel: {} {}",
                                                dev.distribution_board, dev.row_position
                                            ))
                                            .size(11.0)
                                            .color(theme::ACCENT_AMBER),
                                        );
                                    }
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.small_button("↗ Select").clicked() {
                                                out.push(AdminAction::SelectObject(
                                                    dev.uuid.clone(),
                                                ));
                                            }
                                            ui.label(
                                                RichText::new(format!(
                                                    "{}/{} channels wired",
                                                    dev.wired_channels, dev.total_channels
                                                ))
                                                .size(11.0)
                                                .color(theme::TEXT_MUTED),
                                            );
                                        },
                                    );
                                });
                                ui.separator();
                            }
                        });
                });
            } else {
                section_label(ui, "Physical Terminal to Schematic Block Mapping");
                card(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("tech_report_schedule_scroll")
                        .max_height(ui.available_height() - 16.0)
                        .show(ui, |ui| {
                            for term in schedule.iter().filter(|t| {
                                if state.tech_report_wired_only && !t.is_wired {
                                    return false;
                                }
                                q.is_empty()
                                    || t.device_title.to_ascii_lowercase().contains(&q)
                                    || t.channel_title.to_ascii_lowercase().contains(&q)
                                    || t.terminal_label.to_ascii_lowercase().contains(&q)
                                    || t.distribution_board.to_ascii_lowercase().contains(&q)
                                    || t.room_title.to_ascii_lowercase().contains(&q)
                                    || t.connected_blocks
                                        .iter()
                                        .any(|b| b.to_ascii_lowercase().contains(&q))
                            }) {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!("[{}]", term.terminal_label))
                                            .size(11.5)
                                            .monospace()
                                            .strong()
                                            .color(if term.is_wired {
                                                theme::ACCENT_PRIMARY
                                            } else {
                                                theme::TEXT_MUTED
                                            }),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "{} → {}",
                                            term.device_title, term.channel_title
                                        ))
                                        .size(12.0)
                                        .strong()
                                        .color(theme::TEXT_PRIMARY),
                                    );
                                    if !term.distribution_board.is_empty()
                                        || !term.row_position.is_empty()
                                    {
                                        ui.label(
                                            RichText::new(format!(
                                                "({} {})",
                                                term.distribution_board, term.row_position
                                            ))
                                            .size(11.0)
                                            .color(theme::ACCENT_AMBER),
                                        );
                                    }
                                    if !term.room_title.is_empty() {
                                        ui.label(
                                            RichText::new(format!("• {}", term.room_title))
                                                .size(11.0)
                                                .color(theme::TEXT_SECONDARY),
                                        );
                                    }
                                    let conn_label = if term.connected_blocks.is_empty() {
                                        if term.connected_pages.is_empty() {
                                            "Unwired".to_string()
                                        } else {
                                            format!("Pages: {}", term.connected_pages.join(", "))
                                        }
                                    } else {
                                        format!(
                                            "⇄ {} ({})",
                                            term.connected_blocks.join(", "),
                                            term.connected_pages.join(", ")
                                        )
                                    };
                                    ui.label(
                                        RichText::new(conn_label)
                                            .size(11.0)
                                            .color(if term.is_wired {
                                                theme::ACCENT_PRIMARY
                                            } else {
                                                theme::TEXT_MUTED
                                            }),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.small_button("↗ Go to").clicked() {
                                                out.push(AdminAction::SelectObject(
                                                    term.channel_uuid.clone(),
                                                ));
                                            }
                                        },
                                    );
                                });
                                ui.separator();
                            }
                        });
                });
            }
        },
    )
}

fn render_multiplier_manager(
    ctx: &egui::Context,
    state: &mut AdminModalsState,
    project: &LoxoneProject,
    out: &mut Vec<AdminAction>,
) -> bool {
    let status = state.status.clone();
    modal_window(
        ctx,
        "admin_multiplier_manager",
        "Multiplier Project Manager (Projekt Multiplikator)",
        [940.0, 620.0],
        status.as_deref(),
        |ui| {
            let units = loxone_core::collect_multiplier_units(project);
            let hw_devices = loxone_core::collect_provisioned_hardware(project);
            let is_mult = loxone_core::is_multiplier_project(project);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(if is_mult {
                        "● Multiplier Project Active"
                    } else {
                        "○ Standard Single-Miniserver Project (add a unit to convert to Multiplier)"
                    })
                    .size(12.5)
                    .strong()
                    .color(if is_mult {
                        theme::ACCENT_PRIMARY
                    } else {
                        theme::TEXT_SECONDARY
                    }),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if button(ui, "＋ New Unit Draft", false).clicked() {
                        state.mult_selected_uuid = None;
                        state.mult_title_draft = format!("Unit {}", units.len() + 101);
                        state.mult_serial_draft = format!("504F94FF{:04}", units.len() + 101);
                        state.mult_address_draft = format!("192.168.1.{}", units.len() + 101);
                        state.mult_desc_draft.clear();
                        state.mult_overrides_draft.clear();
                    }
                });
            });

            ui.add_space(6.0);
            ui.columns(2, |cols| {
                cols[0].vertical(|ui| {
                    section_label(ui, &format!("Managed Units ({})", units.len()));
                    card(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("multiplier_units_scroll")
                            .max_height(460.0)
                            .show(ui, |ui| {
                                if units.is_empty() {
                                    ui.label(
                                        RichText::new(
                                            "No Multiplier units configured yet. Fill in the form on the right and click Save Unit.",
                                        )
                                        .size(11.5)
                                        .color(theme::TEXT_MUTED),
                                    );
                                }
                                for u in &units {
                                    let is_sel =
                                        state.mult_selected_uuid.as_deref() == Some(&u.uuid);
                                    ui.horizontal(|ui| {
                                        if ui
                                            .selectable_label(
                                                is_sel,
                                                format!(
                                                    "{} [{}] ({})",
                                                    u.title, u.miniserver_serial, u.address
                                                ),
                                            )
                                            .clicked()
                                        {
                                            state.mult_selected_uuid = Some(u.uuid.clone());
                                            state.mult_title_draft = u.title.clone();
                                            state.mult_serial_draft = u.miniserver_serial.clone();
                                            state.mult_address_draft = u.address.clone();
                                            state.mult_desc_draft = u.description.clone();
                                            state.mult_overrides_draft =
                                                u.device_serial_overrides.clone();
                                        }
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui.small_button("🗑").clicked() {
                                                    out.push(AdminAction::Edit(
                                                        loxone_core::build_remove_multiplier_unit_edit(&u.uuid),
                                                    ));
                                                    if state.mult_selected_uuid.as_deref()
                                                        == Some(&u.uuid)
                                                    {
                                                        state.mult_selected_uuid = None;
                                                    }
                                                }
                                                if ui
                                                    .small_button("💾 Export .Loxone")
                                                    .on_hover_text(
                                                        "Instantiate and export a concrete .Loxone file for this unit",
                                                    )
                                                    .clicked()
                                                {
                                                    if let Some(derived) =
                                                        loxone_core::instantiate_multiplier_unit_project(
                                                            project, &u.uuid,
                                                        )
                                                    {
                                                        if let Ok(xml) =
                                                            loxone_core::writer::serialize_loxone_xml(
                                                                &derived,
                                                            )
                                                        {
                                                            let safe_name = u
                                                                .title
                                                                .replace(' ', "_")
                                                                .replace('/', "_");
                                                            let out_path = std::env::temp_dir()
                                                                .join(format!("{safe_name}.Loxone"));
                                                            if std::fs::write(&out_path, xml).is_ok()
                                                            {
                                                                state.status = Some(format!(
                                                                    "Exported unit project to {}",
                                                                    out_path.display()
                                                                ));
                                                            }
                                                        }
                                                    }
                                                }
                                            },
                                        );
                                    });
                                    ui.separator();
                                }
                            });
                    });
                });

                cols[1].vertical(|ui| {
                    section_label(
                        ui,
                        if state.mult_selected_uuid.is_some() {
                            "Edit Selected Multiplier Unit"
                        } else {
                            "Create New Multiplier Unit"
                        },
                    );
                    card(ui, |ui| {
                        egui::Grid::new("mult_unit_form_grid")
                            .num_columns(2)
                            .spacing([10.0, 6.0])
                            .show(ui, |ui| {
                                ui.label("Unit Name:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut state.mult_title_draft)
                                        .desired_width(240.0),
                                );
                                ui.end_row();

                                ui.label("Miniserver Serial:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut state.mult_serial_draft)
                                        .desired_width(240.0),
                                );
                                ui.end_row();

                                ui.label("IP / Cloud Address:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut state.mult_address_draft)
                                        .desired_width(240.0),
                                );
                                ui.end_row();

                                ui.label("Description:");
                                ui.add(
                                    egui::TextEdit::singleline(&mut state.mult_desc_draft)
                                        .desired_width(240.0),
                                );
                                ui.end_row();
                            });

                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("Per-Unit Hardware Serial Overrides")
                                .size(11.5)
                                .strong()
                                .color(theme::TEXT_SECONDARY),
                        );
                        egui::ScrollArea::vertical()
                            .id_salt("mult_overrides_scroll")
                            .max_height(210.0)
                            .show(ui, |ui| {
                                for dev in hw_devices.iter().filter(|d| d.node_type != "LoxLIVE") {
                                    let current_ov = state
                                        .mult_overrides_draft
                                        .iter()
                                        .find(|(du, _)| du == &dev.uuid)
                                        .map(|(_, s)| s.clone())
                                        .unwrap_or_default();
                                    let mut draft_ov = current_ov.clone();
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "{} ({}):",
                                                dev.title, dev.node_type
                                            ))
                                            .size(11.0)
                                            .color(theme::TEXT_PRIMARY),
                                        );
                                        if ui
                                            .add(
                                                egui::TextEdit::singleline(&mut draft_ov)
                                                    .desired_width(120.0)
                                                    .hint_text(&dev.serial),
                                            )
                                            .changed()
                                        {
                                            state
                                                .mult_overrides_draft
                                                .retain(|(du, _)| du != &dev.uuid);
                                            if !draft_ov.trim().is_empty() {
                                                state.mult_overrides_draft.push((
                                                    dev.uuid.clone(),
                                                    draft_ov.trim().to_ascii_uppercase(),
                                                ));
                                            }
                                        }
                                    });
                                }
                            });

                        ui.add_space(8.0);
                        if button(ui, "✓ Save Multiplier Unit", true).clicked()
                            && !state.mult_title_draft.trim().is_empty()
                        {
                            let entry = loxone_core::MultiplierUnitEntry {
                                uuid: state.mult_selected_uuid.clone().unwrap_or_default(),
                                title: state.mult_title_draft.clone(),
                                miniserver_serial: state.mult_serial_draft.clone(),
                                address: state.mult_address_draft.clone(),
                                description: state.mult_desc_draft.clone(),
                                device_serial_overrides: state.mult_overrides_draft.clone(),
                            };
                            let (saved_uuid, edit) =
                                loxone_core::build_upsert_multiplier_unit_edit(project, &entry);
                            state.mult_selected_uuid = Some(saved_uuid);
                            out.push(AdminAction::Edit(edit));
                            state.status = Some("Saved Multiplier unit.".to_string());
                        }
                    });
                });
            });
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_roundtrip() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(days_from_civil(2024, 2, 29), 19782);
        assert_eq!(weekday_mon0(2024, 1, 1), 0); // Monday
        assert_eq!(weekday_mon0(1970, 1, 1), 3); // Thursday
    }

    #[test]
    fn test_stage3_edit_by_type_find_replace_and_backups() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Sample.Loxone");
        let project = loxone_core::parser::load_loxone_file(&path).expect("load Sample.Loxone");

        // 1. Edit Objects of Same Type
        let types = collect_editable_object_types(&project);
        assert!(!types.is_empty());
        let mut rows = build_same_type_object_rows(&project, "LightController2");
        assert!(!rows.is_empty());
        rows[0].title = "Updated Salon Lighting".to_string();
        let batch = build_same_type_batch_edit(&rows).expect("should produce batch edit");
        assert!(matches!(batch, ProjectEdit::Batch(ref v) if v.len() == 1));

        // 2. Find & Replace
        let matches = preview_find_and_replace(
            &project,
            "Oświetlenie",
            "Światło",
            false,
            true,
            true,
            false,
        );
        assert!(!matches.is_empty());
        let fr_batch =
            build_find_and_replace_batch_edit(&matches).expect("find/replace batch edit");
        assert!(matches!(fr_batch, ProjectEdit::Batch(ref v) if v.len() == matches.len()));

        // 3. Project Backups in temporary directory
        let tmp_dir = std::env::temp_dir().join("loxone_portable_stage3_test_backups");
        let _ = std::fs::remove_dir_all(&tmp_dir);
        let saved_path =
            save_project_backup_to_dir(&project, &tmp_dir, "stage3").expect("save backup");
        assert!(saved_path.exists());
        let listed = list_project_backups_in_dir(&tmp_dir);
        assert_eq!(listed.len(), 1);
        let _ = std::fs::remove_dir_all(&tmp_dir);

        // 4. Auto Configuration Wizard & Component BOM CSV
        let auto_rows = build_auto_config_room_rows(&project);
        assert!(!auto_rows.is_empty());
        let (auto_batch, gen_count) =
            build_auto_configuration_edits(&project, &auto_rows).expect("should generate room logic");
        assert!(gen_count > 0);
        assert!(matches!(auto_batch, ProjectEdit::Batch(ref v) if !v.is_empty()));

        let bom_csv = loxone_core::export::export_component_bom_csv(&project);
        assert!(bom_csv.contains("PartGroup,ComponentType,Title,SerialNumber"));
        assert!(bom_csv.contains("HardwareSummary"));
        assert!(bom_csv.contains("FunctionBlockSummary"));
    }
}
