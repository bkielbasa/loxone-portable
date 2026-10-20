use egui::{Color32, Context, CornerRadius, Stroke, Visuals};

pub const CANVAS_BG: Color32 = Color32::from_rgb(0x11, 0x11, 0x11);
pub const GRID_DOT: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);
pub const PANEL_BG: Color32 = Color32::from_rgb(0x11, 0x11, 0x11);
pub const CARD_BG: Color32 = Color32::from_rgb(0x17, 0x17, 0x17);
pub const ELEVATED_BG: Color32 = Color32::from_rgb(0x24, 0x24, 0x24);
pub const CARD_BORDER: Color32 = Color32::from_rgb(0x32, 0x32, 0x32);
pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(0x26, 0x26, 0x26);
pub const LOXONE_GREEN: Color32 = Color32::from_rgb(0x69, 0xC3, 0x50);
pub const ACCENT_BLUE: Color32 = Color32::from_rgb(0x69, 0xC3, 0x50);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xF5, 0xF5, 0xF5);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x9A, 0x9A, 0x9A);
pub const WIRE_DIGITAL: Color32 = Color32::from_rgb(0x69, 0xC3, 0x50);
pub const WIRE_ANALOG: Color32 = Color32::from_rgb(0xF5, 0x9E, 0x0B);
pub const STATUS_DISCONNECTED: Color32 = Color32::from_rgb(0x88, 0x88, 0x88);
pub const STATUS_CONNECTING: Color32 = Color32::from_rgb(0xF5, 0x9E, 0x0B);
pub const STATUS_CONNECTED: Color32 = Color32::from_rgb(0x69, 0xC3, 0x50);
pub const STATUS_ERROR: Color32 = Color32::from_rgb(0xE5, 0x53, 0x4B);

// Semantic aliases used across modals
pub const BG_PANEL: Color32 = PANEL_BG;
pub const BG_DARK: Color32 = CARD_BG;
pub const BG_WIDGET: Color32 = ELEVATED_BG;
pub const ACCENT_PRIMARY: Color32 = LOXONE_GREEN;
pub const ACCENT_GREEN_DIM: Color32 = Color32::from_rgb(0x23, 0x3D, 0x1D);
pub const ACCENT_AMBER: Color32 = WIRE_ANALOG;
pub const TEXT_SECONDARY: Color32 = TEXT_MUTED;

pub fn apply_minimalist_theme(ctx: &Context) {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = PANEL_BG;
    visuals.window_fill = CARD_BG;
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.extreme_bg_color = CANVAS_BG;
    visuals.faint_bg_color = CARD_BG;
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(0x69, 0xC3, 0x50, 48);
    visuals.selection.stroke = Stroke::new(1.0_f32, LOXONE_GREEN);
    visuals.widgets.noninteractive.bg_fill = CARD_BG;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.widgets.inactive.bg_fill = CARD_BG;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER_SUBTLE);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
    visuals.widgets.hovered.bg_fill = ELEVATED_BG;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, LOXONE_GREEN);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
    visuals.widgets.active.bg_fill = Color32::from_rgb(0x2D, 0x2D, 0x2D);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, LOXONE_GREEN);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);
    ctx.set_visuals(visuals);
}

/// Blends Loxone block RGB (`Cl`) at 46% with Encore carbon surface (`#181818`) for a vibrant, high-contrast header
/// and returns `(blended_header_fill, full_accent_strip_color)`.
pub fn blend_block_header_color(cl: Option<[u8; 3]>) -> (Color32, Color32) {
    let [r, g, b] = cl.unwrap_or([0x69, 0xC3, 0x50]);
    let is_near_black = r < 15 && g < 15 && b < 15;
    let [ar, ag, ab] = if is_near_black {
        [0x69, 0xC3, 0x50]
    } else {
        [r, g, b]
    };
    let blend = |c: u8, base: u8| -> u8 { ((c as u16 * 46 + base as u16 * 54) / 100) as u8 };
    let header_fill = Color32::from_rgb(blend(ar, 0x18), blend(ag, 0x18), blend(ab, 0x18));
    let accent_strip = Color32::from_rgb(ar, ag, ab);
    (header_fill, accent_strip)
}

/// Draws a full-screen modal backdrop that blocks pointer events from reaching
/// underlying layers and returns `(backdrop_clicked, popup_was_open_before)`.
pub fn draw_modal_backdrop(
    ctx: &Context,
    backdrop_id: &str,
    order: egui::Order,
    alpha: u8,
) -> (bool, bool) {
    let popup_was_open = ctx.memory(|m| m.any_popup_open());
    let screen = ctx.screen_rect();
    let mut clicked = false;
    egui::Area::new(egui::Id::new(backdrop_id))
        .order(order)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            let (_, resp) = ui.allocate_exact_size(screen.size(), egui::Sense::click_and_drag());
            if alpha > 0 {
                ui.painter()
                    .rect_filled(screen, 0.0, Color32::from_black_alpha(alpha));
            }
            if resp.clicked() {
                clicked = true;
            }
        });
    (clicked, popup_was_open)
}

/// Checks whether an open modal window should close because the user clicked outside its bounds.
/// Safely ignores clicks on the frame the modal was first opened and clicks while a popup (e.g. ComboBox) is open.
pub fn should_close_modal_on_outside_click(
    ctx: &Context,
    modal_id: &str,
    backdrop_clicked: bool,
    popup_was_open_before: bool,
    win_rect: Option<egui::Rect>,
) -> bool {
    let id = egui::Id::new(modal_id).with("open_pass_tracker");
    let curr_pass = ctx.cumulative_pass_nr();

    let popup_open_after = ctx.memory(|m| m.any_popup_open());
    let any_popup = popup_was_open_before || popup_open_after;

    let (any_pressed, any_released, any_click, interact_pos) = ctx.input(|i| {
        (
            i.pointer.any_pressed(),
            i.pointer.any_released(),
            i.pointer.any_click(),
            i.pointer.interact_pos(),
        )
    });

    let pos_outside = match (win_rect, interact_pos) {
        (Some(rect), Some(pos)) => !rect.contains(pos),
        _ => false,
    };

    ctx.data_mut(|d| {
        let prev: Option<(u64, bool)> = d.get_temp(id);
        let was_already_open =
            prev.is_some_and(|(last, _)| last == curr_pass || last + 1 == curr_pass);
        let mut pressed_outside = if was_already_open {
            prev.unwrap().1
        } else {
            false
        };

        if was_already_open && !any_popup && any_pressed && pos_outside {
            pressed_outside = true;
        }

        let mut should_close = false;
        if was_already_open && !any_popup {
            if backdrop_clicked {
                should_close = true;
            } else if pressed_outside && (any_click || any_released) && pos_outside {
                should_close = true;
            }
        }

        if any_released || any_popup {
            pressed_outside = false;
        }

        d.insert_temp(id, (curr_pass, pressed_outside));
        should_close
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modal_closes_when_clicking_outside_but_not_inside() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0));
        let modal_rect =
            egui::Rect::from_center_size(egui::pos2(640.0, 400.0), egui::vec2(400.0, 300.0));

        // Frame 1: modal opens (even if a click event is still in the input from double-clicking to open, it should NOT close on the opening frame)
        let mut raw = egui::RawInput {
            screen_rect: Some(screen),
            events: vec![egui::Event::PointerButton {
                pos: egui::pos2(100.0, 100.0),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let mut closed_frame_1 = false;
        let _ = ctx.run(raw.clone(), |ctx| {
            let (backdrop_clicked, popup_was_open) =
                draw_modal_backdrop(ctx, "test_backdrop", egui::Order::Foreground, 150);
            closed_frame_1 = should_close_modal_on_outside_click(
                ctx,
                "test_modal",
                backdrop_clicked,
                popup_was_open,
                Some(modal_rect),
            );
        });
        assert!(!closed_frame_1, "Modal must not close on the frame it opens");

        // Frame 2: click inside the modal (press + release at center) -> must NOT close
        raw.events = vec![
            egui::Event::PointerMoved(egui::pos2(640.0, 400.0)),
            egui::Event::PointerButton {
                pos: egui::pos2(640.0, 400.0),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ];
        let mut closed_inside_press = false;
        let _ = ctx.run(raw.clone(), |ctx| {
            let (backdrop_clicked, popup_was_open) =
                draw_modal_backdrop(ctx, "test_backdrop", egui::Order::Foreground, 150);
            closed_inside_press = should_close_modal_on_outside_click(
                ctx,
                "test_modal",
                backdrop_clicked,
                popup_was_open,
                Some(modal_rect),
            );
        });
        assert!(!closed_inside_press);

        raw.events = vec![egui::Event::PointerButton {
            pos: egui::pos2(640.0, 400.0),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }];
        let mut closed_inside_release = false;
        let _ = ctx.run(raw.clone(), |ctx| {
            let (backdrop_clicked, popup_was_open) =
                draw_modal_backdrop(ctx, "test_backdrop", egui::Order::Foreground, 150);
            closed_inside_release = should_close_modal_on_outside_click(
                ctx,
                "test_modal",
                backdrop_clicked,
                popup_was_open,
                Some(modal_rect),
            );
        });
        assert!(
            !closed_inside_release,
            "Clicking inside the modal must not close it"
        );

        // Frame 3: click outside the modal at (100, 100) -> MUST close
        raw.events = vec![
            egui::Event::PointerMoved(egui::pos2(100.0, 100.0)),
            egui::Event::PointerButton {
                pos: egui::pos2(100.0, 100.0),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ];
        let _ = ctx.run(raw.clone(), |ctx| {
            let (backdrop_clicked, popup_was_open) =
                draw_modal_backdrop(ctx, "test_backdrop", egui::Order::Foreground, 150);
            let _ = should_close_modal_on_outside_click(
                ctx,
                "test_modal",
                backdrop_clicked,
                popup_was_open,
                Some(modal_rect),
            );
        });

        raw.events = vec![egui::Event::PointerButton {
            pos: egui::pos2(100.0, 100.0),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }];
        let mut closed_outside = false;
        let _ = ctx.run(raw, |ctx| {
            let (backdrop_clicked, popup_was_open) =
                draw_modal_backdrop(ctx, "test_backdrop", egui::Order::Foreground, 150);
            closed_outside = should_close_modal_on_outside_click(
                ctx,
                "test_modal",
                backdrop_clicked,
                popup_was_open,
                Some(modal_rect),
            );
        });
        assert!(closed_outside, "Clicking outside the modal must close it");
    }
}

