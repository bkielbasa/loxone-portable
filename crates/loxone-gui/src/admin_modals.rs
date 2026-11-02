//! Project administration dialogs: project properties, users & rights,
//! operating modes & calendar, and the Miniserver network search.

use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use egui::{CornerRadius, Margin, RichText, Stroke};
use loxone_core::discovery::{discover_miniservers, DiscoveredMiniserver};
use loxone_core::model::{LoxoneProject, TreeNode};
use loxone_core::project_admin::{
    calendar_entries, delete_user_edits, find_node_by_uuid, find_nodes_by_type,
    granted_permission_refs, group_member_uuids, new_calendar_entry_edit,
    new_operating_mode_edit, new_user_edit, new_user_group_edit, operating_modes,
    recurrence_label, set_group_membership_edit, set_permission_edit, active_modes_on,
    USER_FIELDS,
};
use loxone_core::writer::ProjectEdit;

use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AdminModal {
    #[default]
    None,
    Properties,
    Users,
    Modes,
    Network,
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
}

#[derive(Default)]
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
        egui::Button::new(RichText::new(text).size(12.0).color(color))
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
        RichText::new(text)
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
                    RichText::new(title)
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
                                    RichText::new(*label).size(12.0).color(theme::TEXT_SECONDARY),
                                ),
                            );
                            let value = &mut state.props_draft[i].1;
                            ui.add(egui::TextEdit::singleline(value).desired_width(300.0));
                            if *key == "Timezone" {
                                egui::ComboBox::from_id_salt("tz_pick")
                                    .selected_text("Pick…")
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
                                    RichText::new(*label).size(12.0).color(theme::TEXT_SECONDARY),
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
                    "No changes".to_string()
                } else {
                    format!("✔ Apply {} change(s)", changed.len())
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
                    (UsersTab::Users, format!("Users ({})", users.len())),
                    (UsersTab::Groups, format!("User groups ({})", groups.len())),
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
                                        RichText::new("None in this project")
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
                                    UsersTab::Users => "New user name",
                                    UsersTab::Groups => "New group name",
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
        ui.label(RichText::new("Select a user on the left.").color(theme::TEXT_SECONDARY));
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
                egui::Label::new(RichText::new(*label).size(12.0).color(theme::TEXT_SECONDARY)),
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
        ui.label(RichText::new("No user groups").color(theme::TEXT_SECONDARY));
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
        RichText::new("Rights come from the user's groups; edit them on the User groups tab.")
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
        ui.label(RichText::new("Select a user group on the left.").color(theme::TEXT_SECONDARY));
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
    section_label(ui, &format!("Members ({})", members.len()));
    if users.is_empty() {
        ui.label(RichText::new("No users").color(theme::TEXT_SECONDARY));
    }
    for u in users {
        let mut member = members.contains(&u.uuid);
        if ui.checkbox(&mut member, u.title.as_str()).changed() {
            out.push(AdminAction::Edit(set_group_membership_edit(group, &u.uuid, member)));
        }
    }
    let granted = granted_permission_refs(group);
    section_label(ui, &format!("Rights ({} granted)", granted.len()));
    if permissions.is_empty() {
        ui.label(RichText::new("This project defines no rights").color(theme::TEXT_SECONDARY));
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
                    section_label(ui, &format!("Operating modes ({})", modes.len()));
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
                                                    .on_hover_text("Delete this user-defined mode")
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
                                                    RichText::new("built-in")
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
                                .hint_text("New operating mode")
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
                    section_label(ui, &format!("Calendar entries ({})", entries.len()));
                    card(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("cal_list")
                            .max_height(height - 230.0)
                            .show(ui, |ui| {
                                if entries.is_empty() {
                                    ui.label(
                                        RichText::new("No calendar entries")
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
                                                RichText::new(h)
                                                    .size(11.0)
                                                    .strong()
                                                    .color(theme::TEXT_SECONDARY),
                                            );
                                        }
                                        ui.end_row();
                                        for e in &entries {
                                            ui.label(RichText::new(&e.title).color(theme::TEXT_PRIMARY))
                                                .on_hover_text(recurrence_label(e.recurrence));
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
                                            if ui.small_button("🗑").on_hover_text("Delete entry").clicked() {
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
                                .hint_text("New entry, e.g. Summer holidays")
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
                            RichText::new(format!("{} {}", WEEKDAYS[wd as usize], year))
                                .color(theme::TEXT_SECONDARY),
                        );
                        if ui.small_button("Today").clicked() {
                            let (_, m, d) = today();
                            state.preview = Some((m, d));
                        }
                    });
                    let wd = weekday_mon0(year, pm, pd);
                    let active = active_modes_on(project, pm, pd, wd);
                    ui.horizontal_wrapped(|ui| {
                        if active.is_empty() {
                            ui.label(RichText::new("No mode active").color(theme::TEXT_SECONDARY));
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
                    .add_enabled(!busy, egui::Button::new("🔄 Search again"))
                    .clicked()
                {
                    state.net_results.clear();
                    state.start_search();
                }
                ui.label(
                    RichText::new("Sends a broadcast on the local network and lists every Miniserver that answers.")
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
                            ui.label(RichText::new(h).size(11.0).strong().color(theme::TEXT_SECONDARY));
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
}
