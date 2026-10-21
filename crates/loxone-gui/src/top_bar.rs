use std::path::Path;

use egui::{
    Align, Color32, Context, CornerRadius, Frame, Layout, Margin, Painter, Pos2, Rect, Response,
    RichText, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
};
use loxone_core::miniserver::ConnectionStatus;
use loxone_core::model::LoxoneProject;

use crate::theme::{
    ACCENT_BLUE, BORDER_SUBTLE, CARD_BG, PANEL_BG, STATUS_CONNECTED, STATUS_CONNECTING,
    STATUS_DISCONNECTED, STATUS_ERROR, TEXT_MUTED, TEXT_PRIMARY,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RibbonTab {
    AddDevice,
    #[default]
    Schematic,
    Project,
    Miniserver,
    View,
    /// Contextual optional tab: visible when a block or tree item is selected.
    SelectedObject,
    /// Contextual optional tab: visible when Live View is active.
    LiveView,
}

impl RibbonTab {
    pub fn is_contextual(self) -> bool {
        matches!(self, RibbonTab::SelectedObject | RibbonTab::LiveView)
    }

    pub fn base_label(self) -> &'static str {
        match self {
            RibbonTab::AddDevice => "Add Device",
            RibbonTab::Schematic => "Function Blocks",
            RibbonTab::Project => "Tools",
            RibbonTab::Miniserver => "Testing",
            RibbonTab::View => "Diagnostics",
            RibbonTab::SelectedObject => "Object",
            RibbonTab::LiveView => "Live View",
        }
    }
}

pub fn visible_ribbon_tabs(has_selected_object: bool, live_view_active: bool) -> Vec<RibbonTab> {
    let mut tabs = vec![
        RibbonTab::AddDevice,
        RibbonTab::Schematic,
        RibbonTab::Project,
        RibbonTab::Miniserver,
        RibbonTab::View,
    ];
    if has_selected_object {
        tabs.push(RibbonTab::SelectedObject);
    }
    if live_view_active {
        tabs.push(RibbonTab::LiveView);
    }
    tabs
}

pub fn resolve_active_ribbon_tab(
    current: RibbonTab,
    has_selected_object: bool,
    live_view_active: bool,
) -> RibbonTab {
    match current {
        RibbonTab::SelectedObject if !has_selected_object => RibbonTab::Schematic,
        RibbonTab::LiveView if !live_view_active => RibbonTab::Schematic,
        other => other,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedObjectContext {
    pub uuid: String,
    pub title: String,
    pub object_type: String,
    pub is_page_block: bool,
    pub can_place_on_page: bool,
    pub kb_url: Option<&'static str>,
}

pub fn build_selected_object_context(
    project: Option<&LoxoneProject>,
    selected_uuid: Option<&str>,
) -> Option<SelectedObjectContext> {
    let proj = project?;
    let sel = selected_uuid?.trim();
    if sel.is_empty() {
        return None;
    }
    let summary = proj.xref.objects_by_uuid.get(sel)?;
    let title = if summary.title.trim().is_empty() {
        crate::i18n::block_display_name(&summary.object_type).to_string()
    } else {
        summary.title.clone()
    };
    let is_page_block = matches!(
        summary.location,
        loxone_core::xref::ObjectLocation::PageBlock { .. }
    );
    let can_place_on_page = matches!(
        summary.location,
        loxone_core::xref::ObjectLocation::TreeNode { .. }
    ) && crate::insert_modal::reference_kind_for_tree_node(&summary.object_type)
        .is_some();
    let kb_url = loxone_core::block_schema::lookup_block_schema(&summary.object_type)
        .map(|s| s.kb_url);

    Some(SelectedObjectContext {
        uuid: summary.uuid.clone(),
        title,
        object_type: summary.object_type.clone(),
        is_page_block,
        can_place_on_page,
        kb_url,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopBarAction {
    ToggleLeftPanel,
    ToggleRightPanel,
    ShowProjectLauncher,
    OpenFile,
    SaveFile,
    AddBlock,
    AddBlockCategory(&'static str),
    AddDeviceFilter(&'static str),
    Undo,
    Redo,
    VerifyProject,
    OpenMiniserverModal,
    ToggleLiveView,
    OpenCommandPalette,
    ToggleCompactPins,
    FitPage,
    ZoomIn,
    ZoomOut,
    ResetZoom,
    SetLanguage(crate::i18n::Language),
    SelectRibbonTab(RibbonTab),
    ConfigureSelectedObject,
    CenterSelectedOnCanvas,
    DuplicateSelected,
    DeleteSelected,
    OpenSelectedDocs,
    OpenPeripheryModal,
    OpenHardwareInventory,
    ToggleSimulation,
    StepSimulation,
    ResetSimulation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarGroup {
    File,
    Authoring,
    Miniserver,
    View,
    Contextual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarIcon {
    SidebarLeft,
    SidebarRight,
    OpenFolder,
    SaveDisk { dirty: bool },
    ProjectGrid,
    AddBlock,
    Undo,
    Redo,
    VerifyCheck,
    Download,
    LivePulse { active: bool },
    Search,
    FitPage,
    PinsMode { all_pins: bool },
    ZoomOut,
    ZoomIn,
    ConfigureSliders,
    CenterTarget,
    DuplicateCopy,
    DeleteTrash,
    BookDocs,
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

pub fn status_badge_visuals_localized(
    status: &ConnectionStatus,
    configured_host: Option<&str>,
) -> StatusBadgeVisuals {
    let mut vis = status_badge_visuals(status, configured_host);
    if let Some((prefix, rest)) = vis.label.split_once(" · ") {
        vis.label = format!("{} · {}", crate::i18n::tr(prefix), rest);
    } else {
        vis.label = crate::i18n::tr(&vis.label).to_string();
    }
    vis.tooltip = crate::i18n::tr(&vis.tooltip).to_string();
    vis
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
        ToolbarIcon::VerifyCheck => {
            // Shield/circle with checkmark
            painter.circle_stroke(p(0.50, 0.50), w * 0.38, stroke);
            painter.add(Shape::line(
                vec![p(0.32, 0.52), p(0.45, 0.65), p(0.70, 0.36)],
                Stroke::new((sw * 1.15).max(1.5), fg),
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
        ToolbarIcon::ProjectGrid => {
            for (x0, x1) in [(0.12_f32, 0.45_f32), (0.55_f32, 0.88_f32)] {
                for (y0, y1) in [(0.12_f32, 0.45_f32), (0.55_f32, 0.88_f32)] {
                    let cell = Rect::from_min_max(p(x0, y0), p(x1, y1));
                    painter.rect_filled(cell, 1.5, fg.gamma_multiply(0.18));
                    painter.rect_stroke(cell, 1.5, stroke, StrokeKind::Inside);
                }
            }
        }
        ToolbarIcon::ConfigureSliders => {
            for (ny, knob_x) in [(0.24_f32, 0.36_f32), (0.50_f32, 0.68_f32), (0.76_f32, 0.44_f32)]
            {
                painter.line_segment([p(0.10, ny), p(0.90, ny)], stroke);
                let center = p(knob_x, ny);
                painter.circle_filled(center, w * 0.13, CAPSULE_BG);
                painter.circle_stroke(center, w * 0.13, stroke);
            }
        }
        ToolbarIcon::CenterTarget => {
            let center = p(0.50, 0.50);
            painter.circle_stroke(center, w * 0.30, stroke);
            painter.circle_filled(center, w * 0.08, fg);
            painter.line_segment([p(0.50, 0.08), p(0.50, 0.24)], stroke);
            painter.line_segment([p(0.50, 0.76), p(0.50, 0.92)], stroke);
            painter.line_segment([p(0.08, 0.50), p(0.24, 0.50)], stroke);
            painter.line_segment([p(0.76, 0.50), p(0.92, 0.50)], stroke);
        }
        ToolbarIcon::DuplicateCopy => {
            let back = Rect::from_min_max(p(0.12, 0.12), p(0.66, 0.66));
            let front = Rect::from_min_max(p(0.34, 0.34), p(0.88, 0.88));
            painter.rect_stroke(back, 2.0, stroke, StrokeKind::Inside);
            painter.rect_filled(front, 2.0, CAPSULE_BG);
            painter.rect_filled(front, 2.0, fg.gamma_multiply(0.20));
            painter.rect_stroke(front, 2.0, stroke, StrokeKind::Inside);
        }
        ToolbarIcon::DeleteTrash => {
            painter.line_segment([p(0.16, 0.24), p(0.84, 0.24)], stroke);
            painter.add(Shape::line(
                vec![p(0.36, 0.24), p(0.36, 0.13), p(0.64, 0.13), p(0.64, 0.24)],
                stroke,
            ));
            let body = Rect::from_min_max(p(0.24, 0.24), p(0.76, 0.88));
            painter.rect_stroke(body, 2.0, stroke, StrokeKind::Inside);
            painter.line_segment([p(0.41, 0.38), p(0.41, 0.74)], stroke);
            painter.line_segment([p(0.59, 0.38), p(0.59, 0.74)], stroke);
        }
        ToolbarIcon::BookDocs => {
            let left_page = Rect::from_min_max(p(0.10, 0.18), p(0.50, 0.84));
            let right_page = Rect::from_min_max(p(0.50, 0.18), p(0.90, 0.84));
            painter.rect_filled(left_page, 1.5, fg.gamma_multiply(0.16));
            painter.rect_filled(right_page, 1.5, fg.gamma_multiply(0.16));
            painter.rect_stroke(left_page, 1.5, stroke, StrokeKind::Inside);
            painter.rect_stroke(right_page, 1.5, stroke, StrokeKind::Inside);
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
        ToolbarItem {
            group: ToolbarGroup::Authoring,
            icon: "✓",
            vector_icon: ToolbarIcon::VerifyCheck,
            label: "Verify".to_string(),
            shortcut_hint: None,
            tooltip: "Run Project Health Check / Linter (dangling references, missing Room/Category, unconnected blocks)"
                .to_string(),
            action: TopBarAction::VerifyProject,
            enabled: has_project,
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

pub fn build_selected_object_toolbar_items(ctx_info: &SelectedObjectContext) -> Vec<ToolbarItem> {
    let mut items = Vec::new();
    if ctx_info.is_page_block {
        items.push(ToolbarItem {
            group: ToolbarGroup::Contextual,
            icon: "⚙",
            vector_icon: ToolbarIcon::ConfigureSliders,
            label: "Configure Object…".to_string(),
            shortcut_hint: None,
            tooltip: "Open Loxone Object Configuration modal for this block".to_string(),
            action: TopBarAction::ConfigureSelectedObject,
            enabled: true,
            accent: true,
        });
        items.push(ToolbarItem {
            group: ToolbarGroup::Contextual,
            icon: "🎯",
            vector_icon: ToolbarIcon::CenterTarget,
            label: "Center on Canvas".to_string(),
            shortcut_hint: None,
            tooltip: "Center schematic viewport on the selected block".to_string(),
            action: TopBarAction::CenterSelectedOnCanvas,
            enabled: true,
            accent: false,
        });
        items.push(ToolbarItem {
            group: ToolbarGroup::Contextual,
            icon: "⧉",
            vector_icon: ToolbarIcon::DuplicateCopy,
            label: "Duplicate".to_string(),
            shortcut_hint: Some("⌘D"),
            tooltip: "Duplicate selected block on the active page (⌘D / Ctrl+D)".to_string(),
            action: TopBarAction::DuplicateSelected,
            enabled: true,
            accent: false,
        });
    } else if ctx_info.can_place_on_page {
        items.push(ToolbarItem {
            group: ToolbarGroup::Contextual,
            icon: "➕",
            vector_icon: ToolbarIcon::AddBlock,
            label: "Place on Page".to_string(),
            shortcut_hint: None,
            tooltip: "Place a reference to this tree object onto the active schematic page"
                .to_string(),
            action: TopBarAction::CenterSelectedOnCanvas,
            enabled: true,
            accent: true,
        });
    }

    if ctx_info.kb_url.is_some() {
        items.push(ToolbarItem {
            group: ToolbarGroup::Contextual,
            icon: "📖",
            vector_icon: ToolbarIcon::BookDocs,
            label: "Loxone Docs".to_string(),
            shortcut_hint: None,
            tooltip: "Open Loxone Knowledge Base documentation for this block type".to_string(),
            action: TopBarAction::OpenSelectedDocs,
            enabled: true,
            accent: false,
        });
    }

    items.push(ToolbarItem {
        group: ToolbarGroup::Contextual,
        icon: "🗑",
        vector_icon: ToolbarIcon::DeleteTrash,
        label: "Delete".to_string(),
        shortcut_hint: Some("⌫"),
        tooltip: "Delete the selected object (⌫ / Delete)".to_string(),
        action: TopBarAction::DeleteSelected,
        enabled: true,
        accent: false,
    });

    items
}

#[derive(Debug, Clone, Default)]
struct RibbonMemoryState {
    active_tab: RibbonTab,
}

fn ribbon_memory_id() -> egui::Id {
    egui::Id::new("loxone_top_ribbon_tab_state")
}

pub fn current_ribbon_tab(ctx: &Context) -> RibbonTab {
    ctx.data(|d| {
        d.get_temp::<RibbonMemoryState>(ribbon_memory_id())
            .map(|s| s.active_tab)
            .unwrap_or(RibbonTab::Schematic)
    })
}

pub fn set_ribbon_tab(ctx: &Context, tab: RibbonTab) {
    ctx.data_mut(|d| {
        d.insert_temp(ribbon_memory_id(), RibbonMemoryState { active_tab: tab });
    });
}

const CAPSULE_BG: Color32 = Color32::from_rgb(0x16, 0x16, 0x16);
const BTN_HOVER_BG: Color32 = Color32::from_rgb(0x24, 0x24, 0x24);
const BTN_ACTIVE_BG: Color32 = Color32::from_rgb(0x2E, 0x2E, 0x2E);
const LOXONE_GREEN: Color32 = Color32::from_rgb(0x69, 0xC3, 0x50);
const CONTEXTUAL_OBJECT_ACCENT: Color32 = Color32::from_rgb(0x69, 0xC3, 0x50);

/// Paints Encore's signature solid Loxone-green rounded-square badge with a crisp white 3D cube icon.
fn paint_encore_green_cube_badge(ui: &mut Ui, size: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(7), LOXONE_GREEN);
        let c = rect.center();
        let r = size * 0.28;
        let top = Pos2::new(c.x, c.y - r);
        let right = Pos2::new(c.x + r * 0.88, c.y - r * 0.45);
        let bottom_right = Pos2::new(c.x + r * 0.88, c.y + r * 0.50);
        let bottom = Pos2::new(c.x, c.y + r);
        let bottom_left = Pos2::new(c.x - r * 0.88, c.y + r * 0.50);
        let left = Pos2::new(c.x - r * 0.88, c.y - r * 0.45);
        let mid = Pos2::new(c.x, c.y + r * 0.04);
        let stroke = Stroke::new(1.5_f32, Color32::from_rgb(0x0F, 0x1A, 0x0C));
        painter.add(Shape::convex_polygon(
            vec![top, right, bottom_right, bottom, bottom_left, left],
            Color32::WHITE,
            stroke,
        ));
        painter.line_segment([left, mid], stroke);
        painter.line_segment([right, mid], stroke);
        painter.line_segment([bottom, mid], stroke);
    }
    resp
}

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
    let label = crate::i18n::tr(label);
    let tooltip = crate::i18n::tr(tooltip);
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
            painter.rect_filled(rect, CornerRadius::same(6), BTN_ACTIVE_BG);
        } else if hovered {
            painter.rect_filled(rect, CornerRadius::same(6), BTN_HOVER_BG);
            painter.rect_stroke(
                rect,
                CornerRadius::same(6),
                Stroke::new(1.0_f32, LOXONE_GREEN.gamma_multiply(0.75)),
                StrokeKind::Inside,
            );
        } else if enabled && primary_green {
            painter.rect_filled(
                rect,
                CornerRadius::same(6),
                LOXONE_GREEN.gamma_multiply(0.18),
            );
            painter.rect_stroke(
                rect,
                CornerRadius::same(6),
                Stroke::new(1.0_f32, LOXONE_GREEN.gamma_multiply(0.60)),
                StrokeKind::Inside,
            );
        } else if enabled && accent {
            painter.rect_filled(
                rect,
                CornerRadius::same(6),
                LOXONE_GREEN.gamma_multiply(0.15),
            );
            painter.rect_stroke(
                rect,
                CornerRadius::same(6),
                Stroke::new(1.0_f32, LOXONE_GREEN.gamma_multiply(0.50)),
                StrokeKind::Inside,
            );
        }

        let fg = if !enabled {
            TEXT_MUTED.gamma_multiply(0.42)
        } else if primary_green || accent {
            LOXONE_GREEN
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
                Color32::from_rgb(0x14, 0x14, 0x14).gamma_multiply(badge_alpha),
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

fn truncate_tab_title(s: &str, max_chars: usize) -> String {
    let mut chars = s.chars();
    let head: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}…", head.trim_end())
    } else {
        head
    }
}

/// Renders an Encore-style segmented pill tab button (`#242424` active fill, crisp border, bright white label).
fn render_ribbon_tab_button(
    ui: &mut Ui,
    tab: RibbonTab,
    is_active: bool,
    selected_ctx: Option<&SelectedObjectContext>,
) -> Response {
    let display_text = match tab {
        RibbonTab::SelectedObject => {
            if let Some(sel) = selected_ctx {
                format!(
                    "{} {}",
                    crate::i18n::tr("Object:"),
                    truncate_tab_title(&sel.title, 18)
                )
            } else {
                crate::i18n::tr("Object").to_string()
            }
        }
        RibbonTab::LiveView => crate::i18n::tr("Live View").to_string(),
        other => crate::i18n::tr(other.base_label()).to_string(),
    };

    let font_id = if is_active {
        egui::FontId::new(11.8, egui::FontFamily::Proportional)
    } else {
        egui::FontId::new(11.5, egui::FontFamily::Proportional)
    };
    let galley = ui
        .painter()
        .layout_no_wrap(display_text, font_id, TEXT_PRIMARY);
    let dot_extra = if tab.is_contextual() { 10.0_f32 } else { 0.0_f32 };
    let pad_x = 11.0_f32;
    let btn_h = 26.0_f32;
    let btn_w = galley.size().x + pad_x * 2.0 + dot_extra;

    let (rect, resp) = ui.allocate_exact_size(Vec2::new(btn_w, btn_h), Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let hovered = resp.hovered();

        if is_active {
            // Encore segmented pill active state: solid #262626 pill inside #141414 track
            painter.rect_filled(
                rect,
                CornerRadius::same(6),
                Color32::from_rgb(0x26, 0x26, 0x26),
            );
            painter.rect_stroke(
                rect,
                CornerRadius::same(6),
                Stroke::new(1.0_f32, Color32::from_rgb(0x36, 0x36, 0x36)),
                StrokeKind::Inside,
            );
        } else if hovered {
            painter.rect_filled(
                rect,
                CornerRadius::same(6),
                Color32::from_rgb(0x1E, 0x1E, 0x1E),
            );
        }

        let mut text_x = rect.center().x - (galley.size().x + dot_extra) * 0.5;
        if tab.is_contextual() {
            let dot_center = Pos2::new(text_x + 3.5, rect.center().y);
            painter.circle_filled(dot_center, 3.2, LOXONE_GREEN);
            text_x += dot_extra;
        }

        let text_color = if is_active || hovered {
            Color32::WHITE
        } else if tab.is_contextual() {
            LOXONE_GREEN
        } else {
            TEXT_MUTED
        };
        let text_pos = Pos2::new(text_x, rect.center().y - galley.size().y * 0.5);
        painter.galley(text_pos, galley, text_color);
    }

    resp
}

fn render_ribbon_chip(
    ui: &mut Ui,
    icon_glyph: &str,
    label: &str,
    tooltip: &str,
    enabled: bool,
    tint: Color32,
) -> Response {
    let label_tr = crate::i18n::tr(label);
    let tooltip_tr = crate::i18n::tr(tooltip);
    let text = format!("{} {}", icon_glyph, label_tr);
    let galley = ui.painter().layout_no_wrap(
        text,
        egui::FontId::proportional(11.3),
        TEXT_PRIMARY,
    );
    let pad_x = 8.0_f32;
    let btn_h = 26.0_f32;
    let total_w = galley.size().x + pad_x * 2.0;
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(total_w, btn_h), sense);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let hovered = enabled && resp.hovered();
        let active = enabled && resp.is_pointer_button_down_on();
        if active {
            painter.rect_filled(rect, CornerRadius::same(5), BTN_ACTIVE_BG);
        } else if hovered {
            painter.rect_filled(rect, CornerRadius::same(5), tint.gamma_multiply(0.24));
            painter.rect_stroke(
                rect,
                CornerRadius::same(5),
                Stroke::new(1.0_f32, tint.gamma_multiply(0.75)),
                StrokeKind::Inside,
            );
        } else {
            painter.rect_filled(rect, CornerRadius::same(5), tint.gamma_multiply(0.10));
            painter.rect_stroke(
                rect,
                CornerRadius::same(5),
                Stroke::new(1.0_f32, tint.gamma_multiply(0.32)),
                StrokeKind::Inside,
            );
        }
        let text_color = if !enabled {
            TEXT_MUTED.gamma_multiply(0.45)
        } else if hovered {
            Color32::WHITE
        } else {
            TEXT_PRIMARY
        };
        let pos = Pos2::new(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        );
        painter.galley(pos, galley, text_color);
    }
    resp.on_hover_text(tooltip_tr)
}

#[allow(clippy::too_many_arguments)]
fn render_ribbon_row_1(
    ui: &mut Ui,
    project: Option<&LoxoneProject>,
    loaded_path: Option<&Path>,
    active_page_index: usize,
    can_save: bool,
    is_dirty: bool,
    can_undo: bool,
    can_redo: bool,
    compact_pins: bool,
    status_visuals: &StatusBadgeVisuals,
    visible_tabs: &[RibbonTab],
    active_tab: RibbonTab,
    selected_ctx: Option<&SelectedObjectContext>,
) -> Option<TopBarAction> {
    let mut action = None;
    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 6.0;

        // 1. Encore-style Left Header Identity (Green Cube Badge + Page/Project Title + Quick Actions)
        let cube_resp = paint_encore_green_cube_badge(ui, 24.0).on_hover_text(crate::i18n::tr(
            "Click to switch project / show project list",
        ));
        if cube_resp.clicked() && project.is_some() {
            action = Some(TopBarAction::ShowProjectLauncher);
        }

        let proj_title = project
            .map(|p| {
                if p.metadata.title.trim().is_empty() {
                    crate::i18n::tr("Project")
                } else {
                    p.metadata.title.as_str()
                }
            })
            .unwrap_or("Loxone Portable");
        let dirty_suffix = if is_dirty { " *" } else { "" };

        if let Some(proj) = project {
            let page_name = proj
                .pages
                .get(active_page_index)
                .map(|pg| pg.title.as_str())
                .unwrap_or_else(|| crate::i18n::tr("No Page"));
            let page_resp = ui
                .label(
                    RichText::new(truncate_tab_title(page_name, 22))
                        .size(13.5)
                        .color(Color32::WHITE)
                        .strong(),
                )
                .interact(Sense::click())
                .on_hover_text(crate::i18n::tr(
                    "Click to switch page or jump (⌘K / Ctrl+K)",
                ));
            if page_resp.clicked() {
                action = Some(TopBarAction::OpenCommandPalette);
            }
        } else {
            ui.label(
                RichText::new(format!("{}{}", proj_title, dirty_suffix))
                    .size(13.5)
                    .color(Color32::WHITE)
                    .strong(),
            );
        }

        // Encore-style subtle dark status pill next to title
        let proj_pill_resp = Frame::new()
            .fill(Color32::from_rgb(0x1C, 0x1C, 0x1C))
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .corner_radius(CornerRadius::same(5))
            .inner_margin(Margin::symmetric(6, 2))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label(
                    RichText::new(format!("{}{}", truncate_tab_title(proj_title, 18), dirty_suffix))
                        .size(10.8)
                        .color(TEXT_MUTED),
                );
            })
            .response
            .interact(Sense::click());
        if proj_pill_resp.clicked() && project.is_some() {
            action = Some(TopBarAction::ShowProjectLauncher);
        }
        if let Some(loaded) = loaded_path {
            proj_pill_resp.on_hover_text(format!(
                "{}\n{}",
                loaded.display(),
                crate::i18n::tr("Click to switch project / show project list")
            ));
        }

        // Compact quick-access capsule (Sidebar, Save, Undo, Redo)
        Frame::new()
            .fill(CAPSULE_BG)
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(3, 2))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
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
                if render_vector_button(
                    ui,
                    ToolbarIcon::SaveDisk { dirty: is_dirty },
                    "",
                    None,
                    "Save current project as .Loxone XML file (⌘S / Ctrl+S)",
                    can_save,
                    is_dirty,
                    false,
                )
                .clicked()
                    && can_save
                {
                    action = Some(TopBarAction::SaveFile);
                }
                if render_vector_button(
                    ui,
                    ToolbarIcon::Undo,
                    "",
                    None,
                    "Undo last edit (⌘Z / Ctrl+Z)",
                    can_undo,
                    false,
                    false,
                )
                .clicked()
                    && can_undo
                {
                    action = Some(TopBarAction::Undo);
                }
                if render_vector_button(
                    ui,
                    ToolbarIcon::Redo,
                    "",
                    None,
                    "Redo edit (⌘⇧Z / Ctrl+Y)",
                    can_redo,
                    false,
                    false,
                )
                .clicked()
                    && can_redo
                {
                    action = Some(TopBarAction::Redo);
                }
            });

        // 2. Encore Segmented Pill Tab Control
        Frame::new()
            .fill(Color32::from_rgb(0x16, 0x16, 0x16))
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .corner_radius(CornerRadius::same(7))
            .inner_margin(Margin::symmetric(3, 2))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let mut seen_contextual = false;
                for &tab in visible_tabs {
                    if tab.is_contextual() && !seen_contextual {
                        render_capsule_divider(ui);
                        seen_contextual = true;
                    }
                    let resp = render_ribbon_tab_button(
                        ui,
                        tab,
                        tab == active_tab,
                        selected_ctx,
                    );
                    if resp.clicked() {
                        action = Some(TopBarAction::SelectRibbonTab(tab));
                    }
                }
            });

        // 3. Right Pinned Global Utilities (Inspector, Settings/Language menu, Search, Status Dot)
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 5.0;

            Frame::new()
                .fill(CAPSULE_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .corner_radius(CornerRadius::same(7))
                .inner_margin(Margin::symmetric(3, 2))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if render_vector_button(
                        ui,
                        ToolbarIcon::SidebarRight,
                        "",
                        None,
                        "Toggle Property Inspector (⌘I / Ctrl+I)",
                        true,
                        false,
                        false,
                    )
                    .clicked()
                    {
                        action = Some(TopBarAction::ToggleRightPanel);
                    }

                    render_capsule_divider(ui);

                    // Settings gear menu button (houses Language selection & View preferences)
                    let lang = crate::i18n::current_language();
                    let menu_resp = ui.menu_button(
                        RichText::new("⚙").size(13.0).color(TEXT_PRIMARY),
                        |ui| {
                            ui.set_min_width(190.0);
                            ui.label(
                                RichText::new(format!("⚙ {}", crate::i18n::tr("Settings")))
                                    .size(11.5)
                                    .color(TEXT_PRIMARY)
                                    .strong(),
                            );
                            ui.separator();
                            ui.label(
                                RichText::new(format!("🌐 {}", crate::i18n::tr("Language")))
                                    .size(10.8)
                                    .color(TEXT_MUTED),
                            );
                            for avail in crate::i18n::available_languages() {
                                let is_cur = avail == lang;
                                let prefix = if is_cur { "✓ " } else { "   " };
                                let item_label = format!(
                                    "{}{} — {}",
                                    prefix,
                                    avail.code(),
                                    avail.display_name()
                                );
                                if ui.selectable_label(is_cur, item_label).clicked() {
                                    action = Some(TopBarAction::SetLanguage(avail));
                                    ui.close_menu();
                                }
                            }
                            ui.separator();
                            let pin_label = if compact_pins {
                                crate::i18n::tr("Pins: Connected")
                            } else {
                                crate::i18n::tr("Pins: All")
                            };
                            if ui.button(format!("⚡ {}", pin_label)).clicked() {
                                action = Some(TopBarAction::ToggleCompactPins);
                                ui.close_menu();
                            }
                            if ui
                                .button(format!("◧ {}", crate::i18n::tr("Explorer")))
                                .clicked()
                            {
                                action = Some(TopBarAction::ToggleLeftPanel);
                                ui.close_menu();
                            }
                            if ui
                                .button(format!("◨ {}", crate::i18n::tr("Inspector")))
                                .clicked()
                            {
                                action = Some(TopBarAction::ToggleRightPanel);
                                ui.close_menu();
                            }
                        },
                    );
                    menu_resp
                        .response
                        .on_hover_text(crate::i18n::tr("Settings"));
                });

            let search_resp = Frame::new()
                .fill(Color32::from_rgb(0x16, 0x16, 0x16))
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .corner_radius(CornerRadius::same(7))
                .inner_margin(Margin::symmetric(7, 4))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    let (icon_r, _) =
                        ui.allocate_exact_size(Vec2::splat(13.0), Sense::hover());
                    paint_toolbar_icon(ui.painter(), icon_r, ToolbarIcon::Search, TEXT_MUTED);
                    ui.label(
                        RichText::new(crate::i18n::tr("Search…"))
                            .size(11.0)
                            .color(TEXT_MUTED),
                    );
                    Frame::new()
                        .fill(CARD_BG)
                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                        .corner_radius(CornerRadius::same(3))
                        .inner_margin(Margin::symmetric(4, 1))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("⌘K")
                                    .monospace()
                                    .size(9.5)
                                    .color(TEXT_MUTED),
                            );
                        });
                })
                .response
                .interact(Sense::click())
                .on_hover_text(crate::i18n::tr(
                    "Open Command Palette to search blocks, pages, or I/O (⌘K / Ctrl+K)",
                ));
            if search_resp.clicked() {
                action = Some(TopBarAction::OpenCommandPalette);
            }

            let status_pill_resp = Frame::new()
                .fill(CAPSULE_BG)
                .stroke(Stroke::new(
                    1.0_f32,
                    status_visuals.dot_color.gamma_multiply(0.55),
                ))
                .corner_radius(CornerRadius::same(7))
                .inner_margin(Margin::symmetric(6, 4))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    paint_status_indicator(ui, status_visuals);
                })
                .response
                .interact(Sense::click())
                .on_hover_text(format!("{} — {}", status_visuals.label, status_visuals.tooltip));
            if status_pill_resp.clicked() {
                action = Some(TopBarAction::OpenMiniserverModal);
            }
        });
    });
    action
}

#[allow(clippy::too_many_arguments)]
fn render_ribbon_row_2(
    ui: &mut Ui,
    active_tab: RibbonTab,
    project: Option<&LoxoneProject>,
    compact_pins: bool,
    zoom: f32,
    connection_status: &ConnectionStatus,
    status_visuals: &StatusBadgeVisuals,
    can_save: bool,
    is_dirty: bool,
    can_undo: bool,
    can_redo: bool,
    live_view_active: bool,
    selected_ctx: Option<&SelectedObjectContext>,
) -> Option<TopBarAction> {
    let mut action = None;
    let has_project = project.is_some();
    let view_items = build_view_toolbar_items(compact_pins);

    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 6.0;

        // Left Pinned LIVE tile button (as in Loxone Config ribbon)
        let live_item = build_live_view_toolbar_item(has_project, live_view_active);
        if let Some(act) = render_toolbar_capsule(ui, std::slice::from_ref(&live_item)) {
            action = Some(act);
        }

        match active_tab {
            RibbonTab::AddDevice => {
                Frame::new()
                    .fill(CAPSULE_BG)
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .corner_radius(CornerRadius::same(7))
                    .inner_margin(Margin::symmetric(4, 2))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let device_tiles: &[(&str, &str, &str, Color32, TopBarAction)] = &[
                            (
                                "🔌",
                                "Provision Hardware…",
                                "Provision Extensions, Loxone Tree devices, or Loxone Air devices into Periphery",
                                LOXONE_GREEN,
                                TopBarAction::OpenPeripheryModal,
                            ),
                            (
                                "🔢",
                                "Serial Inventory",
                                "Inspect provisioned hardware and swap physical Serial Numbers in-place",
                                ACCENT_BLUE,
                                TopBarAction::OpenHardwareInventory,
                            ),
                            (
                                "➕",
                                "All Devices",
                                "Insert a function block or reference on the active page (A)",
                                LOXONE_GREEN,
                                TopBarAction::AddDeviceFilter(""),
                            ),
                            (
                                "🔍",
                                "Search Devices",
                                "Open Command Palette to search blocks, pages, or I/O (⌘K / Ctrl+K)",
                                ACCENT_BLUE,
                                TopBarAction::OpenCommandPalette,
                            ),
                            (
                                "🌿",
                                "Tree",
                                "Tree",
                                Color32::from_rgb(0x52, 0xB7, 0x88),
                                TopBarAction::AddDeviceFilter("Tree"),
                            ),
                            (
                                "📡",
                                "Air",
                                "Air",
                                Color32::from_rgb(0x5E, 0x9C, 0xDB),
                                TopBarAction::AddDeviceFilter("Air"),
                            ),
                            (
                                "🔗",
                                "Link",
                                "Link",
                                Color32::from_rgb(0x9B, 0x72, 0xCF),
                                TopBarAction::AddDeviceFilter("Extension"),
                            ),
                            (
                                "🔌",
                                "Virtual I/O",
                                "Virtual I/O",
                                Color32::from_rgb(0xE4, 0xB3, 0x3B),
                                TopBarAction::AddDeviceFilter("Virtual"),
                            ),
                            (
                                "🚩",
                                "Memory Flags",
                                "Memory Flags",
                                Color32::from_rgb(0xD9, 0x6C, 0x43),
                                TopBarAction::AddDeviceFilter("Merker"),
                            ),
                        ];
                        for (icon, label, tip, tint, act) in device_tiles {
                            let enabled = has_project
                                || matches!(act, TopBarAction::OpenCommandPalette);
                            if render_ribbon_chip(ui, icon, label, tip, enabled, *tint).clicked()
                                && enabled
                            {
                                action = Some(act.clone());
                            }
                        }
                    });
            }
            RibbonTab::Schematic => {
                let add_item = ToolbarItem {
                    group: ToolbarGroup::Authoring,
                    icon: "➕",
                    vector_icon: ToolbarIcon::AddBlock,
                    label: "Insert Block…".to_string(),
                    shortcut_hint: Some("A"),
                    tooltip: "Insert a function block or reference on the active page (A)"
                        .to_string(),
                    action: TopBarAction::AddBlock,
                    enabled: has_project,
                    accent: true,
                };
                if let Some(act) = render_toolbar_capsule(ui, std::slice::from_ref(&add_item)) {
                    action = Some(act);
                }

                Frame::new()
                    .fill(CAPSULE_BG)
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .corner_radius(CornerRadius::same(7))
                    .inner_margin(Margin::symmetric(4, 2))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let categories: &[(&str, &str, &str, Color32)] = &[
                            ("💡", "Lighting", "Lighting", Color32::from_rgb(0xE4, 0xB3, 0x3B)),
                            ("🌡", "Climate", "Climate", Color32::from_rgb(0xD9, 0x6C, 0x43)),
                            ("🪟", "Shading", "Shading", Color32::from_rgb(0x5E, 0x9C, 0xDB)),
                            ("⚡", "Energy", "Energy", Color32::from_rgb(0x52, 0xB7, 0x88)),
                            ("🔑", "Access", "Security", Color32::from_rgb(0xCC, 0x4B, 0x4B)),
                            ("🎛", "Control", "Control", ACCENT_BLUE),
                            ("🎵", "Multimedia", "Audio", Color32::from_rgb(0x9B, 0x72, 0xCF)),
                            ("✥", "Central", "Central", LOXONE_GREEN),
                        ];
                        for &(icon, label, cat_key, tint) in categories {
                            if render_ribbon_chip(ui, icon, label, label, has_project, tint)
                                .clicked()
                                && has_project
                            {
                                action = Some(TopBarAction::AddBlockCategory(cat_key));
                            }
                        }
                    });

                if let Some(sel) = selected_ctx {
                    let quick_items = if sel.is_page_block {
                        vec![ToolbarItem {
                            group: ToolbarGroup::Contextual,
                            icon: "⚙",
                            vector_icon: ToolbarIcon::ConfigureSliders,
                            label: "Configure Object…".to_string(),
                            shortcut_hint: None,
                            tooltip: "Open Loxone Object Configuration modal for this block"
                                .to_string(),
                            action: TopBarAction::ConfigureSelectedObject,
                            enabled: true,
                            accent: true,
                        }]
                    } else if sel.can_place_on_page {
                        vec![ToolbarItem {
                            group: ToolbarGroup::Contextual,
                            icon: "➕",
                            vector_icon: ToolbarIcon::AddBlock,
                            label: "Place on Page".to_string(),
                            shortcut_hint: None,
                            tooltip:
                                "Place a reference to this tree object onto the active schematic page"
                                    .to_string(),
                            action: TopBarAction::CenterSelectedOnCanvas,
                            enabled: true,
                            accent: true,
                        }]
                    } else {
                        Vec::new()
                    };
                    if !quick_items.is_empty() {
                        if let Some(act) = render_toolbar_capsule(ui, &quick_items) {
                            action = Some(act);
                        }
                    }
                }
            }
            RibbonTab::Project => {
                let mut file_items = build_file_toolbar_items(can_save, is_dirty);
                file_items.push(ToolbarItem {
                    group: ToolbarGroup::File,
                    icon: "⊞",
                    vector_icon: ToolbarIcon::ProjectGrid,
                    label: "Switch Project".to_string(),
                    shortcut_hint: None,
                    tooltip: "Click to switch project / show project list".to_string(),
                    action: TopBarAction::ShowProjectLauncher,
                    enabled: has_project,
                    accent: false,
                });
                if let Some(act) = render_toolbar_capsule(ui, &file_items) {
                    action = Some(act);
                }

                let auth_items = build_authoring_toolbar_items(has_project, can_undo, can_redo);
                if let Some(act) = render_toolbar_capsule(ui, &auth_items[1..]) {
                    action = Some(act);
                }

                if let Some(pins_item) = view_items.get(2) {
                    if let Some(act) = render_toolbar_capsule(ui, std::slice::from_ref(pins_item)) {
                        action = Some(act);
                    }
                }
            }
            RibbonTab::Miniserver => {
                let ms_border = match connection_status {
                    ConnectionStatus::Disconnected => BORDER_SUBTLE,
                    _ => status_visuals.dot_color.gamma_multiply(0.60),
                };
                Frame::new()
                    .fill(CAPSULE_BG)
                    .stroke(Stroke::new(1.0_f32, ms_border))
                    .corner_radius(CornerRadius::same(7))
                    .inner_margin(Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        paint_status_indicator(ui, status_visuals);
                        ui.label(
                            RichText::new(&status_visuals.label)
                                .size(11.4)
                                .color(TEXT_PRIMARY)
                                .strong(),
                        );
                    })
                    .response
                    .interact(Sense::click())
                    .on_hover_text(&status_visuals.tooltip)
                    .clicked()
                    .then(|| {
                        action = Some(TopBarAction::OpenMiniserverModal);
                    });

                let mut ms_items =
                    build_miniserver_toolbar_items(connection_status.is_connecting());
                ms_items.push(ToolbarItem {
                    group: ToolbarGroup::Authoring,
                    icon: "✓",
                    vector_icon: ToolbarIcon::VerifyCheck,
                    label: "Verify".to_string(),
                    shortcut_hint: None,
                    tooltip: "Run Project Health Check / Linter (dangling references, missing Room/Category, unconnected blocks)"
                        .to_string(),
                    action: TopBarAction::VerifyProject,
                    enabled: has_project,
                    accent: false,
                });
                if let Some(pins_item) = view_items.get(2) {
                    ms_items.push(pins_item.clone());
                }
                if let Some(act) = render_toolbar_capsule(ui, &ms_items) {
                    action = Some(act);
                }

                Frame::new()
                    .fill(CAPSULE_BG)
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .corner_radius(CornerRadius::same(7))
                    .inner_margin(Margin::symmetric(4, 2))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        if render_ribbon_chip(
                            ui,
                            "▶",
                            "Simulate Offline",
                            "Toggle interactive offline logic simulation on the schematic canvas",
                            has_project,
                            LOXONE_GREEN,
                        )
                        .clicked()
                            && has_project
                        {
                            action = Some(TopBarAction::ToggleSimulation);
                        }
                        if render_ribbon_chip(
                            ui,
                            "⏭",
                            "Step +1s",
                            "Advance offline simulation clock by +1.0 second (for timers, delays & sequences)",
                            has_project,
                            ACCENT_BLUE,
                        )
                        .clicked()
                            && has_project
                        {
                            action = Some(TopBarAction::StepSimulation);
                        }
                        if render_ribbon_chip(
                            ui,
                            "↺",
                            "Reset Sim",
                            "Reset all offline simulation overrides, latches, and virtual clock",
                            has_project,
                            Color32::from_rgb(0xE4, 0xB3, 0x3B),
                        )
                        .clicked()
                            && has_project
                        {
                            action = Some(TopBarAction::ResetSimulation);
                        }
                    });
            }
            RibbonTab::View => {
                let diag_items = vec![
                    ToolbarItem {
                        group: ToolbarGroup::Authoring,
                        icon: "✓",
                        vector_icon: ToolbarIcon::VerifyCheck,
                        label: "Verify".to_string(),
                        shortcut_hint: None,
                        tooltip: "Run Project Health Check / Linter (dangling references, missing Room/Category, unconnected blocks)"
                            .to_string(),
                        action: TopBarAction::VerifyProject,
                        enabled: has_project,
                        accent: true,
                    },
                    ToolbarItem {
                        group: ToolbarGroup::Miniserver,
                        icon: "⬇",
                        vector_icon: ToolbarIcon::Download,
                        label: "Miniserver".to_string(),
                        shortcut_hint: None,
                        tooltip: "Connect to Miniserver and download project backup (⌘⇧D / Ctrl+Shift+D)"
                            .to_string(),
                        action: TopBarAction::OpenMiniserverModal,
                        enabled: !connection_status.is_connecting(),
                        accent: false,
                    },
                ];
                if let Some(act) = render_toolbar_capsule(ui, &diag_items) {
                    action = Some(act);
                }

                let panel_items = vec![
                    ToolbarItem {
                        group: ToolbarGroup::View,
                        icon: "◧",
                        vector_icon: ToolbarIcon::SidebarLeft,
                        label: "Explorer".to_string(),
                        shortcut_hint: Some("⌘B"),
                        tooltip: "Toggle Explorer Sidebar (⌘B / Ctrl+B)".to_string(),
                        action: TopBarAction::ToggleLeftPanel,
                        enabled: true,
                        accent: false,
                    },
                    ToolbarItem {
                        group: ToolbarGroup::View,
                        icon: "◨",
                        vector_icon: ToolbarIcon::SidebarRight,
                        label: "Inspector".to_string(),
                        shortcut_hint: Some("⌘I"),
                        tooltip: "Toggle Property Inspector (⌘I / Ctrl+I)".to_string(),
                        action: TopBarAction::ToggleRightPanel,
                        enabled: true,
                        accent: false,
                    },
                ];
                if let Some(act) = render_toolbar_capsule(ui, &panel_items) {
                    action = Some(act);
                }

                if let Some(proj) = project {
                    Frame::new()
                        .fill(CARD_BG)
                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!(
                                    "{}: {}  ·  {}: {}  ·  {}: {}",
                                    crate::i18n::tr("Pages"),
                                    proj.pages.len(),
                                    crate::i18n::tr("Rooms"),
                                    proj.places.len(),
                                    crate::i18n::tr("Categories"),
                                    proj.categories.len(),
                                ))
                                .size(11.0)
                                .color(TEXT_MUTED),
                            );
                        });
                }
            }
            RibbonTab::SelectedObject => {
                if let Some(sel) = selected_ctx {
                    Frame::new()
                        .fill(CONTEXTUAL_OBJECT_ACCENT.gamma_multiply(0.14))
                        .stroke(Stroke::new(
                            1.0_f32,
                            CONTEXTUAL_OBJECT_ACCENT.gamma_multiply(0.45),
                        ))
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing.x = 5.0;
                            ui.label(
                                RichText::new(crate::i18n::block_display_name(&sel.object_type))
                                    .size(11.0)
                                    .color(Color32::from_rgb(0xB8, 0xF2, 0xA6))
                                    .strong(),
                            );
                            ui.label(RichText::new("·").size(11.0).color(TEXT_MUTED));
                            ui.label(
                                RichText::new(truncate_tab_title(&sel.title, 28))
                                    .size(11.2)
                                    .color(Color32::WHITE)
                                    .strong(),
                            );
                        });

                    let ctx_items = build_selected_object_toolbar_items(sel);
                    if let Some(act) = render_toolbar_capsule(ui, &ctx_items) {
                        action = Some(act);
                    }
                }
            }
            RibbonTab::LiveView => {
                Frame::new()
                    .fill(LOXONE_GREEN.gamma_multiply(0.16))
                    .stroke(Stroke::new(1.0_f32, LOXONE_GREEN.gamma_multiply(0.55)))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        paint_status_indicator(ui, status_visuals);
                        ui.label(
                            RichText::new(format!(
                                "⚡ {}  ·  {}",
                                crate::i18n::tr("Live View: ON"),
                                status_visuals.label
                            ))
                            .size(11.2)
                            .color(Color32::from_rgb(0xC8, 0xFA, 0x9E))
                            .strong(),
                        );
                    });

                let mut live_items =
                    build_miniserver_toolbar_items(connection_status.is_connecting());
                if let Some(pins_item) = view_items.get(2) {
                    live_items.push(pins_item.clone());
                }
                if let Some(act) = render_toolbar_capsule(ui, &live_items) {
                    action = Some(act);
                }

                Frame::new()
                    .fill(CAPSULE_BG)
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .corner_radius(CornerRadius::same(7))
                    .inner_margin(Margin::symmetric(4, 2))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        if render_ribbon_chip(
                            ui,
                            "▶",
                            "Simulate Offline",
                            "Toggle interactive offline logic simulation on the schematic canvas",
                            has_project,
                            LOXONE_GREEN,
                        )
                        .clicked()
                            && has_project
                        {
                            action = Some(TopBarAction::ToggleSimulation);
                        }
                        if render_ribbon_chip(
                            ui,
                            "⏭",
                            "Step +1s",
                            "Advance offline simulation clock by +1.0 second",
                            has_project,
                            ACCENT_BLUE,
                        )
                        .clicked()
                            && has_project
                        {
                            action = Some(TopBarAction::StepSimulation);
                        }
                        if render_ribbon_chip(
                            ui,
                            "↺",
                            "Reset Sim",
                            "Reset all offline simulation overrides, latches, and virtual clock",
                            has_project,
                            Color32::from_rgb(0xE4, 0xB3, 0x3B),
                        )
                        .clicked()
                            && has_project
                        {
                            action = Some(TopBarAction::ResetSimulation);
                        }
                    });
            }
        }

        // Right-aligned Pinned Zoom & Fit Capsule across all tabs
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
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
                            .on_hover_text(crate::i18n::tr("Reset zoom to 100% (0)"));
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

    action
}

#[allow(clippy::too_many_arguments)]
#[cfg(test)]
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
    render_top_bar_with_selection(
        ctx,
        project,
        loaded_path,
        active_page_index,
        compact_pins,
        zoom,
        connection_status,
        configured_host,
        is_dirty,
        can_undo,
        can_redo,
        live_view_active,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn render_top_bar_with_selection(
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
    selected_uuid: Option<&str>,
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
    let status_visuals = status_badge_visuals_localized(connection_status, effective_host);

    let selected_ctx = build_selected_object_context(project, selected_uuid);
    let visible_tabs = visible_ribbon_tabs(selected_ctx.is_some(), live_view_active);
    let stored_tab = current_ribbon_tab(ctx);
    let mut active_tab =
        resolve_active_ribbon_tab(stored_tab, selected_ctx.is_some(), live_view_active);
    if active_tab != stored_tab {
        set_ribbon_tab(ctx, active_tab);
    }

    // Encore-style Top Loxone-Green Accent Strip + Centered Hanging Environment/Status Pill
    egui::TopBottomPanel::top("encore_env_banner")
        .exact_height(20.0)
        .frame(
            Frame::new()
                .fill(PANEL_BG)
                .stroke(Stroke::NONE)
                .inner_margin(Margin::ZERO),
        )
        .show(ctx, |ui| {
            let full_rect = ui.max_rect();
            let top_strip = egui::Rect::from_min_max(
                full_rect.min,
                egui::pos2(full_rect.max.x, full_rect.min.y + 4.0),
            );
            ui.painter()
                .rect_filled(top_strip, CornerRadius::ZERO, LOXONE_GREEN);

            let env_proj = project
                .map(|p| {
                    if p.metadata.title.trim().is_empty() {
                        "loxone-config"
                    } else {
                        p.metadata.title.as_str()
                    }
                })
                .unwrap_or("loxone-portable");
            let env_target = if live_view_active {
                crate::i18n::tr("Live View")
            } else {
                effective_host.unwrap_or(&status_visuals.label)
            };
            let pill_text = format!(
                "{}  •  {}",
                truncate_tab_title(env_proj, 24),
                truncate_tab_title(env_target, 22)
            );
            let font_id = egui::FontId::proportional(11.0);
            let dark_fg = Color32::from_rgb(0x0B, 0x16, 0x08);
            let galley = ui
                .painter()
                .layout_no_wrap(pill_text, font_id, dark_fg);
            let pill_w = (galley.size().x + 28.0).max(140.0);
            let pill_rect = egui::Rect::from_min_max(
                egui::pos2(full_rect.center().x - pill_w * 0.5, full_rect.min.y),
                egui::pos2(full_rect.center().x + pill_w * 0.5, full_rect.min.y + 19.0),
            );
            let pill_resp = ui.interact(
                pill_rect,
                ui.id().with("encore_hanging_env_pill"),
                Sense::click(),
            );
            let pill_fill = if pill_resp.hovered() {
                Color32::from_rgb(0x78, 0xD2, 0x5E)
            } else {
                LOXONE_GREEN
            };
            ui.painter().rect_filled(
                pill_rect,
                CornerRadius {
                    nw: 0,
                    ne: 0,
                    sw: 7,
                    se: 7,
                },
                pill_fill,
            );
            let text_pos = egui::pos2(
                pill_rect.center().x - galley.size().x * 0.5,
                pill_rect.center().y - galley.size().y * 0.5 + 0.5,
            );
            ui.painter().galley(text_pos, galley, dark_fg);

            if pill_resp
                .on_hover_text(format!("{} — {}", status_visuals.label, status_visuals.tooltip))
                .clicked()
            {
                action = Some(TopBarAction::OpenMiniserverModal);
            }
        });

    egui::TopBottomPanel::top("top_title_bar")
        .exact_height(36.0)
        .frame(
            Frame::new()
                .fill(PANEL_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .inner_margin(Margin::symmetric(10, 4)),
        )
        .show(ctx, |ui| {
            if let Some(row1_act) = render_ribbon_row_1(
                ui,
                project,
                loaded_path,
                active_page_index,
                can_save,
                is_dirty,
                can_undo,
                can_redo,
                compact_pins,
                &status_visuals,
                &visible_tabs,
                active_tab,
                selected_ctx.as_ref(),
            ) {
                if let TopBarAction::SelectRibbonTab(new_tab) = row1_act {
                    active_tab = new_tab;
                    set_ribbon_tab(ctx, new_tab);
                    ctx.request_repaint();
                }
                action = Some(row1_act);
            }
        });

    egui::TopBottomPanel::top("top_ribbon_bar")
        .exact_height(38.0)
        .frame(
            Frame::new()
                .fill(PANEL_BG)
                .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                .inner_margin(Margin::symmetric(10, 4)),
        )
        .show(ctx, |ui| {
            if let Some(row2_act) = render_ribbon_row_2(
                ui,
                active_tab,
                project,
                compact_pins,
                zoom,
                connection_status,
                &status_visuals,
                can_save,
                is_dirty,
                can_undo,
                can_redo,
                live_view_active,
                selected_ctx.as_ref(),
            ) {
                if row2_act == TopBarAction::OpenSelectedDocs {
                    if let Some(url) = selected_ctx.as_ref().and_then(|c| c.kb_url) {
                        ctx.open_url(egui::OpenUrl::new_tab(url));
                    }
                }
                action = Some(row2_act);
            }
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
        assert_eq!(auth_items.len(), 4);
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
        assert_eq!(auth_items[3].action, TopBarAction::VerifyProject);
        assert_eq!(auth_items[3].vector_icon, ToolbarIcon::VerifyCheck);
        assert!(auth_items[3].enabled);

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

    #[test]
    fn test_ribbon_tabs_and_contextual_object_toolbar() {
        crate::i18n::set_language(crate::i18n::Language::En);

        // 1. Core tabs when no contextual state is active
        let core_only = visible_ribbon_tabs(false, false);
        assert_eq!(
            core_only,
            vec![
                RibbonTab::AddDevice,
                RibbonTab::Schematic,
                RibbonTab::Project,
                RibbonTab::Miniserver,
                RibbonTab::View,
            ]
        );

        // 2. Contextual tabs appear dynamically when object selected or Live View active
        let with_obj = visible_ribbon_tabs(true, false);
        assert!(with_obj.contains(&RibbonTab::SelectedObject));
        assert!(!with_obj.contains(&RibbonTab::LiveView));

        let with_both = visible_ribbon_tabs(true, true);
        assert!(with_both.contains(&RibbonTab::SelectedObject));
        assert!(with_both.contains(&RibbonTab::LiveView));

        // 3. Fallback when contextual condition ends
        assert_eq!(
            resolve_active_ribbon_tab(RibbonTab::SelectedObject, false, false),
            RibbonTab::Schematic
        );
        assert_eq!(
            resolve_active_ribbon_tab(RibbonTab::LiveView, true, false),
            RibbonTab::Schematic
        );
        assert_eq!(
            resolve_active_ribbon_tab(RibbonTab::SelectedObject, true, false),
            RibbonTab::SelectedObject
        );

        // 4. Contextual object context & toolbar items from Klimczaki.Loxone
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Klimczaki.Loxone");
        let proj = loxone_core::parser::load_loxone_file(&path).expect("load Klimczaki.Loxone");
        let first_block = &proj.pages[0].blocks[0];
        let obj_ctx = build_selected_object_context(Some(&proj), Some(&first_block.uuid))
            .expect("selected object context should resolve");
        assert!(obj_ctx.is_page_block);
        assert!(!obj_ctx.title.is_empty());

        let obj_items = build_selected_object_toolbar_items(&obj_ctx);
        assert!(obj_items
            .iter()
            .any(|i| i.action == TopBarAction::ConfigureSelectedObject));
        assert!(obj_items
            .iter()
            .any(|i| i.action == TopBarAction::CenterSelectedOnCanvas));
        assert!(obj_items
            .iter()
            .any(|i| i.action == TopBarAction::DuplicateSelected));
        assert!(obj_items
            .iter()
            .any(|i| i.action == TopBarAction::DeleteSelected));

        // 5. Headless render across all 7 RibbonTabs
        let ctx = egui::Context::default();
        let status = ConnectionStatus::Connected {
            host: "192.168.1.100".to_string(),
            server_info: "OK".to_string(),
        };
        for tab in [
            RibbonTab::AddDevice,
            RibbonTab::Schematic,
            RibbonTab::Project,
            RibbonTab::Miniserver,
            RibbonTab::View,
            RibbonTab::SelectedObject,
            RibbonTab::LiveView,
        ] {
            set_ribbon_tab(&ctx, tab);
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                let _ = render_top_bar_with_selection(
                    ctx,
                    Some(&proj),
                    Some(&path),
                    0,
                    true,
                    1.0,
                    &status,
                    Some("192.168.1.100"),
                    true,
                    true,
                    true,
                    true,
                    Some(&first_block.uuid),
                );
            });
            assert_eq!(current_ribbon_tab(&ctx), tab);
        }
    }
}
