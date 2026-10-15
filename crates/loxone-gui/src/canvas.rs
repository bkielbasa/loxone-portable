use std::collections::HashMap;

use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use loxone_core::model::{ConnectorPin, LoxoneProject, Page, PageBlock};
use loxone_core::xref::XrefIndex;

use crate::theme::{
    blend_block_header_color, ACCENT_BLUE, BORDER_SUBTLE, CANVAS_BG, CARD_BG, CARD_BORDER,
    GRID_DOT, TEXT_MUTED, TEXT_PRIMARY, WIRE_ANALOG, WIRE_DIGITAL,
};

pub const LOXONE_UNITS_PER_PX: f32 = 8.0;

#[derive(Debug, Clone)]
pub struct CanvasViewport {
    pub pan: Vec2,
    pub zoom: f32,
    pub compact_pins: bool,
}

impl Default for CanvasViewport {
    fn default() -> Self {
        Self {
            pan: Vec2::new(40.0, 40.0),
            zoom: 1.0,
            compact_pins: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanvasAction {
    SelectObject(String),
    ClearSelection,
    NavigateToBlock { page_index: usize, block_uuid: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinSide {
    Input,
    Output,
}

impl CanvasViewport {
    pub fn loxone_to_screen(&self, canvas_origin: Pos2, lx: f32, ly: f32) -> Pos2 {
        let x = canvas_origin.x + self.pan.x + (lx / LOXONE_UNITS_PER_PX) * self.zoom;
        let y = canvas_origin.y + self.pan.y + (ly / LOXONE_UNITS_PER_PX) * self.zoom;
        Pos2::new(x, y)
    }

    pub fn loxone_rect_to_screen(&self, canvas_origin: Pos2, block: &PageBlock) -> Rect {
        let min = self.loxone_to_screen(canvas_origin, block.px, block.py);
        let max_lx = block.px2.max(block.px + 512.0);
        let max_ly = block.py2.max(block.py + 192.0);
        let max = self.loxone_to_screen(canvas_origin, max_lx, max_ly);
        Rect::from_min_max(min, max)
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
        let zx = ((view_size.x - margin) / width_px).clamp(0.25, 2.0);
        let zy = ((view_size.y - margin) / height_px).clamp(0.25, 2.0);
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

fn path_hits_obstacles(pts: &[Pos2], obstacles: &[Rect]) -> bool {
    pts.windows(2)
        .any(|w| obstacles.iter().any(|&r| segment_hits_rect(w[0], w[1], r)))
}

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

fn draw_rounded_orthogonal_wire(
    painter: &egui::Painter,
    waypoints: &[Pos2],
    stroke: Stroke,
    zoom: f32,
) {
    let mut clean: Vec<Pos2> = Vec::with_capacity(waypoints.len());
    for &p in waypoints {
        if clean.last().is_none_or(|&last| last.distance(p) > 0.5) {
            clean.push(p);
        }
    }
    if clean.len() < 2 {
        return;
    }
    if clean.len() == 2 {
        painter.line_segment([clean[0], clean[1]], stroke);
        return;
    }

    let radius = (5.0 * zoom).clamp(2.5, 8.0);
    let mut poly: Vec<Pos2> = Vec::with_capacity(clean.len() * 3);
    poly.push(clean[0]);

    for i in 1..(clean.len() - 1) {
        let prev = clean[i - 1];
        let curr = clean[i];
        let next = clean[i + 1];
        let v1 = curr - prev;
        let v2 = next - curr;
        let l1 = v1.length();
        let l2 = v2.length();
        if l1 < 1.0 || l2 < 1.0 {
            poly.push(curr);
            continue;
        }
        let d1 = v1 / l1;
        let d2 = v2 / l2;
        if (d1.x * d2.x + d1.y * d2.y).abs() > 0.98 {
            continue;
        }
        let r = radius.min(l1 * 0.45).min(l2 * 0.45);
        let p_in = curr - d1 * r;
        let p_out = curr + d2 * r;
        let p_mid = Pos2::new(
            0.25 * p_in.x + 0.5 * curr.x + 0.25 * p_out.x,
            0.25 * p_in.y + 0.5 * curr.y + 0.25 * p_out.y,
        );
        poly.push(p_in);
        poly.push(p_mid);
        poly.push(p_out);
    }
    poly.push(*clean.last().unwrap());
    painter.add(egui::Shape::line(poly, stroke));
}


pub fn render_page_canvas(
    ui: &mut Ui,
    project: &LoxoneProject,
    page_index: usize,
    viewport: &mut CanvasViewport,
    selected_uuid: Option<&str>,
) -> Option<CanvasAction> {
    let page = project.pages.get(page_index)?;
    let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let canvas_rect = response.rect;
    let origin = canvas_rect.min;

    painter.rect_filled(canvas_rect, 0.0, CANVAS_BG);

    // Handle pan and cursor-centered zoom
    if response.dragged() {
        viewport.pan += response.drag_delta();
    }
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta);
        let zoom_delta = ui.input(|i| i.zoom_delta());
        if (zoom_delta - 1.0).abs() > 0.001 {
            if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let old_zoom = viewport.zoom;
                viewport.zoom = (viewport.zoom * zoom_delta).clamp(0.2, 3.0);
                let rel = hover_pos - origin - viewport.pan;
                viewport.pan -= rel * (viewport.zoom / old_zoom - 1.0);
            }
        } else if scroll.y.abs() > 0.1 && ui.input(|i| i.modifiers.command || i.modifiers.ctrl) {
            if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let factor = (1.0 + scroll.y * 0.003).clamp(0.8, 1.25);
                let old_zoom = viewport.zoom;
                viewport.zoom = (viewport.zoom * factor).clamp(0.2, 3.0);
                let rel = hover_pos - origin - viewport.pan;
                viewport.pan -= rel * (viewport.zoom / old_zoom - 1.0);
            }
        } else if scroll.length_sq() > 0.1 {
            viewport.pan += scroll;
        }
    }

    // Subtle dot grid
    let step = (24.0 * viewport.zoom).max(14.0);
    let start_x = origin.x + (viewport.pan.x % step);
    let start_y = origin.y + (viewport.pan.y % step);
    let mut gx = start_x;
    while gx < canvas_rect.max.x {
        let mut gy = start_y;
        while gy < canvas_rect.max.y {
            painter.circle_filled(Pos2::new(gx, gy), 1.0, GRID_DOT);
            gy += step;
        }
        gx += step;
    }

    struct BlockLayout<'a> {
        block: &'a PageBlock,
        rect: Rect,
        header_h: f32,
        row_h: f32,
        inputs: Vec<&'a ConnectorPin>,
        outputs: Vec<&'a ConnectorPin>,
    }

    let mut action: Option<CanvasAction> = None;
    let mut pin_screen_positions: HashMap<String, (Pos2, usize)> = HashMap::new();
    let pointer_pos = response.interact_pointer_pos();
    let clicked = response.clicked();
    let mut clicked_any_block = false;

    let positioned_blocks: Vec<&PageBlock> =
        page.blocks.iter().filter(|b| b.has_position).collect();
    let mut layouts: Vec<BlockLayout> = Vec::with_capacity(positioned_blocks.len());
    let mut block_rects: Vec<Rect> = Vec::with_capacity(positioned_blocks.len());

    // Pass 1: Precompute positioned block rects and pin screen coordinates
    for (block_idx, &block) in positioned_blocks.iter().enumerate() {
        let mut rect = viewport.loxone_rect_to_screen(origin, block);

        if block.is_note() {
            layouts.push(BlockLayout {
                block,
                rect,
                header_h: 0.0,
                row_h: 0.0,
                inputs: Vec::new(),
                outputs: Vec::new(),
            });
            block_rects.push(rect);
            continue;
        }

        if block.is_ref_bar() {
            let v_pad = (1.5 * viewport.zoom).min(rect.height() * 0.18);
            rect = Rect::from_min_max(
                Pos2::new(rect.min.x, rect.min.y + v_pad),
                Pos2::new(rect.max.x, rect.max.y - v_pad),
            );
            for pin in &block.pins {
                let side = classify_pin_side(block, pin, &project.xref);
                let anchor = match side {
                    PinSide::Output => Pos2::new(rect.max.x, rect.center().y),
                    PinSide::Input => Pos2::new(rect.min.x, rect.center().y),
                };
                pin_screen_positions.insert(pin.uuid.clone(), (anchor, block_idx));
            }
            layouts.push(BlockLayout {
                block,
                rect,
                header_h: 0.0,
                row_h: 0.0,
                inputs: Vec::new(),
                outputs: Vec::new(),
            });
            block_rects.push(rect);
            continue;
        }

        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        for pin in &block.pins {
            if viewport.compact_pins && !is_pin_connected(pin, &project.xref) {
                continue;
            }
            match classify_pin_side(block, pin, &project.xref) {
                PinSide::Input => inputs.push(pin),
                PinSide::Output => outputs.push(pin),
            }
        }

        let header_h = (18.0 * viewport.zoom).clamp(6.0, 26.0);
        let row_h = (11.5 * viewport.zoom).clamp(4.0, 17.0);
        let body_pad = 4.0 * viewport.zoom;
        let max_rows = inputs.len().max(outputs.len());
        let needed_h = header_h + body_pad * 2.0 + (max_rows as f32) * row_h;
        if needed_h > rect.height() {
            rect.max.y = rect.min.y + needed_h;
        }

        let body_top = rect.min.y + header_h + body_pad;
        for (i, pin) in inputs.iter().enumerate() {
            let py = body_top + (i as f32 + 0.5) * row_h;
            let anchor = Pos2::new(rect.min.x, py);
            pin_screen_positions.insert(pin.uuid.clone(), (anchor, block_idx));
        }
        for (i, pin) in outputs.iter().enumerate() {
            let py = body_top + (i as f32 + 0.5) * row_h;
            let anchor = Pos2::new(rect.max.x, py);
            pin_screen_positions.insert(pin.uuid.clone(), (anchor, block_idx));
        }

        layouts.push(BlockLayout {
            block,
            rect,
            header_h,
            row_h,
            inputs,
            outputs,
        });
        block_rects.push(rect);
    }

    // Pass 2: Draw on-page direct wires BEHIND blocks using obstacle-avoiding orthogonal routing
    let mut wire_slot = 0usize;
    for (dst_block_idx, layout) in layouts.iter().enumerate() {
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
                    let is_highlighted = selected_uuid == Some(block.uuid.as_str())
                        || selected_uuid
                            == Some(positioned_blocks[src_block_idx].uuid.as_str());
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
                    let wire_color = if is_highlighted {
                        ACCENT_BLUE
                    } else {
                        base_color.gamma_multiply(0.82)
                    };
                    let wire_width = if is_highlighted {
                        (2.2 * viewport.zoom).clamp(1.3, 3.2)
                    } else {
                        (1.4 * viewport.zoom).clamp(0.9, 2.2)
                    };
                    let waypoints = compute_wire_waypoints(
                        src_pos,
                        dst_pos,
                        src_block_idx,
                        dst_block_idx,
                        &block_rects,
                        wire_slot,
                        viewport.zoom,
                    );
                    draw_rounded_orthogonal_wire(
                        &painter,
                        &waypoints,
                        Stroke::new(wire_width, wire_color),
                        viewport.zoom,
                    );
                    wire_slot += 1;
                }
            }
        }
    }

    // Pass 3: Draw blocks, reference bars, and notes ON TOP of wires
    for layout in &layouts {
        let block = layout.block;
        let rect = layout.rect;
        let is_selected = selected_uuid == Some(block.uuid.as_str());
        let (header_fill, accent_color) = blend_block_header_color(block.color_rgb);

        if clicked {
            if let Some(pos) = pointer_pos {
                if rect.contains(pos) {
                    clicked_any_block = true;
                    action = Some(CanvasAction::SelectObject(block.uuid.clone()));
                }
            }
        }

        let border_stroke = if is_selected {
            Stroke::new(1.5_f32, ACCENT_BLUE)
        } else {
            Stroke::new(1.0_f32, CARD_BORDER)
        };

        if block.is_note() {
            let note_bg = Color32::from_rgb(0x28, 0x24, 0x18);
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
            let clipped = painter.with_clip_rect(rect.shrink(4.0 * viewport.zoom));
            clipped.text(
                rect.min + Vec2::new(8.0 * viewport.zoom, 6.0 * viewport.zoom),
                Align2::LEFT_TOP,
                note_text,
                FontId::proportional((10.5 * viewport.zoom).clamp(7.0, 15.0)),
                TEXT_PRIMARY,
            );
            continue;
        }

        if block.is_ref_bar() {
            painter.rect(
                rect,
                CornerRadius::same(4),
                header_fill,
                border_stroke,
                egui::StrokeKind::Inside,
            );
            let strip = Rect::from_min_max(
                rect.min,
                Pos2::new(
                    (rect.min.x + 3.0 * viewport.zoom).min(rect.max.x),
                    rect.max.y,
                ),
            );
            painter.rect_filled(strip, CornerRadius::same(2), accent_color);

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
            let max_chars = ((rect.width() - 14.0 * viewport.zoom)
                / (5.8 * viewport.zoom).max(3.5))
            .max(6.0) as usize;
            let label = truncate_text(&raw_label, max_chars);

            let text_clip = painter.with_clip_rect(rect.shrink(2.0));
            text_clip.text(
                Pos2::new(rect.min.x + 7.0 * viewport.zoom, rect.center().y),
                Align2::LEFT_CENTER,
                label,
                FontId::proportional((10.0 * viewport.zoom).clamp(6.5, 13.0)),
                TEXT_PRIMARY,
            );

            for pin in &block.pins {
                if let Some(&(anchor, _)) = pin_screen_positions.get(&pin.uuid) {
                    painter.circle_filled(
                        anchor,
                        (2.8 * viewport.zoom).clamp(1.8, 4.5),
                        WIRE_DIGITAL,
                    );
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
            CornerRadius::same(6),
            CARD_BG,
            border_stroke,
            egui::StrokeKind::Inside,
        );
        painter.rect_filled(header_rect, CornerRadius::same(6), header_fill);
        let strip = Rect::from_min_max(
            header_rect.min,
            Pos2::new(
                (header_rect.min.x + 3.0 * viewport.zoom).min(header_rect.max.x),
                header_rect.max.y,
            ),
        );
        painter.rect_filled(strip, CornerRadius::same(2), accent_color);

        let raw_title = if block.title.is_empty() {
            &block.block_type
        } else {
            &block.title
        };
        let title_max_chars = ((header_rect.width() - 14.0 * viewport.zoom)
            / (6.0 * viewport.zoom).max(3.5))
        .max(6.0) as usize;
        let title = truncate_text(raw_title, title_max_chars);
        let header_clip = painter.with_clip_rect(header_rect.shrink(1.5));
        header_clip.text(
            Pos2::new(
                header_rect.min.x + 7.0 * viewport.zoom,
                header_rect.center().y,
            ),
            Align2::LEFT_CENTER,
            title,
            FontId::proportional((10.5 * viewport.zoom).clamp(6.5, 14.0)),
            TEXT_PRIMARY,
        );

        let body_top = header_rect.max.y + 4.0 * viewport.zoom;
        let row_h = layout.row_h;
        let has_both_sides = !layout.inputs.is_empty() && !layout.outputs.is_empty();
        let avail_label_w = if has_both_sides {
            rect.width() * 0.46 - 8.0 * viewport.zoom
        } else {
            rect.width() - 16.0 * viewport.zoom
        };
        let pin_max_chars =
            (avail_label_w / (5.4 * viewport.zoom).max(3.2)).max(4.0) as usize;
        let pin_font = FontId::monospace((8.8 * viewport.zoom).clamp(5.5, 11.5));

        for (i, pin) in layout.inputs.iter().enumerate() {
            let py = body_top + (i as f32 + 0.5) * row_h;
            let anchor = Pos2::new(rect.min.x, py);
            painter.circle_filled(anchor, (2.6 * viewport.zoom).clamp(1.6, 4.0), WIRE_DIGITAL);
            painter.text(
                Pos2::new(rect.min.x + 5.5 * viewport.zoom, py),
                Align2::LEFT_CENTER,
                truncate_text(&pin.key, pin_max_chars),
                pin_font.clone(),
                TEXT_MUTED,
            );
        }

        for (i, pin) in layout.outputs.iter().enumerate() {
            let py = body_top + (i as f32 + 0.5) * row_h;
            let anchor = Pos2::new(rect.max.x, py);
            painter.circle_filled(anchor, (2.6 * viewport.zoom).clamp(1.6, 4.0), WIRE_ANALOG);
            painter.text(
                Pos2::new(rect.max.x - 5.5 * viewport.zoom, py),
                Align2::RIGHT_CENTER,
                truncate_text(&pin.key, pin_max_chars),
                pin_font.clone(),
                TEXT_MUTED,
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
            let owner_short = truncate_text(&first_meta.owner_title, 14);
            let pin_short = truncate_text(&first_meta.pin_key, 6);
            let tag = if extra_count > 0 {
                format!("🔗 {}:{} (+{})", owner_short, pin_short, extra_count)
            } else {
                format!("🔗 {}:{}", owner_short, pin_short)
            };

            let char_w = (5.2 * viewport.zoom).clamp(3.6, 6.8);
            let pill_w = ((tag.chars().count() as f32) * char_w + 10.0 * viewport.zoom)
                .clamp(46.0 * viewport.zoom, 138.0 * viewport.zoom);
            let pill_h = (11.0 * viewport.zoom).clamp(7.0, 15.0);
            let gap = 5.0 * viewport.zoom;
            let pill_rect = Rect::from_min_max(
                Pos2::new(dst_pos.x - pill_w - gap, dst_pos.y - pill_h * 0.5),
                Pos2::new(dst_pos.x - gap, dst_pos.y + pill_h * 0.5),
            );

            painter.line_segment(
                [Pos2::new(pill_rect.max.x, dst_pos.y), dst_pos],
                Stroke::new(1.1_f32, BORDER_SUBTLE),
            );
            painter.rect(
                pill_rect,
                CornerRadius::same(3),
                CARD_BG,
                Stroke::new(1.0_f32, BORDER_SUBTLE),
                egui::StrokeKind::Inside,
            );
            let pill_clip = painter.with_clip_rect(pill_rect.shrink(1.0));
            pill_clip.text(
                pill_rect.center(),
                Align2::CENTER_CENTER,
                tag,
                FontId::proportional((8.2 * viewport.zoom).clamp(5.5, 10.5)),
                ACCENT_BLUE,
            );

            if clicked {
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

    if clicked && !clicked_any_block {
        action = Some(CanvasAction::ClearSelection);
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
}

