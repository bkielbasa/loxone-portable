#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod advanced_blocks_modal;
mod app;
mod canvas;
mod command_palette;
mod efm_modal;
pub mod i18n;
mod icons;
mod insert_modal;
mod inspector;
mod light_controller_modal;
mod miniserver_modal;
mod object_config_modal;
mod periphery_modal;
mod project_launcher;
mod room_controller_modal;
mod state_alarm_modal;
mod theme;
mod top_bar;
mod tree_panel;

use std::path::PathBuf;

use app::LoxonePortableApp;

fn resolve_initial_project_path() -> Option<PathBuf> {
    if let Some(arg) = std::env::args().nth(1) {
        let p = PathBuf::from(arg);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn main() -> eframe::Result<()> {
    let initial_path = resolve_initial_project_path();
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 600.0])
            .with_title("Loxone Portable — Project Viewer & Schematic Inspector"),
        ..Default::default()
    };

    eframe::run_native(
        "Loxone Portable",
        native_options,
        Box::new(move |cc| Ok(Box::new(LoxonePortableApp::new(cc, initial_path)))),
    )
}
