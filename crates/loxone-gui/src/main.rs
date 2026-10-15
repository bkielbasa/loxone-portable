mod app;
mod canvas;
mod command_palette;
mod inspector;
mod miniserver_modal;
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
    let default_path = PathBuf::from("Klimczaki.Loxone");
    if default_path.exists() {
        return Some(default_path);
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
