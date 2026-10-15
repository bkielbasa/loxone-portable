use egui::{Color32, Context, CornerRadius, Stroke, Visuals};

pub const CANVAS_BG: Color32 = Color32::from_rgb(0x11, 0x13, 0x17);
pub const GRID_DOT: Color32 = Color32::from_rgb(0x1E, 0x22, 0x2B);
pub const PANEL_BG: Color32 = Color32::from_rgb(0x16, 0x19, 0x20);
pub const CARD_BG: Color32 = Color32::from_rgb(0x1B, 0x1F, 0x2A);
pub const CARD_BORDER: Color32 = Color32::from_rgb(0x2C, 0x33, 0x44);
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(0x24, 0x29, 0x36);
pub const ACCENT_BLUE: Color32 = Color32::from_rgb(0x5E, 0x81, 0xFF);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xE6, 0xEA, 0xF2);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x8B, 0x95, 0xA7);
pub const WIRE_DIGITAL: Color32 = Color32::from_rgb(0x38, 0xBD, 0xF8);
pub const WIRE_ANALOG: Color32 = Color32::from_rgb(0xF5, 0x9E, 0x0B);

pub fn apply_minimalist_theme(ctx: &Context) {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = PANEL_BG;
    visuals.window_fill = PANEL_BG;
    visuals.extreme_bg_color = CANVAS_BG;
    visuals.faint_bg_color = CARD_BG;
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(0x5E, 0x81, 0xFF, 60);
    visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT_BLUE);
    visuals.widgets.noninteractive.bg_fill = CARD_BG;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.widgets.inactive.bg_fill = CARD_BG;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x24, 0x2A, 0x38);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT_BLUE);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
    visuals.widgets.active.bg_fill = Color32::from_rgb(0x2A, 0x32, 0x45);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);
    ctx.set_visuals(visuals);
}

/// Blends Loxone block RGB (`Cl`) at 35% with dark surface (`#1B1F2A`) for a clean, desaturated header
/// and returns `(blended_header_fill, full_accent_strip_color)`.
pub fn blend_block_header_color(cl: Option<[u8; 3]>) -> (Color32, Color32) {
    let [r, g, b] = cl.unwrap_or([0x4B, 0x55, 0x63]);
    let is_near_black = r < 15 && g < 15 && b < 15;
    let [ar, ag, ab] = if is_near_black {
        [0x5E, 0x81, 0xFF]
    } else {
        [r, g, b]
    };
    let blend = |c: u8, base: u8| -> u8 { ((c as u16 * 35 + base as u16 * 65) / 100) as u8 };
    let header_fill = Color32::from_rgb(blend(ar, 0x1B), blend(ag, 0x1F), blend(ab, 0x2A));
    let accent_strip = Color32::from_rgb(ar, ag, ab);
    (header_fill, accent_strip)
}
