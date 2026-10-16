use std::path::Path;

use egui::{
    Align, Color32, Context, CornerRadius, Frame, Layout, Margin, Painter, Pos2, Rect, Response,
    RichText, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
};
use loxone_core::miniserver::ConnectionStatus;
use loxone_core::model::LoxoneProject;

use crate::icons::{ui_icon, LoxoneIcon};
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
    ToggleLiveView,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarIcon {
    SidebarLeft,
    SidebarRight,
    OpenFolder,
    SaveDisk { dirty: bool },
    AddBlock,
    Undo,
    Redo,
    Download,
    LivePulse { active: bool },
    Search,
    FitPage,
    PinsMode { all_pins: bool },
    ZoomOut,
    ZoomIn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolbarItem {
    pub group: ToolbarGroup,
    pub icon: &'static str,
    pub vector_icon: ToolbarIcon,
    pub label: String,
    pub shortcut_hint: Option<&'static str>,
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

/// Paints a crisp anti-aliased vector icon for the top toolbar inside `rect`.
pub fn paint_toolbar_icon(painter: &Painter, rect: Rect, icon: ToolbarIcon, fg: Color32) {
    let r = rect.shrink((rect.width() * 0.08).max(0.8));
    let w = r.width();
    let h = r.height();
    let p = |nx: f32, ny: f32| Pos2::new(r.min.x + nx * w, r.min.y + ny * h);
    let sw = (w * 0.11).clamp(1.3, 1.9);
    let stroke = Stroke::new(sw, fg);

    match icon {
        ToolbarIcon::SidebarLeft => {
            let box_r = Rect::from_min_max(p(0.08, 0.14), p(0.92, 0.86));
            painter.rect_stroke(box_r, 2.5, stroke, StrokeKind::Inside);
            let left_pane = Rect::from_min_max(p(0.12, 0.18), p(0.38, 0.82));
            painter.rect_filled(left_pane, 1.5, fg.gamma_multiply(0.35));
            painter.line_segment([p(0.38, 0.16), p(0.38, 0.84)], stroke);
        }
        ToolbarIcon::SidebarRight => {
            let box_r = Rect::from_min_max(p(0.08, 0.14), p(0.92, 0.86));
            painter.rect_stroke(box_r, 2.5, stroke, StrokeKind::Inside);
            let right_pane = Rect::from_min_max(p(0.62, 0.18), p(0.88, 0.82));
            painter.rect_filled(right_pane, 1.5, fg.gamma_multiply(0.35));
            painter.line_segment([p(0.62, 0.16), p(0.62, 0.84)], stroke);
        }
        ToolbarIcon::OpenFolder => {
            let pts = vec![
                p(0.08, 0.24),
                p(0.38, 0.24),
                p(0.48, 0.36),
                p(0.90, 0.36),
                p(0.90, 0.82),
                p(0.08, 0.82),
            ];
            painter.add(Shape::closed_line(pts.clone(), stroke));
            painter.add(Shape::convex_polygon(
                vec![p(0.08, 0.42), p(0.90, 0.42), p(0.90, 0.82), p(0.08, 0.82)],
                fg.gamma_multiply(0.18),
                Stroke::NONE,
            ));
            painter.line_segment([p(0.10, 0.42), p(0.88, 0.42)], stroke);
        }
        ToolbarIcon::SaveDisk { dirty } => {
            let pts = vec![
                p(0.12, 0.12),
                p(0.74, 0.12),
                p(0.88, 0.26),
                p(0.88, 0.88),
                p(0.12, 0.88),
            ];
            painter.add(Shape::closed_line(pts, stroke));
            let top_slot = Rect::from_min_max(p(0.28, 0.13), p(0.66, 0.38));
            painter.rect_stroke(top_slot, 1.0, stroke, StrokeKind::Inside);
            let bottom_label = Rect::from_min_max(p(0.24, 0.54), p(0.76, 0.87));
            painter.rect_filled(bottom_label, 1.0, fg.gamma_multiply(0.20));
            painter.rect_stroke(bottom_label, 1.0, stroke, StrokeKind::Inside);
            if dirty {
                let badge = p(0.84, 0.18);
                painter.circle_filled(badge, w * 0.20, Color32::from_rgb(0xF5, 0x9E, 0x0B));
            }
        }
        ToolbarIcon::AddBlock => {
            let box_r = Rect::from_min_max(p(0.10, 0.10), p(0.90, 0.90));
            let green = Color32::from_rgb(0x69, 0xC3, 0x32);
            let tint = if fg.a() < 160 {
                green.gamma_multiply(0.35)
            } else {
                green
            };
            painter.rect_filled(box_r, 3.5, tint.gamma_multiply(0.24));
            painter.rect_stroke(box_r, 3.5, Stroke::new(sw, tint), StrokeKind::Inside);
            let plus_stroke = Stroke::new((sw * 1.15).max(1.6), fg);
            painter.line_segment([p(0.50, 0.27), p(0.50, 0.73)], plus_stroke);
            painter.line_segment([p(0.27, 0.50), p(0.73, 0.50)], plus_stroke);
        }
        ToolbarIcon::Undo => {
            // Smooth counter-clockwise arc + left arrowhead
            let arc_pts = vec![
                p(0.20, 0.42),
                p(0.45, 0.28),
                p(0.72, 0.34),
                p(0.84, 0.54),
                p(0.76, 0.76),
                p(0.50, 0.80),
            ];
            painter.add(Shape::line(arc_pts, stroke));
            painter.add(Shape::line(
                vec![p(0.40, 0.20), p(0.16, 0.42), p(0.40, 0.58)],
                stroke,
            ));
        }
        ToolbarIcon::Redo => {
            // Smooth clockwise arc + right arrowhead
            let arc_pts = vec![
                p(0.80, 0.42),
                p(0.55, 0.28),
                p(0.28, 0.34),
                p(0.16, 0.54),
                p(0.24, 0.76),
                p(0.50, 0.80),
            ];
            painter.add(Shape::line(arc_pts, stroke));
            painter.add(Shape::line(
                vec![p(0.60, 0.20), p(0.84, 0.42), p(0.60, 0.58)],
                stroke,
            ));
        }
        ToolbarIcon::Download => {
            // Downward arrow into tray
            painter.line_segment([p(0.50, 0.14), p(0.50, 0.64)], stroke);
            painter.add(Shape::line(
                vec![p(0.28, 0.44), p(0.50, 0.66), p(0.72, 0.44)],
                stroke,
            ));
            painter.add(Shape::line(
                vec![p(0.16, 0.66), p(0.16, 0.84), p(0.84, 0.84), p(0.84, 0.66)],
                stroke,
            ));
        }
        ToolbarIcon::LivePulse { active } => {
            let pulse_color = if active {
                Color32::from_rgb(0x59, 0xB8, 0x28)
            } else {
                fg
            };
            let pulse_stroke = Stroke::new((sw * 1.15).max(1.5), pulse_color);
            painter.add(Shape::line(
                vec![
                    p(0.06, 0.56),
                    p(0.26, 0.56),
                    p(0.38, 0.18),
                    p(0.54, 0.84),
                    p(0.68, 0.40),
                    p(0.78, 0.56),
                    p(0.94, 0.56),
                ],
                pulse_stroke,
            ));
            if active {
                painter.circle_filled(
                    p(0.38, 0.18),
                    w * 0.12,
                    Color32::from_rgb(0x8E, 0xE8, 0x58),
                );
            }
        }
        ToolbarIcon::Search => {
            let center = p(0.43, 0.43);
            painter.circle_stroke(center, w * 0.27, stroke);
            painter.line_segment(
                [p(0.63, 0.63), p(0.88, 0.88)],
                Stroke::new(sw * 1.25, fg),
            );
        }
        ToolbarIcon::FitPage => {
            // Four corner viewfinder brackets
            painter.add(Shape::line(
                vec![p(0.12, 0.34), p(0.12, 0.12), p(0.34, 0.12)],
                stroke,
            ));
            painter.add(Shape::line(
                vec![p(0.66, 0.12), p(0.88, 0.12), p(0.88, 0.34)],
                stroke,
            ));
            painter.add(Shape::line(
                vec![p(0.12, 0.66), p(0.12, 0.88), p(0.34, 0.88)],
                stroke,
            ));
            painter.add(Shape::line(
                vec![p(0.66, 0.88), p(0.88, 0.88), p(0.88, 0.66)],
                stroke,
            ));
            let inner = Rect::from_min_max(p(0.33, 0.33), p(0.67, 0.67));
            painter.rect_filled(inner, 1.5, fg.gamma_multiply(0.25));
        }
        ToolbarIcon::PinsMode { all_pins } => {
            let body = Rect::from_min_max(p(0.28, 0.16), p(0.72, 0.84));
            painter.rect_filled(body, 2.0, fg.gamma_multiply(0.20));
            painter.rect_stroke(body, 2.0, stroke, StrokeKind::Inside);
            for ny in [0.30_f32, 0.50_f32, 0.70_f32] {
                painter.line_segment([p(0.08, ny), p(0.28, ny)], stroke);
                painter.line_segment([p(0.72, ny), p(0.92, ny)], stroke);
                if all_pins {
                    painter.circle_filled(p(0.10, ny), w * 0.06, ACCENT_BLUE);
                    painter.circle_filled(p(0.90, ny), w * 0.06, ACCENT_BLUE);
                }
            }
        }
        ToolbarIcon::ZoomOut => {
            painter.line_segment([p(0.22, 0.50), p(0.78, 0.50)], Stroke::new(sw * 1.15, fg));
        }
        ToolbarIcon::ZoomIn => {
            let s = Stroke::new(sw * 1.15, fg);
            painter.line_segment([p(0.22, 0.50), p(0.78, 0.50)], s);
            painter.line_segment([p(0.50, 0.22), p(0.50, 0.78)], s);
        }
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
            vector_icon: ToolbarIcon::OpenFolder,
            label: "Open".to_string(),
            shortcut_hint: None,
            tooltip: "Open .Loxone project file from disk (⌘O / Ctrl+O)".to_string(),
            action: TopBarAction::OpenFile,
            enabled: true,
            accent: false,
        },
        ToolbarItem {
            group: ToolbarGroup::File,
            icon: "💾",
            vector_icon: ToolbarIcon::SaveDisk { dirty: is_dirty },
            label: save_label,
            shortcut_hint: None,
            tooltip: "Save current project as .Loxone XML file (⌘S / Ctrl+S)".to_string(),
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
            vector_icon: ToolbarIcon::AddBlock,
            label: "Add Block".to_string(),
            shortcut_hint: Some("A"),
            tooltip: "Insert a function block or reference on the active page (A)".to_string(),
            action: TopBarAction::AddBlock,
            enabled: has_project,
            accent: true,
        },
        ToolbarItem {
            group: ToolbarGroup::Authoring,
            icon: "↶",
            vector_icon: ToolbarIcon::Undo,
            label: "Undo".to_string(),
            shortcut_hint: None,
            tooltip: "Undo last edit (⌘Z / Ctrl+Z)".to_string(),
            action: TopBarAction::Undo,
            enabled: can_undo,
            accent: false,
        },
        ToolbarItem {
            group: ToolbarGroup::Authoring,
            icon: "↷",
            vector_icon: ToolbarIcon::Redo,
            label: "Redo".to_string(),
            shortcut_hint: None,
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
        vector_icon: ToolbarIcon::Download,
        label: "Download".to_string(),
        shortcut_hint: None,
        tooltip: "Connect to Miniserver and download project backup (⌘⇧D / Ctrl+Shift+D)"
            .to_string(),
        action: TopBarAction::OpenMiniserverModal,
        enabled: !is_busy,
        accent: true,
    }]
}

pub fn build_live_view_toolbar_item(has_project: bool, live_view_active: bool) -> ToolbarItem {
    ToolbarItem {
        group: ToolbarGroup::Miniserver,
        icon: "⚡",
        vector_icon: ToolbarIcon::LivePulse {
            active: live_view_active,
        },
        label: if live_view_active {
            "Live View: ON".to_string()
        } else {
            "Live View".to_string()
        },
        shortcut_hint: Some("L"),
        tooltip:
            "Toggle LiveView real-time I/O value overlay on schematic wires and pins (L)".to_string(),
        action: TopBarAction::ToggleLiveView,
        enabled: has_project,
        accent: live_view_active,
    }
}

pub fn build_view_toolbar_items(compact_pins: bool) -> Vec<ToolbarItem> {
    let pin_label = if compact_pins {
        "Pins: Connected"
    } else {
        "Pins: All"
    };
    vec![
        ToolbarItem {
            group: ToolbarGroup::View,
            icon: "ℹ",
            vector_icon: ToolbarIcon::SidebarRight,
            label: String::new(),
            shortcut_hint: None,
            tooltip: "Toggle Property Inspector (⌘I / Ctrl+I)".to_string(),
            action: TopBarAction::ToggleRightPanel,
            enabled: true,
            accent: false,
        },
        ToolbarItem {
            group: ToolbarGroup::View,
            icon: "⊡",
            vector_icon: ToolbarIcon::FitPage,
            label: "Fit".to_string(),
            shortcut_hint: Some("F"),
            tooltip: "Fit active page to view (F)".to_string(),
            action: TopBarAction::FitPage,
            enabled: true,
            accent: false,
        },
        ToolbarItem {
            group: ToolbarGroup::View,
            icon: "⚡",
            vector_icon: ToolbarIcon::PinsMode {
                all_pins: !compact_pins,
            },
            label: pin_label.to_string(),
            shortcut_hint: Some("P"),
            tooltip: "Toggle connected vs all connector pins (P)".to_string(),
            action: TopBarAction::ToggleCompactPins,
            enabled: true,
            accent: !compact_pins,
        },
    ]
}

const CAPSULE_BG: Color32 = Color32::from_rgb(0x1C, 0x21, 0x2C);
const BTN_HOVER_BG: Color32 = Color32::from_rgb(0x2A, 0x33, 0x46);
const BTN_ACTIVE_BG: Color32 = Color32::from_rgb(0x32, 0x3D, 0x54);
const LOXONE_GREEN: Color32 = Color32::from_rgb(0x69, 0xC3, 0x32);

/// Renders a single polished toolbar button with a vector icon, optional label, and optional keycap badge.
fn render_vector_button(
    ui: &mut Ui,
    vector_icon: ToolbarIcon,
    label: &str,
    shortcut_hint: Option<&str>,
    tooltip: &str,
    enabled: bool,
    accent: bool,
    primary_green: bool,
) -> Response {
    let icon_size = 14.5_f32;
    let pad_x = if label.is_empty() { 7.5_f32 } else { 8.5_f32 };
    let btn_height = 26.0_f32;

    let label_galley = if label.is_empty() {
        None
    } else {
        Some(ui.painter().layout_no_wrap(
            label.to_string(),
            egui::FontId::proportional(11.8),
            TEXT_PRIMARY,
        ))
    };
    let hint_galley = shortcut_hint.map(|h| {
        ui.painter().layout_no_wrap(
            h.to_string(),
            egui::FontId::monospace(9.8),
            TEXT_MUTED,
        )
    });

    let mut total_w = pad_x * 2.0 + icon_size;
    if let Some(g) = &label_galley {
        total_w += 5.5 + g.size().x;
    }
    if let Some(hg) = &hint_galley {
        total_w += 5.5 + hg.size().x + 8.0;
    }

    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(total_w, btn_height), sense);

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let hovered = enabled && resp.hovered();
        let active = enabled && resp.is_pointer_button_down_on();

        if active {
            painter.rect_filled(rect, CornerRadius::same(5), BTN_ACTIVE_BG);
        } else if hovered {
            painter.rect_filled(rect, CornerRadius::same(5), BTN_HOVER_BG);
            painter.rect_stroke(
                rect,
                CornerRadius::same(5),
                Stroke::new(1.0_f32, ACCENT_BLUE.gamma_multiply(0.65)),
                StrokeKind::Inside,
            );
        } else if enabled && primary_green {
            painter.rect_filled(
                rect,
                CornerRadius::same(5),
                LOXONE_GREEN.gamma_multiply(0.14),
            );
            painter.rect_stroke(
                rect,
                CornerRadius::same(5),
                Stroke::new(1.0_f32, LOXONE_GREEN.gamma_multiply(0.45)),
                StrokeKind::Inside,
            );
        } else if enabled && accent {
            painter.rect_filled(
                rect,
                CornerRadius::same(5),
                ACCENT_BLUE.gamma_multiply(0.16),
            );
            painter.rect_stroke(
                rect,
                CornerRadius::same(5),
                Stroke::new(1.0_f32, ACCENT_BLUE.gamma_multiply(0.50)),
                StrokeKind::Inside,
            );
        }

        let fg = if !enabled {
            TEXT_MUTED.gamma_multiply(0.42)
        } else if primary_green {
            Color32::from_rgb(0x8E, 0xE8, 0x58)
        } else if accent {
            Color32::from_rgb(0x88, 0xA8, 0xFF)
        } else if hovered {
            Color32::WHITE
        } else {
            TEXT_PRIMARY
        };

        let mut cursor_x = rect.min.x + pad_x;
        let icon_rect = Rect::from_center_size(
            Pos2::new(cursor_x + icon_size * 0.5, rect.center().y),
            Vec2::splat(icon_size),
        );
        paint_toolbar_icon(painter, icon_rect, vector_icon, fg);
        cursor_x += icon_size;

        if let Some(galley) = label_galley {
            cursor_x += 5.5;
            let text_pos = Pos2::new(cursor_x, rect.center().y - galley.size().y * 0.5);
            let text_color = if !enabled {
                TEXT_MUTED.gamma_multiply(0.45)
            } else if hovered || primary_green || accent {
                Color32::WHITE
            } else {
                TEXT_PRIMARY
            };
            painter.galley(text_pos, galley.clone(), text_color);
            cursor_x += galley.size().x;
        }

        if let Some(hg) = hint_galley {
            cursor_x += 5.5;
            let badge_w = hg.size().x + 8.0;
            let badge_h = 15.0_f32;
            let badge_rect = Rect::from_center_size(
                Pos2::new(cursor_x + badge_w * 0.5, rect.center().y),
                Vec2::new(badge_w, badge_h),
            );
            let badge_alpha = if enabled { 1.0 } else { 0.45 };
            painter.rect_filled(
                badge_rect,
                CornerRadius::same(3),
                Color32::from_rgb(0x13, 0x16, 0x1E).gamma_multiply(badge_alpha),
            );
            painter.rect_stroke(
                badge_rect,
                CornerRadius::same(3),
                Stroke::new(1.0_f32, BORDER_SUBTLE.gamma_multiply(badge_alpha)),
                StrokeKind::Inside,
            );
            let h_pos = Pos2::new(
                badge_rect.center().x - hg.size().x * 0.5,
                badge_rect.center().y - hg.size().y * 0.5,
            );
            painter.galley(h_pos, hg, TEXT_MUTED.gamma_multiply(badge_alpha));
        }
    }

    resp.on_hover_text(tooltip)
}

fn render_capsule_divider(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(5.0, 20.0), Sense::hover());
    let x = rect.center().x;
    ui.painter().line_segment(
        [Pos2::new(x, rect.min.y + 3.0), Pos2::new(x, rect.max.y - 3.0)],
        Stroke::new(1.0_f32, BORDER_SUBTLE),
    );
}

fn render_toolbar_capsule(ui: &mut Ui, items: &[ToolbarItem]) -> Option<TopBarAction> {
    let mut triggered = None;
    Frame::new()
        .fill(CAPSULE_BG)
        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::symmetric(3, 2))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    render_capsule_divider(ui);
                }
                let primary_green = matches!(item.action, TopBarAction::AddBlock);
                let resp = render_vector_button(
                    ui,
                    item.vector_icon,
                    &item.label,
                    item.shortcut_hint,
                    &item.tooltip,
                    item.enabled,
                    item.accent && !primary_green,
                    primary_green,
                );
                if resp.clicked() && item.enabled {
                    triggered = Some(item.action.clone());
                }
            }
        });
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
    live_view_active: bool,
) -> Option<TopBarAction> {
    let mut action = None;
    let can_save = project.and_then(|p| p.raw_xml.as_ref()).is_some();
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
        .exact_height(46.0)
        .frame(
            Frame::new()
                .fill(PANEL_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .inner_margin(Margin::symmetric(10, 6)),
        )
        .show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 7.0;

                // 1. Left Explorer Sidebar Toggle + Project/Page Breadcrumb Capsule
                Frame::new()
                    .fill(CAPSULE_BG)
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .corner_radius(CornerRadius::same(7))
                    .inner_margin(Margin::symmetric(4, 2))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        if render_vector_button(
                            ui,
                            ToolbarIcon::SidebarLeft,
                            "",
                            None,
                            "Toggle Explorer Sidebar (⌘B / Ctrl+B)",
                            true,
                            false,
                            false,
                        )
                        .clicked()
                        {
                            action = Some(TopBarAction::ToggleLeftPanel);
                        }

                        render_capsule_divider(ui);

                        ui_icon(ui, LoxoneIcon::Miniserver, 15.0);
                        let proj_title = project
                            .map(|p| {
                                if p.metadata.title.trim().is_empty() {
                                    "Project"
                                } else {
                                    p.metadata.title.as_str()
                                }
                            })
                            .unwrap_or("Loxone Portable");
                        let dirty_suffix = if is_dirty { " *" } else { "" };
                        let title_resp = ui.label(
                            RichText::new(format!("{}{}", proj_title, dirty_suffix))
                                .size(12.0)
                                .color(TEXT_PRIMARY)
                                .strong(),
                        );
                        if let Some(loaded) = loaded_path {
                            title_resp.on_hover_text(loaded.display().to_string());
                        } else if project.is_some() {
                            title_resp.on_hover_text(
                                "Downloaded from Miniserver (unsaved in-memory project)",
                            );
                        }

                        if let Some(proj) = project {
                            let page_name = proj
                                .pages
                                .get(active_page_index)
                                .map(|pg| pg.title.as_str())
                                .unwrap_or("No Page");
                            ui.label(RichText::new("›").size(13.0).color(TEXT_MUTED));

                            let page_pill = Frame::new()
                                .fill(CARD_BG)
                                .corner_radius(CornerRadius::same(5))
                                .inner_margin(Margin::symmetric(6, 2))
                                .show(ui, |ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    ui_icon(ui, LoxoneIcon::Page, 13.5);
                                    ui.label(
                                        RichText::new(page_name)
                                            .size(11.5)
                                            .color(Color32::from_rgb(0xC5, 0xD4, 0xF5))
                                            .strong(),
                                    );
                                })
                                .response
                                .interact(Sense::click())
                                .on_hover_text("Click to switch page or jump (⌘K / Ctrl+K)");
                            if page_pill.clicked() {
                                action = Some(TopBarAction::OpenCommandPalette);
                            }
                        }
                    });

                // 2. File Capsule (Open, Save .Loxone)
                let file_items = build_file_toolbar_items(can_save, is_dirty);
                if let Some(act) = render_toolbar_capsule(ui, &file_items) {
                    action = Some(act);
                }

                // 3. Authoring Capsule (+ Add Block [A], Undo, Redo)
                let auth_items =
                    build_authoring_toolbar_items(project.is_some(), can_undo, can_redo);
                if let Some(act) = render_toolbar_capsule(ui, &auth_items) {
                    action = Some(act);
                }

                // 4. Unified Miniserver Status, Download & LiveView Capsule
                let ms_border = if live_view_active {
                    Color32::from_rgb(0x59, 0xB8, 0x28).gamma_multiply(0.85)
                } else {
                    match connection_status {
                        ConnectionStatus::Disconnected => BORDER_SUBTLE,
                        _ => status_visuals.dot_color.gamma_multiply(0.60),
                    }
                };
                Frame::new()
                    .fill(CAPSULE_BG)
                    .stroke(Stroke::new(1.0_f32, ms_border))
                    .corner_radius(CornerRadius::same(7))
                    .inner_margin(Margin::symmetric(4, 2))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 3.0;
                        let status_resp = Frame::new()
                            .corner_radius(CornerRadius::same(5))
                            .inner_margin(Margin::symmetric(6, 3))
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing.x = 5.0;
                                paint_status_indicator(ui, &status_visuals);
                                ui.label(
                                    RichText::new(&status_visuals.label)
                                        .size(11.5)
                                        .color(TEXT_PRIMARY),
                                );
                            })
                            .response
                            .interact(Sense::click())
                            .on_hover_text(&status_visuals.tooltip);
                        if status_resp.clicked() {
                            action = Some(TopBarAction::OpenMiniserverModal);
                        }

                        render_capsule_divider(ui);

                        let ms_items =
                            build_miniserver_toolbar_items(connection_status.is_connecting());
                        for item in &ms_items {
                            let resp = render_vector_button(
                                ui,
                                item.vector_icon,
                                &item.label,
                                item.shortcut_hint,
                                &item.tooltip,
                                item.enabled,
                                item.accent,
                                false,
                            );
                            if resp.clicked() && item.enabled {
                                action = Some(item.action.clone());
                            }
                        }

                        render_capsule_divider(ui);

                        let live_item =
                            build_live_view_toolbar_item(project.is_some(), live_view_active);
                        let live_resp = render_vector_button(
                            ui,
                            live_item.vector_icon,
                            &live_item.label,
                            live_item.shortcut_hint,
                            &live_item.tooltip,
                            live_item.enabled,
                            false,
                            live_view_active,
                        );
                        if live_resp.clicked() && live_item.enabled {
                            action = Some(live_item.action);
                        }
                    });

                // 5. Command Palette Search Bar (⌘K)
                let search_resp = Frame::new()
                    .fill(Color32::from_rgb(0x13, 0x16, 0x1E))
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .corner_radius(CornerRadius::same(7))
                    .inner_margin(Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let (icon_r, _) =
                            ui.allocate_exact_size(Vec2::splat(13.5), Sense::hover());
                        paint_toolbar_icon(
                            ui.painter(),
                            icon_r,
                            ToolbarIcon::Search,
                            TEXT_MUTED,
                        );
                        ui.label(
                            RichText::new("Search…")
                                .size(11.5)
                                .color(TEXT_MUTED),
                        );
                        Frame::new()
                            .fill(CARD_BG)
                            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                            .corner_radius(CornerRadius::same(3))
                            .inner_margin(Margin::symmetric(5, 1))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("⌘K")
                                        .monospace()
                                        .size(9.8)
                                        .color(TEXT_MUTED),
                                );
                            });
                    })
                    .response
                    .interact(Sense::click())
                    .on_hover_text("Open Command Palette to search blocks, pages, or I/O (⌘K / Ctrl+K)");
                if search_resp.clicked() {
                    action = Some(TopBarAction::OpenCommandPalette);
                }

                // 6. Right-Aligned View, Pins & Zoom Controls
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;

                    let view_items = build_view_toolbar_items(compact_pins);
                    // Inspector toggle button on the far right
                    if let Some(insp_item) = view_items.first() {
                        Frame::new()
                            .fill(CAPSULE_BG)
                            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                            .corner_radius(CornerRadius::same(7))
                            .inner_margin(Margin::symmetric(3, 2))
                            .show(ui, |ui| {
                                if render_vector_button(
                                    ui,
                                    insp_item.vector_icon,
                                    &insp_item.label,
                                    insp_item.shortcut_hint,
                                    &insp_item.tooltip,
                                    insp_item.enabled,
                                    insp_item.accent,
                                    false,
                                )
                                .clicked()
                                {
                                    action = Some(insp_item.action.clone());
                                }
                            });
                    }

                    // Pins mode toggle capsule
                    if let Some(pins_item) = view_items.get(2) {
                        Frame::new()
                            .fill(CAPSULE_BG)
                            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                            .corner_radius(CornerRadius::same(7))
                            .inner_margin(Margin::symmetric(3, 2))
                            .show(ui, |ui| {
                                if render_vector_button(
                                    ui,
                                    pins_item.vector_icon,
                                    &pins_item.label,
                                    pins_item.shortcut_hint,
                                    &pins_item.tooltip,
                                    pins_item.enabled,
                                    pins_item.accent,
                                    false,
                                )
                                .clicked()
                                {
                                    action = Some(pins_item.action.clone());
                                }
                            });
                    }

                    // Unified Zoom & Fit Capsule: [ - | 100% | + | Fit ]
                    Frame::new()
                        .fill(CAPSULE_BG)
                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                        .corner_radius(CornerRadius::same(7))
                        .inner_margin(Margin::symmetric(3, 2))
                        .show(ui, |ui| {
                            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                                ui.spacing_mut().item_spacing.x = 2.0;
                                if render_vector_button(
                                    ui,
                                    ToolbarIcon::ZoomOut,
                                    "",
                                    None,
                                    "Zoom out (⌘- / Scroll down)",
                                    true,
                                    false,
                                    false,
                                )
                                .clicked()
                                {
                                    action = Some(TopBarAction::ZoomOut);
                                }

                                let pct_resp = Frame::new()
                                    .corner_radius(CornerRadius::same(4))
                                    .inner_margin(Margin::symmetric(5, 3))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new(format!("{:.0}%", zoom * 100.0))
                                                .monospace()
                                                .size(11.2)
                                                .color(TEXT_PRIMARY),
                                        );
                                    })
                                    .response
                                    .interact(Sense::click())
                                    .on_hover_text("Reset zoom to 100% (0)");
                                if pct_resp.clicked() {
                                    action = Some(TopBarAction::ResetZoom);
                                }

                                if render_vector_button(
                                    ui,
                                    ToolbarIcon::ZoomIn,
                                    "",
                                    None,
                                    "Zoom in (⌘+ / Scroll up)",
                                    true,
                                    false,
                                    false,
                                )
                                .clicked()
                                {
                                    action = Some(TopBarAction::ZoomIn);
                                }

                                if let Some(fit_item) = view_items.get(1) {
                                    render_capsule_divider(ui);
                                    if render_vector_button(
                                        ui,
                                        fit_item.vector_icon,
                                        &fit_item.label,
                                        fit_item.shortcut_hint,
                                        &fit_item.tooltip,
                                        fit_item.enabled,
                                        fit_item.accent,
                                        false,
                                    )
                                    .clicked()
                                    {
                                        action = Some(fit_item.action.clone());
                                    }
                                }
                            });
                        });
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
        assert_eq!(file_items_no_save[0].vector_icon, ToolbarIcon::OpenFolder);
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
        assert_eq!(
            file_items_dirty[1].vector_icon,
            ToolbarIcon::SaveDisk { dirty: true }
        );
        assert_eq!(file_items_dirty[1].label, "Save .Loxone *");

        let auth_items = build_authoring_toolbar_items(true, true, false);
        assert_eq!(auth_items.len(), 3);
        assert_eq!(auth_items[0].group, ToolbarGroup::Authoring);
        assert_eq!(auth_items[0].action, TopBarAction::AddBlock);
        assert_eq!(auth_items[0].vector_icon, ToolbarIcon::AddBlock);
        assert_eq!(auth_items[0].shortcut_hint, Some("A"));
        assert!(auth_items[0].enabled);
        assert_eq!(auth_items[1].action, TopBarAction::Undo);
        assert_eq!(auth_items[1].vector_icon, ToolbarIcon::Undo);
        assert!(auth_items[1].enabled);
        assert_eq!(auth_items[2].action, TopBarAction::Redo);
        assert_eq!(auth_items[2].vector_icon, ToolbarIcon::Redo);
        assert!(!auth_items[2].enabled);

        let ms_items_idle = build_miniserver_toolbar_items(false);
        assert!(!ms_items_idle.is_empty());
        assert_eq!(ms_items_idle[0].group, ToolbarGroup::Miniserver);
        assert_eq!(ms_items_idle[0].action, TopBarAction::OpenMiniserverModal);
        assert_eq!(ms_items_idle[0].vector_icon, ToolbarIcon::Download);
        assert!(ms_items_idle[0].enabled);

        let ms_items_busy = build_miniserver_toolbar_items(true);
        assert!(!ms_items_busy[0].enabled);

        let live_item_off = build_live_view_toolbar_item(true, false);
        assert_eq!(live_item_off.action, TopBarAction::ToggleLiveView);
        assert_eq!(live_item_off.label, "Live View");
        assert!(!live_item_off.accent);

        let live_item_on = build_live_view_toolbar_item(true, true);
        assert_eq!(live_item_on.label, "Live View: ON");
        assert!(live_item_on.accent);

        let view_items = build_view_toolbar_items(true);
        assert_eq!(view_items.len(), 3);
        assert_eq!(view_items[0].group, ToolbarGroup::View);
        assert_eq!(
            view_items[2].vector_icon,
            ToolbarIcon::PinsMode { all_pins: false }
        );

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
                true,
            );
        });
    }
}
