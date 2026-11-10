//! End-to-end mutation & Loxone Config round-trip verification engine.
//!
//! Generates a mutated `.Loxone` project exercising all major authoring subsystems
//! (rooms, categories, schematic pages, blocks, specialized child configs, wires,
//! inverted pins, periphery provisioning, Autopilot rules, To-Do notes, and
//! Multiplier units), verifies lossless internal round-trip, and compares the
//! output against a file opened and re-saved by real Loxone Config 17 in the
//! Windows Parallels VM.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result};
use loxone_core::advanced_blocks_config::{
    collect_audio_out_channels, AudioFavoritePreset, AudioZoneConfig,
};
use loxone_core::efm_config::{EfmConfig, EfmNode, EfmNodeType};
use loxone_core::factory::{new_category, new_page, new_page_block, new_room};
use loxone_core::light_config::{HclMode, LightControllerConfig, LightScene};
use loxone_core::linter::{lint_project, DiagnosticSeverity};
use loxone_core::model::LoxoneProject;
use loxone_core::parser::{parse_loxone_xml, parse_raw_dom};
use loxone_core::periphery_provisioning::{build_provision_hardware_edit, HardwareTemplateKind};
use loxone_core::project_admin::{
    build_add_todo_edit, build_autopilot_rule_edits, build_upsert_multiplier_unit_edit,
    AutopilotRuleSpec, MultiplierUnitEntry,
};
use loxone_core::room_controller_config::{RoomControllerConfig, RoomTempMode};
use loxone_core::writer::{apply_edit_with_inverse, serialize_loxone_xml, ProjectEdit};
use serde::{Deserialize, Serialize};

use crate::corpus::load_file;
use crate::diff::{diff_files, DiffOptions, DiffSummary, NodeDiff};
use crate::dom::{node_uuid, walk, NodeBrief};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedAuthoredObject {
    pub uuid: String,
    pub object_type: String,
    pub title: String,
    pub subsystem: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MutationManifest {
    pub source_title: String,
    pub expected_objects: Vec<ExpectedAuthoredObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoundtripVerificationReport {
    pub internal_roundtrip_passed: bool,
    pub authored_count: usize,
    pub internal_surviving_count: usize,
    pub internal_missing_uuids: Vec<String>,
    pub linter_error_count: usize,
    pub config_roundtrip: Option<ConfigRoundtripReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigRoundtripReport {
    pub config_saved_path: String,
    pub config_accepted_all_authored: bool,
    pub surviving_authored_count: usize,
    pub dropped_authored_objects: Vec<ExpectedAuthoredObject>,
    pub diff_summary: DiffSummary,
    pub config_added_nodes: Vec<NodeBrief>,
    pub config_removed_nodes: Vec<NodeBrief>,
    pub config_normalized_nodes: Vec<NodeDiff>,
}

/// Applies a comprehensive set of authoring mutations across all major `loxone-core`
/// subsystems to `base_project`, returning the mutated project and a manifest of
/// every newly created object UUID.
pub fn build_comprehensive_mutation_project(
    base_project: &LoxoneProject,
) -> Result<(LoxoneProject, MutationManifest)> {
    let mut project = base_project.clone();
    let mut expected = Vec::new();

    // 1. Add Room & Category
    let room = new_room("E2E Test Lab");
    let room_uuid = room.uuid.clone();
    expected.push(ExpectedAuthoredObject {
        uuid: room_uuid.clone(),
        object_type: "Place".to_string(),
        title: room.title.clone(),
        subsystem: "rooms_categories".to_string(),
    });
    apply_edit_with_inverse(&mut project, &ProjectEdit::AddRoom { room })
        .map_err(|e| anyhow::anyhow!(e))?;

    let category = new_category("E2E Automation", "lighting");
    let category_uuid = category.uuid.clone();
    expected.push(ExpectedAuthoredObject {
        uuid: category_uuid.clone(),
        object_type: "Category".to_string(),
        title: category.title.clone(),
        subsystem: "rooms_categories".to_string(),
    });
    apply_edit_with_inverse(&mut project, &ProjectEdit::AddCategory { category })
        .map_err(|e| anyhow::anyhow!(e))?;

    // 2. Add a new schematic page assigned to the new room & category
    let mut page = new_page("E2E Roundtrip Page");
    page.room_uuid = Some(room_uuid.clone());
    page.category_uuid = Some(category_uuid.clone());
    let page_uuid = page.uuid.clone();
    expected.push(ExpectedAuthoredObject {
        uuid: page_uuid.clone(),
        object_type: "Page".to_string(),
        title: page.title.clone(),
        subsystem: "schematic_page".to_string(),
    });
    apply_edit_with_inverse(&mut project, &ProjectEdit::AddPage { page })
        .map_err(|e| anyhow::anyhow!(e))?;

    // 3. Add function blocks on the new page
    let mut sw_block = new_page_block("PushButton", "E2E Wall Switch", 128.0, 128.0);
    sw_block.room_uuid = Some(room_uuid.clone());
    sw_block.category_uuid = Some(category_uuid.clone());
    let sw_uuid = sw_block.uuid.clone();
    expected.push(ExpectedAuthoredObject {
        uuid: sw_uuid.clone(),
        object_type: "PushButton".to_string(),
        title: sw_block.title.clone(),
        subsystem: "function_blocks".to_string(),
    });
    apply_edit_with_inverse(
        &mut project,
        &ProjectEdit::AddPageBlock {
            page_uuid: page_uuid.clone(),
            block: sw_block,
        },
    )
    .map_err(|e| anyhow::anyhow!(e))?;

    let mut and_block = new_page_block("And", "E2E Logic Gate", 448.0, 128.0);
    and_block.room_uuid = Some(room_uuid.clone());
    and_block.category_uuid = Some(category_uuid.clone());
    let and_uuid = and_block.uuid.clone();
    expected.push(ExpectedAuthoredObject {
        uuid: and_uuid.clone(),
        object_type: "And".to_string(),
        title: and_block.title.clone(),
        subsystem: "function_blocks".to_string(),
    });
    apply_edit_with_inverse(
        &mut project,
        &ProjectEdit::AddPageBlock {
            page_uuid: page_uuid.clone(),
            block: and_block,
        },
    )
    .map_err(|e| anyhow::anyhow!(e))?;

    // Wire PushButton.Q -> And.I1 and invert And.I1
    let src_pin_uuid = project
        .pages
        .iter()
        .find(|p| p.uuid == page_uuid)
        .and_then(|p| p.blocks.iter().find(|b| b.uuid == sw_uuid))
        .and_then(|b| b.pins.iter().find(|pin| pin.key == "Q"))
        .map(|pin| pin.uuid.clone())
        .context("PushButton.Q pin should exist")?;

    apply_edit_with_inverse(
        &mut project,
        &ProjectEdit::AddWireLink {
            target_block_uuid: and_uuid.clone(),
            target_pin_key: "I1".to_string(),
            source_pin_uuid: src_pin_uuid,
        },
    )
    .map_err(|e| anyhow::anyhow!(e))?;

    apply_edit_with_inverse(
        &mut project,
        &ProjectEdit::SetPinInverted {
            control_uuid: and_uuid.clone(),
            pin_key: "I1".to_string(),
            inverted: true,
        },
    )
    .map_err(|e| anyhow::anyhow!(e))?;

    // 4. Add & configure LightController2, HeatIRoomController2, EFM, MusicPlayer
    let mut lc_block = new_page_block("LightController2", "E2E Light Controller", 768.0, 128.0);
    lc_block.room_uuid = Some(room_uuid.clone());
    lc_block.category_uuid = Some(category_uuid.clone());
    let lc_uuid = lc_block.uuid.clone();
    expected.push(ExpectedAuthoredObject {
        uuid: lc_uuid.clone(),
        object_type: "LightController2".to_string(),
        title: lc_block.title.clone(),
        subsystem: "specialized_blocks".to_string(),
    });
    apply_edit_with_inverse(
        &mut project,
        &ProjectEdit::AddPageBlock {
            page_uuid: page_uuid.clone(),
            block: lc_block,
        },
    )
    .map_err(|e| anyhow::anyhow!(e))?;

    if let Some(blk) = project
        .pages
        .iter()
        .find(|p| p.uuid == page_uuid)
        .and_then(|p| p.blocks.iter().find(|b| b.uuid == lc_uuid))
        .cloned()
    {
        let mut cfg = LightControllerConfig::from_block(&blk);
        cfg.hcl.mode = HclMode::SunriseSunset;
        cfg.hcl.tmin_kelvin = 2700;
        cfg.hcl.tmax_kelvin = 6000;
        let new_sc = LightScene::new_user_scene("E2E Cinema Mood", 8, &cfg.circuits);
        cfg.scenes.push(new_sc);
        let edit = cfg.to_project_edits(&blk);
        apply_edit_with_inverse(&mut project, &edit).map_err(|e| anyhow::anyhow!(e))?;
    }

    let mut irc_block = new_page_block("HeatIRoomController2", "E2E Room Climate", 768.0, 384.0);
    irc_block.room_uuid = Some(room_uuid.clone());
    let irc_uuid = irc_block.uuid.clone();
    expected.push(ExpectedAuthoredObject {
        uuid: irc_uuid.clone(),
        object_type: "HeatIRoomController2".to_string(),
        title: irc_block.title.clone(),
        subsystem: "specialized_blocks".to_string(),
    });
    apply_edit_with_inverse(
        &mut project,
        &ProjectEdit::AddPageBlock {
            page_uuid: page_uuid.clone(),
            block: irc_block,
        },
    )
    .map_err(|e| anyhow::anyhow!(e))?;

    if let Some(blk) = project
        .pages
        .iter()
        .find(|p| p.uuid == page_uuid)
        .and_then(|p| p.blocks.iter().find(|b| b.uuid == irc_uuid))
        .cloned()
    {
        let mut cfg = RoomControllerConfig::from_block(&project, &blk);
        cfg.schedule.default_mode = RoomTempMode::Comfort;
        cfg.schedule
            .set_interval(0, 360, 1320, RoomTempMode::Comfort);
        for edit in cfg.to_edits(&blk) {
            apply_edit_with_inverse(&mut project, &edit).map_err(|e| anyhow::anyhow!(e))?;
        }
    }

    let efm_block = new_page_block("EFM", "E2E Energy Flow", 1152.0, 128.0);
    let efm_uuid = efm_block.uuid.clone();
    expected.push(ExpectedAuthoredObject {
        uuid: efm_uuid.clone(),
        object_type: "EFM".to_string(),
        title: efm_block.title.clone(),
        subsystem: "specialized_blocks".to_string(),
    });
    apply_edit_with_inverse(
        &mut project,
        &ProjectEdit::AddPageBlock {
            page_uuid: page_uuid.clone(),
            block: efm_block,
        },
    )
    .map_err(|e| anyhow::anyhow!(e))?;

    if let Some(blk) = project
        .pages
        .iter()
        .find(|p| p.uuid == page_uuid)
        .and_then(|p| p.blocks.iter().find(|b| b.uuid == efm_uuid))
        .cloned()
    {
        let mut cfg = EfmConfig::from_block(&blk);
        cfg.root.children.push(EfmNode::new_child(
            "E2E Solar Inverter",
            EfmNodeType::Production,
            "",
        ));
        let edit = cfg.to_project_edits(&blk);
        apply_edit_with_inverse(&mut project, &edit).map_err(|e| anyhow::anyhow!(e))?;
    }

    let az_block = new_page_block("MusicPlayer", "E2E Audio Player", 1152.0, 384.0);
    let az_uuid = az_block.uuid.clone();
    expected.push(ExpectedAuthoredObject {
        uuid: az_uuid.clone(),
        object_type: "MusicPlayer".to_string(),
        title: az_block.title.clone(),
        subsystem: "specialized_blocks".to_string(),
    });
    apply_edit_with_inverse(
        &mut project,
        &ProjectEdit::AddPageBlock {
            page_uuid: page_uuid.clone(),
            block: az_block,
        },
    )
    .map_err(|e| anyhow::anyhow!(e))?;

    if let Some(blk) = project
        .pages
        .iter()
        .find(|p| p.uuid == page_uuid)
        .and_then(|p| p.blocks.iter().find(|b| b.uuid == az_uuid))
        .cloned()
    {
        let mut cfg = AudioZoneConfig::from_block(&blk, &project);
        cfg.default_vol = 32;
        cfg.favorites.push(AudioFavoritePreset {
            slot: 1,
            title: "E2E Chillout Radio".to_string(),
            source_uri: "tunein://s24939".to_string(),
        });
        let all_channels = collect_audio_out_channels(&project);
        let edit = cfg.to_project_edit(&page_uuid, &az_uuid, &all_channels);
        apply_edit_with_inverse(&mut project, &edit).map_err(|e| anyhow::anyhow!(e))?;
    }

    // 5. Periphery Hardware Provisioning (RelayExtension, RgbwDimmerTree, TouchPureAir)
    for (kind, title, serial) in [
        (
            HardwareTemplateKind::RelayExtension,
            "E2E Relay Extension",
            "B030E2E1",
        ),
        (
            HardwareTemplateKind::RgbwDimmerTree,
            "E2E RGBW Dimmer Tree",
            "B050E2E2",
        ),
        (
            HardwareTemplateKind::TouchPureAir,
            "E2E Touch Pure Air",
            "504FE2E3",
        ),
    ] {
        let hw_edit = build_provision_hardware_edit(
            &project,
            kind,
            title,
            serial,
            None,
            Some(&room_uuid),
            Some(&category_uuid),
        );
        if let ProjectEdit::AddTreeNode { ref node, .. } = hw_edit {
            expected.push(ExpectedAuthoredObject {
                uuid: node.uuid.clone(),
                object_type: node.control_type.clone(),
                title: node.title.clone(),
                subsystem: "periphery_provisioning".to_string(),
            });
        }
        apply_edit_with_inverse(&mut project, &hw_edit).map_err(|e| anyhow::anyhow!(e))?;
    }

    // 6. Project Admin: To-Do, Autopilot Rule, Multiplier Unit
    let (todo_uuid, todo_edit) = build_add_todo_edit(
        &page_uuid,
        "E2E Commissioning Check",
        "Verify KNX & Tree bus termination",
        128.0,
        512.0,
    );
    expected.push(ExpectedAuthoredObject {
        uuid: todo_uuid,
        object_type: "Text".to_string(),
        title: "E2E Commissioning Check".to_string(),
        subsystem: "project_admin".to_string(),
    });
    apply_edit_with_inverse(&mut project, &todo_edit).map_err(|e| anyhow::anyhow!(e))?;

    if let Some((rule_uuid, rule_edit)) = build_autopilot_rule_edits(
        &project,
        &AutopilotRuleSpec {
            title: "E2E Night Lock Rule".to_string(),
            page_uuid: page_uuid.clone(),
            logic_op: "AND".to_string(),
            condition_source_uuids: vec![sw_uuid],
            target_block_uuid: lc_uuid,
            target_pin_key: "Dis".to_string(),
            px: 1472.0,
            py: 128.0,
        },
    ) {
        expected.push(ExpectedAuthoredObject {
            uuid: rule_uuid,
            object_type: "Autopilot".to_string(),
            title: "E2E Night Lock Rule".to_string(),
            subsystem: "project_admin".to_string(),
        });
        apply_edit_with_inverse(&mut project, &rule_edit).map_err(|e| anyhow::anyhow!(e))?;
    }

    let (mult_uuid, mult_edit) = build_upsert_multiplier_unit_edit(
        &project,
        &MultiplierUnitEntry {
            uuid: String::new(),
            title: "E2E Unit 101".to_string(),
            miniserver_serial: "504F94FFE2E1".to_string(),
            address: "10.10.1.101".to_string(),
            description: "Automated E2E Multiplier unit".to_string(),
            device_serial_overrides: Vec::new(),
        },
    );
    expected.push(ExpectedAuthoredObject {
        uuid: mult_uuid,
        object_type: "MultiplierUnit".to_string(),
        title: "E2E Unit 101".to_string(),
        subsystem: "project_admin".to_string(),
    });
    apply_edit_with_inverse(&mut project, &mult_edit).map_err(|e| anyhow::anyhow!(e))?;

    let manifest = MutationManifest {
        source_title: project.metadata.title.clone(),
        expected_objects: expected,
    };
    Ok((project, manifest))
}

/// Verifies that `mutated` round-trips losslessly through `serialize_loxone_xml` +
/// `parse_loxone_xml`, and optionally compares `mutated_path` with `config_saved_path`
/// saved by official Loxone Config 17.
pub fn verify_roundtrip(
    mutated: &LoxoneProject,
    manifest: &MutationManifest,
    mutated_path: Option<&Path>,
    config_saved_path: Option<&Path>,
) -> Result<RoundtripVerificationReport> {
    let xml = serialize_loxone_xml(mutated).context("Failed to serialize mutated project")?;
    let reparsed = parse_loxone_xml(&xml).context("Failed to re-parse serialized XML")?;
    let raw_roots = parse_raw_dom(&xml).context("Failed to parse raw DOM")?;

    let reparsed_uuids = collect_all_dom_uuids(&raw_roots);
    let mut internal_missing = Vec::new();
    for obj in &manifest.expected_objects {
        if !reparsed_uuids.contains(&obj.uuid) {
            internal_missing.push(obj.uuid.clone());
        }
    }

    let diags = lint_project(&reparsed);
    let linter_error_count = diags
        .iter()
        .filter(|d| d.severity == DiagnosticSeverity::Error)
        .count();

    let config_roundtrip = match (mutated_path, config_saved_path) {
        (Some(m_path), Some(c_path)) if c_path.exists() => Some(compare_with_loxone_config_save(
            m_path,
            c_path,
            manifest,
        )?),
        _ => None,
    };

    let internal_surviving = manifest
        .expected_objects
        .len()
        .saturating_sub(internal_missing.len());

    Ok(RoundtripVerificationReport {
        internal_roundtrip_passed: internal_missing.is_empty(),
        authored_count: manifest.expected_objects.len(),
        internal_surviving_count: internal_surviving,
        internal_missing_uuids: internal_missing,
        linter_error_count,
        config_roundtrip,
    })
}

pub fn compare_with_loxone_config_save(
    mutated_path: &Path,
    config_saved_path: &Path,
    manifest: &MutationManifest,
) -> Result<ConfigRoundtripReport> {
    let config_loaded = load_file(config_saved_path)?;
    let config_uuids = collect_all_dom_uuids(&config_loaded.roots);
    let config_titles = collect_all_dom_titles(&config_loaded.roots);

    let mut dropped = Vec::new();
    for obj in &manifest.expected_objects {
        if !config_uuids.contains(&obj.uuid) && !config_titles.contains(&obj.title) {
            dropped.push(obj.clone());
        }
    }

    let diff_report = diff_files(mutated_path, config_saved_path, &DiffOptions::default())?;
    let surviving_count = manifest
        .expected_objects
        .len()
        .saturating_sub(dropped.len());

    Ok(ConfigRoundtripReport {
        config_saved_path: config_saved_path.display().to_string(),
        config_accepted_all_authored: dropped.is_empty(),
        surviving_authored_count: surviving_count,
        dropped_authored_objects: dropped,
        diff_summary: diff_report.summary,
        config_added_nodes: diff_report.added,
        config_removed_nodes: diff_report.removed,
        config_normalized_nodes: diff_report.changed,
    })
}

fn collect_all_dom_uuids(roots: &[loxone_core::model::RawNode]) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    walk(roots, "", &mut |node, _| {
        if let Some(u) = node_uuid(node) {
            set.insert(u.to_string());
        }
    });
    set
}

fn collect_all_dom_titles(roots: &[loxone_core::model::RawNode]) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    walk(roots, "", &mut |node, _| {
        if let Some(t) = node.attr("Title") {
            set.insert(t.to_string());
        }
    });
    set
}

pub fn render_roundtrip_markdown(report: &RoundtripVerificationReport) -> String {
    let mut out = String::new();
    out.push_str("# E2E Round-Trip Compatibility Report\n\n");
    out.push_str(&format!(
        "- **Internal Lossless Round-Trip**: {}\n",
        if report.internal_roundtrip_passed {
            "PASS"
        } else {
            "FAIL"
        }
    ));
    out.push_str(&format!(
        "- **Authored Objects**: {}/{} preserved\n",
        report.internal_surviving_count, report.authored_count
    ));
    out.push_str(&format!(
        "- **Linter Error Diagnostics**: {}\n",
        report.linter_error_count
    ));

    if !report.internal_missing_uuids.is_empty() {
        out.push_str("\n## Missing After Internal Round-Trip\n\n");
        for u in &report.internal_missing_uuids {
            out.push_str(&format!("- `{u}`\n"));
        }
    }

    if let Some(cfg) = &report.config_roundtrip {
        out.push_str("\n## Loxone Config 17 Round-Trip Comparison\n\n");
        out.push_str(&format!(
            "- **Config Saved File**: `{}`\n",
            cfg.config_saved_path
        ));
        out.push_str(&format!(
            "- **Authored Objects Preserved by Loxone Config**: {}/{} ({})\n",
            cfg.surviving_authored_count,
            report.authored_count,
            if cfg.config_accepted_all_authored {
                "ALL ACCEPTED"
            } else {
                "DROPPED NODES DETECTED"
            }
        ));
        out.push_str(&format!(
            "- **Diff Summary (Mutated -> Config Re-Saved)**: +{} added, -{} removed, ~{} changed nodes ({} field changes)\n",
            cfg.diff_summary.added,
            cfg.diff_summary.removed,
            cfg.diff_summary.changed_nodes,
            cfg.diff_summary.field_changes
        ));

        if !cfg.dropped_authored_objects.is_empty() {
            out.push_str("\n### Authored Objects Dropped by Loxone Config\n\n");
            for d in &cfg.dropped_authored_objects {
                out.push_str(&format!(
                    "- `{}` (`{}` — **{}**, subsystem `{}`)\n",
                    d.uuid, d.object_type, d.title, d.subsystem
                ));
            }
        }

        if !cfg.config_normalized_nodes.is_empty() {
            out.push_str("\n### Attributes / Children Normalized by Loxone Config\n\n");
            for nd in cfg.config_normalized_nodes.iter().take(25) {
                out.push_str(&format!(
                    "- **{}** (`{}` at `{}`):\n",
                    nd.brief.title.as_deref().unwrap_or(""),
                    nd.brief.type_name.as_deref().unwrap_or(&nd.brief.tag),
                    nd.brief.path
                ));
                for ch in nd.changes.iter().take(8) {
                    out.push_str(&format!(
                        "  - `{}`: {:?} -> {:?} ({})\n",
                        ch.field, ch.old, ch.new, ch.note
                    ));
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comprehensive_mutation_and_internal_roundtrip() {
        let sample_path = Path::new("../../Sample.Loxone");
        if !sample_path.exists() {
            return;
        }
        let xml = std::fs::read_to_string(sample_path).expect("read Sample.Loxone");
        let base = parse_loxone_xml(&xml).expect("parse Sample.Loxone");
        let (mutated, manifest) =
            build_comprehensive_mutation_project(&base).expect("mutations should succeed");
        assert!(manifest.expected_objects.len() >= 10);

        let report =
            verify_roundtrip(&mutated, &manifest, None, None).expect("verification should succeed");
        assert!(
            report.internal_roundtrip_passed,
            "missing UUIDs: {:?}",
            report.internal_missing_uuids
        );
        assert_eq!(report.internal_surviving_count, report.authored_count);
    }
}
