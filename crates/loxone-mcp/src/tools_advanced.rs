use std::path::Path;

use loxone_core::advanced_blocks_config::{
    collect_audio_out_channels, collect_audio_zone_candidates, AudioFavoritePreset, AudioZoneConfig,
};
use loxone_core::efm_config::{EfmConfig, EfmNode, EfmNodeType};
use loxone_core::export::{
    export_component_bom_csv, page_to_svg, project_documentation_html, technical_report_html,
};
use loxone_core::light_config::{
    HclMode, LightCircuitType, LightControllerConfig, LightScene,
};
use loxone_core::linter::{
    export_project_analysis_csv, lint_project, AnalysisCategory, DiagnosticSeverity,
};
use loxone_core::object_config::{build_object_config_edits, build_object_config_rows};
use loxone_core::periphery_provisioning::{
    build_replace_device_edit, build_resolve_serial_conflicts_edit, collect_provisioned_hardware,
    detect_serial_number_conflicts,
};
use loxone_core::room_controller_config::{
    collect_energy_source_candidates, RoomControllerConfig, RoomTempMode,
};
use loxone_core::simulator::SimulatedDateTime;
use loxone_core::writer::{build_page_auto_layout_edits, ProjectEdit};
use serde_json::{json, Value};

use crate::helpers::{build_auto_config_plans, compute_find_and_replace};
use crate::protocol::JsonRpcResponse;
use crate::session::McpSession;

pub fn call_advanced_tool(
    session: &mut dyn McpSession,
    id: Option<Value>,
    tool_name: &str,
    args: &Value,
) -> JsonRpcResponse {
    match tool_name {
        "config_object_assignments_get" => {
            let Some(block_uuid) = args.get("block_uuid").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'block_uuid'");
            };
            let only_selected = args
                .get("only_selected")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let Some(proj) = session.project() else {
                return JsonRpcResponse::tool_err(id, "No project loaded");
            };
            let Some(page) = proj
                .pages
                .iter()
                .find(|p| p.blocks.iter().any(|b| b.uuid == block_uuid))
            else {
                return JsonRpcResponse::tool_err(
                    id,
                    format!("Block '{block_uuid}' not found on any page"),
                );
            };
            let rows = build_object_config_rows(proj, &page.uuid, block_uuid);
            let items: Vec<Value> = rows
                .into_iter()
                .filter(|r| !only_selected || r.is_input || r.is_output)
                .map(|r| {
                    json!({
                        "target_uuid": r.target_uuid,
                        "name": r.name,
                        "display_name": r.display_name,
                        "device_name": r.device_name,
                        "room_name": r.room_name,
                        "category_name": r.category_name,
                        "object_type": r.object_type,
                        "can_be_input": r.can_be_input,
                        "can_be_output": r.can_be_output,
                        "is_input": r.is_input,
                        "is_output": r.is_output
                    })
                })
                .collect();
            JsonRpcResponse::tool_ok(
                id,
                json!({
                    "block_uuid": block_uuid,
                    "page_uuid": page.uuid,
                    "count": items.len(),
                    "rows": items
                }),
            )
        }
        "config_object_assignments_set" => {
            let Some(block_uuid) = args.get("block_uuid").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'block_uuid'");
            };
            let Some(assignments) = args.get("assignments").and_then(Value::as_array) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'assignments'");
            };
            let edits = {
                let Some(proj) = session.project() else {
                    return JsonRpcResponse::tool_err(id, "No project loaded");
                };
                let Some(page) = proj
                    .pages
                    .iter()
                    .find(|p| p.blocks.iter().any(|b| b.uuid == block_uuid))
                else {
                    return JsonRpcResponse::tool_err(
                        id,
                        format!("Block '{block_uuid}' not found on any page"),
                    );
                };
                let mut rows = build_object_config_rows(proj, &page.uuid, block_uuid);
                for item in assignments {
                    if let Some(tu) = item.get("target_uuid").and_then(Value::as_str) {
                        if let Some(row) = rows.iter_mut().find(|r| r.target_uuid == tu) {
                            if let Some(inp) = item.get("is_input").and_then(Value::as_bool) {
                                if row.can_be_input || !inp {
                                    row.is_input = inp;
                                }
                            }
                            if let Some(out) = item.get("is_output").and_then(Value::as_bool) {
                                if row.can_be_output || !out {
                                    row.is_output = out;
                                }
                            }
                        }
                    }
                }
                build_object_config_edits(proj, &page.uuid, block_uuid, &rows)
            };
            let count = edits.len();
            if !edits.is_empty() {
                if let Err(e) = session.apply_edit(ProjectEdit::Batch(edits)) {
                    return JsonRpcResponse::tool_err(id, e);
                }
            }
            JsonRpcResponse::tool_ok(
                id,
                json!({
                    "block_uuid": block_uuid,
                    "edits_applied": count
                }),
            )
        }
        "config_light_controller_get" => {
            let Some(block_uuid) = args.get("block_uuid").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'block_uuid'");
            };
            let Some(proj) = session.project() else {
                return JsonRpcResponse::tool_err(id, "No project loaded");
            };
            let Some(block) = proj
                .pages
                .iter()
                .flat_map(|p| p.blocks.iter())
                .find(|b| b.uuid == block_uuid)
            else {
                return JsonRpcResponse::tool_err(id, format!("Block '{block_uuid}' not found"));
            };
            let cfg = LightControllerConfig::from_block(block);
            let circuits: Vec<Value> = cfg
                .circuits
                .iter()
                .map(|c| {
                    json!({
                        "index": c.index,
                        "code": c.code_label(),
                        "name": c.name,
                        "circuit_type": format!("{:?}", c.circuit_type)
                    })
                })
                .collect();
            let scenes: Vec<Value> = cfg
                .scenes
                .iter()
                .map(|s| {
                    json!({
                        "uuid": s.uuid,
                        "sid": s.sid,
                        "input_pin": s.input_label(),
                        "name": s.name,
                        "short_name": s.short_name,
                        "read_only": s.read_only,
                        "mix_mode": s.mix_mode,
                        "add_mode": s.add_mode,
                        "move_default": s.move_default,
                        "circuit_pcts": s.circuit_pcts
                    })
                })
                .collect();
            let rules: Vec<Value> = cfg
                .automation_rules
                .iter()
                .map(|r| {
                    json!({
                        "operating_mode_id": r.operating_mode_id,
                        "scene_sid": r.scene_sid,
                        "action": format!("{:?}", r.action)
                    })
                })
                .collect();
            JsonRpcResponse::tool_ok(
                id,
                json!({
                    "block_uuid": cfg.block_uuid,
                    "block_title": cfg.block_title,
                    "circuits": circuits,
                    "scenes": scenes,
                    "automation_rules": rules,
                    "hcl": {
                        "mode": format!("{:?}", cfg.hcl.mode),
                        "tmin_kelvin": cfg.hcl.tmin_kelvin,
                        "tmax_kelvin": cfg.hcl.tmax_kelvin,
                        "from_seconds": cfg.hcl.from_seconds,
                        "to_seconds": cfg.hcl.to_seconds
                    }
                }),
            )
        }
        "config_light_controller_set" => {
            let Some(block_uuid) = args.get("block_uuid").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'block_uuid'");
            };
            let edit = {
                let Some(proj) = session.project() else {
                    return JsonRpcResponse::tool_err(id, "No project loaded");
                };
                let Some(block) = proj
                    .pages
                    .iter()
                    .flat_map(|p| p.blocks.iter())
                    .find(|b| b.uuid == block_uuid)
                else {
                    return JsonRpcResponse::tool_err(
                        id,
                        format!("Block '{block_uuid}' not found"),
                    );
                };
                let mut cfg = LightControllerConfig::from_block(block);
                if let Some(circuits) = args.get("circuits").and_then(Value::as_array) {
                    for c_val in circuits {
                        let Some(idx) = c_val.get("index").and_then(Value::as_u64) else {
                            continue;
                        };
                        let idx = idx as usize;
                        if idx < 8 {
                            if let Some(nm) = c_val.get("name").and_then(Value::as_str) {
                                cfg.circuits[idx].name = nm.to_string();
                            }
                            if let Some(ct) = c_val.get("circuit_type").and_then(Value::as_str) {
                                cfg.circuits[idx].circuit_type = match ct {
                                    "Switch" => LightCircuitType::Switch,
                                    "Dimmer0To10V" => LightCircuitType::Dimmer0To10V,
                                    "Dimmer1To10V" => LightCircuitType::Dimmer1To10V,
                                    "Rgb" => LightCircuitType::Rgb,
                                    "LumitechDmx" => LightCircuitType::LumitechDmx,
                                    "SmartActuator" => LightCircuitType::SmartActuator,
                                    _ => LightCircuitType::Dimmer0To100,
                                };
                            }
                        }
                    }
                }
                if let Some(scenes) = args.get("scenes").and_then(Value::as_array) {
                    for s_val in scenes {
                        let sid = s_val
                            .get("sid")
                            .and_then(Value::as_i64)
                            .map(|v| v as i32)
                            .unwrap_or_else(|| cfg.next_available_sid());
                        let name = s_val
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("Scene");
                        let existing_pos = cfg.scenes.iter().position(|s| s.sid == sid);
                        let scene_ref = if let Some(pos) = existing_pos {
                            &mut cfg.scenes[pos]
                        } else {
                            let new_sc = LightScene::new_user_scene(name, sid, &cfg.circuits);
                            let insert_pos = cfg
                                .scenes
                                .iter()
                                .position(|s| s.sid == 778)
                                .unwrap_or(cfg.scenes.len());
                            cfg.scenes.insert(insert_pos, new_sc);
                            &mut cfg.scenes[insert_pos]
                        };
                        if let Some(nm) = s_val.get("name").and_then(Value::as_str) {
                            scene_ref.name = nm.to_string();
                        }
                        if let Some(sn) = s_val.get("short_name").and_then(Value::as_str) {
                            scene_ref.short_name = sn.to_string();
                        }
                        if let Some(mm) = s_val.get("mix_mode").and_then(Value::as_bool) {
                            scene_ref.mix_mode = mm;
                        }
                        if let Some(am) = s_val.get("add_mode").and_then(Value::as_bool) {
                            scene_ref.add_mode = am;
                        }
                        if let Some(mv) = s_val.get("move_default").and_then(Value::as_bool) {
                            scene_ref.move_default = mv;
                        }
                        if let Some(pcts) = s_val.get("circuit_pcts").and_then(Value::as_array) {
                            for (i, pv) in pcts.iter().enumerate().take(8) {
                                if let Some(n) = pv.as_u64() {
                                    scene_ref.circuit_pcts[i] = n.min(100) as u8;
                                }
                            }
                        }
                    }
                }
                if let Some(hcl) = args.get("hcl").and_then(Value::as_object) {
                    if let Some(m) = hcl.get("mode").and_then(Value::as_str) {
                        cfg.hcl.mode = if m.eq_ignore_ascii_case("FixedPeriod") {
                            HclMode::FixedPeriod
                        } else {
                            HclMode::SunriseSunset
                        };
                    }
                    if let Some(tmin) = hcl.get("tmin_kelvin").and_then(Value::as_u64) {
                        cfg.hcl.tmin_kelvin = tmin as u32;
                    }
                    if let Some(tmax) = hcl.get("tmax_kelvin").and_then(Value::as_u64) {
                        cfg.hcl.tmax_kelvin = tmax as u32;
                    }
                    if let Some(fs) = hcl.get("from_seconds").and_then(Value::as_u64) {
                        cfg.hcl.from_seconds = fs as u32;
                    }
                    if let Some(ts) = hcl.get("to_seconds").and_then(Value::as_u64) {
                        cfg.hcl.to_seconds = ts as u32;
                    }
                }
                cfg.to_project_edits(block)
            };
            match session.apply_edit(edit) {
                Ok(()) => JsonRpcResponse::tool_ok(id, json!({ "block_uuid": block_uuid, "updated": true })),
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        _ => call_advanced_tool_part2(session, id, tool_name, args),
    }
}

fn parse_temp_mode(s: &str) -> RoomTempMode {
    match s {
        "Comfort" | "1" => RoomTempMode::Comfort,
        "BuildingProtection" | "2" => RoomTempMode::BuildingProtection,
        "FixedValue" | "3" => RoomTempMode::FixedValue,
        _ => RoomTempMode::Economy,
    }
}

fn efm_node_to_json(node: &EfmNode) -> Value {
    let children: Vec<Value> = node.children.iter().map(efm_node_to_json).collect();
    json!({
        "uuid": node.uuid,
        "node_type": format!("{:?}", node.node_type),
        "title": node.title,
        "ctrl_uuid": node.ctrl_uuid,
        "calc_rest": node.calc_rest,
        "rest_name": node.rest_name,
        "children": children
    })
}

fn call_advanced_tool_part2(
    session: &mut dyn McpSession,
    id: Option<Value>,
    tool_name: &str,
    args: &Value,
) -> JsonRpcResponse {
    match tool_name {
        "config_room_controller_get" => {
            let Some(block_uuid) = args.get("block_uuid").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'block_uuid'");
            };
            let Some(proj) = session.project() else {
                return JsonRpcResponse::tool_err(id, "No project loaded");
            };
            let Some(block) = proj
                .pages
                .iter()
                .flat_map(|p| p.blocks.iter())
                .find(|b| b.uuid == block_uuid)
            else {
                return JsonRpcResponse::tool_err(id, format!("Block '{block_uuid}' not found"));
            };
            let cfg = RoomControllerConfig::from_block(proj, block);
            let groups: Vec<Value> = cfg
                .groups
                .iter()
                .map(|g| {
                    json!({
                        "object_uuid": g.object_uuid,
                        "src_name": g.src_name,
                        "display_name": g.display_name,
                        "object_type_label": g.object_type_label,
                        "heating": g.heating,
                        "cooling": g.cooling,
                        "pwm": g.pwm
                    })
                })
                .collect();
            let rows: Vec<Value> = cfg
                .schedule
                .rows
                .iter()
                .enumerate()
                .map(|(ix, r)| {
                    json!({
                        "row_ix": ix,
                        "mode_uuid": r.mode_uuid,
                        "label": r.label,
                        "is_user_mode": r.is_user_mode
                    })
                })
                .collect();
            let entries: Vec<Value> = cfg
                .schedule
                .entries
                .iter()
                .map(|e| {
                    json!({
                        "row_ix": e.row_ix,
                        "from_min": e.from_min,
                        "to_min": e.to_min,
                        "mode": format!("{:?}", e.mode)
                    })
                })
                .collect();
            JsonRpcResponse::tool_ok(
                id,
                json!({
                    "block_uuid": block.uuid,
                    "block_title": block.title,
                    "groups": groups,
                    "prio_h": cfg.prio_h,
                    "prio_c": cfg.prio_c,
                    "schedule": {
                        "default_mode": format!("{:?}", cfg.schedule.default_mode),
                        "rows": rows,
                        "entries": entries
                    }
                }),
            )
        }
        "config_room_controller_set" => {
            let Some(block_uuid) = args.get("block_uuid").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'block_uuid'");
            };
            let edits = {
                let Some(proj) = session.project() else {
                    return JsonRpcResponse::tool_err(id, "No project loaded");
                };
                let Some(block) = proj
                    .pages
                    .iter()
                    .flat_map(|p| p.blocks.iter())
                    .find(|b| b.uuid == block_uuid)
                else {
                    return JsonRpcResponse::tool_err(
                        id,
                        format!("Block '{block_uuid}' not found"),
                    );
                };
                let mut cfg = RoomControllerConfig::from_block(proj, block);
                if let Some(dm) = args.get("default_mode").and_then(Value::as_str) {
                    cfg.schedule.default_mode = parse_temp_mode(dm);
                }
                if let Some(intervals) = args.get("set_intervals").and_then(Value::as_array) {
                    for iv in intervals {
                        let row_ix = iv.get("row_ix").and_then(Value::as_u64).unwrap_or(0) as usize;
                        let from_min =
                            iv.get("from_min").and_then(Value::as_u64).unwrap_or(0) as u16;
                        let to_min =
                            iv.get("to_min").and_then(Value::as_u64).unwrap_or(1440) as u16;
                        let mode = iv
                            .get("mode")
                            .and_then(Value::as_str)
                            .map(parse_temp_mode)
                            .unwrap_or(RoomTempMode::Comfort);
                        cfg.schedule.set_interval(row_ix, from_min, to_min, mode);
                    }
                }
                if let Some(add_srcs) =
                    args.get("add_energy_source_uuids").and_then(Value::as_array)
                {
                    let cands = collect_energy_source_candidates(proj, &[]);
                    for src_v in add_srcs {
                        if let Some(u) = src_v.as_str() {
                            if let Some(c) = cands.iter().find(|c| c.object_uuid == u) {
                                cfg.add_group(c);
                            }
                        }
                    }
                }
                cfg.to_edits(block)
            };
            let count = edits.len();
            if !edits.is_empty() {
                if let Err(e) = session.apply_edit(ProjectEdit::Batch(edits)) {
                    return JsonRpcResponse::tool_err(id, e);
                }
            }
            JsonRpcResponse::tool_ok(
                id,
                json!({
                    "block_uuid": block_uuid,
                    "edits_applied": count
                }),
            )
        }
        "config_efm_get" => {
            let Some(block_uuid) = args.get("block_uuid").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'block_uuid'");
            };
            let Some(proj) = session.project() else {
                return JsonRpcResponse::tool_err(id, "No project loaded");
            };
            let Some(block) = proj
                .pages
                .iter()
                .flat_map(|p| p.blocks.iter())
                .find(|b| b.uuid == block_uuid)
            else {
                return JsonRpcResponse::tool_err(id, format!("Block '{block_uuid}' not found"));
            };
            let cfg = EfmConfig::from_block(block);
            JsonRpcResponse::tool_ok(
                id,
                json!({
                    "block_uuid": cfg.block_uuid,
                    "unit_power": cfg.unit_power,
                    "unit_energy": cfg.unit_energy,
                    "stat_power_interval": format!("{:?}", cfg.stat_power_interval),
                    "stat_energy_interval": format!("{:?}", cfg.stat_energy_interval),
                    "stat_self_sufficiency_interval": format!("{:?}", cfg.stat_self_sufficiency_interval),
                    "root": efm_node_to_json(&cfg.root)
                }),
            )
        }
        "config_efm_set" => {
            let Some(block_uuid) = args.get("block_uuid").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'block_uuid'");
            };
            let edit = {
                let Some(proj) = session.project() else {
                    return JsonRpcResponse::tool_err(id, "No project loaded");
                };
                let Some(block) = proj
                    .pages
                    .iter()
                    .flat_map(|p| p.blocks.iter())
                    .find(|b| b.uuid == block_uuid)
                else {
                    return JsonRpcResponse::tool_err(
                        id,
                        format!("Block '{block_uuid}' not found"),
                    );
                };
                let mut cfg = EfmConfig::from_block(block);
                if let Some(up) = args.get("unit_power").and_then(Value::as_str) {
                    cfg.unit_power = up.to_string();
                }
                if let Some(ue) = args.get("unit_energy").and_then(Value::as_str) {
                    cfg.unit_energy = ue.to_string();
                }
                if let Some(children) = args.get("add_children").and_then(Value::as_array) {
                    for c_val in children {
                        let title = c_val
                            .get("title")
                            .and_then(Value::as_str)
                            .unwrap_or("Meter");
                        let nt = match c_val.get("node_type").and_then(Value::as_str) {
                            Some("Grid") => EfmNodeType::Grid,
                            Some("Battery") => EfmNodeType::Battery,
                            Some("Production") => EfmNodeType::Production,
                            Some("Group") => EfmNodeType::Group,
                            _ => EfmNodeType::Consumer,
                        };
                        let ctrl = c_val
                            .get("ctrl_uuid")
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        cfg.root.children.push(EfmNode::new_child(title, nt, ctrl));
                    }
                }
                cfg.to_project_edits(block)
            };
            match session.apply_edit(edit) {
                Ok(()) => JsonRpcResponse::tool_ok(
                    id,
                    json!({ "block_uuid": block_uuid, "updated": true }),
                ),
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        "config_audio_zone_get" => {
            let Some(block_uuid) = args.get("block_uuid").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'block_uuid'");
            };
            let Some(proj) = session.project() else {
                return JsonRpcResponse::tool_err(id, "No project loaded");
            };
            let Some(block) = proj
                .pages
                .iter()
                .flat_map(|p| p.blocks.iter())
                .find(|b| b.uuid == block_uuid)
            else {
                return JsonRpcResponse::tool_err(id, format!("Block '{block_uuid}' not found"));
            };
            let cfg = AudioZoneConfig::from_block(block, proj);
            let all_channels = collect_audio_out_channels(proj);
            let zone_candidates = collect_audio_zone_candidates(proj, block_uuid);

            let channels_json: Vec<Value> = all_channels
                .iter()
                .map(|ch| {
                    json!({
                        "uuid": ch.uuid,
                        "title": ch.title,
                        "parent_title": ch.parent_title,
                        "assigned": cfg.is_channel_assigned(&ch.uuid),
                        "max_vol": cfg.channel_max_vol(ch),
                        "line_out_vol": cfg.channel_line_out_vol(ch),
                        "gain_db": cfg.channel_gain(ch)
                    })
                })
                .collect();
            let favorites_json: Vec<Value> = cfg
                .favorites
                .iter()
                .map(|f| {
                    json!({
                        "slot": f.slot,
                        "title": f.title,
                        "source_uri": f.source_uri
                    })
                })
                .collect();
            let candidates_json: Vec<Value> = zone_candidates
                .iter()
                .map(|c| {
                    json!({
                        "uuid": c.uuid,
                        "title": c.title,
                        "block_type": c.block_type,
                        "room_name": c.room_name,
                        "grouped": cfg.is_player_grouped(&c.uuid)
                    })
                })
                .collect();

            JsonRpcResponse::tool_ok(
                id,
                json!({
                    "block_uuid": block.uuid,
                    "title": block.title,
                    "block_type": block.block_type,
                    "speaker_cfg": cfg.speaker_cfg,
                    "presence_enabled": cfg.presence_enabled,
                    "airplay_enabled": cfg.airplay_enabled,
                    "spotify_enabled": cfg.spotify_enabled,
                    "high_pass_filter": cfg.high_pass_filter,
                    "default_vol": cfg.default_vol,
                    "max_vol": cfg.max_vol,
                    "step_vol": cfg.step_vol,
                    "alarm_vol": cfg.alarm_vol,
                    "bell_vol": cfg.bell_vol,
                    "buzzer_vol": cfg.buzzer_vol,
                    "tts_vol": cfg.tts_vol,
                    "assigned_channel_uuids": cfg.assigned_channel_uuids,
                    "master_player_uuid": cfg.master_player_uuid,
                    "grouped_player_uuids": cfg.grouped_player_uuids,
                    "favorites": favorites_json,
                    "available_channels": channels_json,
                    "available_zone_candidates": candidates_json
                }),
            )
        }
        "config_audio_zone_set" => {
            let Some(block_uuid) = args.get("block_uuid").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'block_uuid'");
            };
            let edit = {
                let Some(proj) = session.project() else {
                    return JsonRpcResponse::tool_err(id, "No project loaded");
                };
                let Some((page_uuid, block)) = proj.pages.iter().find_map(|p| {
                    p.blocks
                        .iter()
                        .find(|b| b.uuid == block_uuid)
                        .map(|b| (p.uuid.clone(), b))
                }) else {
                    return JsonRpcResponse::tool_err(id, format!("Block '{block_uuid}' not found"));
                };
                let all_channels = collect_audio_out_channels(proj);
                let mut cfg = AudioZoneConfig::from_block(block, proj);

                if let Some(v) = args.get("speaker_cfg").and_then(Value::as_u64) {
                    cfg.speaker_cfg = v as u32;
                }
                if let Some(v) = args.get("presence_enabled").and_then(Value::as_bool) {
                    cfg.presence_enabled = v;
                }
                if let Some(v) = args.get("airplay_enabled").and_then(Value::as_bool) {
                    cfg.airplay_enabled = v;
                }
                if let Some(v) = args.get("spotify_enabled").and_then(Value::as_bool) {
                    cfg.spotify_enabled = v;
                }
                if let Some(v) = args.get("high_pass_filter").and_then(Value::as_bool) {
                    cfg.high_pass_filter = v;
                }
                if let Some(v) = args.get("default_vol").and_then(Value::as_u64) {
                    cfg.default_vol = v as u32;
                }
                if let Some(v) = args.get("max_vol").and_then(Value::as_u64) {
                    cfg.max_vol = v as u32;
                }
                if let Some(v) = args.get("step_vol").and_then(Value::as_u64) {
                    cfg.step_vol = v as u32;
                }
                if let Some(v) = args.get("alarm_vol").and_then(Value::as_u64) {
                    cfg.alarm_vol = v as u32;
                }
                if let Some(v) = args.get("bell_vol").and_then(Value::as_u64) {
                    cfg.bell_vol = v as u32;
                }
                if let Some(v) = args.get("buzzer_vol").and_then(Value::as_u64) {
                    cfg.buzzer_vol = v as u32;
                }
                if let Some(v) = args.get("tts_vol").and_then(Value::as_u64) {
                    cfg.tts_vol = v as u32;
                }
                if let Some(arr) = args.get("assigned_channel_uuids").and_then(Value::as_array) {
                    cfg.assigned_channel_uuids = arr
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect();
                }
                if let Some(mst) = args.get("master_player_uuid") {
                    cfg.master_player_uuid = mst
                        .as_str()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string);
                }
                if let Some(arr) = args.get("grouped_player_uuids").and_then(Value::as_array) {
                    cfg.grouped_player_uuids = arr
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect();
                }
                if let Some(favs) = args.get("favorites").and_then(Value::as_array) {
                    cfg.favorites = favs
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, f)| {
                            let title = f.get("title")?.as_str()?.to_string();
                            let source_uri = f.get("source_uri")?.as_str()?.to_string();
                            let slot = f
                                .get("slot")
                                .and_then(Value::as_u64)
                                .map(|s| s as u32)
                                .unwrap_or((idx as u32) + 1);
                            Some(AudioFavoritePreset {
                                slot,
                                title,
                                source_uri,
                            })
                        })
                        .collect();
                }
                cfg.to_project_edit(&page_uuid, block_uuid, &all_channels)
            };
            match session.apply_edit(edit) {
                Ok(()) => JsonRpcResponse::tool_ok(
                    id,
                    json!({ "block_uuid": block_uuid, "updated": true }),
                ),
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        "tree_replace_device" => {
            let (Some(device_uuid), Some(new_serial)) = (
                args.get("device_uuid").and_then(Value::as_str),
                args.get("new_serial").and_then(Value::as_str),
            ) else {
                return JsonRpcResponse::tool_err(
                    id,
                    "Missing required parameters 'device_uuid' and 'new_serial'",
                );
            };
            let description = args.get("description").and_then(Value::as_str);
            let distribution_board = args.get("distribution_board").and_then(Value::as_str);
            let row_position = args.get("row_position").and_then(Value::as_str);
            let lighting_group = args.get("lighting_group").and_then(Value::as_str);

            let edit = build_replace_device_edit(
                device_uuid,
                new_serial,
                description,
                distribution_board,
                row_position,
                lighting_group,
            );
            if let Err(e) = session.apply_edit(edit) {
                return JsonRpcResponse::tool_err(id, e);
            }
            let updated_dev = session.project().and_then(|proj| {
                collect_provisioned_hardware(proj)
                    .into_iter()
                    .find(|d| d.uuid == device_uuid)
            });
            JsonRpcResponse::tool_ok(
                id,
                json!({
                    "device_uuid": device_uuid,
                    "serial": updated_dev.as_ref().map(|d| d.serial.as_str()).unwrap_or(new_serial),
                    "description": updated_dev.as_ref().map(|d| d.description.as_str()).unwrap_or(""),
                    "distribution_board": updated_dev.as_ref().map(|d| d.distribution_board.as_str()).unwrap_or(""),
                    "row_position": updated_dev.as_ref().map(|d| d.row_position.as_str()).unwrap_or(""),
                    "lighting_group": updated_dev.as_ref().map(|d| d.lighting_group.as_str()).unwrap_or(""),
                    "updated": true
                }),
            )
        }
        "tree_resolve_serial_conflicts" => {
            let preview_only = args
                .get("preview_only")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let (conflicts_json, reassignments, batch_opt) = {
                let Some(proj) = session.project() else {
                    return JsonRpcResponse::tool_err(id, "No project loaded");
                };
                let conflicts: Vec<Value> = detect_serial_number_conflicts(proj)
                    .into_iter()
                    .map(|c| {
                        json!({
                            "serial": c.serial,
                            "device_uuids": c.device_uuids,
                            "device_titles": c.device_titles
                        })
                    })
                    .collect();
                let (reassignments, batch) = build_resolve_serial_conflicts_edit(proj);
                (conflicts, reassignments, batch)
            };
            if !preview_only {
                if let Some(batch) = batch_opt {
                    if let Err(e) = session.apply_edit(batch) {
                        return JsonRpcResponse::tool_err(id, e);
                    }
                }
            }
            let reassigned_json: Vec<Value> = reassignments
                .iter()
                .map(|(uuid, old_sn, new_sn)| {
                    json!({
                        "device_uuid": uuid,
                        "old_serial": old_sn,
                        "new_serial": new_sn
                    })
                })
                .collect();
            JsonRpcResponse::tool_ok(
                id,
                json!({
                    "preview_only": preview_only,
                    "conflict_groups": conflicts_json,
                    "resolved_count": reassigned_json.len(),
                    "reassignments": reassigned_json
                }),
            )
        }
        "page_auto_layout" => {
            let Some(page_uuid) = args.get("page_uuid").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'page_uuid'");
            };
            let layout_opt = {
                let Some(proj) = session.project() else {
                    return JsonRpcResponse::tool_err(id, "No project loaded");
                };
                build_page_auto_layout_edits(proj, page_uuid)
            };
            let Some((moved_count, batch)) = layout_opt else {
                return JsonRpcResponse::tool_err(
                    id,
                    format!("Page '{page_uuid}' not found or has no blocks"),
                );
            };
            if moved_count > 0 {
                if let Err(e) = session.apply_edit(batch) {
                    return JsonRpcResponse::tool_err(id, e);
                }
            }
            JsonRpcResponse::tool_ok(
                id,
                json!({
                    "page_uuid": page_uuid,
                    "moved_blocks": moved_count
                }),
            )
        }
        "wizard_auto_configure_rooms" => {
            let preview_only = args
                .get("preview_only")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let filter_rooms: Option<Vec<String>> = args
                .get("room_uuids")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                });
            let gen_lighting = args.get("gen_lighting").and_then(Value::as_bool);
            let gen_shading = args.get("gen_shading").and_then(Value::as_bool);
            let gen_climate = args.get("gen_climate").and_then(Value::as_bool);
            let gen_audio = args.get("gen_audio").and_then(Value::as_bool);

            let (plans, batch_opt, generated_blocks) = {
                let Some(proj) = session.project() else {
                    return JsonRpcResponse::tool_err(id, "No project loaded");
                };
                build_auto_config_plans(
                    proj,
                    filter_rooms.as_deref(),
                    gen_lighting,
                    gen_shading,
                    gen_climate,
                    gen_audio,
                )
            };
            if !preview_only {
                if let Some(batch) = batch_opt {
                    if let Err(e) = session.apply_edit(batch) {
                        return JsonRpcResponse::tool_err(id, e);
                    }
                }
            }
            JsonRpcResponse::tool_ok(
                id,
                json!({
                    "preview_only": preview_only,
                    "generated_blocks": generated_blocks,
                    "rooms": plans
                }),
            )
        }
        "bulk_find_and_replace" => {
            let (Some(query), Some(replacement)) = (
                args.get("query").and_then(Value::as_str),
                args.get("replacement").and_then(Value::as_str),
            ) else {
                return JsonRpcResponse::tool_err(id, "Missing 'query' or 'replacement'");
            };
            let preview_only = args
                .get("preview_only")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let case_sensitive = args
                .get("case_sensitive")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let in_titles = args
                .get("in_titles")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let in_descriptions = args
                .get("in_descriptions")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let in_attributes = args
                .get("in_attributes")
                .and_then(Value::as_bool)
                .unwrap_or(false);

            let (matches, batch_opt) = {
                let Some(proj) = session.project() else {
                    return JsonRpcResponse::tool_err(id, "No project loaded");
                };
                compute_find_and_replace(
                    proj,
                    query,
                    replacement,
                    case_sensitive,
                    in_titles,
                    in_descriptions,
                    in_attributes,
                )
            };
            if !preview_only {
                if let Some(batch) = batch_opt {
                    if let Err(e) = session.apply_edit(batch) {
                        return JsonRpcResponse::tool_err(id, e);
                    }
                }
            }
            JsonRpcResponse::tool_ok(
                id,
                json!({
                    "preview_only": preview_only,
                    "match_count": matches.len(),
                    "matches": matches
                }),
            )
        }
        "analysis_run" => {
            let Some(proj) = session.project() else {
                return JsonRpcResponse::tool_err(id, "No project loaded");
            };
            let cat_filter = args
                .get("category")
                .and_then(Value::as_str)
                .map(str::to_lowercase);
            let sev_filter = args
                .get("severity")
                .and_then(Value::as_str)
                .map(str::to_lowercase);

            let diags = lint_project(proj);
            let mut cat_counts = serde_json::Map::new();
            for cat in AnalysisCategory::ALL {
                let cnt = diags.iter().filter(|d| d.category() == cat).count();
                cat_counts.insert(cat.label().to_string(), json!(cnt));
            }

            let items: Vec<Value> = diags
                .into_iter()
                .filter(|d| {
                    if let Some(cf) = &cat_filter {
                        if !d.category().label().to_lowercase().contains(cf) {
                            return false;
                        }
                    }
                    if let Some(sf) = &sev_filter {
                        let s_str = match d.severity {
                            DiagnosticSeverity::Error => "error",
                            DiagnosticSeverity::Warning => "warning",
                            DiagnosticSeverity::Info => "info",
                        };
                        if s_str != sf {
                            return false;
                        }
                    }
                    true
                })
                .map(|d| {
                    let sev = match d.severity {
                        DiagnosticSeverity::Error => "Error",
                        DiagnosticSeverity::Warning => "Warning",
                        DiagnosticSeverity::Info => "Info",
                    };
                    json!({
                        "category": d.category().label(),
                        "severity": sev,
                        "code": d.code,
                        "page_index": d.page_index,
                        "page_title": d.page_title,
                        "block_uuid": d.block_uuid,
                        "block_title": d.block_title,
                        "block_type": d.block_type,
                        "message": d.message
                    })
                })
                .collect();

            JsonRpcResponse::tool_ok(
                id,
                json!({
                    "total_findings": items.len(),
                    "category_counts": cat_counts,
                    "diagnostics": items
                }),
            )
        }
        "sim_control" => {
            let Some(action) = args.get("action").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'action'");
            };
            let proj_clone = {
                let Some(proj) = session.project() else {
                    return JsonRpcResponse::tool_err(id, "No project loaded");
                };
                proj.clone()
            };
            match action {
                "start" => {
                    session.simulator_mut().start(&proj_clone);
                }
                "stop" => {
                    session.simulator_mut().stop();
                }
                "reset" => {
                    session.simulator_mut().reset(&proj_clone);
                }
                "step" => {
                    let secs = args.get("seconds").and_then(Value::as_i64).unwrap_or(1);
                    if secs == 1 {
                        session.simulator_mut().step_tick(&proj_clone);
                    } else {
                        session.simulator_mut().advance_seconds(&proj_clone, secs);
                    }
                }
                "set_time" => {
                    let Some(iso) = args.get("iso_time").and_then(Value::as_str) else {
                        return JsonRpcResponse::tool_err(id, "Missing 'iso_time'");
                    };
                    let Some(dt) = SimulatedDateTime::parse_iso(iso) else {
                        return JsonRpcResponse::tool_err(
                            id,
                            format!("Invalid iso_time '{iso}'. Expected YYYY-MM-DD HH:MM:SS"),
                        );
                    };
                    session.simulator_mut().set_simulated_time(&proj_clone, dt);
                }
                "set_input" => {
                    let (Some(ctrl_uuid), Some(val)) = (
                        args.get("control_uuid").and_then(Value::as_str),
                        args.get("value").and_then(Value::as_str),
                    ) else {
                        return JsonRpcResponse::tool_err(
                            id,
                            "Missing 'control_uuid' or 'value' for set_input",
                        );
                    };
                    let pin_key = args.get("pin_key").and_then(Value::as_str);
                    session
                        .simulator_mut()
                        .apply_command(&proj_clone, ctrl_uuid, pin_key, val);
                }
                "status" => {}
                other => {
                    return JsonRpcResponse::tool_err(
                        id,
                        format!("Unknown sim_control action '{other}'"),
                    );
                }
            }

            let sim = session.simulator();
            let page_uuid = args
                .get("page_uuid")
                .and_then(Value::as_str)
                .or_else(|| proj_clone.pages.first().map(|p| p.uuid.as_str()));
            let mut page_values = Vec::new();
            if let Some(pu) = page_uuid {
                if let Some(page) = proj_clone.pages.iter().find(|p| p.uuid == pu) {
                    for blk in &page.blocks {
                        if blk.is_note() {
                            continue;
                        }
                        let ctrl_val = sim
                            .live_store
                            .controls
                            .get(&blk.uuid)
                            .and_then(|s| s.primary_value.as_deref())
                            .unwrap_or("0");
                        let mut pins_map = serde_json::Map::new();
                        for pin in &blk.pins {
                            let pk = (blk.uuid.clone(), pin.key.clone());
                            if let Some(v) = sim.live_store.pin_overrides.get(&pk) {
                                pins_map.insert(pin.key.clone(), Value::String(v.clone()));
                            }
                        }
                        page_values.push(json!({
                            "block_uuid": blk.uuid,
                            "title": blk.title,
                            "block_type": blk.block_type,
                            "value": ctrl_val,
                            "pins": pins_map
                        }));
                    }
                }
            }

            JsonRpcResponse::tool_ok(
                id,
                json!({
                    "enabled": sim.enabled,
                    "tick_count": sim.tick_count,
                    "simulated_time": sim.simulated_time.format_iso(),
                    "forced_inputs": sim.forced_inputs,
                    "page_values": page_values
                }),
            )
        }
        "report_export" => {
            let Some(format_str) = args.get("format").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'format'");
            };
            let Some(proj) = session.project() else {
                return JsonRpcResponse::tool_err(id, "No project loaded");
            };
            let (mime, content) = match format_str {
                "page_svg" => {
                    let page_opt = if let Some(pu) = args.get("page_uuid").and_then(Value::as_str) {
                        proj.pages.iter().find(|p| p.uuid == pu)
                    } else if let Some(idx) = args.get("page_index").and_then(Value::as_u64) {
                        proj.pages.get(idx as usize)
                    } else {
                        proj.pages.first()
                    };
                    let Some(page) = page_opt else {
                        return JsonRpcResponse::tool_err(id, "Page not found for page_svg");
                    };
                    ("image/svg+xml", page_to_svg(proj, page))
                }
                "documentation_html" => ("text/html", project_documentation_html(proj)),
                "technical_report_html" => ("text/html", technical_report_html(proj)),
                "project_analysis_csv" => {
                    let diags = lint_project(proj);
                    ("text/csv", export_project_analysis_csv(proj, &diags))
                }
                "component_bom_csv" => ("text/csv", export_component_bom_csv(proj)),
                other => {
                    return JsonRpcResponse::tool_err(
                        id,
                        format!("Unsupported report format '{other}'"),
                    );
                }
            };
            let mut written_to = None;
            if let Some(out_path) = args.get("output_path").and_then(Value::as_str) {
                let p = Path::new(out_path);
                if let Some(parent) = p.parent() {
                    if !parent.as_os_str().is_empty() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                }
                if let Err(e) = std::fs::write(p, &content) {
                    return JsonRpcResponse::tool_err(
                        id,
                        format!("Failed to write {}: {}", p.display(), e),
                    );
                }
                written_to = Some(p.display().to_string());
            }
            JsonRpcResponse::tool_ok(
                id,
                json!({
                    "format": format_str,
                    "mime_type": mime,
                    "bytes": content.len(),
                    "output_path": written_to,
                    "content": content
                }),
            )
        }
        other => JsonRpcResponse::tool_err(id, format!("Unknown tool: {other}")),
    }
}
