use std::path::{Path, PathBuf};

use egui::{Context, Key, RichText, Stroke};
use loxone_core::model::LoxoneProject;
use loxone_core::parser::load_loxone_file;

use crate::canvas::{render_page_canvas, CanvasAction, CanvasViewport};
use crate::command_palette::{CommandPaletteState, PaletteAction};
use crate::inspector::{render_right_inspector, InspectorAction};
use crate::theme::{
    apply_minimalist_theme, ACCENT_BLUE, BORDER_SUBTLE, CARD_BG, PANEL_BG, TEXT_MUTED, TEXT_PRIMARY,
};
use crate::tree_panel::{render_left_explorer, LeftTab, TreeAction};

pub struct LoxonePortableApp {
    pub project: Option<LoxoneProject>,
    pub loaded_path: Option<PathBuf>,
    pub error_banner: Option<String>,
    pub active_page_index: usize,
    pub selected_uuid: Option<String>,
    pub left_tab: LeftTab,
    pub tree_filter: String,
    pub show_left_panel: bool,
    pub show_right_panel: bool,
    pub viewport: CanvasViewport,
    pub palette: CommandPaletteState,
    pub needs_initial_fit: bool,
    pub pending_center_block_uuid: Option<String>,
}

impl Default for LoxonePortableApp {
    fn default() -> Self {
        Self {
            project: None,
            loaded_path: None,
            error_banner: None,
            active_page_index: 0,
            selected_uuid: None,
            left_tab: LeftTab::Pages,
            tree_filter: String::new(),
            show_left_panel: true,
            show_right_panel: true,
            viewport: CanvasViewport::default(),
            palette: CommandPaletteState::default(),
            needs_initial_fit: true,
            pending_center_block_uuid: None,
        }
    }
}

impl LoxonePortableApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial_path: Option<PathBuf>) -> Self {
        apply_minimalist_theme(&cc.egui_ctx);
        let mut app = Self::default();
        if let Some(path) = initial_path {
            app.load_file(&path);
        }
        app
    }

    pub fn load_file(&mut self, path: &Path) {
        match load_loxone_file(path) {
            Ok(proj) => {
                self.project = Some(proj);
                self.loaded_path = Some(path.to_path_buf());
                self.error_banner = None;
                self.active_page_index = 0;
                self.selected_uuid = None;
                self.needs_initial_fit = true;
            }
            Err(err) => {
                self.error_banner = Some(format!("Failed to load {}: {}", path.display(), err));
            }
        }
    }

    pub fn execute_palette_action(&mut self, action: PaletteAction) {
        match action {
            PaletteAction::OpenFile => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Loxone Project", &["Loxone", "loxone", "xml"])
                    .pick_file()
                {
                    self.load_file(&path);
                }
            }
            PaletteAction::FitPage => {
                self.needs_initial_fit = true;
            }
            PaletteAction::ResetZoom => {
                self.viewport.zoom = 1.0;
                self.viewport.pan = egui::Vec2::new(40.0, 40.0);
            }
            PaletteAction::ToggleCompactPins => {
                self.viewport.compact_pins = !self.viewport.compact_pins;
            }
            PaletteAction::ToggleLeftPanel => {
                self.show_left_panel = !self.show_left_panel;
            }
            PaletteAction::ToggleRightPanel => {
                self.show_right_panel = !self.show_right_panel;
            }
            PaletteAction::NavigateToPage(idx) => {
                self.active_page_index = idx;
                self.needs_initial_fit = true;
            }
            PaletteAction::NavigateToBlock {
                page_index,
                block_uuid,
            } => {
                self.active_page_index = page_index;
                self.selected_uuid = Some(block_uuid.clone());
                self.pending_center_block_uuid = Some(block_uuid);
                self.show_right_panel = true;
            }
            PaletteAction::SelectObject(uuid) => {
                self.selected_uuid = Some(uuid);
                self.show_right_panel = true;
            }
        }
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        let cmd_or_ctrl = ctx.input(|i| i.modifiers.command || i.modifiers.ctrl);

        if cmd_or_ctrl && ctx.input(|i| i.key_pressed(Key::K)) {
            if self.palette.is_open {
                self.palette.close();
            } else {
                self.palette.open();
            }
        }
        if self.palette.is_open {
            return;
        }

        if cmd_or_ctrl && ctx.input(|i| i.key_pressed(Key::B)) {
            self.show_left_panel = !self.show_left_panel;
        }
        if cmd_or_ctrl && ctx.input(|i| i.key_pressed(Key::I)) {
            self.show_right_panel = !self.show_right_panel;
        }
        if cmd_or_ctrl && ctx.input(|i| i.key_pressed(Key::O)) {
            self.execute_palette_action(PaletteAction::OpenFile);
        }

        if !ctx.wants_keyboard_input() {
            if ctx.input(|i| i.key_pressed(Key::F)) {
                self.needs_initial_fit = true;
            }
            if ctx.input(|i| i.key_pressed(Key::P)) {
                self.viewport.compact_pins = !self.viewport.compact_pins;
            }
            if ctx.input(|i| i.key_pressed(Key::Num0)) {
                self.execute_palette_action(PaletteAction::ResetZoom);
            }
        }
    }
}

impl eframe::App for LoxonePortableApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);

        // Top Minimalist Header Bar
        egui::TopBottomPanel::top("top_header")
            .exact_height(38.0)
            .frame(
                egui::Frame::new()
                    .fill(PANEL_BG)
                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                    .inner_margin(egui::Margin::symmetric(12, 6)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new("☰").color(TEXT_MUTED))
                        .on_hover_text("Toggle Explorer (Ctrl+B)")
                        .clicked()
                    {
                        self.show_left_panel = !self.show_left_panel;
                    }

                    let title_str = self
                        .project
                        .as_ref()
                        .map(|p| {
                            let page_name = p
                                .pages
                                .get(self.active_page_index)
                                .map(|pg| pg.title.as_str())
                                .unwrap_or("No Page");
                            format!("{}  /  {}", p.metadata.title, page_name)
                        })
                        .unwrap_or_else(|| "Loxone Portable".to_string());
                    let title_resp = ui.label(RichText::new(title_str).color(TEXT_PRIMARY).strong());
                    if let Some(loaded) = &self.loaded_path {
                        title_resp.on_hover_text(loaded.display().to_string());
                    }

                    ui.add_space(16.0);
                    let pill = egui::Frame::new()
                        .fill(CARD_BG)
                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                        .corner_radius(6)
                        .inner_margin(egui::Margin::symmetric(12, 3))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("🔍  Search or jump to page, block, device…   ⌘K / Ctrl+K")
                                    .size(11.5)
                                    .color(TEXT_MUTED),
                            );
                        })
                        .response
                        .interact(egui::Sense::click());
                    if pill.clicked() {
                        self.palette.open();
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(RichText::new("ℹ").color(TEXT_MUTED))
                            .on_hover_text("Toggle Inspector (Ctrl+I)")
                            .clicked()
                        {
                            self.show_right_panel = !self.show_right_panel;
                        }
                        if ui
                            .button(RichText::new("Fit (F)").size(11.5).color(TEXT_PRIMARY))
                            .clicked()
                        {
                            self.needs_initial_fit = true;
                        }
                        let pin_mode_label = if self.viewport.compact_pins {
                            "Pins: Connected (P)"
                        } else {
                            "Pins: All (P)"
                        };
                        if ui
                            .button(RichText::new(pin_mode_label).size(11.5).color(ACCENT_BLUE))
                            .clicked()
                        {
                            self.viewport.compact_pins = !self.viewport.compact_pins;
                        }
                        ui.label(
                            RichText::new(format!("{:.0}%", self.viewport.zoom * 100.0))
                                .monospace()
                                .size(11.5)
                                .color(TEXT_MUTED),
                        );
                    });
                });
            });

        if let Some(err) = &self.error_banner {
            egui::TopBottomPanel::top("error_banner").show(ctx, |ui| {
                ui.colored_label(egui::Color32::LIGHT_RED, err);
            });
        }

        // Left Explorer Panel
        if self.show_left_panel {
            if let Some(proj) = &self.project {
                let mut tree_act = None;
                egui::SidePanel::left("left_explorer")
                    .default_width(250.0)
                    .frame(
                        egui::Frame::new()
                            .fill(PANEL_BG)
                            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                            .inner_margin(egui::Margin::symmetric(10, 6)),
                    )
                    .show(ctx, |ui| {
                        tree_act = render_left_explorer(
                            ui,
                            proj,
                            &mut self.left_tab,
                            &mut self.tree_filter,
                            self.active_page_index,
                            self.selected_uuid.as_deref(),
                        );
                    });

                if let Some(act) = tree_act {
                    match act {
                        TreeAction::SelectPage(idx) => {
                            self.active_page_index = idx;
                            self.needs_initial_fit = true;
                        }
                        TreeAction::SelectObject(uuid) => {
                            self.selected_uuid = Some(uuid);
                            self.show_right_panel = true;
                        }
                        TreeAction::NavigateToBlock {
                            page_index,
                            block_uuid,
                        } => {
                            self.execute_palette_action(PaletteAction::NavigateToBlock {
                                page_index,
                                block_uuid,
                            });
                        }
                    }
                }
            }
        }

        // Right Inspector Panel
        if self.show_right_panel {
            if let Some(proj) = &self.project {
                let mut insp_act = None;
                egui::SidePanel::right("right_inspector")
                    .default_width(300.0)
                    .frame(
                        egui::Frame::new()
                            .fill(PANEL_BG)
                            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                            .inner_margin(egui::Margin::symmetric(10, 6)),
                    )
                    .show(ctx, |ui| {
                        insp_act =
                            render_right_inspector(ui, proj, self.selected_uuid.as_deref());
                    });

                if let Some(act) = insp_act {
                    match act {
                        InspectorAction::NavigateToBlock {
                            page_index,
                            block_uuid,
                        } => {
                            self.execute_palette_action(PaletteAction::NavigateToBlock {
                                page_index,
                                block_uuid,
                            });
                        }
                        InspectorAction::SelectObject(uuid) => {
                            self.selected_uuid = Some(uuid);
                        }
                    }
                }
            }
        }

        // Central Schematic Page Canvas
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(crate::theme::CANVAS_BG))
            .show(ctx, |ui| {
                let view_size = ui.available_size();
                if let Some(proj) = &self.project {
                    if self.needs_initial_fit {
                        if let Some(page) = proj.pages.get(self.active_page_index) {
                            self.viewport.fit_to_page(page, view_size);
                        }
                        self.needs_initial_fit = false;
                    }
                    if let Some(target_uuid) = self.pending_center_block_uuid.take() {
                        if let Some(page) = proj.pages.get(self.active_page_index) {
                            if let Some(block) =
                                page.blocks.iter().find(|b| b.uuid == target_uuid)
                            {
                                self.viewport.center_on_block(block, view_size);
                            }
                        }
                    }

                    if let Some(canvas_act) = render_page_canvas(
                        ui,
                        proj,
                        self.active_page_index,
                        &mut self.viewport,
                        self.selected_uuid.as_deref(),
                    ) {
                        match canvas_act {
                            CanvasAction::SelectObject(uuid) => {
                                self.selected_uuid = Some(uuid);
                                self.show_right_panel = true;
                            }
                            CanvasAction::ClearSelection => {
                                self.selected_uuid = None;
                            }
                            CanvasAction::NavigateToBlock {
                                page_index,
                                block_uuid,
                            } => {
                                self.execute_palette_action(PaletteAction::NavigateToBlock {
                                    page_index,
                                    block_uuid,
                                });
                            }
                        }
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        if ui
                            .button("Open .Loxone Project File (Ctrl+O)…")
                            .clicked()
                        {
                            self.execute_palette_action(PaletteAction::OpenFile);
                        }
                    });
                }
            });

        // Ctrl+K / Cmd+K Command Palette Modal Overlay
        if let Some(palette_act) = self.palette.show(ctx, self.project.as_ref()) {
            self.execute_palette_action(palette_act);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_load_and_navigate_klimczaki() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let mut app = LoxonePortableApp::default();
        app.load_file(&path);

        assert!(app.project.is_some());
        assert!(app.error_banner.is_none());

        // Navigate via Command Palette action to Page 5
        app.execute_palette_action(PaletteAction::NavigateToPage(5));
        assert_eq!(app.active_page_index, 5);

        // Toggle compact pins
        assert!(app.viewport.compact_pins);
        app.execute_palette_action(PaletteAction::ToggleCompactPins);
        assert!(!app.viewport.compact_pins);

        // Navigate to a block and select an object
        let first_block_uuid = app.project.as_ref().unwrap().pages[0].blocks[0].uuid.clone();
        app.execute_palette_action(PaletteAction::NavigateToBlock {
            page_index: 0,
            block_uuid: first_block_uuid.clone(),
        });
        assert_eq!(app.active_page_index, 0);
        assert_eq!(app.selected_uuid.as_deref(), Some(first_block_uuid.as_str()));
        assert_eq!(
            app.pending_center_block_uuid.as_deref(),
            Some(first_block_uuid.as_str())
        );

        // Headless egui pass exercising tree_panel and inspector across PageBlock, TreeNode, and Category
        let ctx = egui::Context::default();
        let proj = app.project.as_ref().unwrap();
        let tree_uuid = proj.periphery_tree[0].uuid.clone();
        let cat_uuid = proj.categories.keys().next().unwrap().clone();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                for mut tab in [
                    LeftTab::Pages,
                    LeftTab::Periphery,
                    LeftTab::Rooms,
                    LeftTab::System,
                ] {
                    let mut filter = String::new();
                    let _ = render_left_explorer(
                        ui,
                        proj,
                        &mut tab,
                        &mut filter,
                        0,
                        Some(&first_block_uuid),
                    );
                }
                let _ = render_right_inspector(ui, proj, Some(&first_block_uuid));
                let _ = render_right_inspector(ui, proj, Some(&tree_uuid));
                let _ = render_right_inspector(ui, proj, Some(&cat_uuid));
            });
        });
    }
}
