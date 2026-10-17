use std::collections::HashMap;

use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use loxone_core::block_schema::lookup_block_schema;
use loxone_core::model::{ConnectorPin, LoxoneProject, Page, PageBlock};
use loxone_core::xref::XrefIndex;

use crate::theme::{
    blend_block_header_color, ACCENT_BLUE, BORDER_SUBTLE, CANVAS_BG, CARD_BG, CARD_BORDER,
    GRID_DOT, TEXT_MUTED, TEXT_PRIMARY, WIRE_ANALOG, WIRE_DIGITAL,
};

pub const LOXONE_UNITS_PER_PX: f32 = 8.0;

#[derive(Debug, Clone, PartialEq)]
pub struct BlockDragState {
    pub block_uuid: String,
    pub start_px: f32,
    pub start_py: f32,
    pub current_px: f32,
    pub current_py: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WireDragState {
    pub block_uuid: String,
    pub pin_uuid: String,
    pub pin_key: String,
    pub is_output: bool,
    pub start_screen_pos: Pos2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedWire {
    pub target_block_uuid: String,
    pub target_pin_key: String,
    pub source_pin_uuid: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CanvasContextMenu {
    EmptyCanvas {
        screen_pos: Pos2,
        lx: f32,
        ly: f32,
    },
    Block {
        screen_pos: Pos2,
        page_uuid: String,
        block_uuid: String,
        block_title: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiveValuePopupState {
    pub screen_pos: Pos2,
    pub control_uuid: String,
    pub block_title: String,
    pub badge: String,
    pub control_kind: loxone_core::miniserver::RefBarLiveControlKind,
    pub value_input: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockResizeState {
    pub block_uuid: String,
    pub start_px2: f32,
    pub start_py2: f32,
    pub current_px2: f32,
    pub current_py2: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InlineRenameState {
    pub screen_pos: Pos2,
    pub block_uuid: String,
    pub title_input: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RefBarChromeLayout {
    pub badge: String,
    pub capsule_rect: Rect,
    pub control_kind: loxone_core::miniserver::RefBarLiveControlKind,
    pub control_rect: Option<Rect>,
    pub capsule_left_pad: f32,
    pub capsule_right_pad: f32,
}

#[derive(Debug, Clone, Default)]
pub struct CanvasInteractionState {
    pub dragging_block: Option<BlockDragState>,
    pub dragging_blocks: Vec<BlockDragState>,
    pub resizing_block: Option<BlockResizeState>,
    pub selected_blocks: Vec<String>,
    pub marquee_start: Option<Pos2>,
    pub marquee_current: Option<Pos2>,
    pub marquee_base_selection: Vec<String>,
    pub wiring_from: Option<WireDragState>,
    pub selected_wire: Option<SelectedWire>,
    pub hovered_pin: Option<(String, String)>,
    pub context_menu: Option<CanvasContextMenu>,
    pub live_value_popup: Option<LiveValuePopupState>,
    pub inline_rename: Option<InlineRenameState>,
}

#[derive(Debug, Clone)]
pub struct CanvasViewport {
    pub pan: Vec2,
    pub zoom: f32,
    pub compact_pins: bool,
    pub interaction: CanvasInteractionState,
}

impl Default for CanvasViewport {
    fn default() -> Self {
        Self {
            pan: Vec2::new(40.0, 40.0),
            zoom: 1.0,
            compact_pins: true,
            interaction: CanvasInteractionState::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CanvasAction {
    SelectObject(String),
    ClearSelection,
    NavigateToBlock {
        page_index: usize,
        block_uuid: String,
    },
    MoveBlock {
        page_uuid: String,
        block_uuid: String,
        px: f32,
        py: f32,
    },
    MoveBlocks {
        page_uuid: String,
        moves: Vec<(String, f32, f32)>,
    },
    ResizeBlock {
        page_uuid: String,
        block_uuid: String,
        px2: f32,
        py2: f32,
    },
    RenameBlockTitle {
        block_uuid: String,
        new_title: String,
    },
    CopySelectedBlocks,
    DuplicateSelectedBlocks,
    PasteClipboardAt {
        px: f32,
        py: f32,
    },
    SendOutputToPage {
        source_page_uuid: String,
        source_block_uuid: String,
        source_pin_key: String,
        target_page_uuid: String,
    },
    ConnectPins {
        target_block_uuid: String,
        target_pin_key: String,
        source_pin_uuid: String,
    },
    DisconnectWire {
        target_block_uuid: String,
        target_pin_key: String,
        source_pin_uuid: String,
    },
    DeleteBlock {
        page_uuid: String,
        block_uuid: String,
    },
    DeleteBlocks {
        page_uuid: String,
        block_uuids: Vec<String>,
    },
    OpenInsertModalAt {
        px: f32,
        py: f32,
    },
    TriggerLiveCommand {
        control_uuid: String,
        pin_key: Option<String>,
        command: String,
    },
    OpenObjectConfig(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinSide {
    Input,
    Output,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayPin {
    pub uuid: String,
    pub key: String,
    pub side: PinSide,
    pub is_schema_only: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PinHitTarget {
    pub block_uuid: String,
    pub pin_uuid: String,
    pub pin_key: String,
    pub side: PinSide,
    pub screen_pos: Pos2,
}

pub fn snap_to_loxone_grid(val: f32) -> f32 {
    (val / 64.0).round() * 64.0
}

pub fn cubic_bezier_point(p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2, t: f32) -> Pos2 {
    let u = 1.0 - t;
    let u2 = u * u;
    let u3 = u2 * u;
    let t2 = t * t;
    let t3 = t2 * t;
    let x = u3 * p0.x + 3.0 * u2 * t * p1.x + 3.0 * u * t2 * p2.x + t3 * p3.x;
    let y = u3 * p0.y + 3.0 * u2 * t * p1.y + 3.0 * u * t2 * p2.y + t3 * p3.y;
    Pos2::new(x, y)
}

pub fn cubic_bezier_points(src: Pos2, dst: Pos2) -> [Pos2; 25] {
    let ctrl_dx = ((dst.x - src.x).abs() * 0.5).max(32.0);
    let c1 = Pos2::new(src.x + ctrl_dx, src.y);
    let c2 = Pos2::new(dst.x - ctrl_dx, dst.y);
    let mut pts = [Pos2::ZERO; 25];
    for (i, slot) in pts.iter_mut().enumerate() {
        let t = (i as f32) / 24.0;
        *slot = cubic_bezier_point(src, c1, c2, dst, t);
    }
    pts
}

pub fn point_to_segment_distance(point: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_sq();
    if len_sq < 1e-6 {
        return point.distance(a);
    }
    let t = ((point - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    let proj = a + ab * t;
    point.distance(proj)
}

pub fn point_to_bezier_distance(point: Pos2, points: &[Pos2]) -> f32 {
    if points.is_empty() {
        return f32::INFINITY;
    }
    if points.len() == 1 {
        return point.distance(points[0]);
    }
    points
        .windows(2)
        .map(|seg| point_to_segment_distance(point, seg[0], seg[1]))
        .fold(f32::INFINITY, f32::min)
}

pub fn wire_midpoint(points: &[Pos2]) -> Pos2 {
    if points.is_empty() {
        return Pos2::ZERO;
    }
    if points.len() == 25 {
        return points[12];
    }
    let total_len: f32 = points.windows(2).map(|w| w[0].distance(w[1])).sum();
    let half = total_len * 0.5;
    let mut accum = 0.0;
    for w in points.windows(2) {
        let seg_len = w[0].distance(w[1]);
        if accum + seg_len >= half && seg_len > 1e-4 {
            let t = (half - accum) / seg_len;
            return w[0] + (w[1] - w[0]) * t;
        }
        accum += seg_len;
    }
    points[points.len() / 2]
}

impl CanvasViewport {
    pub fn loxone_to_screen(&self, canvas_origin: Pos2, lx: f32, ly: f32) -> Pos2 {
        let x = canvas_origin.x + self.pan.x + (lx / LOXONE_UNITS_PER_PX) * self.zoom;
        let y = canvas_origin.y + self.pan.y + (ly / LOXONE_UNITS_PER_PX) * self.zoom;
        Pos2::new(x, y)
    }

    pub fn screen_to_loxone(&self, canvas_origin: Pos2, pos: Pos2) -> (f32, f32) {
        let lx = (pos.x - canvas_origin.x - self.pan.x) / self.zoom * LOXONE_UNITS_PER_PX;
        let ly = (pos.y - canvas_origin.y - self.pan.y) / self.zoom * LOXONE_UNITS_PER_PX;
        (lx, ly)
    }

    pub fn loxone_coords_rect_to_screen(
        &self,
        canvas_origin: Pos2,
        px: f32,
        py: f32,
        px2: f32,
        py2: f32,
    ) -> Rect {
        let min = self.loxone_to_screen(canvas_origin, px, py);
        let max_lx = px2.max(px + 512.0);
        let max_ly = py2.max(py + 192.0);
        let max = self.loxone_to_screen(canvas_origin, max_lx, max_ly);
        Rect::from_min_max(min, max)
    }

    pub fn loxone_rect_to_screen(&self, canvas_origin: Pos2, block: &PageBlock) -> Rect {
        self.loxone_coords_rect_to_screen(canvas_origin, block.px, block.py, block.px2, block.py2)
    }

    pub fn fit_to_page(&mut self, page: &Page, view_size: Vec2) {
        if view_size.x <= 10.0 || view_size.y <= 10.0 {
            self.pan = Vec2::new(40.0, 40.0);
            self.zoom = 1.0;
            return;
        }
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut has_positioned = false;
        for b in page.blocks.iter().filter(|b| b.has_position) {
            has_positioned = true;
            min_x = min_x.min(b.px);
            min_y = min_y.min(b.py);
            max_x = max_x.max(b.px2.max(b.px + 512.0));
            max_y = max_y.max(b.py2.max(b.py + 192.0));
        }
        if !has_positioned {
            self.pan = Vec2::new(40.0, 40.0);
            self.zoom = 1.0;
            return;
        }
        let width_px = (max_x - min_x).max(512.0) / LOXONE_UNITS_PER_PX;
        let height_px = (max_y - min_y).max(192.0) / LOXONE_UNITS_PER_PX;
        let margin = 64.0;
        let zx = ((view_size.x - margin) / width_px).clamp(0.85, 2.0);
        let zy = ((view_size.y - margin) / height_px).clamp(0.85, 2.0);
        self.zoom = zx.min(zy);
        self.pan = Vec2::new(
            margin * 0.5 - (min_x / LOXONE_UNITS_PER_PX) * self.zoom,
            margin * 0.5 - (min_y / LOXONE_UNITS_PER_PX) * self.zoom,
        );
    }

    pub fn center_on_block(&mut self, block: &PageBlock, view_size: Vec2) {
        let cx = ((block.px + block.px2.max(block.px + 512.0)) * 0.5) / LOXONE_UNITS_PER_PX;
        let cy = ((block.py + block.py2.max(block.py + 192.0)) * 0.5) / LOXONE_UNITS_PER_PX;
        self.pan = Vec2::new(
            view_size.x * 0.5 - cx * self.zoom,
            view_size.y * 0.5 - cy * self.zoom,
        );
    }
}

pub fn has_visible_outgoing_wires(pin_uuid: &str, xref: &XrefIndex) -> bool {
    xref.outgoing_wires_by_pin_uuid
        .get(pin_uuid)
        .is_some_and(|targets| targets.iter().any(|t| !t.is_hidden))
}

pub fn classify_pin_side(block: &PageBlock, pin: &ConnectorPin, xref: &XrefIndex) -> PinSide {
    if block.is_input_ref_bar() {
        return PinSide::Output;
    }
    if block.is_output_ref_bar() {
        return PinSide::Input;
    }
    if block.block_type == "Memory" {
        if pin.key.eq_ignore_ascii_case("Input")
            || pin.has_visible_incoming_links()
            || pin.default_value.is_some()
        {
            return PinSide::Input;
        }
        return PinSide::Output;
    }
    if matches!(pin.key.as_str(), "API" | "OutputAPI") {
        return PinSide::Output;
    }
    if pin.has_visible_incoming_links() || pin.default_value.is_some() {
        return PinSide::Input;
    }
    if has_visible_outgoing_wires(&pin.uuid, xref) {
        return PinSide::Output;
    }
    if pin.pin_kind == loxone_core::model::PinKind::Output {
        PinSide::Output
    } else {
        PinSide::Input
    }
}

pub fn is_pin_connected(pin: &ConnectorPin, xref: &XrefIndex) -> bool {
    if pin.key.ends_with("Hidden") {
        return false;
    }
    pin.has_visible_incoming_links()
        || pin.default_value.is_some()
        || has_visible_outgoing_wires(&pin.uuid, xref)
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    let max_chars = max_chars.max(4);
    let count = text.chars().count();
    if count <= max_chars {
        text.to_string()
    } else {
        let mut s: String = text.chars().take(max_chars - 1).collect();
        s.push('…');
        s
    }
}

#[cfg(test)]
pub fn segment_hits_rect(a: Pos2, b: Pos2, rect: Rect) -> bool {
    let min_x = a.x.min(b.x);
    let max_x = a.x.max(b.x);
    let min_y = a.y.min(b.y);
    let max_y = a.y.max(b.y);
    if max_x <= rect.min.x || min_x >= rect.max.x || max_y <= rect.min.y || min_y >= rect.max.y {
        return false;
    }
    if (a.y - b.y).abs() < 0.5 {
        a.y > rect.min.y && a.y < rect.max.y && max_x > rect.min.x && min_x < rect.max.x
    } else if (a.x - b.x).abs() < 0.5 {
        a.x > rect.min.x && a.x < rect.max.x && max_y > rect.min.y && min_y < rect.max.y
    } else {
        true
    }
}

#[cfg(test)]
fn path_hits_obstacles(pts: &[Pos2], obstacles: &[Rect]) -> bool {
    pts.windows(2)
        .any(|w| obstacles.iter().any(|&r| segment_hits_rect(w[0], w[1], r)))
}

#[cfg(test)]
pub fn compute_wire_waypoints(
    src: Pos2,
    dst: Pos2,
    src_block_idx: usize,
    dst_block_idx: usize,
    block_rects: &[Rect],
    wire_slot: usize,
    zoom: f32,
) -> Vec<Pos2> {
    let stub = 12.0 * zoom;
    let p1 = Pos2::new(src.x + stub, src.y);
    let p_end_stub = Pos2::new(dst.x - stub, dst.y);

    let pad = 4.0 * zoom;
    let obstacles: Vec<Rect> = block_rects
        .iter()
        .enumerate()
        .filter(|&(idx, _)| idx != src_block_idx && idx != dst_block_idx)
        .map(|(_, r)| r.expand(pad))
        .collect();

    let src_rect = block_rects
        .get(src_block_idx)
        .copied()
        .unwrap_or(Rect::NOTHING)
        .expand(pad);
    let dst_rect = block_rects
        .get(dst_block_idx)
        .copied()
        .unwrap_or(Rect::NOTHING)
        .expand(pad);

    let mut all_obstacles = obstacles.clone();
    if src_rect != Rect::NOTHING {
        all_obstacles.push(src_rect);
    }
    if dst_rect != Rect::NOTHING && dst_block_idx != src_block_idx {
        all_obstacles.push(dst_rect);
    }

    let stagger = ((wire_slot % 7) as f32 - 3.0) * 4.5 * zoom;

    if p1.x <= p_end_stub.x {
        if (src.y - dst.y).abs() < 1.5 * zoom
            && !obstacles.iter().any(|&r| segment_hits_rect(src, dst, r))
        {
            return vec![src, dst];
        }

        let base_mid_x = ((p1.x + p_end_stub.x) * 0.5 + stagger).clamp(p1.x, p_end_stub.x);
        let mut candidate_xs = vec![base_mid_x, (p1.x + p_end_stub.x) * 0.5, p1.x, p_end_stub.x];
        for r in &obstacles {
            for &cx in &[r.min.x - 8.0 * zoom, r.max.x + 8.0 * zoom] {
                if cx >= p1.x && cx <= p_end_stub.x {
                    candidate_xs.push(cx);
                }
            }
        }
        for mid_x in candidate_xs {
            let pts = vec![
                src,
                p1,
                Pos2::new(mid_x, src.y),
                Pos2::new(mid_x, dst.y),
                p_end_stub,
                dst,
            ];
            if !path_hits_obstacles(&pts[1..5], &all_obstacles) {
                return pts;
            }
        }
    }

    let mut candidate_ys = Vec::new();
    let min_x = p1.x.min(p_end_stub.x) - 8.0 * zoom;
    let max_x = p1.x.max(p_end_stub.x) + 8.0 * zoom;
    let mut band_top = src.y.min(dst.y);
    let mut band_bottom = src.y.max(dst.y);
    for r in &all_obstacles {
        if r.max.x >= min_x && r.min.x <= max_x {
            band_top = band_top.min(r.min.y);
            band_bottom = band_bottom.max(r.max.y);
            candidate_ys.push(r.min.y - (8.0 + (wire_slot % 4) as f32 * 4.0) * zoom);
            candidate_ys.push(r.max.y + (8.0 + (wire_slot % 4) as f32 * 4.0) * zoom);
        }
    }
    let clearance = (12.0 + (wire_slot % 5) as f32 * 4.5) * zoom;
    candidate_ys.push(band_top - clearance);
    candidate_ys.push(band_bottom + clearance);

    let avg_y = (src.y + dst.y) * 0.5;
    candidate_ys.sort_by(|a, b| {
        (a - avg_y)
            .abs()
            .partial_cmp(&(b - avg_y).abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for mid_y in &candidate_ys {
        let pts = vec![
            src,
            p1,
            Pos2::new(p1.x, *mid_y),
            Pos2::new(p_end_stub.x, *mid_y),
            p_end_stub,
            dst,
        ];
        if !path_hits_obstacles(&pts[1..5], &all_obstacles) {
            return pts;
        }
    }

    let fallback_y = candidate_ys.first().copied().unwrap_or(band_top - clearance);
    vec![
        src,
        p1,
        Pos2::new(p1.x, fallback_y),
        Pos2::new(p_end_stub.x, fallback_y),
        p_end_stub,
        dst,
    ]
}

pub fn collect_block_display_pins(
    block: &PageBlock,
    xref: &XrefIndex,
    show_all_pins: bool,
) -> (Vec<DisplayPin>, Vec<DisplayPin>) {
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let has_any_wire = block.pins.iter().any(|p| {
        !p.key.ends_with("Hidden")
            && (p.has_visible_incoming_links() || has_visible_outgoing_wires(&p.uuid, xref))
    });
    for pin in &block.pins {
        if pin.key.ends_with("Hidden") {
            continue;
        }
        if !show_all_pins && has_any_wire && !is_pin_connected(pin, xref) {
            continue;
        }
        let side = classify_pin_side(block, pin, xref);
        let dp = DisplayPin {
            uuid: pin.uuid.clone(),
            key: pin.key.clone(),
            side,
            is_schema_only: false,
        };
        match side {
            PinSide::Input => inputs.push(dp),
            PinSide::Output => outputs.push(dp),
        }
    }
    if show_all_pins {
        if let Some(schema) = lookup_block_schema(&block.block_type) {
            for doc in schema.pin_docs {
                if !block.pins.iter().any(|p| p.key == doc.pin_key) {
                    let side = if doc.is_output {
                        PinSide::Output
                    } else {
                        PinSide::Input
                    };
                    let dp = DisplayPin {
                        uuid: format!("{}:{}", block.uuid, doc.pin_key),
                        key: doc.pin_key.to_string(),
                        side,
                        is_schema_only: true,
                    };
                    match side {
                        PinSide::Input => inputs.push(dp),
                        PinSide::Output => outputs.push(dp),
                    }
                }
            }
        }
    }
    (inputs, outputs)
}

const LIVE_ACTIVE_GREEN: Color32 = Color32::from_rgb(0x59, 0xB8, 0x28);
const LIVE_INACTIVE_CAPSULE_BG: Color32 = Color32::from_rgb(0x2D, 0x35, 0x45);
const OFF_CAPSULE_BG: Color32 = Color32::from_rgb(0x1E, 0x25, 0x32);
const OFF_CAPSULE_BORDER: Color32 = Color32::from_rgb(0x3D, 0x49, 0x5E);
const OFF_CAPSULE_SELECTED_BG: Color32 = Color32::from_rgb(0x1D, 0x3E, 0x6E);

pub fn compute_ref_bar_chrome_layout(
    block: &PageBlock,
    rect: Rect,
    zoom: f32,
    live_enabled: bool,
    page: &Page,
    project: &LoxoneProject,
) -> RefBarChromeLayout {
    let strip_w = (4.5 * zoom).clamp(3.0, 7.0);
    let badge = loxone_core::miniserver::ref_bar_capsule_badge(block, page, project);
    let cap_w = ((badge.chars().count() as f32) * 6.5 * zoom + 10.0 * zoom)
        .clamp(20.0 * zoom, 42.0 * zoom);
    let cap_h = (rect.height() - 4.0 * zoom).max(12.0);
    let is_output_side_left =
        block.block_type == "OutputRef" || block.block_type == "OutputRefLM";

    let capsule_rect = if is_output_side_left {
        Rect::from_min_max(
            Pos2::new(
                rect.min.x + strip_w + 1.5 * zoom,
                rect.center().y - cap_h * 0.5,
            ),
            Pos2::new(
                rect.min.x + strip_w + 1.5 * zoom + cap_w,
                rect.center().y + cap_h * 0.5,
            ),
        )
    } else {
        Rect::from_min_max(
            Pos2::new(
                rect.max.x - cap_w - 2.0 * zoom,
                rect.center().y - cap_h * 0.5,
            ),
            Pos2::new(rect.max.x - 2.0 * zoom, rect.center().y + cap_h * 0.5),
        )
    };

    if !live_enabled {
        let (capsule_left_pad, capsule_right_pad) = if is_output_side_left {
            (cap_w + 3.0 * zoom, 0.0)
        } else {
            (0.0, cap_w + 3.0 * zoom)
        };
        return RefBarChromeLayout {
            badge,
            capsule_rect,
            control_kind: loxone_core::miniserver::RefBarLiveControlKind::None,
            control_rect: None,
            capsule_left_pad,
            capsule_right_pad,
        };
    }

    let control_kind = loxone_core::miniserver::classify_ref_bar_live_control(block, &badge, project);
    let (ctrl_w, ctrl_h) = match control_kind {
        loxone_core::miniserver::RefBarLiveControlKind::ToggleSwitch => (
            (24.0 * zoom).clamp(18.0, 32.0),
            (13.0 * zoom).clamp(10.0, 18.0),
        ),
        loxone_core::miniserver::RefBarLiveControlKind::Pushbutton => (
            (15.0 * zoom).clamp(12.0, 21.0),
            (15.0 * zoom).clamp(12.0, 21.0),
        ),
        loxone_core::miniserver::RefBarLiveControlKind::NumericValue
        | loxone_core::miniserver::RefBarLiveControlKind::Touch5Pad => (
            (16.0 * zoom).clamp(12.0, 22.0),
            (15.0 * zoom).clamp(12.0, 21.0),
        ),
        loxone_core::miniserver::RefBarLiveControlKind::None => (0.0, 0.0),
    };

    if ctrl_w <= 0.0 {
        let (capsule_left_pad, capsule_right_pad) = if is_output_side_left {
            (cap_w + 3.0 * zoom, 0.0)
        } else {
            (0.0, cap_w + 3.0 * zoom)
        };
        return RefBarChromeLayout {
            badge,
            capsule_rect,
            control_kind,
            control_rect: None,
            capsule_left_pad,
            capsule_right_pad,
        };
    }

    let (control_rect, capsule_left_pad, capsule_right_pad) = if is_output_side_left {
        let right_x = rect.max.x - 3.0 * zoom;
        let c_rect = Rect::from_min_max(
            Pos2::new(right_x - ctrl_w, rect.center().y - ctrl_h * 0.5),
            Pos2::new(right_x, rect.center().y + ctrl_h * 0.5),
        );
        (c_rect, cap_w + 3.0 * zoom, ctrl_w + 6.0 * zoom)
    } else {
        let right_x = capsule_rect.min.x - 3.0 * zoom;
        let c_rect = Rect::from_min_max(
            Pos2::new(right_x - ctrl_w, rect.center().y - ctrl_h * 0.5),
            Pos2::new(right_x, rect.center().y + ctrl_h * 0.5),
        );
        (c_rect, 0.0, cap_w + ctrl_w + 8.0 * zoom)
    };

    RefBarChromeLayout {
        badge,
        capsule_rect,
        control_kind,
        control_rect: Some(control_rect),
        capsule_left_pad,
        capsule_right_pad,
    }
}

fn paint_ref_bar_live_control(
    painter: &egui::Painter,
    ctrl_rect: Rect,
    kind: loxone_core::miniserver::RefBarLiveControlKind,
    is_active: bool,
    is_hovered: bool,
    zoom: f32,
) {
    use loxone_core::miniserver::RefBarLiveControlKind;
    match kind {
        RefBarLiveControlKind::ToggleSwitch => {
            let r = (ctrl_rect.height() * 0.5).clamp(4.0, 9.0) as u8;
            let track_fill = if is_active {
                LIVE_ACTIVE_GREEN
            } else {
                Color32::from_rgb(0x1B, 0x22, 0x2F)
            };
            let track_stroke = if is_hovered {
                Stroke::new(1.4_f32, ACCENT_BLUE)
            } else if is_active {
                Stroke::new(1.0_f32, Color32::from_rgb(0x76, 0xD2, 0x45))
            } else {
                Stroke::new(1.1_f32, Color32::from_rgb(0x6D, 0x7A, 0x90))
            };
            painter.rect(
                ctrl_rect,
                CornerRadius::same(r),
                track_fill,
                track_stroke,
                egui::StrokeKind::Inside,
            );
            let knob_r = (ctrl_rect.height() * 0.36).max(3.0);
            let knob_x = if is_active {
                ctrl_rect.max.x - ctrl_rect.height() * 0.5
            } else {
                ctrl_rect.min.x + ctrl_rect.height() * 0.5
            };
            painter.circle_filled(
                Pos2::new(knob_x, ctrl_rect.center().y),
                knob_r,
                if is_active {
                    Color32::WHITE
                } else {
                    Color32::from_rgb(0xB8, 0xC4, 0xD6)
                },
            );
        }
        RefBarLiveControlKind::Pushbutton => {
            let center = ctrl_rect.center();
            let outer_r = (ctrl_rect.width().min(ctrl_rect.height()) * 0.46).max(5.0);
            let inner_r = (outer_r * 0.56).max(2.6);
            let ring_color = if is_hovered {
                ACCENT_BLUE
            } else if is_active {
                LIVE_ACTIVE_GREEN
            } else {
                Color32::from_rgb(0x85, 0x92, 0xA8)
            };
            painter.circle_filled(center, outer_r, Color32::from_rgb(0x1B, 0x22, 0x2F));
            painter.circle_stroke(center, outer_r, Stroke::new(1.3_f32, ring_color));
            painter.circle_filled(
                center,
                inner_r,
                if is_active {
                    LIVE_ACTIVE_GREEN
                } else {
                    Color32::from_rgb(0x5B, 0x67, 0x7C)
                },
            );
        }
        RefBarLiveControlKind::NumericValue => {
            let stroke = if is_hovered {
                Stroke::new(1.3_f32, ACCENT_BLUE)
            } else {
                Stroke::new(1.0_f32, Color32::from_rgb(0x47, 0x53, 0x68))
            };
            painter.rect(
                ctrl_rect,
                CornerRadius::same(3),
                Color32::from_rgb(0x28, 0x30, 0x3F),
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.text(
                ctrl_rect.center(),
                Align2::CENTER_CENTER,
                "#",
                FontId::proportional((10.5 * zoom).clamp(8.5, 14.0)),
                Color32::from_rgb(0xE6, 0xEE, 0xFA),
            );
        }
        RefBarLiveControlKind::Touch5Pad => {
            let stroke = if is_hovered {
                Stroke::new(1.3_f32, ACCENT_BLUE)
            } else {
                Stroke::new(1.0_f32, Color32::from_rgb(0x47, 0x53, 0x68))
            };
            painter.rect(
                ctrl_rect,
                CornerRadius::same(3),
                Color32::from_rgb(0x28, 0x30, 0x3F),
                stroke,
                egui::StrokeKind::Inside,
            );
            let c = ctrl_rect.center();
            let dx = (ctrl_rect.width() * 0.24).max(2.2);
            let dy = (ctrl_rect.height() * 0.24).max(2.2);
            let dot_r = (1.25 * zoom).clamp(1.0, 1.8);
            let dot_col = if is_active {
                LIVE_ACTIVE_GREEN
            } else {
                Color32::from_rgb(0xE6, 0xEE, 0xFA)
            };
            for pt in [
                c,
                Pos2::new(c.x - dx, c.y - dy),
                Pos2::new(c.x + dx, c.y - dy),
                Pos2::new(c.x - dx, c.y + dy),
                Pos2::new(c.x + dx, c.y + dy),
            ] {
                painter.circle_filled(pt, dot_r, dot_col);
            }
        }
        RefBarLiveControlKind::None => {}
    }
}

fn extract_numeric_prefix(raw: &str) -> Option<f64> {
    let trimmed = raw.trim();
    let mut end = 0;
    for (idx, ch) in trimmed.char_indices() {
        if ch.is_ascii_digit() || ch == '.' || (idx == 0 && (ch == '-' || ch == '+')) {
            end = idx + ch.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        None
    } else {
        trimmed[..end].parse::<f64>().ok()
    }
}

fn format_clean_float(val: f64) -> String {
    if (val - val.round()).abs() < 1e-6 {
        format!("{}", val.round() as i64)
    } else {
        format!("{:.1}", val)
    }
}

fn paint_inline_wire_value_label(
    painter: &egui::Painter,
    pos: Pos2,
    align: Align2,
    text: &str,
    is_active: bool,
    zoom: f32,
) {
    if text.is_empty() {
        return;
    }
    let font_sz = (10.5 * zoom).clamp(8.5, 14.5);
    let char_w = (5.6 * zoom).clamp(4.4, 8.0);
    let w = (text.chars().count() as f32 * char_w) + 5.0 * zoom;
    let h = (13.0 * zoom).clamp(10.5, 17.5);
    let bg_rect = match align {
        Align2::LEFT_CENTER => Rect::from_min_max(
            Pos2::new(pos.x - 1.5 * zoom, pos.y - h * 0.5),
            Pos2::new(pos.x + w, pos.y + h * 0.5),
        ),
        Align2::RIGHT_CENTER => Rect::from_min_max(
            Pos2::new(pos.x - w, pos.y - h * 0.5),
            Pos2::new(pos.x + 1.5 * zoom, pos.y + h * 0.5),
        ),
        _ => Rect::from_center_size(pos, Vec2::new(w, h)),
    };
    painter.rect_filled(
        bg_rect,
        CornerRadius::same(2),
        CANVAS_BG.gamma_multiply(0.92),
    );
    let text_color = if is_active {
        Color32::from_rgb(0xDF, 0xF7, 0xD2)
    } else {
        Color32::from_rgb(0xB4, 0xBF, 0xD0)
    };
    painter.text(
        pos,
        align,
        text,
        FontId::proportional(font_sz),
        text_color,
    );
}

pub fn render_page_canvas(
    ui: &mut Ui,
    project: &LoxoneProject,
    page_index: usize,
    viewport: &mut CanvasViewport,
    selected_uuid: Option<&str>,
    live_store: Option<&loxone_core::miniserver::LiveValueStore>,
) -> Option<CanvasAction> {
    let page = project.pages.get(page_index)?;
    let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let canvas_rect = response.rect;
    let origin = canvas_rect.min;

    painter.rect_filled(canvas_rect, 0.0, CANVAS_BG);

    // Handle cursor-centered zoom and middle-mouse / scroll pan
    if response.dragged_by(egui::PointerButton::Middle) {
        viewport.pan += response.drag_delta();
    }
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta);
        let zoom_delta = ui.input(|i| i.zoom_delta());
        if (zoom_delta - 1.0).abs() > 0.001 {
            if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let old_zoom = viewport.zoom;
                viewport.zoom = (viewport.zoom * zoom_delta).clamp(0.3, 4.0);
                let rel = hover_pos - origin - viewport.pan;
                viewport.pan -= rel * (viewport.zoom / old_zoom - 1.0);
            }
        } else if scroll.y.abs() > 0.1 && ui.input(|i| i.modifiers.command || i.modifiers.ctrl) {
            if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let factor = (1.0 + scroll.y * 0.003).clamp(0.8, 1.25);
                let old_zoom = viewport.zoom;
                viewport.zoom = (viewport.zoom * factor).clamp(0.3, 4.0);
                let rel = hover_pos - origin - viewport.pan;
                viewport.pan -= rel * (viewport.zoom / old_zoom - 1.0);
            }
        } else if scroll.length_sq() > 0.1 {
            viewport.pan += scroll;
        }
    }

    // Subtle dot grid
    let step = (24.0 * viewport.zoom).max(14.0);
    let dot_r = (1.3 * viewport.zoom).clamp(1.0, 2.0);
    let start_x = origin.x + (viewport.pan.x % step);
    let start_y = origin.y + (viewport.pan.y % step);
    let mut gx = start_x;
    while gx < canvas_rect.max.x {
        let mut gy = start_y;
        while gy < canvas_rect.max.y {
            painter.circle_filled(Pos2::new(gx, gy), dot_r, GRID_DOT);
            gy += step;
        }
        gx += step;
    }

    struct BlockLayout<'a> {
        block: &'a PageBlock,
        rect: Rect,
        header_h: f32,
        row_h: f32,
        inputs: Vec<DisplayPin>,
        outputs: Vec<DisplayPin>,
    }

    let mut action: Option<CanvasAction> = None;
    let mut pin_screen_positions: HashMap<String, (Pos2, usize)> = HashMap::new();
    let pointer_pos = response.interact_pointer_pos();
    let hover_pos = ui.input(|i| i.pointer.hover_pos()).or(pointer_pos);
    let clicked = response.clicked();
    let mut clicked_any_block = false;
    let mut clicked_any_pin_or_wire = false;
    let mut opened_live_popup_this_frame = false;
    let multi_mod = ui.input(|i| i.modifiers.shift || i.modifiers.command || i.modifiers.ctrl);
    let pan_mod = ui.input(|i| i.key_down(egui::Key::Space) || i.modifiers.alt);
    let show_all_pins = !viewport.compact_pins || viewport.interaction.wiring_from.is_some();

    let positioned_blocks: Vec<&PageBlock> =
        page.blocks.iter().filter(|b| b.has_position).collect();

    // Ensure selected_uuid from tree/inspector is reflected in selected_blocks when appropriate
    if let Some(su) = selected_uuid {
        if viewport.interaction.selected_blocks.is_empty()
            && viewport.interaction.marquee_start.is_none()
            && positioned_blocks.iter().any(|b| b.uuid == su)
        {
            viewport.interaction.selected_blocks.push(su.to_string());
        }
    }

    let mut layouts: Vec<BlockLayout> = Vec::with_capacity(positioned_blocks.len());
    let mut pin_hit_targets: Vec<PinHitTarget> = Vec::new();

    // Pass 1: Precompute positioned block rects and pin screen coordinates
    for (block_idx, &block) in positioned_blocks.iter().enumerate() {
        let active_drag = viewport
            .interaction
            .dragging_blocks
            .iter()
            .find(|d| d.block_uuid == block.uuid)
            .or_else(|| {
                viewport
                    .interaction
                    .dragging_block
                    .as_ref()
                    .filter(|d| d.block_uuid == block.uuid)
            });
        let mut rect = if let Some(resize) = viewport
            .interaction
            .resizing_block
            .as_ref()
            .filter(|r| r.block_uuid == block.uuid)
        {
            viewport.loxone_coords_rect_to_screen(
                origin,
                block.px,
                block.py,
                resize.current_px2,
                resize.current_py2,
            )
        } else if let Some(drag) = active_drag {
            let w_lx = (block.px2 - block.px).max(512.0);
            let h_ly = (block.py2 - block.py).max(192.0);
            viewport.loxone_coords_rect_to_screen(
                origin,
                drag.current_px,
                drag.current_py,
                drag.current_px + w_lx,
                drag.current_py + h_ly,
            )
        } else {
            viewport.loxone_rect_to_screen(origin, block)
        };

        if block.is_note() {
            let min_w = (170.0 * viewport.zoom).max(110.0);
            let min_h = (56.0 * viewport.zoom).max(40.0);
            rect.max.x = rect.max.x.max(rect.min.x + min_w);
            rect.max.y = rect.max.y.max(rect.min.y + min_h);
            layouts.push(BlockLayout {
                block,
                rect,
                header_h: 0.0,
                row_h: 0.0,
                inputs: Vec::new(),
                outputs: Vec::new(),
            });
            continue;
        }

        if block.is_ref_bar() {
            let min_ref_w = (170.0 * viewport.zoom).max(115.0);
            let target_ref_h = (26.0 * viewport.zoom).clamp(20.0, 42.0);
            rect.max.x = rect.max.x.max(rect.min.x + min_ref_w);
            rect.max.y = rect.min.y + rect.height().max(target_ref_h);
            for pin in &block.pins {
                let side = classify_pin_side(block, pin, &project.xref);
                let anchor = match side {
                    PinSide::Output => Pos2::new(rect.max.x, rect.center().y),
                    PinSide::Input => Pos2::new(rect.min.x, rect.center().y),
                };
                pin_screen_positions.insert(pin.uuid.clone(), (anchor, block_idx));
                pin_hit_targets.push(PinHitTarget {
                    block_uuid: block.uuid.clone(),
                    pin_uuid: pin.uuid.clone(),
                    pin_key: pin.key.clone(),
                    side,
                    screen_pos: anchor,
                });
            }
            layouts.push(BlockLayout {
                block,
                rect,
                header_h: 0.0,
                row_h: 0.0,
                inputs: Vec::new(),
                outputs: Vec::new(),
            });
            continue;
        }

        let (inputs, outputs) = collect_block_display_pins(block, &project.xref, show_all_pins);

        let min_block_w = (195.0 * viewport.zoom).max(135.0);
        rect.max.x = rect.max.x.max(rect.min.x + min_block_w);

        let header_h = (28.0 * viewport.zoom).clamp(20.0, 44.0);
        let row_h = (19.0 * viewport.zoom).clamp(14.0, 28.0);
        let body_pad = (6.0 * viewport.zoom).max(4.0);
        let max_rows = inputs.len().max(outputs.len()).max(1);
        let needed_h = header_h + body_pad * 2.0 + (max_rows as f32) * row_h;
        if needed_h > rect.height() {
            rect.max.y = rect.min.y + needed_h;
        }

        let body_top = rect.min.y + header_h + body_pad;
        for (i, pin) in inputs.iter().enumerate() {
            let py = body_top + (i as f32 + 0.5) * row_h;
            let anchor = Pos2::new(rect.min.x, py);
            pin_screen_positions.insert(pin.uuid.clone(), (anchor, block_idx));
            pin_hit_targets.push(PinHitTarget {
                block_uuid: block.uuid.clone(),
                pin_uuid: pin.uuid.clone(),
                pin_key: pin.key.clone(),
                side: PinSide::Input,
                screen_pos: anchor,
            });
        }
        for (i, pin) in outputs.iter().enumerate() {
            let py = body_top + (i as f32 + 0.5) * row_h;
            let anchor = Pos2::new(rect.max.x, py);
            pin_screen_positions.insert(pin.uuid.clone(), (anchor, block_idx));
            pin_hit_targets.push(PinHitTarget {
                block_uuid: block.uuid.clone(),
                pin_uuid: pin.uuid.clone(),
                pin_key: pin.key.clone(),
                side: PinSide::Output,
                screen_pos: anchor,
            });
        }

        layouts.push(BlockLayout {
            block,
            rect,
            header_h,
            row_h,
            inputs,
            outputs,
        });
    }

    // Detect closest hovered pin within 12.0 px
    let pin_hit_radius = (12.0 * viewport.zoom).clamp(10.0, 16.0);
    let hovered_pin_target: Option<PinHitTarget> = hover_pos.and_then(|hp| {
        pin_hit_targets
            .iter()
            .filter_map(|p| {
                let d = p.screen_pos.distance(hp);
                (d <= pin_hit_radius).then_some((d, p.clone()))
            })
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(_, p)| p)
    });
    viewport.interaction.hovered_pin = hovered_pin_target
        .as_ref()
        .map(|p| (p.block_uuid.clone(), p.pin_key.clone()));

    // Handle Delete/Backspace or Escape keys for selected wire / active wire drawing / marquee
    let delete_pressed =
        ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace));
    let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));
    if escape_pressed {
        viewport.interaction.wiring_from = None;
        viewport.interaction.context_menu = None;
        viewport.interaction.selected_wire = None;
        viewport.interaction.inline_rename = None;
        viewport.interaction.resizing_block = None;
        viewport.interaction.marquee_start = None;
        viewport.interaction.marquee_current = None;
        if viewport.interaction.selected_blocks.len() > 1 {
            viewport.interaction.selected_blocks.clear();
            action = Some(CanvasAction::ClearSelection);
        }
    }
    if delete_pressed {
        if let Some(sel) = viewport.interaction.selected_wire.take() {
            action = Some(CanvasAction::DisconnectWire {
                target_block_uuid: sel.target_block_uuid,
                target_pin_key: sel.target_pin_key,
                source_pin_uuid: sel.source_pin_uuid,
            });
        }
    }

    // Double-click a block -> open Object Configuration modal (for Alarm / Central / Room Controller blocks)
    // or inline rename popover for standard blocks
    if response.double_clicked() {
        if let Some(pos) = pointer_pos.or(hover_pos) {
            if let Some(hit_layout) = layouts.iter().rev().find(|l| l.rect.contains(pos)) {
                if loxone_core::supports_object_config(hit_layout.block) {
                    viewport.interaction.selected_blocks = vec![hit_layout.block.uuid.clone()];
                    action = Some(CanvasAction::OpenObjectConfig(
                        hit_layout.block.uuid.clone(),
                    ));
                } else {
                    let current_title = if hit_layout.block.title.is_empty() {
                        hit_layout.block.block_type.clone()
                    } else {
                        hit_layout.block.title.clone()
                    };
                    viewport.interaction.inline_rename = Some(InlineRenameState {
                        screen_pos: Pos2::new(hit_layout.rect.min.x, hit_layout.rect.min.y - 4.0),
                        block_uuid: hit_layout.block.uuid.clone(),
                        title_input: current_title,
                    });
                    action = Some(CanvasAction::SelectObject(hit_layout.block.uuid.clone()));
                }
            }
        }
    }

    // Middle-click or Right-drag pans the canvas
    if response.dragged_by(egui::PointerButton::Middle)
        || response.dragged_by(egui::PointerButton::Secondary)
    {
        viewport.pan += response.drag_delta();
    }

    // Handle drag start, active drag, and drag stop
    if response.drag_started_by(egui::PointerButton::Primary) && !pan_mod {
        let press_pos = ui.input(|i| i.pointer.press_origin()).or(hover_pos);
        if let Some(ppos) = press_pos {
            let pin_at_press = pin_hit_targets
                .iter()
                .filter_map(|p| {
                    let d = p.screen_pos.distance(ppos);
                    (d <= pin_hit_radius).then_some((d, p.clone()))
                })
                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(_, p)| p);

            if let Some(pin_hit) = pin_at_press {
                viewport.interaction.wiring_from = Some(WireDragState {
                    block_uuid: pin_hit.block_uuid,
                    pin_uuid: pin_hit.pin_uuid,
                    pin_key: pin_hit.pin_key,
                    is_output: pin_hit.side == PinSide::Output,
                    start_screen_pos: pin_hit.screen_pos,
                });
                viewport.interaction.context_menu = None;
            } else if let Some(hit_layout) = layouts.iter().rev().find(|l| l.rect.contains(ppos)) {
                let pressed_on_live_control = live_store.is_some()
                    && hit_layout.block.is_ref_bar()
                    && compute_ref_bar_chrome_layout(
                        hit_layout.block,
                        hit_layout.rect,
                        viewport.zoom,
                        true,
                        page,
                        project,
                    )
                    .control_rect
                    .is_some_and(|cr| cr.contains(ppos));
                let hit_uuid = hit_layout.block.uuid.clone();
                let handle_sz = (16.0 * viewport.zoom).clamp(12.0, 22.0);
                let resize_corner = Rect::from_min_max(
                    Pos2::new(
                        hit_layout.rect.max.x - handle_sz,
                        hit_layout.rect.max.y - handle_sz,
                    ),
                    hit_layout.rect.max,
                );
                if pressed_on_live_control {
                    viewport.interaction.context_menu = None;
                } else if hit_layout.block.is_note() && resize_corner.contains(ppos) {
                    let start_px2 = hit_layout.block.px2.max(hit_layout.block.px + 512.0);
                    let start_py2 = hit_layout.block.py2.max(hit_layout.block.py + 192.0);
                    viewport.interaction.resizing_block = Some(BlockResizeState {
                        block_uuid: hit_uuid.clone(),
                        start_px2,
                        start_py2,
                        current_px2: start_px2,
                        current_py2: start_py2,
                    });
                    viewport.interaction.selected_blocks = vec![hit_uuid.clone()];
                    viewport.interaction.context_menu = None;
                    action = Some(CanvasAction::SelectObject(hit_uuid));
                } else {
                    let already_selected = viewport.interaction.selected_blocks.contains(&hit_uuid)
                        || selected_uuid == Some(hit_uuid.as_str());
                    if !already_selected {
                        if multi_mod {
                            viewport.interaction.selected_blocks.push(hit_uuid.clone());
                        } else {
                            viewport.interaction.selected_blocks = vec![hit_uuid.clone()];
                        }
                    } else if viewport.interaction.selected_blocks.is_empty() {
                        viewport.interaction.selected_blocks.push(hit_uuid.clone());
                    }

                    let mut drag_group = Vec::new();
                    for &blk in &positioned_blocks {
                        if viewport.interaction.selected_blocks.contains(&blk.uuid) {
                            drag_group.push(BlockDragState {
                                block_uuid: blk.uuid.clone(),
                                start_px: blk.px,
                                start_py: blk.py,
                                current_px: blk.px,
                                current_py: blk.py,
                            });
                        }
                    }
                    viewport.interaction.dragging_block = drag_group.first().cloned();
                    viewport.interaction.dragging_blocks = drag_group;
                    viewport.interaction.context_menu = None;
                    action = Some(CanvasAction::SelectObject(hit_uuid));
                }
            } else {
                // Drag started on empty canvas -> begin marquee selection box
                viewport.interaction.marquee_start = Some(ppos);
                viewport.interaction.marquee_current = Some(ppos);
                viewport.interaction.marquee_base_selection = if multi_mod {
                    viewport.interaction.selected_blocks.clone()
                } else {
                    Vec::new()
                };
                if !multi_mod {
                    viewport.interaction.selected_blocks.clear();
                }
                viewport.interaction.context_menu = None;
            }
        }
    }

    if response.dragged_by(egui::PointerButton::Primary) {
        let delta = response.drag_delta();
        if let Some(resize) = &mut viewport.interaction.resizing_block {
            let dlx = delta.x / viewport.zoom * LOXONE_UNITS_PER_PX;
            let dly = delta.y / viewport.zoom * LOXONE_UNITS_PER_PX;
            resize.current_px2 = (resize.current_px2 + dlx).max(64.0);
            resize.current_py2 = (resize.current_py2 + dly).max(64.0);
        } else if !viewport.interaction.dragging_blocks.is_empty() {
            let dlx = delta.x / viewport.zoom * LOXONE_UNITS_PER_PX;
            let dly = delta.y / viewport.zoom * LOXONE_UNITS_PER_PX;
            for drag in &mut viewport.interaction.dragging_blocks {
                drag.current_px = (drag.current_px + dlx).max(0.0);
                drag.current_py = (drag.current_py + dly).max(0.0);
            }
            viewport.interaction.dragging_block = viewport.interaction.dragging_blocks.first().cloned();
        } else if let Some(drag) = &mut viewport.interaction.dragging_block {
            let dlx = delta.x / viewport.zoom * LOXONE_UNITS_PER_PX;
            let dly = delta.y / viewport.zoom * LOXONE_UNITS_PER_PX;
            drag.current_px = (drag.current_px + dlx).max(0.0);
            drag.current_py = (drag.current_py + dly).max(0.0);
        } else if let Some(start_pos) = viewport.interaction.marquee_start {
            let cur_pos = hover_pos.or(pointer_pos).unwrap_or(start_pos);
            viewport.interaction.marquee_current = Some(cur_pos);
            let sel_rect = Rect::from_two_pos(start_pos, cur_pos);
            let mut next_selected = viewport.interaction.marquee_base_selection.clone();
            for layout in &layouts {
                if sel_rect.intersects(layout.rect)
                    && !next_selected.contains(&layout.block.uuid)
                {
                    next_selected.push(layout.block.uuid.clone());
                }
            }
            viewport.interaction.selected_blocks = next_selected;
        } else if viewport.interaction.wiring_from.is_none() {
            viewport.pan += delta;
        }
    }

    if response.drag_stopped_by(egui::PointerButton::Primary) {
        if let Some(resize) = viewport.interaction.resizing_block.take() {
            let snapped_px2 = snap_to_loxone_grid(resize.current_px2);
            let snapped_py2 = snap_to_loxone_grid(resize.current_py2);
            if (snapped_px2 - resize.start_px2).abs() > 0.5
                || (snapped_py2 - resize.start_py2).abs() > 0.5
            {
                action = Some(CanvasAction::ResizeBlock {
                    page_uuid: page.uuid.clone(),
                    block_uuid: resize.block_uuid,
                    px2: snapped_px2,
                    py2: snapped_py2,
                });
            }
        }
        let mut dragged = std::mem::take(&mut viewport.interaction.dragging_blocks);
        if dragged.is_empty() {
            if let Some(single) = viewport.interaction.dragging_block.take() {
                dragged.push(single);
            }
        } else {
            viewport.interaction.dragging_block = None;
        }
        if !dragged.is_empty() {
            let mut moves = Vec::new();
            for d in dragged {
                let snapped_px = snap_to_loxone_grid(d.current_px).max(0.0);
                let snapped_py = snap_to_loxone_grid(d.current_py).max(0.0);
                if (snapped_px - d.start_px).abs() > 0.5
                    || (snapped_py - d.start_py).abs() > 0.5
                {
                    moves.push((d.block_uuid, snapped_px, snapped_py));
                }
            }
            if moves.len() == 1 {
                let (block_uuid, px, py) = moves.into_iter().next().unwrap();
                action = Some(CanvasAction::MoveBlock {
                    page_uuid: page.uuid.clone(),
                    block_uuid,
                    px,
                    py,
                });
            } else if moves.len() > 1 {
                action = Some(CanvasAction::MoveBlocks {
                    page_uuid: page.uuid.clone(),
                    moves,
                });
            }
        }
        if viewport.interaction.marquee_start.take().is_some() {
            viewport.interaction.marquee_current = None;
            viewport.interaction.marquee_base_selection.clear();
            if let Some(first) = viewport.interaction.selected_blocks.first().cloned() {
                action = Some(CanvasAction::SelectObject(first));
            } else {
                action = Some(CanvasAction::ClearSelection);
            }
        }
        if let Some(wire_drag) = viewport.interaction.wiring_from.take() {
            if let Some(target_pin) = &hovered_pin_target {
                let target_is_output = target_pin.side == PinSide::Output;
                if target_pin.block_uuid != wire_drag.block_uuid
                    && target_is_output != wire_drag.is_output
                {
                    let (target_block_uuid, target_pin_key, source_pin_uuid) =
                        if wire_drag.is_output {
                            (
                                target_pin.block_uuid.clone(),
                                target_pin.pin_key.clone(),
                                wire_drag.pin_uuid.clone(),
                            )
                        } else {
                            (
                                wire_drag.block_uuid.clone(),
                                wire_drag.pin_key.clone(),
                                target_pin.pin_uuid.clone(),
                            )
                        };
                    action = Some(CanvasAction::ConnectPins {
                        target_block_uuid,
                        target_pin_key,
                        source_pin_uuid,
                    });
                }
            }
        }
    }

    // Handle click-to-wire on pins
    if clicked {
        if let Some(pin_hit) = &hovered_pin_target {
            clicked_any_pin_or_wire = true;
            if let Some(wire_drag) = viewport.interaction.wiring_from.take() {
                let target_is_output = pin_hit.side == PinSide::Output;
                if pin_hit.block_uuid != wire_drag.block_uuid
                    && target_is_output != wire_drag.is_output
                {
                    let (target_block_uuid, target_pin_key, source_pin_uuid) =
                        if wire_drag.is_output {
                            (
                                pin_hit.block_uuid.clone(),
                                pin_hit.pin_key.clone(),
                                wire_drag.pin_uuid.clone(),
                            )
                        } else {
                            (
                                wire_drag.block_uuid.clone(),
                                wire_drag.pin_key.clone(),
                                pin_hit.pin_uuid.clone(),
                            )
                        };
                    action = Some(CanvasAction::ConnectPins {
                        target_block_uuid,
                        target_pin_key,
                        source_pin_uuid,
                    });
                }
            } else {
                viewport.interaction.wiring_from = Some(WireDragState {
                    block_uuid: pin_hit.block_uuid.clone(),
                    pin_uuid: pin_hit.pin_uuid.clone(),
                    pin_key: pin_hit.pin_key.clone(),
                    is_output: pin_hit.side == PinSide::Output,
                    start_screen_pos: pin_hit.screen_pos,
                });
            }
        } else if viewport.interaction.wiring_from.is_some() {
            viewport.interaction.wiring_from = None;
        }
    }

    // Pass 2: Draw on-page direct wires BEHIND blocks using cubic Bézier curves + wire hit-testing
    let mut selected_wire_badge: Option<(Pos2, SelectedWire)> = None;
    let mut live_wire_labels: Vec<(Pos2, Align2, String, bool)> = Vec::new();
    let mut connected_source_pins: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    for layout in &layouts {
        let block = layout.block;
        if block.is_input_ref_bar() {
            continue;
        }
        for pin in &block.pins {
            let Some(&(dst_pos, _)) = pin_screen_positions.get(&pin.uuid) else {
                continue;
            };
            for link in &pin.incoming_links {
                if link.is_hidden() || link.is_detached_ref() {
                    continue;
                }
                if let Some(&(src_pos, src_block_idx)) =
                    pin_screen_positions.get(&link.source_pin_uuid)
                {
                    connected_source_pins.insert(link.source_pin_uuid.clone());
                    let candidate_wire = SelectedWire {
                        target_block_uuid: block.uuid.clone(),
                        target_pin_key: pin.key.clone(),
                        source_pin_uuid: link.source_pin_uuid.clone(),
                    };
                    let bezier_pts = cubic_bezier_points(src_pos, dst_pos);
                    let mid_pt = wire_midpoint(&bezier_pts);

                    if clicked && !clicked_any_pin_or_wire {
                        if let Some(pos) = pointer_pos {
                            if viewport.interaction.selected_wire.as_ref() == Some(&candidate_wire)
                                && pos.distance(mid_pt) <= 10.0
                            {
                                clicked_any_pin_or_wire = true;
                                viewport.interaction.selected_wire = None;
                                action = Some(CanvasAction::DisconnectWire {
                                    target_block_uuid: candidate_wire.target_block_uuid.clone(),
                                    target_pin_key: candidate_wire.target_pin_key.clone(),
                                    source_pin_uuid: candidate_wire.source_pin_uuid.clone(),
                                });
                            } else if point_to_bezier_distance(pos, &bezier_pts) <= 7.0
                                && !layouts.iter().any(|l| l.rect.contains(pos))
                            {
                                clicked_any_pin_or_wire = true;
                                viewport.interaction.selected_wire = Some(candidate_wire.clone());
                            }
                        }
                    }

                    let is_wire_selected =
                        viewport.interaction.selected_wire.as_ref() == Some(&candidate_wire);
                    let src_uuid = &positioned_blocks[src_block_idx].uuid;
                    let is_highlighted = is_wire_selected
                        || selected_uuid == Some(block.uuid.as_str())
                        || selected_uuid == Some(src_uuid.as_str())
                        || viewport.interaction.selected_blocks.contains(&block.uuid)
                        || viewport.interaction.selected_blocks.contains(src_uuid);
                    let base_color = if pin.key.starts_with("AI")
                        || pin.key.starts_with("AQ")
                        || project
                            .xref
                            .pins_by_uuid
                            .get(&link.source_pin_uuid)
                            .is_some_and(|m| m.pin_key.starts_with("AQ"))
                    {
                        WIRE_ANALOG
                    } else {
                        WIRE_DIGITAL
                    };
                    let mut live_wire_active = false;
                    if let Some(store) = live_store {
                        let dst_sig = store.resolve_pin_signal(block, pin, page, project);
                        let src_sig = store
                            .resolve_pin_by_uuid(&link.source_pin_uuid, page, project, 0)
                            .unwrap_or_else(|| dst_sig.clone());
                        let (src_label, dst_label) = if block.is_output_ref_bar()
                            && dst_sig.is_active != src_sig.is_active
                        {
                            live_wire_active = dst_sig.is_active;
                            (dst_sig.formatted.clone(), dst_sig.formatted)
                        } else {
                            live_wire_active = src_sig.is_active || dst_sig.is_active;
                            (src_sig.formatted, dst_sig.formatted)
                        };
                        live_wire_labels.push((
                            Pos2::new(src_pos.x + 7.5 * viewport.zoom, src_pos.y),
                            Align2::LEFT_CENTER,
                            src_label,
                            live_wire_active,
                        ));
                        if src_pos.distance(dst_pos) > 74.0 * viewport.zoom {
                            live_wire_labels.push((
                                Pos2::new(dst_pos.x - 7.5 * viewport.zoom, dst_pos.y),
                                Align2::RIGHT_CENTER,
                                dst_label,
                                live_wire_active,
                            ));
                        }
                    }
                    let wire_color = if is_wire_selected {
                        Color32::from_rgb(0xFF, 0x6B, 0x6B)
                    } else if live_wire_active {
                        LIVE_ACTIVE_GREEN
                    } else if is_highlighted {
                        ACCENT_BLUE
                    } else {
                        base_color.gamma_multiply(0.95)
                    };
                    let wire_width = if is_wire_selected {
                        (3.4 * viewport.zoom).clamp(2.4, 5.0)
                    } else if live_wire_active || is_highlighted {
                        (2.6 * viewport.zoom).clamp(1.9, 4.0)
                    } else {
                        (2.1 * viewport.zoom).clamp(1.5, 3.2)
                    };
                    painter.add(egui::Shape::line(
                        bezier_pts.to_vec(),
                        Stroke::new(wire_width, wire_color),
                    ));
                    if is_wire_selected {
                        selected_wire_badge = Some((mid_pt, candidate_wire));
                    }
                }
            }
        }
    }

    // Pass 3: Draw blocks, reference bars, and notes ON TOP of wires
    let pin_label_color = Color32::from_rgb(0xC8, 0xD2, 0xE3);
    for layout in &layouts {
        let block = layout.block;
        let rect = layout.rect;
        let is_selected = selected_uuid == Some(block.uuid.as_str())
            || viewport.interaction.selected_blocks.contains(&block.uuid);
        let (header_fill, accent_color) = blend_block_header_color(block.color_rgb);

        if clicked && !clicked_any_pin_or_wire {
            if let Some(pos) = pointer_pos {
                if rect.contains(pos) {
                    clicked_any_block = true;
                    viewport.interaction.selected_wire = None;
                    if multi_mod {
                        if let Some(idx) = viewport
                            .interaction
                            .selected_blocks
                            .iter()
                            .position(|u| u == &block.uuid)
                        {
                            viewport.interaction.selected_blocks.remove(idx);
                        } else {
                            viewport.interaction.selected_blocks.push(block.uuid.clone());
                        }
                        action = viewport
                            .interaction
                            .selected_blocks
                            .last()
                            .cloned()
                            .map(CanvasAction::SelectObject)
                            .or(Some(CanvasAction::ClearSelection));
                    } else {
                        viewport.interaction.selected_blocks = vec![block.uuid.clone()];
                        action = Some(CanvasAction::SelectObject(block.uuid.clone()));
                    }
                }
            }
        }

        let border_stroke = if is_selected {
            Stroke::new(2.5_f32, ACCENT_BLUE)
        } else {
            Stroke::new(1.5_f32, CARD_BORDER)
        };
        let strip_w = (4.5 * viewport.zoom).clamp(3.0, 7.0);

        if block.is_note() {
            let note_bg = Color32::from_rgb(0x2E, 0x29, 0x1B);
            painter.rect(
                rect,
                CornerRadius::same(6),
                note_bg,
                border_stroke,
                egui::StrokeKind::Inside,
            );
            let note_text = block
                .text_content
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or(&block.title);
            let clipped = painter.with_clip_rect(rect.shrink(5.0 * viewport.zoom));
            clipped.text(
                rect.min + Vec2::new(10.0 * viewport.zoom, 8.0 * viewport.zoom),
                Align2::LEFT_TOP,
                note_text,
                FontId::proportional((13.0 * viewport.zoom).clamp(10.0, 19.0)),
                TEXT_PRIMARY,
            );
            let grip_sz = (12.0 * viewport.zoom).clamp(9.0, 16.0);
            let grip_rect = Rect::from_min_max(
                Pos2::new(rect.max.x - grip_sz, rect.max.y - grip_sz),
                rect.max,
            );
            if hover_pos.is_some_and(|hp| grip_rect.contains(hp)) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
            }
            for step_i in [3.5_f32, 7.0, 10.5] {
                let d = step_i * viewport.zoom;
                painter.line_segment(
                    [
                        Pos2::new(rect.max.x - d, rect.max.y - 2.0),
                        Pos2::new(rect.max.x - 2.0, rect.max.y - d),
                    ],
                    Stroke::new(
                        1.2_f32,
                        if is_selected { ACCENT_BLUE } else { TEXT_MUTED },
                    ),
                );
            }
            continue;
        }

        if block.is_ref_bar() {
            painter.rect(
                rect,
                CornerRadius::same(5),
                header_fill,
                border_stroke,
                egui::StrokeKind::Inside,
            );
            let strip = Rect::from_min_max(
                rect.min,
                Pos2::new((rect.min.x + strip_w).min(rect.max.x), rect.max.y),
            );
            painter.rect_filled(strip, CornerRadius::same(3), accent_color);

            let raw_label = if !block.title.is_empty() {
                block.title.clone()
            } else if let Some(ref_uuid) = &block.ref_uuid {
                project
                    .xref
                    .objects_by_uuid
                    .get(ref_uuid)
                    .map(|o| o.title.clone())
                    .unwrap_or_else(|| block.block_type.clone())
            } else {
                block.block_type.clone()
            };

            let chrome = compute_ref_bar_chrome_layout(
                block,
                rect,
                viewport.zoom,
                live_store.is_some(),
                page,
                project,
            );
            let ref_live_sig =
                live_store.map(|store| store.resolve_ref_bar_signal(block, page, project));
            let ref_sig_active = ref_live_sig.as_ref().is_some_and(|s| s.is_active);

            // Always draw the unit/pin capsule (colored in Live View, neutral in normal view)
            let (cap_fill, cap_stroke, cap_text_color) = if let Some(sig) = &ref_live_sig {
                if sig.is_active {
                    (
                        LIVE_ACTIVE_GREEN,
                        Stroke::new(1.0_f32, Color32::from_rgb(0x76, 0xD2, 0x45)),
                        Color32::WHITE,
                    )
                } else {
                    (
                        LIVE_INACTIVE_CAPSULE_BG,
                        Stroke::new(1.0_f32, Color32::from_rgb(0x45, 0x50, 0x65)),
                        Color32::from_rgb(0xDC, 0xE5, 0xF4),
                    )
                }
            } else if is_selected {
                (
                    OFF_CAPSULE_SELECTED_BG,
                    Stroke::new(1.0_f32, ACCENT_BLUE),
                    Color32::WHITE,
                )
            } else {
                (
                    OFF_CAPSULE_BG,
                    Stroke::new(1.0_f32, OFF_CAPSULE_BORDER),
                    Color32::from_rgb(0xD8, 0xE2, 0xF0),
                )
            };
            painter.rect(
                chrome.capsule_rect,
                CornerRadius::same(4),
                cap_fill,
                cap_stroke,
                egui::StrokeKind::Inside,
            );
            painter.text(
                chrome.capsule_rect.center(),
                Align2::CENTER_CENTER,
                &chrome.badge,
                FontId::proportional((10.5 * viewport.zoom).clamp(8.5, 14.5)),
                cap_text_color,
            );

            // When Live View is ON, draw the interactive on-canvas control next to the capsule
            if let (Some(sig), Some(ctrl_rect)) = (&ref_live_sig, chrome.control_rect) {
                let ctrl_hovered = hover_pos.is_some_and(|hp| ctrl_rect.contains(hp));
                if ctrl_hovered {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                paint_ref_bar_live_control(
                    &painter,
                    ctrl_rect,
                    chrome.control_kind,
                    sig.is_active,
                    ctrl_hovered,
                    viewport.zoom,
                );
                let ctrl_activated = (clicked && pointer_pos.is_some_and(|pos| ctrl_rect.contains(pos)))
                    || (response.drag_stopped_by(egui::PointerButton::Primary)
                        && ui
                            .input(|i| i.pointer.press_origin())
                            .is_some_and(|p| ctrl_rect.contains(p))
                        && pointer_pos.is_some_and(|pos| ctrl_rect.contains(pos)));
                if ctrl_activated {
                    clicked_any_pin_or_wire = true;
                    clicked_any_block = true;
                    viewport.interaction.selected_blocks = vec![block.uuid.clone()];
                    match chrome.control_kind {
                        loxone_core::miniserver::RefBarLiveControlKind::ToggleSwitch
                        | loxone_core::miniserver::RefBarLiveControlKind::Pushbutton => {
                            let next_cmd = if sig.is_active { "Off" } else { "On" };
                            viewport.interaction.live_value_popup = None;
                            action = Some(CanvasAction::TriggerLiveCommand {
                                control_uuid: block.uuid.clone(),
                                pin_key: None,
                                command: next_cmd.to_string(),
                            });
                        }
                        loxone_core::miniserver::RefBarLiveControlKind::NumericValue
                        | loxone_core::miniserver::RefBarLiveControlKind::Touch5Pad => {
                            let initial_val = extract_numeric_prefix(&sig.raw)
                                .map(format_clean_float)
                                .unwrap_or_else(|| sig.raw.clone());
                            opened_live_popup_this_frame = true;
                            viewport.interaction.live_value_popup = Some(LiveValuePopupState {
                                screen_pos: Pos2::new(ctrl_rect.min.x, rect.max.y + 4.0),
                                control_uuid: block.uuid.clone(),
                                block_title: raw_label.clone(),
                                badge: chrome.badge.clone(),
                                control_kind: chrome.control_kind,
                                value_input: initial_val,
                            });
                            action = Some(CanvasAction::SelectObject(block.uuid.clone()));
                        }
                        loxone_core::miniserver::RefBarLiveControlKind::None => {}
                    }
                }
            }

            let ref_tree_type = block
                .ref_uuid
                .as_deref()
                .and_then(|u| project.xref.objects_by_uuid.get(u))
                .map(|o| o.object_type.as_str());
            let block_icon = crate::icons::icon_for_block_type(&block.block_type, ref_tree_type);
            let icon_sz = (15.0 * viewport.zoom).clamp(11.0, 22.0);
            let icon_rect = Rect::from_center_size(
                Pos2::new(
                    rect.min.x
                        + strip_w
                        + chrome.capsule_left_pad
                        + 4.0 * viewport.zoom
                        + icon_sz * 0.5,
                    rect.center().y,
                ),
                Vec2::splat(icon_sz),
            );

            let text_left = icon_rect.max.x + 5.0 * viewport.zoom;
            let max_chars = ((rect.max.x - chrome.capsule_right_pad - text_left - 6.0 * viewport.zoom)
                / (6.8 * viewport.zoom).max(4.5))
            .max(6.0) as usize;
            let label = truncate_text(&raw_label, max_chars);

            let text_clip = painter.with_clip_rect(rect.shrink(2.0));
            crate::icons::paint_icon(&text_clip, icon_rect, block_icon);
            text_clip.text(
                Pos2::new(text_left, rect.center().y),
                Align2::LEFT_CENTER,
                label,
                FontId::proportional((13.0 * viewport.zoom).clamp(10.0, 19.0)),
                TEXT_PRIMARY,
            );

            for pin in &block.pins {
                if let Some(&(anchor, _)) = pin_screen_positions.get(&pin.uuid) {
                    let is_hovered = viewport.interaction.hovered_pin.as_ref()
                        == Some(&(block.uuid.clone(), pin.key.clone()));
                    let r = if is_hovered {
                        (6.5 * viewport.zoom).clamp(5.0, 9.5)
                    } else {
                        (4.5 * viewport.zoom).clamp(3.5, 6.8)
                    };
                    let dot_color = if is_hovered {
                        ACCENT_BLUE
                    } else if ref_sig_active {
                        LIVE_ACTIVE_GREEN
                    } else {
                        WIRE_DIGITAL
                    };
                    painter.circle_filled(anchor, r, dot_color);
                    painter.circle_stroke(anchor, r, Stroke::new(1.2_f32, CANVAS_BG));
                }
            }
            continue;
        }

        // Standard Function Block Card
        let header_h = layout.header_h.min(rect.height());
        let header_rect =
            Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.min.y + header_h));

        painter.rect(
            rect,
            CornerRadius::same(7),
            CARD_BG,
            border_stroke,
            egui::StrokeKind::Inside,
        );
        painter.rect_filled(header_rect, CornerRadius::same(7), header_fill);
        painter.line_segment(
            [
                Pos2::new(header_rect.min.x, header_rect.max.y),
                Pos2::new(header_rect.max.x, header_rect.max.y),
            ],
            Stroke::new(1.0_f32, CARD_BORDER),
        );
        let strip = Rect::from_min_max(
            header_rect.min,
            Pos2::new(
                (header_rect.min.x + strip_w).min(header_rect.max.x),
                header_rect.max.y,
            ),
        );
        painter.rect_filled(strip, CornerRadius::same(3), accent_color);

        let block_icon = crate::icons::icon_for_block_type(&block.block_type, None);
        let icon_sz = (16.0 * viewport.zoom).clamp(12.0, 23.0);
        let icon_rect = Rect::from_center_size(
            Pos2::new(
                header_rect.min.x + strip_w + 5.0 * viewport.zoom + icon_sz * 0.5,
                header_rect.center().y,
            ),
            Vec2::splat(icon_sz),
        );
        let title_left = icon_rect.max.x + 5.0 * viewport.zoom;

        let raw_title = if block.title.is_empty() {
            &block.block_type
        } else {
            &block.title
        };
        let title_max_chars = ((header_rect.max.x - title_left - 8.0 * viewport.zoom)
            / (7.2 * viewport.zoom).max(4.5))
        .max(8.0) as usize;
        let title = truncate_text(raw_title, title_max_chars);
        let header_clip = painter.with_clip_rect(header_rect.shrink(2.0));
        crate::icons::paint_icon(&header_clip, icon_rect, block_icon);
        header_clip.text(
            Pos2::new(title_left, header_rect.center().y),
            Align2::LEFT_CENTER,
            title,
            FontId::proportional((13.5 * viewport.zoom).clamp(10.5, 20.0)),
            TEXT_PRIMARY,
        );

        let body_top = header_rect.max.y + (6.0 * viewport.zoom).max(4.0);
        let row_h = layout.row_h;
        let has_both_sides = !layout.inputs.is_empty() && !layout.outputs.is_empty();
        let avail_label_w = if has_both_sides {
            rect.width() * 0.46 - 10.0 * viewport.zoom
        } else {
            rect.width() - 20.0 * viewport.zoom
        };
        let pin_max_chars =
            (avail_label_w / (6.4 * viewport.zoom).max(4.0)).max(5.0) as usize;
        let pin_font = FontId::monospace((11.5 * viewport.zoom).clamp(9.0, 16.5));

        for (i, pin) in layout.inputs.iter().enumerate() {
            let py = body_top + (i as f32 + 0.5) * row_h;
            let anchor = Pos2::new(rect.min.x, py);
            let is_hovered = viewport.interaction.hovered_pin.as_ref()
                == Some(&(block.uuid.clone(), pin.key.clone()));
            let r = if is_hovered {
                (6.5 * viewport.zoom).clamp(5.0, 9.5)
            } else {
                (4.5 * viewport.zoom).clamp(3.5, 6.8)
            };
            let mut pin_active = false;
            if let Some(store) = live_store {
                if let Some(cp) = block.pins.iter().find(|p| p.key == pin.key) {
                    let sig = store.resolve_pin_signal(block, cp, page, project);
                    pin_active = sig.is_active;
                    if cp.incoming_links.iter().all(|l| l.is_hidden()) && cp.default_value.is_some()
                    {
                        live_wire_labels.push((
                            Pos2::new(anchor.x - 7.5 * viewport.zoom, anchor.y),
                            Align2::RIGHT_CENTER,
                            sig.formatted,
                            sig.is_active,
                        ));
                    }
                }
            }
            let dot_color = if is_hovered {
                ACCENT_BLUE
            } else if pin_active {
                LIVE_ACTIVE_GREEN
            } else {
                WIRE_DIGITAL
            };
            painter.circle_filled(anchor, r, dot_color);
            painter.circle_stroke(anchor, r, Stroke::new(1.2_f32, CANVAS_BG));
            let display_key = if live_store.is_some() {
                loxone_core::miniserver::loxone_short_pin_label(&block.block_type, &pin.key)
            } else {
                pin.key.as_str()
            };
            painter.text(
                Pos2::new(rect.min.x + 8.5 * viewport.zoom, py),
                Align2::LEFT_CENTER,
                truncate_text(display_key, pin_max_chars),
                pin_font.clone(),
                if is_hovered { TEXT_PRIMARY } else { pin_label_color },
            );
        }

        for (i, pin) in layout.outputs.iter().enumerate() {
            let py = body_top + (i as f32 + 0.5) * row_h;
            let anchor = Pos2::new(rect.max.x, py);
            let is_hovered = viewport.interaction.hovered_pin.as_ref()
                == Some(&(block.uuid.clone(), pin.key.clone()));
            let r = if is_hovered {
                (6.5 * viewport.zoom).clamp(5.0, 9.5)
            } else {
                (4.5 * viewport.zoom).clamp(3.5, 6.8)
            };
            let mut pin_active = false;
            if let Some(store) = live_store {
                if let Some(cp) = block.pins.iter().find(|p| p.key == pin.key) {
                    let sig = store.resolve_pin_signal(block, cp, page, project);
                    pin_active = sig.is_active;
                    if !connected_source_pins.contains(&cp.uuid)
                        && (sig.is_active || store.controls.contains_key(&block.uuid))
                    {
                        live_wire_labels.push((
                            Pos2::new(anchor.x + 7.5 * viewport.zoom, anchor.y),
                            Align2::LEFT_CENTER,
                            sig.formatted,
                            sig.is_active,
                        ));
                    }
                }
            }
            let dot_color = if is_hovered {
                ACCENT_BLUE
            } else if pin_active {
                LIVE_ACTIVE_GREEN
            } else {
                WIRE_ANALOG
            };
            painter.circle_filled(anchor, r, dot_color);
            painter.circle_stroke(anchor, r, Stroke::new(1.2_f32, CANVAS_BG));
            let display_key = if live_store.is_some() {
                loxone_core::miniserver::loxone_short_pin_label(&block.block_type, &pin.key)
            } else {
                pin.key.as_str()
            };
            painter.text(
                Pos2::new(rect.max.x - 8.5 * viewport.zoom, py),
                Align2::RIGHT_CENTER,
                truncate_text(display_key, pin_max_chars),
                pin_font.clone(),
                if is_hovered { TEXT_PRIMARY } else { pin_label_color },
            );
        }
    }

    // Pass 4: Draw consolidated clickable pill tags for detached (FLG=2) and cross-page/periphery links
    for layout in &layouts {
        let block = layout.block;
        if block.is_input_ref_bar() {
            continue;
        }
        for pin in &block.pins {
            let Some(&(dst_pos, _)) = pin_screen_positions.get(&pin.uuid) else {
                continue;
            };
            let mut pill_sources = Vec::new();
            for link in &pin.incoming_links {
                if link.is_hidden() {
                    continue;
                }
                let is_on_page_direct = !link.is_detached_ref()
                    && pin_screen_positions.contains_key(&link.source_pin_uuid);
                if !is_on_page_direct {
                    if let Some(src_meta) = project.xref.pins_by_uuid.get(&link.source_pin_uuid) {
                        pill_sources.push(src_meta);
                    }
                }
            }
            if pill_sources.is_empty() {
                continue;
            }

            let first_meta = pill_sources[0];
            let extra_count = pill_sources.len() - 1;
            let owner_short = truncate_text(&first_meta.owner_title, 16);
            let pin_short = truncate_text(&first_meta.pin_key, 6);
            let tag = if extra_count > 0 {
                format!("🔗 {}:{} (+{})", owner_short, pin_short, extra_count)
            } else {
                format!("🔗 {}:{}", owner_short, pin_short)
            };

            let pill_live = live_store
                .and_then(|store| store.resolve_pin_by_uuid(&first_meta.pin_uuid, page, project, 0));
            let pill_active = pill_live.as_ref().is_some_and(|s| s.is_active);
            let cap_w = (20.0 * viewport.zoom).clamp(15.0, 26.0);
            let cap_extra_w = cap_w + 2.0 * viewport.zoom;

            let char_w = (6.8 * viewport.zoom).clamp(5.0, 9.5);
            let pill_w = ((tag.chars().count() as f32) * char_w + 14.0 * viewport.zoom + cap_extra_w)
                .clamp(68.0 * viewport.zoom, 220.0 * viewport.zoom);
            let pill_h = (18.0 * viewport.zoom).clamp(14.0, 26.0);
            let gap = if live_store.is_some() {
                (44.0 * viewport.zoom).max(28.0)
            } else {
                (8.0 * viewport.zoom).max(5.0)
            };
            let pill_rect = Rect::from_min_max(
                Pos2::new(dst_pos.x - pill_w - gap, dst_pos.y - pill_h * 0.5),
                Pos2::new(dst_pos.x - gap, dst_pos.y + pill_h * 0.5),
            );

            painter.line_segment(
                [Pos2::new(pill_rect.max.x, dst_pos.y), dst_pos],
                Stroke::new(
                    if pill_active { 2.2_f32 } else { 1.5_f32 },
                    if pill_active {
                        LIVE_ACTIVE_GREEN
                    } else {
                        CARD_BORDER
                    },
                ),
            );
            painter.rect(
                pill_rect,
                CornerRadius::same(4),
                CARD_BG,
                Stroke::new(1.2_f32, CARD_BORDER),
                egui::StrokeKind::Inside,
            );
            let cap_rect = Rect::from_min_max(
                Pos2::new(pill_rect.max.x - cap_w - 1.5, pill_rect.min.y + 1.5),
                Pos2::new(pill_rect.max.x - 1.5, pill_rect.max.y - 1.5),
            );
            if let Some(sig) = pill_live {
                painter.rect_filled(
                    cap_rect,
                    CornerRadius::same(3),
                    if sig.is_active {
                        LIVE_ACTIVE_GREEN
                    } else {
                        LIVE_INACTIVE_CAPSULE_BG
                    },
                );
                painter.text(
                    cap_rect.center(),
                    Align2::CENTER_CENTER,
                    "->",
                    FontId::proportional((9.5 * viewport.zoom).clamp(8.0, 13.0)),
                    Color32::WHITE,
                );
                live_wire_labels.push((
                    Pos2::new(dst_pos.x - 6.5 * viewport.zoom, dst_pos.y),
                    Align2::RIGHT_CENTER,
                    sig.formatted,
                    sig.is_active,
                ));
            } else {
                painter.rect(
                    cap_rect,
                    CornerRadius::same(3),
                    OFF_CAPSULE_BG,
                    Stroke::new(1.0_f32, OFF_CAPSULE_BORDER),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    cap_rect.center(),
                    Align2::CENTER_CENTER,
                    "->",
                    FontId::proportional((9.5 * viewport.zoom).clamp(8.0, 13.0)),
                    TEXT_PRIMARY,
                );
            }
            let text_rect = Rect::from_min_max(
                pill_rect.min,
                Pos2::new(pill_rect.max.x - cap_extra_w, pill_rect.max.y),
            );
            let pill_clip = painter.with_clip_rect(text_rect.shrink(1.5));
            pill_clip.text(
                text_rect.center(),
                Align2::CENTER_CENTER,
                tag,
                FontId::proportional((10.5 * viewport.zoom).clamp(8.5, 15.0)),
                ACCENT_BLUE,
            );

            if clicked && !clicked_any_pin_or_wire {
                if let Some(pos) = pointer_pos {
                    if pill_rect.contains(pos) {
                        clicked_any_block = true;
                        if let Some(src_page_idx) = first_meta.page_index {
                            action = Some(CanvasAction::NavigateToBlock {
                                page_index: src_page_idx,
                                block_uuid: first_meta.owner_uuid.clone(),
                            });
                        } else {
                            action =
                                Some(CanvasAction::SelectObject(first_meta.owner_uuid.clone()));
                        }
                    }
                }
            }
        }
    }

    // Pass 4b: Draw inline Live View wire value badges on top of wires and pins
    for (pos, align, text, is_active) in live_wire_labels {
        paint_inline_wire_value_label(&painter, pos, align, &text, is_active, viewport.zoom);
    }

    // Pass 5: Draw active wire preview and selected wire delete badge
    if let Some(wire_drag) = &viewport.interaction.wiring_from {
        if let Some(cur) = hover_pos {
            let (src, dst) = if wire_drag.is_output {
                (wire_drag.start_screen_pos, cur)
            } else {
                (cur, wire_drag.start_screen_pos)
            };
            let pts = cubic_bezier_points(src, dst);
            painter.add(egui::Shape::line(
                pts.to_vec(),
                Stroke::new(2.8_f32, ACCENT_BLUE),
            ));
            painter.circle_filled(wire_drag.start_screen_pos, 5.5, ACCENT_BLUE);
        }
    }

    if let Some((mid_pt, _)) = selected_wire_badge {
        painter.circle_filled(mid_pt, 10.5, Color32::from_rgb(0xD9, 0x38, 0x38));
        painter.circle_stroke(mid_pt, 10.5, Stroke::new(1.4_f32, TEXT_PRIMARY));
        painter.text(
            mid_pt,
            Align2::CENTER_CENTER,
            "✕",
            FontId::proportional(11.5),
            TEXT_PRIMARY,
        );
    }

    // Draw active rubber-band marquee selection rectangle
    if let (Some(start), Some(cur)) = (
        viewport.interaction.marquee_start,
        viewport.interaction.marquee_current,
    ) {
        let mrect = Rect::from_two_pos(start, cur);
        painter.rect(
            mrect,
            CornerRadius::same(4),
            ACCENT_BLUE.gamma_multiply(0.16),
            Stroke::new(1.5_f32, ACCENT_BLUE),
            egui::StrokeKind::Inside,
        );
    }

    // Draw floating multi-selection badge when 2+ blocks are marked
    if viewport.interaction.selected_blocks.len() > 1 {
        let count = viewport.interaction.selected_blocks.len();
        let badge_text = format!(
            "✓ {} items selected  ·  Drag any selected item to move together  ·  ⌫ Delete",
            count
        );
        let badge_w = 420.0_f32.min(canvas_rect.width() - 32.0);
        let badge_h = 28.0_f32;
        let badge_rect = Rect::from_center_size(
            Pos2::new(canvas_rect.center().x, canvas_rect.min.y + 22.0),
            Vec2::new(badge_w, badge_h),
        );
        painter.rect(
            badge_rect,
            CornerRadius::same(6),
            CARD_BG.gamma_multiply(0.95),
            Stroke::new(1.5_f32, ACCENT_BLUE),
            egui::StrokeKind::Inside,
        );
        painter.text(
            badge_rect.center(),
            Align2::CENTER_CENTER,
            badge_text,
            FontId::proportional(12.0),
            TEXT_PRIMARY,
        );
    }

    // Handle right-click context menu trigger
    if response.secondary_clicked() {
        if let Some(pos) = pointer_pos.or(hover_pos) {
            if let Some(hit_layout) = layouts.iter().rev().find(|l| l.rect.contains(pos)) {
                let block_title = if hit_layout.block.title.is_empty() {
                    hit_layout.block.block_type.clone()
                } else {
                    hit_layout.block.title.clone()
                };
                viewport.interaction.context_menu = Some(CanvasContextMenu::Block {
                    screen_pos: pos,
                    page_uuid: page.uuid.clone(),
                    block_uuid: hit_layout.block.uuid.clone(),
                    block_title,
                });
            } else {
                let (lx, ly) = viewport.screen_to_loxone(origin, pos);
                let snapped_lx = snap_to_loxone_grid(lx.max(0.0));
                let snapped_ly = snap_to_loxone_grid(ly.max(0.0));
                viewport.interaction.context_menu = Some(CanvasContextMenu::EmptyCanvas {
                    screen_pos: pos,
                    lx: snapped_lx,
                    ly: snapped_ly,
                });
            }
        }
    }

    // Render active right-click context menu overlay
    if let Some(menu) = viewport.interaction.context_menu.clone() {
        let screen_pos = match &menu {
            CanvasContextMenu::EmptyCanvas { screen_pos, .. } => *screen_pos,
            CanvasContextMenu::Block { screen_pos, .. } => *screen_pos,
        };
        let mut close_menu = false;
        let area_resp = egui::Area::new(egui::Id::new("canvas_context_menu_popup"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen_pos)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .fill(CARD_BG)
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .show(ui, |ui| {
                        ui.set_min_width(225.0);
                        match &menu {
                            CanvasContextMenu::EmptyCanvas { lx, ly, .. } => {
                                if ui
                                    .button("➕ Add Function Block / Reference here...")
                                    .clicked()
                                {
                                    action = Some(CanvasAction::OpenInsertModalAt {
                                        px: *lx,
                                        py: *ly,
                                    });
                                    close_menu = true;
                                }
                                if ui.button("📋 Paste Clipboard Here (Cmd+V)").clicked() {
                                    action = Some(CanvasAction::PasteClipboardAt {
                                        px: *lx,
                                        py: *ly,
                                    });
                                    close_menu = true;
                                }
                            }
                            CanvasContextMenu::Block {
                                page_uuid,
                                block_uuid,
                                block_title,
                                screen_pos,
                            } => {
                                ui.label(
                                    egui::RichText::new(block_title)
                                        .strong()
                                        .color(TEXT_PRIMARY),
                                );
                                ui.separator();
                                if page
                                    .blocks
                                    .iter()
                                    .find(|b| &b.uuid == block_uuid)
                                    .is_some_and(loxone_core::supports_object_config)
                                    && ui
                                        .button("⚙ Konfiguracja obiektu… (Double-click)")
                                        .clicked()
                                {
                                    action =
                                        Some(CanvasAction::OpenObjectConfig(block_uuid.clone()));
                                    close_menu = true;
                                }
                                if ui.button("🔍 Inspect Block").clicked() {
                                    action = Some(CanvasAction::SelectObject(block_uuid.clone()));
                                    close_menu = true;
                                }
                                if ui.button("✏ Rename Title…").clicked() {
                                    viewport.interaction.inline_rename = Some(InlineRenameState {
                                        screen_pos: *screen_pos,
                                        block_uuid: block_uuid.clone(),
                                        title_input: block_title.clone(),
                                    });
                                    close_menu = true;
                                }
                                if ui.button("📋 Copy (Cmd+C)").clicked() {
                                    if !viewport.interaction.selected_blocks.contains(block_uuid) {
                                        viewport.interaction.selected_blocks =
                                            vec![block_uuid.clone()];
                                    }
                                    action = Some(CanvasAction::CopySelectedBlocks);
                                    close_menu = true;
                                }
                                if ui.button("⧉ Duplicate (Cmd+D)").clicked() {
                                    if !viewport.interaction.selected_blocks.contains(block_uuid) {
                                        viewport.interaction.selected_blocks =
                                            vec![block_uuid.clone()];
                                    }
                                    action = Some(CanvasAction::DuplicateSelectedBlocks);
                                    close_menu = true;
                                }
                                if project.pages.len() > 1 {
                                    if let Some(blk) =
                                        page.blocks.iter().find(|b| &b.uuid == block_uuid)
                                    {
                                        let (_, out_pins) = collect_block_display_pins(
                                            blk,
                                            &project.xref,
                                            true,
                                        );
                                        if let Some(first_out) = out_pins.first() {
                                            let out_key = first_out.key.clone();
                                            ui.menu_button(
                                                format!("📤 Send Output ({}) to Page…", out_key),
                                                |ui| {
                                                    for target_p in &project.pages {
                                                        if target_p.uuid == *page_uuid {
                                                            continue;
                                                        }
                                                        if ui.button(&target_p.title).clicked() {
                                                            action = Some(
                                                                CanvasAction::SendOutputToPage {
                                                                    source_page_uuid: page_uuid
                                                                        .clone(),
                                                                    source_block_uuid: block_uuid
                                                                        .clone(),
                                                                    source_pin_key: out_key.clone(),
                                                                    target_page_uuid: target_p
                                                                        .uuid
                                                                        .clone(),
                                                                },
                                                            );
                                                            close_menu = true;
                                                            ui.close_menu();
                                                        }
                                                    }
                                                },
                                            );
                                        }
                                    }
                                }
                                ui.separator();
                                let multi_count = viewport.interaction.selected_blocks.len();
                                if multi_count > 1
                                    && viewport.interaction.selected_blocks.contains(block_uuid)
                                {
                                    if ui
                                        .button(format!(
                                            "🗑 Delete {} Selected Blocks",
                                            multi_count
                                        ))
                                        .clicked()
                                    {
                                        let uuids =
                                            std::mem::take(&mut viewport.interaction.selected_blocks);
                                        action = Some(CanvasAction::DeleteBlocks {
                                            page_uuid: page_uuid.clone(),
                                            block_uuids: uuids,
                                        });
                                        close_menu = true;
                                    }
                                } else if ui.button("🗑 Delete Block").clicked() {
                                    action = Some(CanvasAction::DeleteBlock {
                                        page_uuid: page_uuid.clone(),
                                        block_uuid: block_uuid.clone(),
                                    });
                                    close_menu = true;
                                }
                            }
                        }
                    });
            });
        if clicked && !area_resp.response.rect.contains(pointer_pos.unwrap_or_default()) {
            close_menu = true;
        }
        if close_menu {
            viewport.interaction.context_menu = None;
        }
    }

    // Render active inline rename popover
    if let Some(mut rename) = viewport.interaction.inline_rename.clone() {
        let mut close_rename = false;
        let area_resp = egui::Area::new(egui::Id::new("canvas_inline_rename_popup"))
            .order(egui::Order::Foreground)
            .fixed_pos(rename.screen_pos)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .fill(CARD_BG)
                    .stroke(Stroke::new(1.5_f32, ACCENT_BLUE))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("✏ Rename:")
                                    .size(11.5)
                                    .strong()
                                    .color(TEXT_MUTED),
                            );
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut rename.title_input)
                                    .desired_width(185.0),
                            );
                            let enter_pressed =
                                resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            if enter_pressed || ui.small_button("✓ Save").clicked() {
                                let trimmed = rename.title_input.trim().to_string();
                                if !trimmed.is_empty() {
                                    action = Some(CanvasAction::RenameBlockTitle {
                                        block_uuid: rename.block_uuid.clone(),
                                        new_title: trimmed,
                                    });
                                }
                                close_rename = true;
                            }
                            if ui.small_button("✕").clicked() {
                                close_rename = true;
                            }
                        });
                    });
            });
        if clicked
            && !response.double_clicked()
            && !area_resp.response.rect.contains(pointer_pos.unwrap_or_default())
        {
            close_rename = true;
        }
        if close_rename {
            viewport.interaction.inline_rename = None;
        } else {
            viewport.interaction.inline_rename = Some(rename);
        }
    }

    // Render active on-canvas Live View manual value / T5 control popover
    if live_store.is_none() {
        viewport.interaction.live_value_popup = None;
    }
    if let Some(mut popup) = viewport.interaction.live_value_popup.clone() {
        let mut close_popup = false;
        let area_resp = egui::Area::new(egui::Id::new("canvas_live_value_popup"))
            .order(egui::Order::Foreground)
            .fixed_pos(popup.screen_pos)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .fill(CARD_BG)
                    .stroke(Stroke::new(1.5_f32, LIVE_ACTIVE_GREEN))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.set_min_width(225.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&popup.block_title)
                                    .strong()
                                    .color(TEXT_PRIMARY),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.small_button("✕").clicked() {
                                        close_popup = true;
                                    }
                                    ui.label(
                                        egui::RichText::new(format!("[{}]", popup.badge))
                                            .monospace()
                                            .size(11.0)
                                            .color(LIVE_ACTIVE_GREEN),
                                    );
                                },
                            );
                        });
                        ui.separator();

                        if popup.control_kind
                            == loxone_core::miniserver::RefBarLiveControlKind::Touch5Pad
                        {
                            ui.label(
                                egui::RichText::new("T5 Touch Pad Simulation:")
                                    .size(11.0)
                                    .color(TEXT_MUTED),
                            );
                            ui.horizontal_wrapped(|ui| {
                                for (lbl, val) in [
                                    ("● Center", "1/0/0/0"),
                                    ("↖ Top-L", "1/1/0/0"),
                                    ("↗ Top-R", "1/0/1/0"),
                                    ("↙ Bot-L", "1/0/0/1"),
                                    ("↘ Bot-R", "1/0/0/2"),
                                    ("○ Release (0)", "0/0/0/0"),
                                ] {
                                    if ui.small_button(lbl).clicked() {
                                        action = Some(CanvasAction::TriggerLiveCommand {
                                            control_uuid: popup.control_uuid.clone(),
                                            pin_key: None,
                                            command: val.to_string(),
                                        });
                                        close_popup = true;
                                    }
                                }
                            });
                            ui.add_space(4.0);
                        } else {
                            ui.label(
                                egui::RichText::new("Quick adjust / set value:")
                                    .size(11.0)
                                    .color(TEXT_MUTED),
                            );
                            ui.horizontal(|ui| {
                                for delta in [-1.0_f64, -0.5, 0.0, 0.5, 1.0] {
                                    let btn_lbl = if delta == 0.0 {
                                        "0".to_string()
                                    } else if delta > 0.0 {
                                        format!("+{}", delta)
                                    } else {
                                        format!("{}", delta)
                                    };
                                    if ui.small_button(btn_lbl).clicked() {
                                        let next_str = if delta == 0.0 {
                                            "0".to_string()
                                        } else {
                                            let base = popup
                                                .value_input
                                                .trim()
                                                .replace(',', ".")
                                                .parse::<f64>()
                                                .unwrap_or(0.0);
                                            format_clean_float(base + delta)
                                        };
                                        popup.value_input = next_str.clone();
                                        action = Some(CanvasAction::TriggerLiveCommand {
                                            control_uuid: popup.control_uuid.clone(),
                                            pin_key: None,
                                            command: next_str,
                                        });
                                    }
                                }
                            });
                            ui.add_space(4.0);
                        }

                        ui.horizontal(|ui| {
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut popup.value_input)
                                    .desired_width(135.0)
                                    .hint_text("Value (e.g. 21.5)"),
                            );
                            let enter_pressed =
                                resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let apply_clicked = ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("✓ Set")
                                            .strong()
                                            .color(Color32::WHITE),
                                    )
                                    .fill(LIVE_ACTIVE_GREEN),
                                )
                                .clicked();
                            if enter_pressed || apply_clicked {
                                let trimmed = popup.value_input.trim().replace(',', ".");
                                if !trimmed.is_empty() {
                                    action = Some(CanvasAction::TriggerLiveCommand {
                                        control_uuid: popup.control_uuid.clone(),
                                        pin_key: None,
                                        command: trimmed,
                                    });
                                }
                                close_popup = true;
                            }
                        });
                    });
            });
        if clicked
            && !opened_live_popup_this_frame
            && !area_resp.response.rect.contains(pointer_pos.unwrap_or_default())
        {
            close_popup = true;
        }
        if close_popup {
            viewport.interaction.live_value_popup = None;
        } else {
            viewport.interaction.live_value_popup = Some(popup);
        }
    }

    if clicked && !clicked_any_block && !clicked_any_pin_or_wire {
        viewport.interaction.selected_wire = None;
        if !multi_mod {
            viewport.interaction.selected_blocks.clear();
            if action.is_none() {
                action = Some(CanvasAction::ClearSelection);
            }
        }
    }

    action
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use loxone_core::parser::load_loxone_file;

    #[test]
    fn test_canvas_coordinate_transforms_and_fit_to_page() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should parse");
        let page0 = &project.pages[0];

        let mut vp = CanvasViewport::default();
        let origin = Pos2::new(100.0, 50.0);
        let p = vp.loxone_to_screen(origin, 192.0, 192.0);
        // 192 / 8.0 = 24.0 + pan(40,40) + origin(100,50) = (164.0, 114.0)
        assert_eq!(p, Pos2::new(164.0, 114.0));

        vp.fit_to_page(page0, Vec2::new(1200.0, 800.0));
        assert!(vp.zoom >= 0.2 && vp.zoom <= 2.5);
    }

    #[test]
    fn test_pin_side_and_connection_classification() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should parse");
        let page0 = &project.pages[0];

        let input_ref = page0
            .blocks
            .iter()
            .find(|b| b.block_type == "InputRef")
            .unwrap();
        for pin in &input_ref.pins {
            assert_eq!(classify_pin_side(input_ref, pin, &project.xref), PinSide::Output);
        }

        let output_ref = page0
            .blocks
            .iter()
            .find(|b| b.block_type == "OutputRef")
            .unwrap();
        for pin in &output_ref.pins {
            assert_eq!(classify_pin_side(output_ref, pin, &project.xref), PinSide::Input);
        }
    }

    #[test]
    fn test_alarm2_layout_excludes_unpositioned_blocks_and_routes_around_obstacles() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should parse");
        let alarm2 = project
            .pages
            .iter()
            .find(|p| p.title == "Alarm 2")
            .expect("Alarm 2 page should exist");

        let positioned: Vec<&PageBlock> = alarm2.blocks.iter().filter(|b| b.has_position).collect();
        let unpositioned_count = alarm2.blocks.iter().filter(|b| !b.has_position).count();
        assert_eq!(positioned.len(), 18);
        assert_eq!(unpositioned_count, 51);

        let centrala = positioned
            .iter()
            .find(|b| b.title == "Centrala pożarowa")
            .expect("Centrala pożarowa should be positioned");
        let api_pin = centrala
            .pins
            .iter()
            .find(|p| p.key == "OutputAPI")
            .expect("OutputAPI pin should exist");
        assert_eq!(
            classify_pin_side(centrala, api_pin, &project.xref),
            PinSide::Output
        );
        assert!(!is_pin_connected(api_pin, &project.xref));

        // Verify obstacle-avoiding orthogonal wire routing around an intermediate block
        let src_rect = Rect::from_min_max(Pos2::new(100.0, 100.0), Pos2::new(180.0, 220.0));
        let mid_obstacle = Rect::from_min_max(Pos2::new(220.0, 90.0), Pos2::new(320.0, 230.0));
        let dst_rect = Rect::from_min_max(Pos2::new(380.0, 100.0), Pos2::new(460.0, 220.0));
        let rects = vec![src_rect, mid_obstacle, dst_rect];

        let src_pin = Pos2::new(180.0, 150.0);
        let dst_pin = Pos2::new(380.0, 150.0);
        let waypoints = compute_wire_waypoints(src_pin, dst_pin, 0, 2, &rects, 0, 1.0);
        assert!(waypoints.len() >= 2);
        for seg in waypoints.windows(2) {
            assert!(
                !segment_hits_rect(seg[0], seg[1], mid_obstacle),
                "Wire segment {:?} -> {:?} should not cross intermediate block {:?}",
                seg[0],
                seg[1],
                mid_obstacle
            );
        }
    }

    #[test]
    fn test_screen_to_loxone_is_exact_inverse_of_loxone_to_screen() {
        let origin = Pos2::new(80.0, 45.0);
        for &(zoom, pan) in &[
            (1.0, Vec2::new(40.0, 40.0)),
            (1.5, Vec2::new(-120.0, 85.0)),
            (0.65, Vec2::new(210.0, -30.0)),
        ] {
            let mut vp = CanvasViewport::default();
            vp.zoom = zoom;
            vp.pan = pan;

            let screen_pos = vp.loxone_to_screen(origin, 512.0, 256.0);
            let (lx, ly) = vp.screen_to_loxone(origin, screen_pos);
            assert!((lx - 512.0).abs() < 1e-3, "lx={lx} at zoom={zoom}");
            assert!((ly - 256.0).abs() < 1e-3, "ly={ly} at zoom={zoom}");
        }
    }

    #[test]
    fn test_snap_to_loxone_grid_snaps_to_64_unit_multiples() {
        assert_eq!(snap_to_loxone_grid(0.0), 0.0);
        assert_eq!(snap_to_loxone_grid(31.0), 0.0);
        assert_eq!(snap_to_loxone_grid(33.0), 64.0);
        assert_eq!(snap_to_loxone_grid(500.0), 512.0);
    }

    #[test]
    fn test_point_to_cubic_bezier_distance_detects_wire_click() {
        let src = Pos2::new(100.0, 100.0);
        let dst = Pos2::new(300.0, 200.0);
        let pts = cubic_bezier_points(src, dst);
        let mid = pts[12];
        let near = Pos2::new(mid.x + 3.0, mid.y - 4.0);
        let far = Pos2::new(mid.x + 40.0, mid.y - 40.0);

        assert!(point_to_bezier_distance(near, &pts) <= 8.0);
        assert!(point_to_bezier_distance(far, &pts) > 8.0);
    }

    #[test]
    fn test_collect_block_display_pins_includes_unconnected_schema_pins_when_expanded() {
        let block = PageBlock {
            uuid: "blk-switch-1".to_string(),
            block_type: "Switch".to_string(),
            title: "Hallway Switch".to_string(),
            has_position: true,
            px: 128.0,
            py: 128.0,
            px2: 640.0,
            py2: 320.0,
            color_rgb: None,
            ref_uuid: None,
            extra_ref_uuids: Vec::new(),
            room_uuid: None,
            category_uuid: None,
            text_content: None,
            pins: Vec::new(),
            raw_attributes: Vec::new(),
            extra_children: Vec::new(),
        };
        let xref = XrefIndex::default();

        let (compact_in, compact_out) = collect_block_display_pins(&block, &xref, false);
        assert!(compact_in.is_empty());
        assert!(compact_out.is_empty());

        let (expanded_in, expanded_out) = collect_block_display_pins(&block, &xref, true);
        assert!(
            expanded_in.iter().any(|p| p.key == "Tr"),
            "Expanded Switch pins should include schema input Tr"
        );
        assert!(
            expanded_out.iter().any(|p| p.key == "Q"),
            "Expanded Switch pins should include schema output Q"
        );
    }

    #[test]
    fn test_render_page_canvas_with_live_view_store_renders_energized_wires_and_badges() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should parse");
        let page_idx = project
            .pages
            .iter()
            .position(|p| p.title == "Pokój Agatki")
            .unwrap_or(0);

        let mut store = loxone_core::miniserver::LiveValueStore::new();
        store.seed_project_defaults(&project);
        store.set_control_value("1b8378f9-023f-486d-ffff9ec51fc90b6a", "22.9");
        store.set_control_value("1b8378f9-0244-487c-ffff9ec51fc90b6a", "1");

        let mut vp = CanvasViewport::default();
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let _ = render_page_canvas(ui, &project, page_idx, &mut vp, None, Some(&store));
            });
        });
    }

    #[test]
    fn test_ref_bar_chrome_layout_always_includes_unit_capsule_and_adds_live_control_when_live() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should parse");
        let agatka = project
            .pages
            .iter()
            .find(|p| p.title == "Pokój Agatki")
            .expect("Pokój Agatki page should exist");

        let sufit_p = agatka
            .blocks
            .iter()
            .find(|b| {
                b.is_input_ref_bar()
                    && loxone_core::miniserver::ref_bar_capsule_badge(b, agatka, &project) == "P"
            })
            .expect("Sufit P block should exist");
        let zawor = agatka
            .blocks
            .iter()
            .find(|b| {
                b.is_output_ref_bar()
                    && loxone_core::miniserver::ref_bar_capsule_badge(b, agatka, &project) == "VQ"
            })
            .expect("Zawór block should exist");

        let rect = Rect::from_min_max(Pos2::new(100.0, 200.0), Pos2::new(260.0, 224.0));

        // 1. Live OFF: unit capsule is always rendered; interactive live control is None
        let off_right =
            compute_ref_bar_chrome_layout(sufit_p, rect, 1.0, false, agatka, &project);
        assert_eq!(off_right.badge, "P");
        assert!(off_right.capsule_rect.width() >= 18.0);
        assert!(off_right.control_rect.is_none());
        assert!(off_right.capsule_right_pad > 0.0);

        let off_left = compute_ref_bar_chrome_layout(zawor, rect, 1.0, false, agatka, &project);
        assert_eq!(off_left.badge, "VQ");
        assert!(off_left.capsule_rect.width() >= 18.0);
        assert!(off_left.control_rect.is_none());
        assert!(off_left.capsule_left_pad > 0.0);

        // 2. Live ON: both unit capsule and interactive control rect are present and non-overlapping
        let on_right = compute_ref_bar_chrome_layout(sufit_p, rect, 1.0, true, agatka, &project);
        let ctrl_right = on_right
            .control_rect
            .expect("Live ON ToggleSwitch should have a control rect");
        assert!(ctrl_right.max.x <= on_right.capsule_rect.min.x);
        assert!(on_right.capsule_right_pad > off_right.capsule_right_pad);

        let on_left = compute_ref_bar_chrome_layout(zawor, rect, 1.0, true, agatka, &project);
        let ctrl_left = on_left
            .control_rect
            .expect("Live ON NumericValue should have a control rect");
        assert!(ctrl_left.min.x >= on_left.capsule_rect.max.x);
        assert!(on_left.capsule_right_pad > 0.0);

        assert_eq!(extract_numeric_prefix("22.8°C"), Some(22.8));
        assert_eq!(extract_numeric_prefix("48%"), Some(48.0));
        assert_eq!(format_clean_float(22.5), "22.5");
        assert_eq!(format_clean_float(23.0), "23");
    }
}
