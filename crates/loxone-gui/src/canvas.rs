use std::collections::HashMap;

use egui::{
    epaint::CubicBezierShape, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, Ui,
    Vec2,
};
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
        if page.blocks.is_empty() || view_size.x <= 10.0 || view_size.y <= 10.0 {
            self.pan = Vec2::new(40.0, 40.0);
            self.zoom = 1.0;
            return;
        }
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for b in &page.blocks {
            min_x = min_x.min(b.px);
            min_y = min_y.min(b.py);
            max_x = max_x.max(b.px2.max(b.px + 512.0));
            max_y = max_y.max(b.py2.max(b.py + 192.0));
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

pub fn classify_pin_side(block: &PageBlock, pin: &ConnectorPin, xref: &XrefIndex) -> PinSide {
    if matches!(block.block_type.as_str(), "InputRef" | "Memory") {
        return PinSide::Output;
    }
    if block.block_type == "OutputRef" {
        return PinSide::Input;
    }
    if !pin.incoming_links.is_empty() || pin.default_value.is_some() {
        return PinSide::Input;
    }
    if xref.outgoing_wires_by_pin_uuid.contains_key(&pin.uuid) {
        return PinSide::Output;
    }
    if pin.pin_kind == loxone_core::model::PinKind::Output {
        PinSide::Output
    } else {
        PinSide::Input
    }
}

pub fn is_pin_connected(pin: &ConnectorPin, xref: &XrefIndex) -> bool {
    !pin.incoming_links.is_empty()
        || pin.default_value.is_some()
        || xref.outgoing_wires_by_pin_uuid.contains_key(&pin.uuid)
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

    let mut action: Option<CanvasAction> = None;
    let mut pin_screen_positions: HashMap<String, Pos2> = HashMap::new();
    let pointer_pos = response.interact_pointer_pos();
    let clicked = response.clicked();
    let mut clicked_any_block = false;

    // Pass 1: Draw blocks and record pin anchor coordinates
    for block in &page.blocks {
        let rect = viewport.loxone_rect_to_screen(origin, block);
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

        // Sticky Note
        if matches!(block.block_type.as_str(), "Note" | "Text") {
            let note_bg = Color32::from_rgb(0x28, 0x24, 0x18);
            painter.rect(rect, CornerRadius::same(6), note_bg, border_stroke, egui::StrokeKind::Inside);
            let note_text = block
                .text_content
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or(&block.title);
            painter.text(
                rect.min + Vec2::new(8.0 * viewport.zoom, 6.0 * viewport.zoom),
                Align2::LEFT_TOP,
                note_text,
                FontId::proportional((11.0 * viewport.zoom).clamp(8.0, 16.0)),
                TEXT_PRIMARY,
            );
            continue;
        }

        // Reference Bar (InputRef, OutputRef, Memory)
        let is_ref_bar = matches!(
            block.block_type.as_str(),
            "InputRef" | "OutputRef" | "Memory"
        );
        if is_ref_bar {
            painter.rect(rect, CornerRadius::same(5), header_fill, border_stroke, egui::StrokeKind::Inside);
            let strip = Rect::from_min_max(
                rect.min,
                Pos2::new((rect.min.x + 3.0 * viewport.zoom).min(rect.max.x), rect.max.y),
            );
            painter.rect_filled(strip, CornerRadius::same(2), accent_color);

            let label = if !block.title.is_empty() {
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

            painter.text(
                Pos2::new(rect.min.x + 8.0 * viewport.zoom, rect.center().y),
                Align2::LEFT_CENTER,
                label,
                FontId::proportional((11.0 * viewport.zoom).clamp(8.0, 14.0)),
                TEXT_PRIMARY,
            );

            for pin in &block.pins {
                let side = classify_pin_side(block, pin, &project.xref);
                let anchor = match side {
                    PinSide::Output => Pos2::new(rect.max.x, rect.center().y),
                    PinSide::Input => Pos2::new(rect.min.x, rect.center().y),
                };
                painter.circle_filled(anchor, (3.2 * viewport.zoom).clamp(2.0, 5.0), WIRE_DIGITAL);
                pin_screen_positions.insert(pin.uuid.clone(), anchor);
            }
            continue;
        }

        // Standard Function Block Card
        let header_h = (22.0 * viewport.zoom).clamp(14.0, 32.0).min(rect.height());
        let header_rect = Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.min.y + header_h));

        painter.rect(rect, CornerRadius::same(6), CARD_BG, border_stroke, egui::StrokeKind::Inside);
        painter.rect_filled(header_rect, CornerRadius::same(6), header_fill);
        let strip = Rect::from_min_max(
            header_rect.min,
            Pos2::new((header_rect.min.x + 3.0 * viewport.zoom).min(header_rect.max.x), header_rect.max.y),
        );
        painter.rect_filled(strip, CornerRadius::same(2), accent_color);

        let title = if block.title.is_empty() {
            &block.block_type
        } else {
            &block.title
        };
        painter.text(
            Pos2::new(header_rect.min.x + 8.0 * viewport.zoom, header_rect.center().y),
            Align2::LEFT_CENTER,
            title,
            FontId::proportional((11.5 * viewport.zoom).clamp(8.0, 15.0)),
            TEXT_PRIMARY,
        );

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

        let body_top = header_rect.max.y + 6.0 * viewport.zoom;
        let row_h = (14.0 * viewport.zoom).clamp(9.0, 20.0);

        for (i, pin) in inputs.iter().enumerate() {
            let py = (body_top + (i as f32 + 0.5) * row_h).min(rect.max.y - 4.0);
            let anchor = Pos2::new(rect.min.x, py);
            painter.circle_filled(anchor, (3.0 * viewport.zoom).clamp(2.0, 4.5), WIRE_DIGITAL);
            painter.text(
                Pos2::new(rect.min.x + 6.0 * viewport.zoom, py),
                Align2::LEFT_CENTER,
                &pin.key,
                FontId::monospace((9.5 * viewport.zoom).clamp(7.0, 12.0)),
                TEXT_MUTED,
            );
            pin_screen_positions.insert(pin.uuid.clone(), anchor);
        }

        for (i, pin) in outputs.iter().enumerate() {
            let py = (body_top + (i as f32 + 0.5) * row_h).min(rect.max.y - 4.0);
            let anchor = Pos2::new(rect.max.x, py);
            painter.circle_filled(anchor, (3.0 * viewport.zoom).clamp(2.0, 4.5), WIRE_ANALOG);
            painter.text(
                Pos2::new(rect.max.x - 6.0 * viewport.zoom, py),
                Align2::RIGHT_CENTER,
                &pin.key,
                FontId::monospace((9.5 * viewport.zoom).clamp(7.0, 12.0)),
                TEXT_MUTED,
            );
            pin_screen_positions.insert(pin.uuid.clone(), anchor);
        }
    }

    // Pass 2: Draw wires (on-page cubic beziers + off-page/periphery clickable pill tags)
    for block in &page.blocks {
        let block_rect = viewport.loxone_rect_to_screen(origin, block);
        for (pin_idx, pin) in block.pins.iter().enumerate() {
            if pin.incoming_links.is_empty() {
                continue;
            }
            let dst_pos = pin_screen_positions
                .get(&pin.uuid)
                .copied()
                .unwrap_or_else(|| {
                    Pos2::new(
                        block_rect.min.x,
                        block_rect.min.y + 24.0 + (pin_idx as f32) * 12.0,
                    )
                });

            for link in &pin.incoming_links {
                if let Some(&src_pos) = pin_screen_positions.get(&link.source_pin_uuid) {
                    let dx = ((dst_pos.x - src_pos.x).abs() * 0.45).max(28.0 * viewport.zoom);
                    let bezier = CubicBezierShape::from_points_stroke(
                        [
                            src_pos,
                            Pos2::new(src_pos.x + dx, src_pos.y),
                            Pos2::new(dst_pos.x - dx, dst_pos.y),
                            dst_pos,
                        ],
                        false,
                        Color32::TRANSPARENT,
                        Stroke::new((1.6 * viewport.zoom).clamp(1.0, 2.5), WIRE_DIGITAL),
                    );
                    painter.add(bezier);
                } else if let Some(src_meta) = project.xref.pins_by_uuid.get(&link.source_pin_uuid) {
                    // Off-page or direct periphery link pill tag
                    let pill_w = (96.0 * viewport.zoom).clamp(60.0, 140.0);
                    let pill_h = (14.0 * viewport.zoom).clamp(10.0, 18.0);
                    let pill_rect = Rect::from_min_max(
                        Pos2::new(dst_pos.x - pill_w - 6.0, dst_pos.y - pill_h * 0.5),
                        Pos2::new(dst_pos.x - 6.0, dst_pos.y + pill_h * 0.5),
                    );
                    painter.line_segment(
                        [Pos2::new(pill_rect.max.x, dst_pos.y), dst_pos],
                        Stroke::new(1.2_f32, BORDER_SUBTLE),
                    );
                    painter.rect(
                        pill_rect,
                        CornerRadius::same(4),
                        CARD_BG,
                        Stroke::new(1.0_f32, BORDER_SUBTLE),
                        egui::StrokeKind::Inside,
                    );
                    let tag = format!("🔗 {}:{}", src_meta.owner_title, src_meta.pin_key);
                    painter.text(
                        pill_rect.center(),
                        Align2::CENTER_CENTER,
                        tag,
                        FontId::proportional((9.0 * viewport.zoom).clamp(7.0, 11.0)),
                        ACCENT_BLUE,
                    );
                    if clicked {
                        if let Some(pos) = pointer_pos {
                            if pill_rect.contains(pos) {
                                clicked_any_block = true;
                                if let Some(src_page_idx) = src_meta.page_index {
                                    action = Some(CanvasAction::NavigateToBlock {
                                        page_index: src_page_idx,
                                        block_uuid: src_meta.owner_uuid.clone(),
                                    });
                                } else {
                                    action = Some(CanvasAction::SelectObject(
                                        src_meta.owner_uuid.clone(),
                                    ));
                                }
                            }
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
}
