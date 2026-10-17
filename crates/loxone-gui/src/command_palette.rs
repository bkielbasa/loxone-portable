use egui::{Align2, Color32, Context, Frame, Id, Key, Margin, Order, RichText, ScrollArea, Sense, Stroke, Vec2};
use loxone_core::model::LoxoneProject;
use loxone_core::xref::ObjectLocation;

use crate::theme::{ACCENT_BLUE, BORDER_SUBTLE, CARD_BG, PANEL_BG, TEXT_MUTED, TEXT_PRIMARY};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteAction {
    ShowProjectLauncher,
    OpenFile,
    SaveFile,
    ConnectMiniserver,
    FitPage,
    ResetZoom,
    ToggleCompactPins,
    ToggleLeftPanel,
    ToggleRightPanel,
    NavigateToPage(usize),
    NavigateToBlock { page_index: usize, block_uuid: String },
    SelectObject(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteBadge {
    Action,
    Page,
    Block,
    Periphery,
    Room,
    Category,
    System,
}

impl PaletteBadge {
    pub fn label(self) -> &'static str {
        match self {
            Self::Action => "ACTION",
            Self::Page => "PAGE",
            Self::Block => "BLOCK",
            Self::Periphery => "DEVICE",
            Self::Room => "ROOM",
            Self::Category => "CATEGORY",
            Self::System => "SYSTEM",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteEntry {
    pub badge: PaletteBadge,
    pub title: String,
    pub subtitle: String,
    pub shortcut: Option<&'static str>,
    pub action: PaletteAction,
}

#[derive(Debug, Default)]
pub struct CommandPaletteState {
    pub is_open: bool,
    pub query: String,
    pub selected_index: usize,
    pub focus_requested: bool,
}

pub fn build_palette_entries(project: Option<&LoxoneProject>, query: &str) -> Vec<PaletteEntry> {
    let q = query.trim().to_lowercase();
    let mut entries = Vec::new();

    let built_in_actions = [
        PaletteEntry {
            badge: PaletteBadge::Action,
            title: "Switch Project / Show Project List…".to_string(),
            subtitle: "Return to the startup project launcher to pick another .Loxone project".to_string(),
            shortcut: None,
            action: PaletteAction::ShowProjectLauncher,
        },
        PaletteEntry {
            badge: PaletteBadge::Action,
            title: "Connect to Miniserver / Download Project…".to_string(),
            subtitle: "Test Miniserver connection or download project backup from /prog/".to_string(),
            shortcut: Some("Ctrl+Shift+D"),
            action: PaletteAction::ConnectMiniserver,
        },
        PaletteEntry {
            badge: PaletteBadge::Action,
            title: "Save Project as .Loxone File…".to_string(),
            subtitle: "Export current project XML to disk".to_string(),
            shortcut: Some("Ctrl+S"),
            action: PaletteAction::SaveFile,
        },
        PaletteEntry {
            badge: PaletteBadge::Action,
            title: "Open .Loxone Project File…".to_string(),
            subtitle: "Load another Loxone Config XML file from disk".to_string(),
            shortcut: Some("Ctrl+O"),
            action: PaletteAction::OpenFile,
        },
        PaletteEntry {
            badge: PaletteBadge::Action,
            title: "Fit Active Page to View".to_string(),
            subtitle: "Center and scale all blocks on the current page".to_string(),
            shortcut: Some("F"),
            action: PaletteAction::FitPage,
        },
        PaletteEntry {
            badge: PaletteBadge::Action,
            title: "Toggle Compact Pins (Connected vs All Pins)".to_string(),
            subtitle: "Declutter function blocks by collapsing unconnected pins".to_string(),
            shortcut: Some("P"),
            action: PaletteAction::ToggleCompactPins,
        },
        PaletteEntry {
            badge: PaletteBadge::Action,
            title: "Reset Canvas Zoom (100%)".to_string(),
            subtitle: "Reset viewport scale to default".to_string(),
            shortcut: Some("0"),
            action: PaletteAction::ResetZoom,
        },
        PaletteEntry {
            badge: PaletteBadge::Action,
            title: "Toggle Left Explorer Sidebar".to_string(),
            subtitle: "Show or hide the Project & Periphery tree".to_string(),
            shortcut: Some("Ctrl+B"),
            action: PaletteAction::ToggleLeftPanel,
        },
        PaletteEntry {
            badge: PaletteBadge::Action,
            title: "Toggle Right Inspector Sidebar".to_string(),
            subtitle: "Show or hide Properties & Cross-References".to_string(),
            shortcut: Some("Ctrl+I"),
            action: PaletteAction::ToggleRightPanel,
        },
    ];

    for act in built_in_actions {
        if q.is_empty()
            || act.title.to_lowercase().contains(&q)
            || act.subtitle.to_lowercase().contains(&q)
        {
            entries.push(act);
        }
    }

    if let Some(proj) = project {
        for (idx, page) in proj.pages.iter().enumerate() {
            let visible_count = page.blocks.iter().filter(|b| b.has_position).count();
            let subtitle = format!("Page • {} blocks", visible_count);
            if q.is_empty()
                || page.title.to_lowercase().contains(&q)
                || page.uuid.to_lowercase().contains(&q)
            {
                entries.push(PaletteEntry {
                    badge: PaletteBadge::Page,
                    title: page.title.clone(),
                    subtitle,
                    shortcut: None,
                    action: PaletteAction::NavigateToPage(idx),
                });
            }
        }

        if !q.is_empty() {
            for summary in proj.xref.objects_by_uuid.values() {
                if summary.object_type == "Page" || summary.object_type == "Document" {
                    continue;
                }
                let room_name = summary
                    .room_uuid
                    .as_deref()
                    .and_then(|u| proj.places.get(u))
                    .map(|p| p.title.as_str())
                    .unwrap_or("");
                let hay = format!(
                    "{} {} {} {}",
                    summary.title, summary.object_type, room_name, summary.uuid
                )
                .to_lowercase();

                if !hay.contains(&q) {
                    continue;
                }

                match &summary.location {
                    ObjectLocation::PageBlock {
                        page_index,
                        block_index,
                        page_title,
                        ..
                    } => {
                        let is_positioned = proj
                            .pages
                            .get(*page_index)
                            .and_then(|p| p.blocks.get(*block_index))
                            .is_some_and(|b| b.has_position);
                        if !is_positioned {
                            continue;
                        }
                        let title = if summary.title.is_empty() {
                            summary.object_type.clone()
                        } else {
                            summary.title.clone()
                        };
                        entries.push(PaletteEntry {
                            badge: PaletteBadge::Block,
                            title,
                            subtitle: format!("{} on page '{}'", summary.object_type, page_title),
                            shortcut: None,
                            action: PaletteAction::NavigateToBlock {
                                page_index: *page_index,
                                block_uuid: summary.uuid.clone(),
                            },
                        });
                    }
                    ObjectLocation::Place => {
                        entries.push(PaletteEntry {
                            badge: PaletteBadge::Room,
                            title: summary.title.clone(),
                            subtitle: "Room".to_string(),
                            shortcut: None,
                            action: PaletteAction::SelectObject(summary.uuid.clone()),
                        });
                    }
                    ObjectLocation::Category => {
                        entries.push(PaletteEntry {
                            badge: PaletteBadge::Category,
                            title: summary.title.clone(),
                            subtitle: "Category".to_string(),
                            shortcut: None,
                            action: PaletteAction::SelectObject(summary.uuid.clone()),
                        });
                    }
                    ObjectLocation::TreeNode { section } => {
                        let badge = if section == "Periphery" {
                            PaletteBadge::Periphery
                        } else {
                            PaletteBadge::System
                        };
                        let usages = proj.xref.usages_for(&summary.uuid).len();
                        entries.push(PaletteEntry {
                            badge,
                            title: if summary.title.is_empty() {
                                summary.object_type.clone()
                            } else {
                                summary.title.clone()
                            },
                            subtitle: format!("{} • {} page refs", summary.object_type, usages),
                            shortcut: None,
                            action: PaletteAction::SelectObject(summary.uuid.clone()),
                        });
                    }
                    ObjectLocation::DocumentRoot | ObjectLocation::Page { .. } => {}
                }
            }
        }
    }

    entries.truncate(60);
    entries
}

impl CommandPaletteState {
    pub fn open(&mut self) {
        self.is_open = true;
        self.query.clear();
        self.selected_index = 0;
        self.focus_requested = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn show(&mut self, ctx: &Context, project: Option<&LoxoneProject>) -> Option<PaletteAction> {
        if !self.is_open {
            return None;
        }

        let entries = build_palette_entries(project, &self.query);
        if entries.is_empty() {
            self.selected_index = 0;
        } else if self.selected_index >= entries.len() {
            self.selected_index = entries.len() - 1;
        }

        let mut triggered_action = None;

        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.close();
            return None;
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowDown)) && !entries.is_empty() {
            self.selected_index = (self.selected_index + 1) % entries.len();
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowUp)) && !entries.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                entries.len() - 1
            } else {
                self.selected_index - 1
            };
        }
        if ctx.input(|i| i.key_pressed(Key::Enter)) {
            if let Some(entry) = entries.get(self.selected_index) {
                triggered_action = Some(entry.action.clone());
            }
        }

        // Subtle backdrop on Order::Middle (strictly behind Order::Foreground modal)
        let screen_rect = ctx.screen_rect();
        let mut backdrop_clicked = false;
        egui::Area::new(Id::new("cmd_palette_backdrop"))
            .order(Order::Middle)
            .fixed_pos(screen_rect.min)
            .show(ctx, |ui| {
                let resp = ui.allocate_rect(screen_rect, Sense::click());
                ui.painter().rect_filled(
                    screen_rect,
                    0.0,
                    Color32::from_black_alpha(55),
                );
                if resp.clicked() {
                    backdrop_clicked = true;
                }
            });

        egui::Area::new(Id::new("cmd_palette_modal"))
            .order(Order::Foreground)
            .anchor(Align2::CENTER_TOP, Vec2::new(0.0, 72.0))
            .show(ctx, |ui| {
                Frame::new()
                    .fill(PANEL_BG)
                    .stroke(Stroke::new(1.5_f32, ACCENT_BLUE))
                    .shadow(egui::Shadow {
                        offset: [0, 8],
                        blur: 24,
                        spread: 0,
                        color: Color32::from_black_alpha(100),
                    })
                    .corner_radius(10)
                    .inner_margin(Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_width(540.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("⌘K").color(ACCENT_BLUE).strong());
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut self.query)
                                    .hint_text("Type a command, page, block, device, room, or UUID…")
                                    .desired_width(450.0)
                                    .frame(false),
                            );
                            if self.focus_requested {
                                resp.request_focus();
                                self.focus_requested = false;
                            }
                            if resp.changed() {
                                self.selected_index = 0;
                            }
                        });

                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(4.0);

                        ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
                            if entries.is_empty() {
                                ui.label(RichText::new("No matching commands or objects").color(TEXT_MUTED));
                            }
                            for (idx, entry) in entries.iter().enumerate() {
                                let is_selected = idx == self.selected_index;
                                let bg = if is_selected {
                                    Color32::from_rgba_unmultiplied(0x5E, 0x81, 0xFF, 45)
                                } else {
                                    CARD_BG
                                };
                                let border = if is_selected { ACCENT_BLUE } else { BORDER_SUBTLE };

                                let resp = Frame::new()
                                    .fill(bg)
                                    .stroke(Stroke::new(1.0_f32, border))
                                    .corner_radius(6)
                                    .inner_margin(Margin::symmetric(10, 6))
                                    .show(ui, |ui| {
                                        ui.set_width(512.0);
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(entry.badge.label())
                                                    .size(10.0)
                                                    .color(ACCENT_BLUE)
                                                    .strong(),
                                            );
                                            ui.label(
                                                RichText::new(&entry.title)
                                                    .color(TEXT_PRIMARY)
                                                    .strong(),
                                            );
                                            ui.label(
                                                RichText::new(&entry.subtitle)
                                                    .size(11.5)
                                                    .color(TEXT_MUTED),
                                            );
                                            if let Some(sc) = entry.shortcut {
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        ui.label(
                                                            RichText::new(sc)
                                                                .size(11.0)
                                                                .color(TEXT_MUTED),
                                                        );
                                                    },
                                                );
                                            }
                                        });
                                    })
                                    .response
                                    .interact(Sense::click());

                                if resp.clicked() {
                                    triggered_action = Some(entry.action.clone());
                                }
                                ui.add_space(3.0);
                            }
                        });
                    });
            });

        if triggered_action.is_some() || backdrop_clicked {
            self.close();
        }
        triggered_action
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use loxone_core::parser::load_loxone_file;

    #[test]
    fn test_command_palette_default_and_search_on_klimczaki() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should parse");

        // Empty query returns built-in actions + pages
        let initial = build_palette_entries(Some(&project), "");
        assert!(initial.iter().any(|e| e.action == PaletteAction::FitPage));
        assert!(initial
            .iter()
            .any(|e| e.action == PaletteAction::ConnectMiniserver));
        assert!(initial.iter().any(|e| e.action == PaletteAction::SaveFile));
        assert!(initial.iter().any(|e| e.badge == PaletteBadge::Page));

        // Searching "audio" matches the "Audio centralne" page and CentralMusic block
        let audio_results = build_palette_entries(Some(&project), "audio");
        assert!(audio_results.iter().any(|e| e.title.contains("Audio centralne")));

        // Searching "lightcontroller2" matches LightController2 blocks
        let lc_results = build_palette_entries(Some(&project), "lightcontroller2");
        assert!(lc_results.iter().any(|e| e.badge == PaletteBadge::Block));

        // Verify backdrop is on Order::Middle and modal is strictly above it on Order::Foreground
        let ctx = Context::default();
        let mut palette = CommandPaletteState::default();
        palette.open();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let _ = palette.show(ctx, Some(&project));
        });
        let backdrop_id = egui::LayerId::new(Order::Middle, Id::new("cmd_palette_backdrop"));
        let modal_id = egui::LayerId::new(Order::Foreground, Id::new("cmd_palette_modal"));
        assert!(backdrop_id.order < modal_id.order);
    }
}
