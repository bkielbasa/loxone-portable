use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense, Stroke, Ui, Vec2};
use loxone_core::miniserver::ConnectionProfile;

use crate::icons::{ui_icon, LoxoneIcon};
use crate::theme::{ACCENT_BLUE, BORDER_SUBTLE, CARD_BG, PANEL_BG, STATUS_CONNECTED, TEXT_MUTED, TEXT_PRIMARY};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectSourceKind {
    Recent,
    Workspace,
    Backup,
}

impl ProjectSourceKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Recent => "RECENT",
            Self::Workspace => "WORKSPACE",
            Self::Backup => "BACKUP",
        }
    }

    pub fn color(self) -> Color32 {
        match self {
            Self::Recent => ACCENT_BLUE,
            Self::Workspace => STATUS_CONNECTED,
            Self::Backup => Color32::from_rgb(230, 175, 75),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredProject {
    pub path: PathBuf,
    pub file_name: String,
    pub source: ProjectSourceKind,
    pub size_bytes: u64,
    pub modified_secs: u64,
}

impl DiscoveredProject {
    pub fn from_path(path: PathBuf, source: ProjectSourceKind) -> Option<Self> {
        let meta = std::fs::metadata(&path).ok()?;
        if !meta.is_file() {
            return None;
        }
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("project.Loxone")
            .to_string();
        let size_bytes = meta.len();
        let modified_secs = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Some(Self {
            path,
            file_name,
            source,
            size_bytes,
            modified_secs,
        })
    }

    pub fn formatted_size(&self) -> String {
        if self.size_bytes >= 1_048_576 {
            format!("{:.2} MB", self.size_bytes as f64 / 1_048_576.0)
        } else if self.size_bytes >= 1024 {
            format!("{:.1} KB", self.size_bytes as f64 / 1024.0)
        } else {
            format!("{} B", self.size_bytes)
        }
    }
}

pub fn default_recent_projects_path() -> Option<PathBuf> {
    ConnectionProfile::default_config_path().map(|p| p.with_file_name("recent_projects.txt"))
}

pub fn load_recent_project_paths_from(config_file: &Path) -> Vec<PathBuf> {
    let Ok(raw) = std::fs::read_to_string(config_file) else {
        return Vec::new();
    };
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .collect()
}

pub fn record_recent_project_in(config_file: &Path, project_path: &Path) {
    let canonical = std::fs::canonicalize(project_path).unwrap_or_else(|_| project_path.to_path_buf());
    let mut existing = load_recent_project_paths_from(config_file);
    existing.retain(|p| {
        let c = std::fs::canonicalize(p).unwrap_or_else(|_| p.clone());
        c != canonical
    });
    existing.insert(0, canonical);
    existing.truncate(15);
    if let Some(parent) = config_file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let body: String = existing
        .iter()
        .map(|p| format!("{}\n", p.display()))
        .collect();
    let _ = std::fs::write(config_file, body);
}

pub fn record_recent_project(project_path: &Path) {
    if let Some(cfg) = default_recent_projects_path() {
        record_recent_project_in(&cfg, project_path);
    }
}

fn is_loxone_project_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("loxone"))
        .unwrap_or(false)
}

fn collect_loxone_files_in_dir(dir: &Path, max_depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && is_loxone_project_file(&path) {
            out.push(path);
        } else if max_depth > 0 && path.is_dir() {
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !dir_name.starts_with('.') && dir_name != "target" && dir_name != "node_modules" {
                collect_loxone_files_in_dir(&path, max_depth - 1, out);
            }
        }
    }
}

pub fn discover_projects(
    workspace_dirs: &[PathBuf],
    recent_paths: &[PathBuf],
    backup_dir: Option<&Path>,
) -> Vec<DiscoveredProject> {
    let mut seen_canonical: HashSet<PathBuf> = HashSet::new();
    let mut results = Vec::new();

    // 1. Recent projects first (in MRU order)
    for path in recent_paths {
        if !is_loxone_project_file(path) {
            continue;
        }
        let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.clone());
        if seen_canonical.insert(canonical) {
            if let Some(item) = DiscoveredProject::from_path(path.clone(), ProjectSourceKind::Recent) {
                results.push(item);
            }
        }
    }

    // 2. Workspace / current directory .Loxone files
    let mut workspace_files = Vec::new();
    for dir in workspace_dirs {
        collect_loxone_files_in_dir(dir, 1, &mut workspace_files);
    }
    workspace_files.sort();
    for path in workspace_files {
        let canonical = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if seen_canonical.insert(canonical) {
            if let Some(item) = DiscoveredProject::from_path(path, ProjectSourceKind::Workspace) {
                results.push(item);
            }
        }
    }

    // 3. Local backups directory (newest first)
    if let Some(bdir) = backup_dir {
        let mut backup_files = Vec::new();
        collect_loxone_files_in_dir(bdir, 0, &mut backup_files);
        let mut backup_items: Vec<DiscoveredProject> = backup_files
            .into_iter()
            .filter_map(|path| {
                let canonical = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
                if seen_canonical.insert(canonical) {
                    DiscoveredProject::from_path(path, ProjectSourceKind::Backup)
                } else {
                    None
                }
            })
            .collect();
        backup_items.sort_by(|a, b| b.modified_secs.cmp(&a.modified_secs));
        results.extend(backup_items);
    }

    results
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LauncherAction {
    LoadProject(PathBuf),
    BrowseFileDialog,
    ConnectMiniserver,
    RefreshList,
}

#[derive(Debug, Clone)]
pub struct ProjectLauncherState {
    pub projects: Vec<DiscoveredProject>,
    pub filter_query: String,
}

impl Default for ProjectLauncherState {
    fn default() -> Self {
        let mut state = Self {
            projects: Vec::new(),
            filter_query: String::new(),
        };
        state.refresh();
        state
    }
}

impl ProjectLauncherState {
    pub fn refresh(&mut self) {
        let mut workspace_dirs = Vec::new();
        if let Ok(cwd) = std::env::current_dir() {
            workspace_dirs.push(cwd);
        }
        let recent_paths = default_recent_projects_path()
            .map(|p| load_recent_project_paths_from(&p))
            .unwrap_or_default();
        let backup_dir = ConnectionProfile::default_backup_dir();
        self.projects = discover_projects(&workspace_dirs, &recent_paths, backup_dir.as_deref());
    }

    pub fn filtered_projects(&self) -> Vec<&DiscoveredProject> {
        let q = self.filter_query.trim().to_lowercase();
        self.projects
            .iter()
            .filter(|p| {
                q.is_empty()
                    || p.file_name.to_lowercase().contains(&q)
                    || p.path.to_string_lossy().to_lowercase().contains(&q)
                    || p.source.label().to_lowercase().contains(&q)
            })
            .collect()
    }
}

pub fn render_project_launcher(
    ui: &mut Ui,
    state: &mut ProjectLauncherState,
) -> Option<LauncherAction> {
    let mut action = None;
    let avail = ui.available_size();
    let card_width = avail.x.clamp(420.0, 680.0) - 32.0;
    let top_pad = ((avail.y - 500.0) * 0.35).max(20.0);

    ui.vertical_centered(|ui| {
        ui.add_space(top_pad);

        Frame::new()
            .fill(PANEL_BG)
            .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::symmetric(24, 20))
            .show(ui, |ui| {
                ui.set_max_width(card_width);
                ui.set_min_width(card_width);

                // Header
                ui.horizontal(|ui| {
                    ui_icon(ui, LoxoneIcon::Miniserver, 20.0);
                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Select a Loxone Project to Load")
                                .size(17.0)
                                .strong()
                                .color(TEXT_PRIMARY),
                        );
                        ui.label(
                            RichText::new(
                                "Choose a .Loxone project from your workspace, recent files, or local backups.",
                            )
                            .size(12.0)
                            .color(TEXT_MUTED),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .button(RichText::new("🔄 Refresh").size(11.5))
                            .on_hover_text("Rescan workspace, recent files, and local backups")
                            .clicked()
                        {
                            action = Some(LauncherAction::RefreshList);
                        }
                        ui.label(
                            RichText::new(format!("{} project(s)", state.projects.len()))
                                .size(11.5)
                                .color(TEXT_MUTED),
                        );
                    });
                });

                ui.add_space(12.0);

                // Search / Filter input
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔍").size(13.0).color(TEXT_MUTED));
                    ui.add(
                        egui::TextEdit::singleline(&mut state.filter_query)
                            .hint_text("Filter projects by name or path…")
                            .desired_width(ui.available_width()),
                    );
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                let filtered = state.filtered_projects();
                if filtered.is_empty() {
                    Frame::new()
                        .fill(CARD_BG)
                        .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::symmetric(16, 24))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new("No .Loxone projects matched.")
                                        .size(13.0)
                                        .color(TEXT_MUTED),
                                );
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new(
                                        "Use 'Browse .Loxone File…' below or download a project from your Miniserver.",
                                    )
                                    .size(11.5)
                                    .color(TEXT_MUTED),
                                );
                            });
                        });
                } else {
                    let list_paths: Vec<DiscoveredProject> =
                        filtered.into_iter().cloned().collect();
                    ScrollArea::vertical()
                        .max_height(290.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for item in list_paths {
                                let row_frame = Frame::new()
                                    .fill(CARD_BG)
                                    .stroke(Stroke::new(1.0_f32, BORDER_SUBTLE))
                                    .corner_radius(CornerRadius::same(6))
                                    .inner_margin(Margin::symmetric(14, 10));

                                let inner = row_frame.show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui_icon(ui, LoxoneIcon::Page, 16.0);
                                        ui.add_space(6.0);
                                        ui.vertical(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new(&item.file_name)
                                                        .size(13.5)
                                                        .strong()
                                                        .color(TEXT_PRIMARY),
                                                );
                                                Frame::new()
                                                    .fill(item.source.color().gamma_multiply(0.18))
                                                    .stroke(Stroke::new(
                                                        1.0_f32,
                                                        item.source.color().gamma_multiply(0.65),
                                                    ))
                                                    .corner_radius(CornerRadius::same(4))
                                                    .inner_margin(Margin::symmetric(6, 1))
                                                    .show(ui, |ui| {
                                                        ui.label(
                                                            RichText::new(item.source.label())
                                                                .size(9.5)
                                                                .strong()
                                                                .color(item.source.color()),
                                                        );
                                                    });
                                                ui.label(
                                                    RichText::new(item.formatted_size())
                                                        .size(11.0)
                                                        .color(TEXT_MUTED),
                                                );
                                            });
                                            ui.label(
                                                RichText::new(item.path.display().to_string())
                                                    .monospace()
                                                    .size(10.5)
                                                    .color(TEXT_MUTED),
                                            );
                                        });

                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                if ui
                                                    .add(
                                                        egui::Button::new(
                                                            RichText::new("Load Project →")
                                                                .size(12.0)
                                                                .strong()
                                                                .color(TEXT_PRIMARY),
                                                        )
                                                        .fill(ACCENT_BLUE.gamma_multiply(0.32))
                                                        .stroke(Stroke::new(1.0_f32, ACCENT_BLUE))
                                                        .min_size(Vec2::new(112.0, 28.0)),
                                                    )
                                                    .clicked()
                                                {
                                                    action = Some(LauncherAction::LoadProject(
                                                        item.path.clone(),
                                                    ));
                                                }
                                            },
                                        );
                                    });
                                });

                                let row_resp = ui.interact(
                                    inner.response.rect,
                                    ui.id().with(("proj_row", &item.path)),
                                    Sense::click(),
                                );
                                if row_resp.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                                if row_resp.clicked() && action.is_none() {
                                    action = Some(LauncherAction::LoadProject(item.path.clone()));
                                }
                                ui.add_space(6.0);
                            }
                        });
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                // Bottom Action Buttons
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("📂 Browse .Loxone File… (Ctrl+O)")
                                    .size(12.5)
                                    .color(TEXT_PRIMARY),
                            )
                            .min_size(Vec2::new(220.0, 32.0)),
                        )
                        .clicked()
                    {
                        action = Some(LauncherAction::BrowseFileDialog);
                    }
                    ui.add_space(8.0);
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(
                                    "⬇ Connect & Download from Miniserver… (Ctrl+Shift+D)",
                                )
                                .size(12.5)
                                .color(TEXT_PRIMARY),
                            )
                            .min_size(Vec2::new(310.0, 32.0)),
                        )
                        .clicked()
                    {
                        action = Some(LauncherAction::ConnectMiniserver);
                    }
                });
            });
    });

    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_projects_and_recent_persistence() {
        let temp_dir = std::env::temp_dir().join(format!(
            "loxone_launcher_test_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let ws_dir = temp_dir.join("workspace");
        let bkp_dir = temp_dir.join("backups");
        std::fs::create_dir_all(&ws_dir).unwrap();
        std::fs::create_dir_all(&bkp_dir).unwrap();

        let proj_a = ws_dir.join("Alpha.Loxone");
        let proj_b = ws_dir.join("Beta.Loxone");
        let proj_bkp = bkp_dir.join("pre_upload_2026_Alpha.Loxone");
        std::fs::write(&proj_a, "<Document/>").unwrap();
        std::fs::write(&proj_b, "<Document/>").unwrap();
        std::fs::write(&proj_bkp, "<Document/>").unwrap();

        let recent_cfg = temp_dir.join("recent_projects.json");
        record_recent_project_in(&recent_cfg, &proj_b);
        let recents = load_recent_project_paths_from(&recent_cfg);
        assert_eq!(recents.len(), 1);

        let discovered = discover_projects(&[ws_dir.clone()], &recents, Some(&bkp_dir));
        assert_eq!(discovered.len(), 3);
        // Beta.Loxone was recorded as Recent so it appears first with ProjectSourceKind::Recent
        assert_eq!(discovered[0].file_name, "Beta.Loxone");
        assert_eq!(discovered[0].source, ProjectSourceKind::Recent);
        // Alpha.Loxone is discovered from Workspace
        assert_eq!(discovered[1].file_name, "Alpha.Loxone");
        assert_eq!(discovered[1].source, ProjectSourceKind::Workspace);
        // pre_upload_2026_Alpha.Loxone is discovered from Backup
        assert_eq!(discovered[2].file_name, "pre_upload_2026_Alpha.Loxone");
        assert_eq!(discovered[2].source, ProjectSourceKind::Backup);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
