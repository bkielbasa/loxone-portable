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

use app::{AutoScreenshotConfig, LoxonePortableApp};
use tree_panel::LeftTab;

fn parse_cli_args() -> (Option<PathBuf>, Option<AutoScreenshotConfig>) {
    let mut initial_path: Option<PathBuf> = None;
    let mut screenshot_path: Option<PathBuf> = None;
    let mut tab: Option<LeftTab> = None;
    let mut page_query: Option<String> = None;
    let mut identify_uuid: Option<String> = None;
    let mut enable_live = false;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--screenshot" => {
                if let Some(val) = args.next() {
                    screenshot_path = Some(PathBuf::from(val));
                }
            }
            "--tab" => {
                if let Some(val) = args.next() {
                    tab = match val.to_lowercase().as_str() {
                        "pages" | "program" => Some(LeftTab::Pages),
                        "periphery" | "devices" => Some(LeftTab::Periphery),
                        "rooms" | "places" => Some(LeftTab::Rooms),
                        "system" => Some(LeftTab::System),
                        _ => None,
                    };
                }
            }
            "--page" => {
                if let Some(val) = args.next() {
                    page_query = Some(val);
                }
            }
            "--identify" => {
                if let Some(val) = args.next() {
                    identify_uuid = Some(val);
                }
            }
            "--live" => {
                enable_live = true;
            }
            other if !other.starts_with('-') => {
                let p = PathBuf::from(other);
                if p.exists() && initial_path.is_none() {
                    initial_path = Some(p);
                }
            }
            _ => {}
        }
    }

    let auto_screenshot = screenshot_path.map(|output_path| AutoScreenshotConfig {
        output_path,
        tab,
        page_query,
        identify_uuid,
        enable_live,
    });

    (initial_path, auto_screenshot)
}

fn main() -> eframe::Result<()> {
    let (initial_path, auto_screenshot) = parse_cli_args();
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
        Box::new(move |cc| {
            Ok(Box::new(LoxonePortableApp::new(
                cc,
                initial_path,
                auto_screenshot,
            )))
        }),
    )
}
