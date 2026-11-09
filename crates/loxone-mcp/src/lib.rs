#![recursion_limit = "512"]

pub mod helpers;
pub mod protocol;
pub mod session;
pub mod tools_advanced;
pub mod transport;

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::{json, Value};

    use crate::protocol::JsonRpcRequest;
    use crate::session::{handle_jsonrpc_request, HeadlessSession, McpSession};

    fn call_tool(session: &mut dyn McpSession, name: &str, arguments: Value) -> Value {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": name,
                "arguments": arguments
            })),
        };
        let resp = handle_jsonrpc_request(session, &req).expect("response");
        assert!(
            resp.error.is_none(),
            "JSON-RPC error on {name}: {:?}",
            resp.error
        );
        let result = resp.result.expect("result");
        assert_eq!(
            result.get("isError").and_then(Value::as_bool),
            Some(false),
            "Tool {name} returned isError=true: {}",
            result
        );
        result
            .get("structuredContent")
            .cloned()
            .unwrap_or(result)
    }

    #[test]
    fn test_mcp_initialize_and_tool_catalog() {
        let mut session = HeadlessSession::default();
        let init_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: Some(json!({})),
        };
        let init_resp = handle_jsonrpc_request(&mut session, &init_req).unwrap();
        let res = init_resp.result.unwrap();
        assert_eq!(res["serverInfo"]["name"], "loxone-mcp");

        let list_req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/list".to_string(),
            params: None,
        };
        let list_resp = handle_jsonrpc_request(&mut session, &list_req).unwrap();
        let tools = list_resp.result.unwrap()["tools"]
            .as_array()
            .unwrap()
            .clone();
        assert!(
            tools.len() >= 30,
            "Expected at least 30 MCP tools, got {}",
            tools.len()
        );
    }

    #[test]
    fn test_headless_end_to_end_authoring_wiring_simulation_and_undo() {
        let mut session = HeadlessSession::default();

        // 1. Create a clean project
        let status = call_tool(
            &mut session,
            "project_create",
            json!({ "title": "MCP Smart Villa" }),
        );
        assert_eq!(status["loaded"], true);
        assert_eq!(status["metadata"]["title"], "MCP Smart Villa");

        // 2. Create a room and a page
        let room = call_tool(
            &mut session,
            "room_create",
            json!({ "title": "Kitchen", "sqm": "24" }),
        );
        let room_uuid = room["room_uuid"].as_str().unwrap().to_string();

        let page = call_tool(
            &mut session,
            "page_create",
            json!({ "title": "Kitchen Logic", "room_uuid": room_uuid }),
        );
        let page_uuid = page["page_uuid"].as_str().unwrap().to_string();

        // 3. Provision a Touch Pure Tree device in the Periphery tree
        let dev = call_tool(
            &mut session,
            "tree_add_device",
            json!({
                "template": "TouchPureTree",
                "title": "Kitchen Touch",
                "room_uuid": room_uuid
            }),
        );
        let touch_child_uuid = dev["children"][0]["uuid"].as_str().unwrap().to_string();

        // 4. Place an InputRef for the Touch sensor and a Switch + LightController2 on Kitchen Logic
        let in_ref = call_tool(
            &mut session,
            "block_add_reference",
            json!({
                "page_uuid": page_uuid,
                "target_uuid": touch_child_uuid,
                "px": 256.0,
                "py": 256.0
            }),
        );
        let in_ref_uuid = in_ref["block_uuid"].as_str().unwrap().to_string();

        let sw = call_tool(
            &mut session,
            "block_add",
            json!({
                "page_uuid": page_uuid,
                "block_type": "Switch",
                "title": "Island Switch",
                "room_uuid": room_uuid,
                "px": 1024.0,
                "py": 256.0
            }),
        );
        let sw_uuid = sw["block_uuid"].as_str().unwrap().to_string();

        let lc = call_tool(
            &mut session,
            "block_add",
            json!({
                "page_uuid": page_uuid,
                "block_type": "LightController2",
                "title": "Kitchen Lighting",
                "room_uuid": room_uuid,
                "px": 1920.0,
                "py": 256.0
            }),
        );
        let lc_uuid = lc["block_uuid"].as_str().unwrap().to_string();

        // 5. Wire InputRef O -> Switch Tr, and configure LightController2 circuit + scene
        call_tool(
            &mut session,
            "wire_connect",
            json!({
                "source_block_uuid": in_ref_uuid,
                "source_pin_key": "O",
                "target_block_uuid": sw_uuid,
                "target_pin_key": "Tr"
            }),
        );

        call_tool(
            &mut session,
            "config_light_controller_set",
            json!({
                "block_uuid": lc_uuid,
                "circuits": [
                    { "index": 0, "name": "Island Pendants", "circuit_type": "Dimmer0To100" }
                ],
                "scenes": [
                    { "sid": 1, "name": "Cooking Bright", "circuit_pcts": [90, 0, 0, 0, 0, 0, 0, 0] }
                ]
            }),
        );
        let lc_cfg = call_tool(
            &mut session,
            "config_light_controller_get",
            json!({ "block_uuid": lc_uuid }),
        );
        assert_eq!(lc_cfg["circuits"][0]["name"], "Island Pendants");

        // 6. Run offline simulation and toggle Switch
        let sim_res = call_tool(
            &mut session,
            "sim_control",
            json!({
                "action": "set_input",
                "control_uuid": sw_uuid,
                "value": "1",
                "page_uuid": page_uuid
            }),
        );
        assert_eq!(sim_res["enabled"], true);

        // 7. Run Project Analysis and export Component BOM CSV
        let analysis = call_tool(&mut session, "analysis_run", json!({}));
        assert!(analysis.get("total_findings").is_some());

        let bom = call_tool(
            &mut session,
            "report_export",
            json!({ "format": "component_bom_csv" }),
        );
        assert!(bom["content"]
            .as_str()
            .unwrap()
            .contains("LightController2"));

        // 8. Test Auto-Layout on Kitchen Logic page
        let layout_res = call_tool(
            &mut session,
            "page_auto_layout",
            json!({ "page_uuid": page_uuid }),
        );
        assert!(layout_res["moved_blocks"].as_u64().unwrap() > 0);
        let page_after = call_tool(
            &mut session,
            "page_get",
            json!({ "page_uuid": page_uuid }),
        );
        for blk in page_after["blocks"].as_array().unwrap() {
            let px = blk["px"].as_f64().unwrap();
            let py = blk["py"].as_f64().unwrap();
            assert_eq!((px % 64.0).abs(), 0.0);
            assert_eq!((py % 64.0).abs(), 0.0);
        }

        // 9. Test To-Do Manager and Autopilot Rule Designer (Stage 9)
        let added_todo = call_tool(
            &mut session,
            "todo_add",
            json!({
                "page_uuid": page_uuid,
                "title": "Verify island dimmer curve",
                "text": "Check DALI minimum brightness"
            }),
        );
        let todo_uuid = added_todo["todo_uuid"].as_str().unwrap().to_string();
        let todos = call_tool(&mut session, "todo_list", json!({}));
        assert_eq!(todos["count"], 1);
        call_tool(
            &mut session,
            "todo_resolve",
            json!({ "todo_uuid": todo_uuid, "delete_block": false }),
        );
        let todos_after = call_tool(&mut session, "todo_list", json!({}));
        assert_eq!(todos_after["todos"][0]["is_resolved"], true);

        let ap_rule = call_tool(
            &mut session,
            "autopilot_rule_create",
            json!({
                "title": "Island Trigger Rule",
                "page_uuid": page_uuid,
                "logic_op": "AND",
                "condition_source_uuids": [in_ref_uuid],
                "target_block_uuid": lc_uuid,
                "target_pin_key": "Tr"
            }),
        );
        let ap_rule_uuid = ap_rule["rule_uuid"].as_str().unwrap().to_string();

        // Stage 12: Pin Inversion & Multi-Block Alignment
        let inv_res = call_tool(
            &mut session,
            "block_pin_set_inverted",
            json!({
                "block_uuid": ap_rule_uuid,
                "pin_key": "I",
                "inverted": true
            }),
        );
        assert_eq!(inv_res["inverted"], true);

        let align_res = call_tool(
            &mut session,
            "page_align_blocks",
            json!({
                "page_uuid": page_uuid,
                "mode": "align_top",
                "block_uuids": [lc_uuid, ap_rule_uuid]
            }),
        );
        assert!(align_res["moved_blocks"].as_u64().is_some());

        // 10. Test Undo & Redo
        let undo_res = call_tool(&mut session, "project_undo", json!({}));
        assert_eq!(undo_res["undone"], true);
        let redo_res = call_tool(&mut session, "project_redo", json!({}));
        assert_eq!(redo_res["redone"], true);
    }

    #[test]
    fn test_sample_loxone_specialized_configs_and_wizards() {
        let sample_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Sample.Loxone");
        let mut session =
            HeadlessSession::new(Some(&sample_path), false).expect("open Sample.Loxone");

        // Burglar Alarm object assignments
        let alarm_uuid = "19eb19a9-024d-3a10-ffff51535bcccc04";
        let obj_cfg = call_tool(
            &mut session,
            "config_object_assignments_get",
            json!({ "block_uuid": alarm_uuid, "only_selected": true }),
        );
        assert!(obj_cfg["count"].as_u64().unwrap() >= 30);

        // Auto-Configuration Room Wizard preview + execution
        let wiz = call_tool(
            &mut session,
            "wizard_auto_configure_rooms",
            json!({ "preview_only": true }),
        );
        assert!(wiz["generated_blocks"].as_u64().unwrap() > 0);

        // Bulk Find & Replace preview
        let fr = call_tool(
            &mut session,
            "bulk_find_and_replace",
            json!({
                "query": "Oświetlenie",
                "replacement": "Lighting",
                "preview_only": true
            }),
        );
        assert!(fr["match_count"].as_u64().unwrap() > 0);

        // AudioZone / MusicPlayer configuration & Player Group / Favorites
        let mp_uuid = "19eb19a5-0392-28eb-ffff51535bcccc04";
        let az_before = call_tool(
            &mut session,
            "config_audio_zone_get",
            json!({ "block_uuid": mp_uuid }),
        );
        assert_eq!(az_before["default_vol"], 20);
        assert_eq!(az_before["assigned_channel_uuids"].as_array().unwrap().len(), 4);

        call_tool(
            &mut session,
            "config_audio_zone_set",
            json!({
                "block_uuid": mp_uuid,
                "default_vol": 28,
                "airplay_enabled": true,
                "favorites": [
                    { "slot": 1, "title": "Chill Lounge", "source_uri": "tunein://s12345" }
                ]
            }),
        );
        let az_after = call_tool(
            &mut session,
            "config_audio_zone_get",
            json!({ "block_uuid": mp_uuid }),
        );
        assert_eq!(az_after["default_vol"], 28);
        assert_eq!(az_after["airplay_enabled"], true);
        assert_eq!(az_after["favorites"][0]["title"], "Chill Lounge");

        // Stage 7: Device replacement & serial conflict resolution
        let conflicts_preview = call_tool(
            &mut session,
            "tree_resolve_serial_conflicts",
            json!({ "preview_only": true }),
        );
        let first_reassigned_uuid = conflicts_preview["reassignments"][0]["device_uuid"]
            .as_str()
            .unwrap()
            .to_string();
        let replaced = call_tool(
            &mut session,
            "tree_replace_device",
            json!({
                "device_uuid": first_reassigned_uuid,
                "new_serial": "B030-99AA",
                "distribution_board": "DB-Main",
                "row_position": "R2",
                "lighting_group": "LG-1"
            }),
        );
        assert_eq!(replaced["serial"], "B03099AA");
        assert_eq!(replaced["distribution_board"], "DB-Main");
        assert_eq!(replaced["row_position"], "R2");
        assert_eq!(replaced["lighting_group"], "LG-1");

        let resolved = call_tool(
            &mut session,
            "tree_resolve_serial_conflicts",
            json!({ "preview_only": false }),
        );
        assert!(resolved["resolved_count"].as_u64().unwrap() > 0);
        let after_conflicts = call_tool(
            &mut session,
            "tree_resolve_serial_conflicts",
            json!({ "preview_only": true }),
        );
        assert_eq!(after_conflicts["resolved_count"], 0);

        // Stage 10: Project Diff against Sample.Loxone baseline on disk
        let diff_res = call_tool(&mut session, "project_diff", json!({}));
        assert!(diff_res["modified_blocks"].as_u64().unwrap() >= 1);
        assert!(diff_res["modified_tree_nodes"].as_u64().unwrap() >= 1);
        assert!(diff_res["total_changes"].as_u64().unwrap() >= 2);

        // Stage 11: BOM and Terminal Schedule
        let bom_res = call_tool(
            &mut session,
            "project_bom_get",
            json!({ "distribution_board": "DB-Main" }),
        );
        assert_eq!(bom_res["device_count"], 1);
        assert_eq!(bom_res["devices"][0]["serial"], "B03099AA");

        let sched_res = call_tool(
            &mut session,
            "project_terminal_schedule_get",
            json!({ "wired_only": true }),
        );
        assert!(sched_res["count"].as_u64().unwrap() > 0);

        // Stage 13: Multiplier Project & Unit Instantiation
        let mult_upsert = call_tool(
            &mut session,
            "multiplier_unit_upsert",
            json!({
                "title": "Villa 04",
                "miniserver_serial": "504f94ff0004",
                "address": "10.0.4.10",
                "device_serial_overrides": [
                    { "device_uuid": first_reassigned_uuid, "serial": "B0300004" }
                ]
            }),
        );
        let unit_uuid = mult_upsert["unit_uuid"].as_str().unwrap().to_string();
        let mult_list = call_tool(&mut session, "multiplier_units_list", json!({}));
        assert_eq!(mult_list["is_multiplier_project"], true);
        assert_eq!(mult_list["count"], 1);

        let mult_export = call_tool(
            &mut session,
            "multiplier_export_unit",
            json!({ "unit_uuid": unit_uuid }),
        );
        assert!(mult_export["project_title"]
            .as_str()
            .unwrap()
            .contains("Villa 04"));
        assert!(mult_export["xml"].as_str().unwrap().contains("B0300004"));
    }
}
