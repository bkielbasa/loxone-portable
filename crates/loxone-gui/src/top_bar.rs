use std::path::Path;

use egui::{Color32, Context, Frame, Margin, RichText, Sense, Stroke, Ui, Vec2};
use loxone_core::miniserver::ConnectionStatus;
use loxone_core::model::LoxoneProject;

use crate::theme::{
    ACCENT_BLUE, BORDER_SUBTLE, CARD_BG, PANEL_BG, STATUS_CONNECTED, STATUS_CONNECTING,
    STATUS_DISCONNECTED, STATUS_ERROR, TEXT_MUTED, TEXT_PRIMARY,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopBarAction {
    ToggleLeftPanel,
    ToggleRightPanel,
    OpenFile,
    SaveFile,
    AddBlock,
    Undo,
    Redo,
    OpenMiniserverModal,
    OpenCommandPalette,
    ToggleCompactPins,
    FitPage,
    ZoomIn,
    ZoomOut,
    ResetZoom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarGroup {
    File,
    Authoring,
    Miniserver,
    View,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolbarItem {
    pub group: ToolbarGroup,
    pub icon: &'static str,
    pub label: String,
    pub tooltip: String,
    pub action: TopBarAction,
    pub enabled: bool,
    pub accent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusBadgeVisuals {
    pub dot_color: Color32,
    pub has_glow_ring: bool,
    pub is_pulsing: bool,
    pub icon_glyph: &'static str,
    pub label: String,
    pub tooltip: String,
}

pub fn status_badge_visuals(
    status: &ConnectionStatus,
    configured_host: Option<&str>,
) -> StatusBadgeVisuals {
    match status {
        ConnectionStatus::Disconnected => {
            let clean_host = configured_host
                .map(str::trim)
                .filter(|h| !h.is_empty());
            let label = match clean_host {
                Some(host) => format!("Disconnected · {}", host),
                None => "Disconnected".to_string(),
            };
            StatusBadgeVisuals {
                dot_color: STATUS_DISCONNECTED,
                has_glow_ring: false,
                is_pulsing: false,
                icon_glyph: "○",
                label,
                tooltip: "Miniserver not connected. Click to configure connection or download project (Ctrl+Shift+D)."
                    .to_string(),
            }
        }
        ConnectionStatus::Connecting { host, message } => StatusBadgeVisuals {
            dot_color: STATUS_CONNECTING,
            has_glow_ring: false,
            is_pulsing: true,
            icon_glyph: "◐",
            label: format!("Connecting · {}", host),
            tooltip: format!("{} ({})", message, host),
        },
        ConnectionStatus::Connected { host, server_info } => StatusBadgeVisuals {
            dot_color: STATUS_CONNECTED,
            has_glow_ring: true,
            is_pulsing: false,
            icon_glyph: "●",
            label: format!("Connected · {}", host),
            tooltip: format!("Connected to {} — {}", host, server_info),
        },
        ConnectionStatus::Error { host, error } => {
            let label = if host.trim().is_empty() {
                "Connection Error".to_string()
            } else {
                format!("Error · {}", host)
            };
            StatusBadgeVisuals {
                dot_color: STATUS_ERROR,
                has_glow_ring: false,
                is_pulsing: false,
                icon_glyph: "●",
                label,
                tooltip: format!("Miniserver connection error ({}): {}", host, error),
            }
        }
    }
}

pub fn paint_status_indicator(ui: &mut Ui, visuals: &StatusBadgeVisuals) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
    let painter = ui.painter_at(rect);
    let center = rect.center();

    if visuals.has_glow_ring {
        painter.circle_filled(center, 6.2, visuals.dot_color.gamma_multiply(0.22));
        painter.circle_stroke(
            center,
            5.8,
            Stroke::new(1.0_f32, visuals.dot_color.gamma_multiply(0.55)),
        );
        painter.circle_filled(center, 3.6, visuals.dot_color);
    } else if visuals.is_pulsing {
        painter.circle_stroke(
            center,
            5.4,
            Stroke::new(1.2_f32, visuals.dot_color.gamma_multiply(0.75)),
        );
        painter.circle_filled(center, 2.8, visuals.dot_color);
    } else if visuals.icon_glyph == "○" {
        painter.circle_stroke(center, 4.0, Stroke::new(1.4_f32, visuals.dot_color));
    } else {
        painter.circle_filled(center, 4.2, visuals.dot_color);
    }
}

pub fn build_file_toolbar_items(can_save: bool, is_dirty: bool) -> Vec<ToolbarItem> {
    let save_label = if is_dirty {
        "Save .Loxone *".to_string()
    } else {
        "Save .Loxone".to_string()
    };
    vec![
        ToolbarItem {
            group: ToolbarGroup::File,
            icon: "📂",
            label: "Open".to_string(),
            tooltip: "Open .Loxone project file from disk (Ctrl+O)".to_string(),
            action: TopBarAction::OpenFile,
            enabled: true,
            accent: false,
        },
        ToolbarItem {
            group: ToolbarGroup::File,
            icon: "💾",
            label: save_label,
            tooltip: "Save current project as .Loxone XML file (Ctrl+S)".to_string(),
            action: TopBarAction::SaveFile,
            enabled: can_save,
            accent: is_dirty,
        },
    ]
}

pub fn build_authoring_toolbar_items(
    has_project: bool,
    can_undo: bool,
    can_redo: bool,
) -> Vec<ToolbarItem> {
    vec![
        ToolbarItem {
            group: ToolbarGroup::Authoring,
            icon: "➕",
            label: "Add Block (A)".to_string(),
            tooltip: "Insert a function block or reference on the active page (A)".to_string(),
            action: TopBarAction::AddBlock,
            enabled: has_project,
            accent: true,
        },
        ToolbarItem {
            group: ToolbarGroup::Authoring,
            icon: "↶",
            label: "Undo".to_string(),
            tooltip: "Undo last edit (⌘Z / Ctrl+Z)".to_string(),
            action: TopBarAction::Undo,
            enabled: can_undo,
            accent: false,
        },
        ToolbarItem {
            group: ToolbarGroup::Authoring,
            icon: "↷",
            label: "Redo".to_string(),
            tooltip: "Redo edit (⌘⇧Z / Ctrl+Y)".to_string(),
            action: TopBarAction::Redo,
            enabled: can_redo,
            accent: false,
        },
    ]
}

pub fn build_miniserver_toolbar_items(is_busy: bool) -> Vec<ToolbarItem> {
    vec![ToolbarItem {
        group: ToolbarGroup::Miniserver,
        icon: "⬇",
        label: "Download".to_string(),
        tooltip: "Connect to Miniserver and download project backup (Ctrl+Shift+D)".to_string(),
        action: TopBarAction::OpenMiniserverModal,
        enabled: !is_busy,
        accent: true,
    }]
}

pub fn build_view_toolbar_items(compact_pins: bool) -> Vec<ToolbarItem> {
    let pin_label = if compact_pins {
        "Pins: Connected (P)"
    } else {
        "Pins: All (P)"
    };
    vec![
        ToolbarItem {
            group: ToolbarGroup::View,
            icon: "ℹ",
            label: String::new(),
            tooltip: "Toggle Inspector (Ctrl+I)".to_string(),
            action: TopBarAction::ToggleRightPanel,
            enabled: true,
            accent: false,
        },
        ToolbarItem {
            group: ToolbarGroup::View,
            icon: "⊡",
            label: "Fit (F)".to_string(),
            tooltip: "Fit active page to view (F)".to_string(),
            action: TopBarAction::FitPage,
            enabled: true,
            accent: false,
        },
        ToolbarItem {
            group: ToolbarGroup::View,
            icon: "⚡",
            label: pin_label.to_string(),
            tooltip: "Toggle connected vs all connector pins (P)".to_string(),
            action: TopBarAction::ToggleCompactPins,
            enabled: true,
            accent: true,
        },
    ]
}

fn render_toolbar_items(ui: &mut Ui, items: &[ToolbarItem]) -> Option<TopBarAction> {
    let mut triggered = None;
    for item in items {
        let text_str = if item.label.is_empty() {
            item.icon.to_string()
        } else {
            format!("{} {}", item.icon, item.label)
        };
        let color = if !item.enabled {
            TEXT_MUTED.gamma_multiply(0.5)
        } else if item.accent {
            ACCENT_BLUE
        } else {
            TEXT_PRIMARY
        };
        let btn = egui::Button::new(RichText::new(text_str).size(11.5).color(color));
        let resp = ui
            .add_enabled(item.enabled, btn)
            .on_hover_text(&item.tooltip);
        if resp.clicked() {
            triggered = Some(item.action.clone());
        }
    }
    triggered
}

#[allow(clippy::too_many_arguments)]
pub fn render_top_bar(
    ctx: &Context,
    project: Option<&LoxoneProject>,
    loaded_path: Option<&Path>,
    active_page_index: usize,
    compact_pins: bool,
    zoom: f32,
    connection_status: &ConnectionStatus,
    configured_host: Option<&str>,
    is_dirty: bool,
    can_undo: bool,
    can_redo: bool,
) -> Option<TopBarAction> {
    let mut action = None;
    let can_save = project
        .and_then(|p| p.raw_xml.as_ref())
        .is_some();
    let effective_host = configured_host
        .filter(|h| !h.trim().is_empty())
        .or_else(|| {
            project.and_then(|p| {
                p.metadata
                    .miniserver_ip
                    .as_deref()
                    .or(p.metadata.miniserver_hostname.as_deref())
            })
        });
    let status_visuals = status_badge_visuals(connection_status, effective_host);

    egui::TopBottomPanel::top("top_header")
        .exact_height(40.0)
        .frame(
            Frame::new()
                .fill(PANEL_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .inner_margin(Margin::symmetric(12, 6)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // 1. Sidebar Toggle & Breadcrumb
                if ui
                    .button(RichText::new("☰").color(TEXT_MUTED))
                    .on_hover_text("Toggle Explorer (Ctrl+B)")
                    .clicked()
                {
                    action = Some(TopBarAction::ToggleLeftPanel);
                }

                let dirty_suffix = if is_dirty { " *" } else { "" };
                let title_str = project
                    .map(|p| {
                        let page_name = p
                            .pages
                            .get(active_page_index)
                            .map(|pg| pg.title.as_str())
                            .unwrap_or("No Page");
                        format!("{}{}  /  {}", p.metadata.title, dirty_suffix, page_name)
                    })
                    .unwrap_or_else(|| "Loxone Portable".to_string());
                let title_resp = ui.label(RichText::new(title_str).color(TEXT_PRIMARY).strong());
                if let Some(loaded) = loaded_path {
                    title_resp.on_hover_text(loaded.display().to_string());
                } else if project.is_some() {
                    title_resp.on_hover_text("Downloaded from Miniserver (unsaved in-memory project)");
                }

                ui.separator();

                // 2. File Group (Open, Save .Loxone)
                let file_items = build_file_toolbar_items(can_save, is_dirty);
                if let Some(act) = render_toolbar_items(ui, &file_items) {
                    action = Some(act);
                }

                ui.separator();

                // 2b. Authoring Group (Add Block, Undo, Redo)
                let auth_items =
                    build_authoring_toolbar_items(project.is_some(), can_undo, can_redo);
                if let Some(act) = render_toolbar_items(ui, &auth_items) {
                    action = Some(act);
                }

                ui.separator();

                // 3. Miniserver & Tools Group (Connection Status Badge + Action Strip)
                let border_color = match connection_status {
                    ConnectionStatus::Disconnected => BORDER_SUBTLE,
                    _ => status_visuals.dot_color.gamma_multiply(0.65),
                };
                let status_pill = Frame::new()
                    .fill(CARD_BG)
                    .stroke(Stroke::new(1.0_f32, border_color))
                    .corner_radius(6)
                    .inner_margin(Margin::symmetric(8, 3))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 5.0;
                            paint_status_indicator(ui, &status_visuals);
                            ui.label(
                                RichText::new(&status_visuals.label)
                                    .size(11.5)
                                    .color(TEXT_PRIMARY),
                            );
                        });
                    })
                    .response
                    .interact(Sense::click())
                    .on_hover_text(&status_visuals.tooltip);

                if status_pill.clicked() {
                    action = Some(TopBarAction::OpenMiniserverModal);
                }

                let ms_items = build_miniserver_toolbar_items(connection_status.is_connecting());
                if let Some(act) = render_toolbar_items(ui, &ms_items) {
                    action = Some(act);
                }

                ui.add_space(8.0);

                // 4. Command Palette Search Pill
                let pill = Frame::new()
                    .fill(CARD_BG)
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .corner_radius(6)
                    .inner_margin(Margin::symmetric(12, 3))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("🔍  Search or jump…   ⌘K / Ctrl+K")
                                .size(11.5)
                                .color(TEXT_MUTED),
                        );
                    })
                    .response
                    .interact(Sense::click());
                if pill.clicked() {
                    action = Some(TopBarAction::OpenCommandPalette);
                }

                // 5. Right-Aligned View Group
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let view_items = build_view_toolbar_items(compact_pins);
                    if let Some(act) = render_toolbar_items(ui, &view_items) {
                        action = Some(act);
                    }
                    if ui
                        .small_button(RichText::new("＋").size(12.0).color(TEXT_PRIMARY))
                        .on_hover_text("Zoom in (⌘+ / Scroll up)")
                        .clicked()
                    {
                        action = Some(TopBarAction::ZoomIn);
                    }
                    if ui
                        .small_button(
                            RichText::new(format!("{:.0}%", zoom * 100.0))
                                .monospace()
                                .size(11.5)
                                .color(TEXT_PRIMARY),
                        )
                        .on_hover_text("Reset zoom to 100% (0)")
                        .clicked()
                    {
                        action = Some(TopBarAction::ResetZoom);
                    }
                    if ui
                        .small_button(RichText::new("－").size(12.0).color(TEXT_PRIMARY))
                        .on_hover_text("Zoom out (⌘- / Scroll down)")
                        .clicked()
                    {
                        action = Some(TopBarAction::ZoomOut);
                    }
                });
            });
        });

    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_badge_visuals_for_all_connection_states() {
        let disconnected = status_badge_visuals(&ConnectionStatus::Disconnected, None);
        assert_eq!(disconnected.dot_color, STATUS_DISCONNECTED);
        assert_eq!(disconnected.icon_glyph, "○");
        assert!(!disconnected.has_glow_ring);
        assert_eq!(disconnected.label, "Disconnected");

        let disconnected_with_host =
            status_badge_visuals(&ConnectionStatus::Disconnected, Some("192.168.1.100"));
        assert_eq!(disconnected_with_host.label, "Disconnected · 192.168.1.100");

        let connecting = status_badge_visuals(
            &ConnectionStatus::Connecting {
                host: "192.168.1.100".to_string(),
                message: "Testing connection…".to_string(),
            },
            None,
        );
        assert_eq!(connecting.dot_color, STATUS_CONNECTING);
        assert_eq!(connecting.icon_glyph, "◐");
        assert!(connecting.is_pulsing);
        assert_eq!(connecting.label, "Connecting · 192.168.1.100");

        let connected = status_badge_visuals(
            &ConnectionStatus::Connected {
                host: "192.168.1.100".to_string(),
                server_info: "3 backups available".to_string(),
            },
            None,
        );
        assert_eq!(connected.dot_color, STATUS_CONNECTED);
        assert_eq!(connected.icon_glyph, "●");
        assert!(connected.has_glow_ring);
        assert_eq!(connected.label, "Connected · 192.168.1.100");
        assert!(connected.tooltip.contains("3 backups available"));

        let errored = status_badge_visuals(
            &ConnectionStatus::Error {
                host: "192.168.1.100".to_string(),
                error: "HTTP 401 Unauthorized".to_string(),
            },
            None,
        );
        assert_eq!(errored.dot_color, STATUS_ERROR);
        assert_eq!(errored.icon_glyph, "●");
        assert_eq!(errored.label, "Error · 192.168.1.100");
        assert!(errored.tooltip.contains("HTTP 401 Unauthorized"));
    }

    #[test]
    fn test_declarative_toolbar_groups_and_headless_render() {
        let file_items_no_save = build_file_toolbar_items(false, false);
        assert_eq!(file_items_no_save.len(), 2);
        assert_eq!(file_items_no_save[0].group, ToolbarGroup::File);
        assert!(file_items_no_save[0].enabled);
        assert!(!file_items_no_save[1].enabled);
        assert!(!file_items_no_save[1].accent);
        assert_eq!(file_items_no_save[1].label, "Save .Loxone");

        let file_items_clean = build_file_toolbar_items(true, false);
        assert!(file_items_clean[1].enabled);
        assert!(!file_items_clean[1].accent);
        assert_eq!(file_items_clean[1].label, "Save .Loxone");

        let file_items_dirty = build_file_toolbar_items(true, true);
        assert!(file_items_dirty[1].enabled);
        assert!(file_items_dirty[1].accent);
        assert_eq!(file_items_dirty[1].label, "Save .Loxone *");

        let auth_items = build_authoring_toolbar_items(true, true, false);
        assert_eq!(auth_items.len(), 3);
        assert_eq!(auth_items[0].group, ToolbarGroup::Authoring);
        assert_eq!(auth_items[0].action, TopBarAction::AddBlock);
        assert!(auth_items[0].enabled);
        assert_eq!(auth_items[1].action, TopBarAction::Undo);
        assert!(auth_items[1].enabled);
        assert_eq!(auth_items[2].action, TopBarAction::Redo);
        assert!(!auth_items[2].enabled);

        let ms_items_idle = build_miniserver_toolbar_items(false);
        assert!(!ms_items_idle.is_empty());
        assert_eq!(ms_items_idle[0].group, ToolbarGroup::Miniserver);
        assert_eq!(ms_items_idle[0].action, TopBarAction::OpenMiniserverModal);
        assert!(ms_items_idle[0].enabled);

        let ms_items_busy = build_miniserver_toolbar_items(true);
        assert!(!ms_items_busy[0].enabled);

        let view_items = build_view_toolbar_items(true);
        assert_eq!(view_items.len(), 3);
        assert_eq!(view_items[0].group, ToolbarGroup::View);

        let ctx = egui::Context::default();
        let status = ConnectionStatus::Connected {
            host: "192.168.1.100".to_string(),
            server_info: "1 backup".to_string(),
        };
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let _ = render_top_bar(
                ctx,
                None,
                None,
                0,
                true,
                1.0,
                &status,
                Some("192.168.1.100"),
                true,
                false,
                false,
            );
        });
    }
}
