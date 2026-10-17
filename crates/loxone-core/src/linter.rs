use std::collections::{HashMap, HashSet};

use crate::block_schema::lookup_block_schema;
use crate::model::LoxoneProject;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDiagnostic {
    pub severity: DiagnosticSeverity,
    pub code: &'static str,
    pub page_index: usize,
    pub page_title: String,
    pub block_uuid: String,
    pub block_title: String,
    pub block_type: String,
    pub message: String,
}

/// Runs a static health check across all schematic pages in `project`, returning
/// actionable diagnostics sorted by severity (`Error` -> `Warning` -> `Info`).
pub fn lint_project(project: &LoxoneProject) -> Vec<ProjectDiagnostic> {
    let mut diagnostics = Vec::new();

    // Collect all pin UUIDs that are used as the source of at least one incoming wire link
    let mut used_source_pin_uuids: HashSet<&str> = HashSet::new();
    for page in &project.pages {
        for block in &page.blocks {
            for pin in &block.pins {
                for link in &pin.incoming_links {
                    if !link.source_pin_uuid.is_empty() {
                        used_source_pin_uuids.insert(link.source_pin_uuid.as_str());
                    }
                }
            }
        }
    }

    for (page_index, page) in project.pages.iter().enumerate() {
        let mut title_counts: HashMap<(&str, &str), usize> = HashMap::new();
        for block in &page.blocks {
            if !block.title.trim().is_empty() && block.block_type != "Memo" && block.block_type != "Text" {
                *title_counts
                    .entry((block.block_type.as_str(), block.title.trim()))
                    .or_insert(0) += 1;
            }
        }

        for block in &page.blocks {
            let display_title = if block.title.trim().is_empty() {
                block.block_type.clone()
            } else {
                block.title.clone()
            };

            // 1. Dangling Reference (`Ref` points to non-existent UUID)
            if matches!(
                block.block_type.as_str(),
                "InputRef" | "OutputRef" | "OutputRefLM" | "Memory"
            ) {
                match block.ref_uuid.as_deref() {
                    None | Some("") => {
                        diagnostics.push(ProjectDiagnostic {
                            severity: DiagnosticSeverity::Error,
                            code: "E001",
                            page_index,
                            page_title: page.title.clone(),
                            block_uuid: block.uuid.clone(),
                            block_title: display_title.clone(),
                            block_type: block.block_type.clone(),
                            message: "Reference block has no linked target (Ref is empty).".to_string(),
                        });
                    }
                    Some(target_uuid) if !project.xref.objects_by_uuid.contains_key(target_uuid) => {
                        diagnostics.push(ProjectDiagnostic {
                            severity: DiagnosticSeverity::Error,
                            code: "E002",
                            page_index,
                            page_title: page.title.clone(),
                            block_uuid: block.uuid.clone(),
                            block_title: display_title.clone(),
                            block_type: block.block_type.clone(),
                            message: format!(
                                "Dangling reference: target object '{}' no longer exists in project.",
                                target_uuid
                            ),
                        });
                    }
                    _ => {}
                }
            }

            // 2. Visualization-capable block missing Room or Category
            if let Some(schema) = lookup_block_schema(&block.block_type) {
                let is_visu_block = schema
                    .child_configs
                    .contains(&crate::block_schema::ChildConfigKind::IoData)
                    || schema.find_attr("UseInVisu").is_some();
                if is_visu_block {
                    let visu_disabled = block
                        .raw_attributes
                        .iter()
                        .any(|(k, v)| k == "UseInVisu" && (v == "false" || v == "0"));
                    if !visu_disabled {
                        let room_ok = block
                            .room_uuid
                            .as_deref()
                            .or(page.room_uuid.as_deref())
                            .is_some_and(|u| !u.is_empty() && project.places.contains_key(u));
                        let cat_ok = block
                            .category_uuid
                            .as_deref()
                            .or(page.category_uuid.as_deref())
                            .is_some_and(|u| !u.is_empty() && project.categories.contains_key(u));
                        if !room_ok || !cat_ok {
                            diagnostics.push(ProjectDiagnostic {
                                severity: DiagnosticSeverity::Warning,
                                code: "W001",
                                page_index,
                                page_title: page.title.clone(),
                                block_uuid: block.uuid.clone(),
                                block_title: display_title.clone(),
                                block_type: block.block_type.clone(),
                                message: format!(
                                    "Visualization block missing valid {} assignment.",
                                    match (room_ok, cat_ok) {
                                        (false, false) => "Room & Category",
                                        (false, true) => "Room",
                                        _ => "Category",
                                    }
                                ),
                            });
                        }
                    }
                }
            }

            // 3. Unconnected block (neither incoming nor outgoing wires, excluding annotations)
            if !matches!(block.block_type.as_str(), "Memo" | "Text" | "Page") {
                let has_incoming = block.pins.iter().any(|p| !p.incoming_links.is_empty());
                let has_outgoing = block
                    .pins
                    .iter()
                    .any(|p| !p.uuid.is_empty() && used_source_pin_uuids.contains(p.uuid.as_str()));
                if !has_incoming && !has_outgoing && block.extra_ref_uuids.is_empty() {
                    diagnostics.push(ProjectDiagnostic {
                        severity: DiagnosticSeverity::Info,
                        code: "I001",
                        page_index,
                        page_title: page.title.clone(),
                        block_uuid: block.uuid.clone(),
                        block_title: display_title.clone(),
                        block_type: block.block_type.clone(),
                        message: "Block has no connected input or output wires.".to_string(),
                    });
                }
            }

            // 4. Duplicate block title of the same type on the same page
            if !block.title.trim().is_empty()
                && !matches!(
                    block.block_type.as_str(),
                    "Memo" | "Text" | "InputRef" | "OutputRef" | "OutputRefLM" | "Memory"
                )
                && title_counts
                    .get(&(block.block_type.as_str(), block.title.trim()))
                    .copied()
                    .unwrap_or(0)
                    > 1
            {
                diagnostics.push(ProjectDiagnostic {
                    severity: DiagnosticSeverity::Info,
                    code: "I002",
                    page_index,
                    page_title: page.title.clone(),
                    block_uuid: block.uuid.clone(),
                    block_title: display_title,
                    block_type: block.block_type.clone(),
                    message: "Duplicate block title of the same type on this page.".to_string(),
                });
            }
        }
    }

    diagnostics.sort_by(|a, b| {
        a.severity
            .cmp(&b.severity)
            .then_with(|| a.page_index.cmp(&b.page_index))
            .then_with(|| a.block_title.cmp(&b.block_title))
    });
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factory::new_page_block;
    use crate::parser::load_loxone_file;
    use crate::writer::{apply_edit, ProjectEdit};
    use std::path::PathBuf;

    #[test]
    fn test_lint_project_detects_dangling_ref_missing_room_and_unconnected_block() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        let mut project = load_loxone_file(&path).expect("Klimczaki.Loxone should load");
        let page_0_uuid = project.pages[0].uuid.clone();

        // Add an unconnected Switch block without Room/Category
        let mut sw = new_page_block("Switch", "Orphan Switch", 128.0, 128.0);
        sw.room_uuid = None;
        sw.category_uuid = None;
        // Also clear page 0 room/cat inheritance so W001 triggers deterministically
        project.pages[0].room_uuid = None;
        project.pages[0].category_uuid = None;
        let sw_uuid = sw.uuid.clone();
        apply_edit(
            &mut project,
            &ProjectEdit::AddPageBlock {
                page_uuid: page_0_uuid.clone(),
                block: sw,
            },
        )
        .unwrap();

        // Add a dangling InputRef block
        let mut bad_ref = new_page_block("InputRef", "Broken Sensor", 256.0, 256.0);
        bad_ref.ref_uuid = Some("deadbeef-0000-0000-0000000000000000".to_string());
        let bad_ref_uuid = bad_ref.uuid.clone();
        apply_edit(
            &mut project,
            &ProjectEdit::AddPageBlock {
                page_uuid: page_0_uuid,
                block: bad_ref,
            },
        )
        .unwrap();

        let diags = lint_project(&project);
        assert!(diags
            .iter()
            .any(|d| d.block_uuid == bad_ref_uuid && d.code == "E002"));
        assert!(diags
            .iter()
            .any(|d| d.block_uuid == sw_uuid && d.code == "W001"));
        assert!(diags
            .iter()
            .any(|d| d.block_uuid == sw_uuid && d.code == "I001"));
    }
}
