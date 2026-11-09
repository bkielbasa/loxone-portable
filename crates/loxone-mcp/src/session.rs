use std::path::{Path, PathBuf};

use loxone_core::block_schema::{all_block_schemas, lookup_block_schema};
use loxone_core::factory::{
    ensure_schema_pins_on_block, new_category, new_page, new_page_block, new_reference_block,
    new_room, new_tree_node,
};
use loxone_core::linter::{export_project_analysis_csv, lint_project};
use loxone_core::model::{LoxoneProject, PinKind, RawNode, TreeSectionKind};
use loxone_core::parser::load_loxone_file;
use loxone_core::periphery_provisioning::{
    build_hardware_tree_node, collect_parent_bus_candidates, collect_provisioned_hardware,
    HardwareTemplateKind,
};
use loxone_core::simulator::OfflineSimulator;
use loxone_core::writer::{
    apply_edit_with_inverse, create_cross_page_link_edits, save_loxone_file, serialize_loxone_xml,
    ProjectEdit,
};
use serde_json::{json, Value};

use crate::helpers::{
    block_to_json, build_clean_project, find_tree_node, pin_kind_str, tree_node_to_json,
};
use crate::protocol::{
    resource_definitions, tool_definitions, JsonRpcRequest, JsonRpcResponse, MCP_PROTOCOL_VERSION,
};
use crate::tools_advanced::call_advanced_tool;

pub trait McpSession {
    fn mode_name(&self) -> &'static str;
    fn project(&self) -> Option<&LoxoneProject>;
    fn project_mut(&mut self) -> Option<&mut LoxoneProject>;
    fn loaded_path(&self) -> Option<&Path>;
    fn is_dirty(&self) -> bool;
    fn undo_depth(&self) -> usize;
    fn redo_depth(&self) -> usize;
    fn open_project_file(&mut self, path: &Path) -> Result<(), String>;
    fn save_project_file(&mut self, path: Option<&Path>) -> Result<PathBuf, String>;
    fn create_clean_project(&mut self, title: &str, path: Option<&Path>) -> Result<(), String>;
    fn apply_edit(&mut self, edit: ProjectEdit) -> Result<(), String>;
    fn undo(&mut self) -> Result<bool, String>;
    fn redo(&mut self) -> Result<bool, String>;
    fn simulator(&self) -> &OfflineSimulator;
    fn simulator_mut(&mut self) -> &mut OfflineSimulator;
    fn notify_select_or_page(&mut self, _page_uuid: Option<&str>, _object_uuid: Option<&str>) {}
}

#[derive(Debug, Default)]
pub struct HeadlessSession {
    pub project: Option<LoxoneProject>,
    pub loaded_path: Option<PathBuf>,
    pub auto_save: bool,
    pub is_dirty: bool,
    pub undo_stack: Vec<ProjectEdit>,
    pub redo_stack: Vec<ProjectEdit>,
    pub simulator: OfflineSimulator,
}

impl HeadlessSession {
    pub fn new(initial_project: Option<&Path>, auto_save: bool) -> Result<Self, String> {
        let mut s = Self {
            project: None,
            loaded_path: None,
            auto_save,
            is_dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            simulator: OfflineSimulator::default(),
        };
        if let Some(p) = initial_project {
            s.open_project_file(p)?;
        }
        Ok(s)
    }

    fn maybe_auto_save(&mut self) -> Result<(), String> {
        if self.auto_save && self.is_dirty {
            if let Some(path) = self.loaded_path.clone() {
                let _ = self.save_project_file(Some(&path))?;
            }
        }
        Ok(())
    }
}

impl McpSession for HeadlessSession {
    fn mode_name(&self) -> &'static str {
        "headless"
    }

    fn project(&self) -> Option<&LoxoneProject> {
        self.project.as_ref()
    }

    fn project_mut(&mut self) -> Option<&mut LoxoneProject> {
        self.project.as_mut()
    }

    fn loaded_path(&self) -> Option<&Path> {
        self.loaded_path.as_deref()
    }

    fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    fn redo_depth(&self) -> usize {
        self.redo_stack.len()
    }

    fn open_project_file(&mut self, path: &Path) -> Result<(), String> {
        let proj = load_loxone_file(path)
            .map_err(|e| format!("Failed to load {}: {}", path.display(), e))?;
        self.simulator = OfflineSimulator::default();
        self.simulator.live_store.seed_project_defaults(&proj);
        self.project = Some(proj);
        self.loaded_path = Some(path.to_path_buf());
        self.is_dirty = false;
        self.undo_stack.clear();
        self.redo_stack.clear();
        Ok(())
    }

    fn save_project_file(&mut self, path: Option<&Path>) -> Result<PathBuf, String> {
        let target = path
            .map(Path::to_path_buf)
            .or_else(|| self.loaded_path.clone())
            .ok_or_else(|| "No path specified and no loaded_path set on session".to_string())?;
        let proj = self
            .project
            .as_mut()
            .ok_or_else(|| "No project loaded in session".to_string())?;
        save_loxone_file(&target, proj)
            .map_err(|e| format!("Failed to save {}: {}", target.display(), e))?;
        self.loaded_path = Some(target.clone());
        self.is_dirty = false;
        Ok(target)
    }

    fn create_clean_project(&mut self, title: &str, path: Option<&Path>) -> Result<(), String> {
        let mut proj = build_clean_project(title).map_err(|e| e.to_string())?;
        if let Some(p) = path {
            save_loxone_file(p, &mut proj)
                .map_err(|e| format!("Failed to save {}: {}", p.display(), e))?;
            self.loaded_path = Some(p.to_path_buf());
        } else {
            self.loaded_path = None;
        }
        self.simulator = OfflineSimulator::default();
        self.simulator.live_store.seed_project_defaults(&proj);
        self.project = Some(proj);
        self.is_dirty = false;
        self.undo_stack.clear();
        self.redo_stack.clear();
        Ok(())
    }

    fn apply_edit(&mut self, edit: ProjectEdit) -> Result<(), String> {
        let proj = self
            .project
            .as_mut()
            .ok_or_else(|| "No project loaded. Call project_open or project_create first.".to_string())?;
        let inverse = apply_edit_with_inverse(proj, &edit).map_err(|e| e.to_string())?;
        self.undo_stack.push(inverse);
        self.redo_stack.clear();
        self.is_dirty = true;
        if self.simulator.enabled {
            self.simulator.recompute(proj);
        }
        self.maybe_auto_save()?;
        Ok(())
    }

    fn undo(&mut self) -> Result<bool, String> {
        let Some(inv) = self.undo_stack.pop() else {
            return Ok(false);
        };
        let proj = self
            .project
            .as_mut()
            .ok_or_else(|| "No project loaded".to_string())?;
        let redo_edit = apply_edit_with_inverse(proj, &inv).map_err(|e| e.to_string())?;
        self.redo_stack.push(redo_edit);
        self.is_dirty = true;
        if self.simulator.enabled {
            self.simulator.recompute(proj);
        }
        self.maybe_auto_save()?;
        Ok(true)
    }

    fn redo(&mut self) -> Result<bool, String> {
        let Some(redo_edit) = self.redo_stack.pop() else {
            return Ok(false);
        };
        let proj = self
            .project
            .as_mut()
            .ok_or_else(|| "No project loaded".to_string())?;
        let undo_edit = apply_edit_with_inverse(proj, &redo_edit).map_err(|e| e.to_string())?;
        self.undo_stack.push(undo_edit);
        self.is_dirty = true;
        if self.simulator.enabled {
            self.simulator.recompute(proj);
        }
        self.maybe_auto_save()?;
        Ok(true)
    }

    fn simulator(&self) -> &OfflineSimulator {
        &self.simulator
    }

    fn simulator_mut(&mut self) -> &mut OfflineSimulator {
        &mut self.simulator
    }
}

pub fn handle_jsonrpc_request(
    session: &mut dyn McpSession,
    req: &JsonRpcRequest,
) -> Option<JsonRpcResponse> {
    match req.method.as_str() {
        "notifications/initialized" | "notifications/cancelled" => None,
        "initialize" => Some(JsonRpcResponse::ok(
            req.id.clone(),
            json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {
                    "tools": { "listChanged": false },
                    "resources": { "subscribe": false, "listChanged": false }
                },
                "serverInfo": {
                    "name": "loxone-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        )),
        "ping" => Some(JsonRpcResponse::ok(req.id.clone(), json!({}))),
        "tools/list" => Some(JsonRpcResponse::ok(req.id.clone(), tool_definitions())),
        "resources/list" => Some(JsonRpcResponse::ok(req.id.clone(), resource_definitions())),
        "resources/read" => {
            let uri = req
                .params
                .as_ref()
                .and_then(|p| p.get("uri"))
                .and_then(Value::as_str)
                .unwrap_or("");
            Some(handle_resource_read(session, req.id.clone(), uri))
        }
        "tools/call" => {
            let Some(params) = &req.params else {
                return Some(JsonRpcResponse::err(
                    req.id.clone(),
                    -32602,
                    "Missing params for tools/call",
                ));
            };
            let tool_name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            Some(dispatch_tool_call(session, req.id.clone(), tool_name, &args))
        }
        other => {
            if req.id.is_none() {
                None
            } else {
                Some(JsonRpcResponse::err(
                    req.id.clone(),
                    -32601,
                    format!("Method not found: {other}"),
                ))
            }
        }
    }
}

fn handle_resource_read(
    session: &mut dyn McpSession,
    id: Option<Value>,
    uri: &str,
) -> JsonRpcResponse {
    let Some(proj) = session.project() else {
        return JsonRpcResponse::err(id, -32001, "No Loxone project currently loaded");
    };
    match uri {
        "loxone://project/summary" => {
            let summary = build_project_status_json(session);
            JsonRpcResponse::ok(
                id,
                json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&summary).unwrap_or_default()
                    }]
                }),
            )
        }
        "loxone://project/pages" => {
            let pages: Vec<Value> = proj
                .pages
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    json!({
                        "index": i,
                        "uuid": p.uuid,
                        "title": p.title,
                        "block_count": p.blocks.len()
                    })
                })
                .collect();
            JsonRpcResponse::ok(
                id,
                json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&pages).unwrap_or_default()
                    }]
                }),
            )
        }
        "loxone://project/analysis" => {
            let diags = lint_project(proj);
            let csv = export_project_analysis_csv(proj, &diags);
            JsonRpcResponse::ok(
                id,
                json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "text/csv",
                        "text": csv
                    }]
                }),
            )
        }
        "loxone://project/xml" => match serialize_loxone_xml(proj) {
            Ok(xml) => JsonRpcResponse::ok(
                id,
                json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "application/xml",
                        "text": xml
                    }]
                }),
            ),
            Err(e) => JsonRpcResponse::err(id, -32002, e.to_string()),
        },
        _ => JsonRpcResponse::err(id, -32602, format!("Unknown resource URI: {uri}")),
    }
}

pub fn build_project_status_json(session: &dyn McpSession) -> Value {
    let path_str = session
        .loaded_path()
        .map(|p| p.display().to_string());
    if let Some(proj) = session.project() {
        let total_blocks: usize = proj.pages.iter().map(|p| p.blocks.len()).sum();
        let total_wires: usize = proj
            .pages
            .iter()
            .flat_map(|p| p.blocks.iter())
            .flat_map(|b| b.pins.iter())
            .map(|pin| pin.incoming_links.len())
            .sum();
        json!({
            "loaded": true,
            "mode": session.mode_name(),
            "loaded_path": path_str,
            "is_dirty": session.is_dirty(),
            "undo_depth": session.undo_depth(),
            "redo_depth": session.redo_depth(),
            "simulation_enabled": session.simulator().enabled,
            "metadata": {
                "title": proj.metadata.title,
                "uuid": proj.metadata.uuid,
                "config_version": proj.metadata.config_version,
                "miniserver_ip": proj.metadata.miniserver_ip,
                "miniserver_serial": proj.metadata.miniserver_serial
            },
            "counts": {
                "pages": proj.pages.len(),
                "rooms": proj.places.len(),
                "categories": proj.categories.len(),
                "blocks": total_blocks,
                "wires": total_wires
            }
        })
    } else {
        json!({
            "loaded": false,
            "mode": session.mode_name(),
            "loaded_path": path_str,
            "is_dirty": false,
            "undo_depth": 0,
            "redo_depth": 0
        })
    }
}

fn dispatch_tool_call(
    session: &mut dyn McpSession,
    id: Option<Value>,
    tool_name: &str,
    args: &Value,
) -> JsonRpcResponse {
    match tool_name {
        "project_status" => JsonRpcResponse::tool_ok(id, build_project_status_json(session)),
        "project_open" => {
            let Some(path_str) = args.get("path").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'path'");
            };
            match session.open_project_file(Path::new(path_str)) {
                Ok(()) => JsonRpcResponse::tool_ok(id, build_project_status_json(session)),
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        "project_save" => {
            let opt_path = args.get("path").and_then(Value::as_str).map(Path::new);
            match session.save_project_file(opt_path) {
                Ok(saved) => JsonRpcResponse::tool_ok(
                    id,
                    json!({
                        "saved_path": saved.display().to_string(),
                        "status": build_project_status_json(session)
                    }),
                ),
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        "project_create" => {
            let title = args
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("New Project");
            let opt_path = args.get("path").and_then(Value::as_str).map(Path::new);
            match session.create_clean_project(title, opt_path) {
                Ok(()) => JsonRpcResponse::tool_ok(id, build_project_status_json(session)),
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        "project_undo" => match session.undo() {
            Ok(undone) => JsonRpcResponse::tool_ok(
                id,
                json!({
                    "undone": undone,
                    "undo_depth": session.undo_depth(),
                    "redo_depth": session.redo_depth()
                }),
            ),
            Err(e) => JsonRpcResponse::tool_err(id, e),
        },
        "project_redo" => match session.redo() {
            Ok(redone) => JsonRpcResponse::tool_ok(
                id,
                json!({
                    "redone": redone,
                    "undo_depth": session.undo_depth(),
                    "redo_depth": session.redo_depth()
                }),
            ),
            Err(e) => JsonRpcResponse::tool_err(id, e),
        },
        "project_search" => {
            let Some(proj) = session.project() else {
                return JsonRpcResponse::tool_err(id, "No project loaded");
            };
            let q = args
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_lowercase();
            let type_filter = args.get("object_type").and_then(Value::as_str);
            let room_filter = args.get("room_uuid").and_then(Value::as_str);
            let cat_filter = args.get("category_uuid").and_then(Value::as_str);

            let mut results = Vec::new();
            for page in &proj.pages {
                for b in &page.blocks {
                    if let Some(tf) = type_filter {
                        if !b.block_type.eq_ignore_ascii_case(tf) {
                            continue;
                        }
                    }
                    if let Some(rf) = room_filter {
                        if b.room_uuid.as_deref() != Some(rf)
                            && page.room_uuid.as_deref() != Some(rf)
                        {
                            continue;
                        }
                    }
                    if let Some(cf) = cat_filter {
                        if b.category_uuid.as_deref() != Some(cf)
                            && page.category_uuid.as_deref() != Some(cf)
                        {
                            continue;
                        }
                    }
                    if !q.is_empty()
                        && !b.title.to_lowercase().contains(&q)
                        && !b.block_type.to_lowercase().contains(&q)
                        && !b.uuid.to_lowercase().contains(&q)
                    {
                        continue;
                    }
                    results.push(json!({
                        "kind": "page_block",
                        "uuid": b.uuid,
                        "type": b.block_type,
                        "title": b.title,
                        "page_uuid": page.uuid,
                        "page_title": page.title,
                        "room_uuid": b.room_uuid,
                        "category_uuid": b.category_uuid
                    }));
                }
            }
            for (uuid, obj) in &proj.xref.objects_by_uuid {
                if matches!(obj.location, loxone_core::xref::ObjectLocation::PageBlock { .. }) {
                    continue;
                }
                if let Some(tf) = type_filter {
                    if !obj.object_type.eq_ignore_ascii_case(tf) {
                        continue;
                    }
                }
                if !q.is_empty()
                    && !obj.title.to_lowercase().contains(&q)
                    && !obj.object_type.to_lowercase().contains(&q)
                    && !uuid.to_lowercase().contains(&q)
                {
                    continue;
                }
                results.push(json!({
                    "kind": "tree_or_meta",
                    "uuid": uuid,
                    "type": obj.object_type,
                    "title": obj.title
                }));
            }
            JsonRpcResponse::tool_ok(
                id,
                json!({
                    "count": results.len(),
                    "results": results
                }),
            )
        }
        "page_list" => {
            let Some(proj) = session.project() else {
                return JsonRpcResponse::tool_err(id, "No project loaded");
            };
            let pages: Vec<Value> = proj
                .pages
                .iter()
                .enumerate()
                .map(|(idx, p)| {
                    let wires: usize = p
                        .blocks
                        .iter()
                        .flat_map(|b| b.pins.iter())
                        .map(|pin| pin.incoming_links.len())
                        .sum();
                    json!({
                        "index": idx,
                        "uuid": p.uuid,
                        "title": p.title,
                        "room_uuid": p.room_uuid,
                        "category_uuid": p.category_uuid,
                        "block_count": p.blocks.len(),
                        "wire_count": wires
                    })
                })
                .collect();
            JsonRpcResponse::tool_ok(id, json!({ "pages": pages }))
        }
        "page_get" => {
            let Some(proj) = session.project() else {
                return JsonRpcResponse::tool_err(id, "No project loaded");
            };
            let page_opt = if let Some(pu) = args.get("page_uuid").and_then(Value::as_str) {
                proj.pages.iter().find(|p| p.uuid == pu)
            } else if let Some(pt) = args.get("title").and_then(Value::as_str) {
                proj.pages
                    .iter()
                    .find(|p| p.title.eq_ignore_ascii_case(pt))
            } else if let Some(idx) = args.get("index").and_then(Value::as_u64) {
                proj.pages.get(idx as usize)
            } else {
                proj.pages.first()
            };
            let Some(page) = page_opt else {
                return JsonRpcResponse::tool_err(id, "Page not found");
            };
            let blocks: Vec<Value> = page
                .blocks
                .iter()
                .map(|b| block_to_json(b, page, proj))
                .collect();
            JsonRpcResponse::tool_ok(
                id,
                json!({
                    "uuid": page.uuid,
                    "title": page.title,
                    "room_uuid": page.room_uuid,
                    "category_uuid": page.category_uuid,
                    "block_count": blocks.len(),
                    "blocks": blocks
                }),
            )
        }
        "page_create" => {
            let Some(title) = args.get("title").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'title'");
            };
            let mut page = new_page(title);
            page.room_uuid = args
                .get("room_uuid")
                .and_then(Value::as_str)
                .map(str::to_string);
            page.category_uuid = args
                .get("category_uuid")
                .and_then(Value::as_str)
                .map(str::to_string);
            let page_uuid = page.uuid.clone();
            let page_title = page.title.clone();
            match session.apply_edit(ProjectEdit::AddPage { page }) {
                Ok(()) => {
                    session.notify_select_or_page(Some(&page_uuid), None);
                    JsonRpcResponse::tool_ok(
                        id,
                        json!({ "page_uuid": page_uuid, "title": page_title }),
                    )
                }
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        "page_rename" => {
            let (Some(page_uuid), Some(title)) = (
                args.get("page_uuid").and_then(Value::as_str),
                args.get("title").and_then(Value::as_str),
            ) else {
                return JsonRpcResponse::tool_err(
                    id,
                    "Missing required parameters 'page_uuid' and 'title'",
                );
            };
            let edit = ProjectEdit::SetControlAttr {
                control_uuid: page_uuid.to_string(),
                attr_name: "Title".to_string(),
                value: Some(title.to_string()),
            };
            match session.apply_edit(edit) {
                Ok(()) => JsonRpcResponse::tool_ok(
                    id,
                    json!({ "page_uuid": page_uuid, "title": title }),
                ),
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        "page_delete" => {
            let Some(page_uuid) = args.get("page_uuid").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'page_uuid'");
            };
            match session.apply_edit(ProjectEdit::RemovePage {
                page_uuid: page_uuid.to_string(),
            }) {
                Ok(()) => JsonRpcResponse::tool_ok(id, json!({ "deleted_page_uuid": page_uuid })),
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        "room_list" => {
            let Some(proj) = session.project() else {
                return JsonRpcResponse::tool_err(id, "No project loaded");
            };
            let rooms: Vec<Value> = proj
                .places
                .values()
                .map(|r| {
                    json!({
                        "uuid": r.uuid,
                        "title": r.title,
                        "sqm": r.sqm,
                        "p_type": r.p_type
                    })
                })
                .collect();
            JsonRpcResponse::tool_ok(id, json!({ "rooms": rooms }))
        }
        "room_create" => {
            let Some(title) = args.get("title").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'title'");
            };
            let mut room = new_room(title);
            if let Some(sqm) = args.get("sqm").and_then(Value::as_str) {
                room.sqm = Some(sqm.to_string());
            }
            if let Some(pt) = args.get("p_type").and_then(Value::as_str) {
                room.p_type = Some(pt.to_string());
            }
            let room_uuid = room.uuid.clone();
            let room_title = room.title.clone();
            match session.apply_edit(ProjectEdit::AddRoom { room }) {
                Ok(()) => JsonRpcResponse::tool_ok(
                    id,
                    json!({ "room_uuid": room_uuid, "title": room_title }),
                ),
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        "room_rename" => {
            let (Some(room_uuid), Some(title)) = (
                args.get("room_uuid").and_then(Value::as_str),
                args.get("title").and_then(Value::as_str),
            ) else {
                return JsonRpcResponse::tool_err(
                    id,
                    "Missing required parameters 'room_uuid' and 'title'",
                );
            };
            match session.apply_edit(ProjectEdit::SetControlAttr {
                control_uuid: room_uuid.to_string(),
                attr_name: "Title".to_string(),
                value: Some(title.to_string()),
            }) {
                Ok(()) => JsonRpcResponse::tool_ok(
                    id,
                    json!({ "room_uuid": room_uuid, "title": title }),
                ),
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        "room_delete" => {
            let Some(room_uuid) = args.get("room_uuid").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'room_uuid'");
            };
            match session.apply_edit(ProjectEdit::RemoveRoom {
                room_uuid: room_uuid.to_string(),
            }) {
                Ok(()) => JsonRpcResponse::tool_ok(id, json!({ "deleted_room_uuid": room_uuid })),
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        "category_list" => {
            let Some(proj) = session.project() else {
                return JsonRpcResponse::tool_err(id, "No project loaded");
            };
            let cats: Vec<Value> = proj
                .categories
                .values()
                .map(|c| {
                    json!({
                        "uuid": c.uuid,
                        "title": c.title,
                        "cat_group": c.cat_group,
                        "bk_color": c.bk_color
                    })
                })
                .collect();
            JsonRpcResponse::tool_ok(id, json!({ "categories": cats }))
        }
        "category_create" => {
            let Some(title) = args.get("title").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'title'");
            };
            let cat_type = args
                .get("cat_type")
                .and_then(Value::as_str)
                .unwrap_or("0");
            let category = new_category(title, cat_type);
            let category_uuid = category.uuid.clone();
            let category_title = category.title.clone();
            match session.apply_edit(ProjectEdit::AddCategory { category }) {
                Ok(()) => JsonRpcResponse::tool_ok(
                    id,
                    json!({ "category_uuid": category_uuid, "title": category_title }),
                ),
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        "category_rename" => {
            let (Some(category_uuid), Some(title)) = (
                args.get("category_uuid").and_then(Value::as_str),
                args.get("title").and_then(Value::as_str),
            ) else {
                return JsonRpcResponse::tool_err(
                    id,
                    "Missing required parameters 'category_uuid' and 'title'",
                );
            };
            match session.apply_edit(ProjectEdit::SetControlAttr {
                control_uuid: category_uuid.to_string(),
                attr_name: "Title".to_string(),
                value: Some(title.to_string()),
            }) {
                Ok(()) => JsonRpcResponse::tool_ok(
                    id,
                    json!({ "category_uuid": category_uuid, "title": title }),
                ),
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        "category_delete" => {
            let Some(category_uuid) = args.get("category_uuid").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'category_uuid'");
            };
            match session.apply_edit(ProjectEdit::RemoveCategory {
                category_uuid: category_uuid.to_string(),
            }) {
                Ok(()) => JsonRpcResponse::tool_ok(
                    id,
                    json!({ "deleted_category_uuid": category_uuid }),
                ),
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        _ => dispatch_block_wire_or_advanced(session, id, tool_name, args),
    }
}

fn resolve_target_page_uuid(project: &LoxoneProject, args: &Value) -> Option<String> {
    if let Some(pu) = args.get("page_uuid").and_then(Value::as_str) {
        if project.pages.iter().any(|p| p.uuid == pu) {
            return Some(pu.to_string());
        }
    }
    if let Some(pt) = args.get("page_title").and_then(Value::as_str) {
        if let Some(p) = project
            .pages
            .iter()
            .find(|p| p.title.eq_ignore_ascii_case(pt))
        {
            return Some(p.uuid.clone());
        }
    }
    project.pages.first().map(|p| p.uuid.clone())
}

fn dispatch_block_wire_or_advanced(
    session: &mut dyn McpSession,
    id: Option<Value>,
    tool_name: &str,
    args: &Value,
) -> JsonRpcResponse {
    match tool_name {
        "block_catalog" => {
            let q = args
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_lowercase();
            let cat = args
                .get("category")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_lowercase();

            let mut items = Vec::new();
            for s in all_block_schemas() {
                if !cat.is_empty() && !s.category_group.to_lowercase().contains(&cat) {
                    continue;
                }
                if !q.is_empty()
                    && !s.type_name.to_lowercase().contains(&q)
                    && !s.display_name_en.to_lowercase().contains(&q)
                    && !s.summary.to_lowercase().contains(&q)
                {
                    continue;
                }
                let inputs: Vec<Value> = s
                    .pin_docs
                    .iter()
                    .filter(|d| !d.is_output)
                    .map(|d| {
                        json!({
                            "key": d.pin_key,
                            "description": d.description
                        })
                    })
                    .collect();
                let outputs: Vec<Value> = s
                    .pin_docs
                    .iter()
                    .filter(|d| d.is_output)
                    .map(|d| {
                        json!({
                            "key": d.pin_key,
                            "description": d.description
                        })
                    })
                    .collect();
                items.push(json!({
                    "type_name": s.type_name,
                    "display_name": s.display_name_en,
                    "category_group": s.category_group,
                    "summary": s.summary,
                    "kb_url": s.kb_url,
                    "inputs": inputs,
                    "outputs": outputs
                }));
            }
            JsonRpcResponse::tool_ok(
                id,
                json!({
                    "count": items.len(),
                    "schemas": items
                }),
            )
        }
        "block_get" => {
            let Some(proj) = session.project() else {
                return JsonRpcResponse::tool_err(id, "No project loaded");
            };
            let Some(uuid) = args.get("uuid").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'uuid'");
            };
            for page in &proj.pages {
                if let Some(block) = page.blocks.iter().find(|b| b.uuid == uuid) {
                    let mut val = block_to_json(block, page, proj);
                    if let Some(schema) = lookup_block_schema(&block.block_type) {
                        if let Some(obj) = val.as_object_mut() {
                            obj.insert(
                                "schema".to_string(),
                                json!({
                                    "display_name": schema.display_name_en,
                                    "category_group": schema.category_group,
                                    "summary": schema.summary,
                                    "kb_url": schema.kb_url
                                }),
                            );
                        }
                    }
                    return JsonRpcResponse::tool_ok(id, val);
                }
            }
            if let Some(node) = find_tree_node(proj, uuid) {
                return JsonRpcResponse::tool_ok(id, tree_node_to_json(node, proj));
            }
            JsonRpcResponse::tool_err(id, format!("Object '{uuid}' not found in project"))
        }
        "block_add" => {
            let Some(block_type) = args.get("block_type").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'block_type'");
            };
            let title = args.get("title").and_then(Value::as_str).unwrap_or("");
            let (page_uuid, auto_px, auto_py) = {
                let Some(proj) = session.project() else {
                    return JsonRpcResponse::tool_err(id, "No project loaded");
                };
                let Some(pu) = resolve_target_page_uuid(proj, args) else {
                    return JsonRpcResponse::tool_err(id, "Target page not found");
                };
                let next_py = proj
                    .pages
                    .iter()
                    .find(|p| p.uuid == pu)
                    .map(|p| {
                        p.blocks
                            .iter()
                            .filter(|b| b.has_position)
                            .map(|b| b.py2.max(b.py + 512.0))
                            .fold(256.0_f32, f32::max)
                            + 256.0
                    })
                    .unwrap_or(256.0);
                (pu, 1920.0_f32, next_py)
            };
            let px = args
                .get("px")
                .and_then(Value::as_f64)
                .map(|v| v as f32)
                .unwrap_or(auto_px);
            let py = args
                .get("py")
                .and_then(Value::as_f64)
                .map(|v| v as f32)
                .unwrap_or(auto_py);

            let mut block = new_page_block(block_type, title, px, py);
            let room_uuid = args
                .get("room_uuid")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let category_uuid = args
                .get("category_uuid")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            if room_uuid.is_some() || category_uuid.is_some() {
                block.room_uuid = room_uuid.clone();
                block.category_uuid = category_uuid.clone();
                let mut iodata_attrs = Vec::new();
                if let Some(ru) = room_uuid {
                    iodata_attrs.push(("Pr".to_string(), ru));
                }
                if let Some(cu) = category_uuid {
                    iodata_attrs.push(("Cr".to_string(), cu));
                }
                block.extra_children.push(RawNode {
                    tag: "IoData".to_string(),
                    attributes: iodata_attrs,
                    text: None,
                    children: Vec::new(),
                });
            }
            let block_uuid = block.uuid.clone();
            let block_title = block.title.clone();
            let pins_json: Vec<Value> = block
                .pins
                .iter()
                .map(|p| {
                    json!({
                        "uuid": p.uuid,
                        "key": p.key,
                        "kind": pin_kind_str(p.pin_kind),
                        "default_value": p.default_value
                    })
                })
                .collect();
            match session.apply_edit(ProjectEdit::AddPageBlock {
                page_uuid: page_uuid.clone(),
                block,
            }) {
                Ok(()) => {
                    session.notify_select_or_page(Some(&page_uuid), Some(&block_uuid));
                    JsonRpcResponse::tool_ok(
                        id,
                        json!({
                            "block_uuid": block_uuid,
                            "block_type": block_type,
                            "title": block_title,
                            "page_uuid": page_uuid,
                            "px": px,
                            "py": py,
                            "pins": pins_json
                        }),
                    )
                }
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        "block_add_reference" => {
            let Some(target_uuid) = args.get("target_uuid").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'target_uuid'");
            };
            let (page_uuid, block) = {
                let Some(proj) = session.project() else {
                    return JsonRpcResponse::tool_err(id, "No project loaded");
                };
                let Some(pu) = resolve_target_page_uuid(proj, args) else {
                    return JsonRpcResponse::tool_err(id, "Target page not found");
                };
                let Some(target_node) = find_tree_node(proj, target_uuid) else {
                    return JsonRpcResponse::tool_err(
                        id,
                        format!("Periphery/System node '{target_uuid}' not found"),
                    );
                };
                let default_ref_type = if target_node.control_type == "Memory" {
                    "Memory"
                } else if target_node.pins.iter().any(|p| p.pin_kind == PinKind::Input)
                    && !target_node.pins.iter().any(|p| p.pin_kind == PinKind::Output)
                {
                    "OutputRef"
                } else {
                    "InputRef"
                };
                let ref_type = args
                    .get("ref_type")
                    .and_then(Value::as_str)
                    .unwrap_or(default_ref_type);
                let px = args
                    .get("px")
                    .and_then(Value::as_f64)
                    .map(|v| v as f32)
                    .unwrap_or(512.0);
                let py = args
                    .get("py")
                    .and_then(Value::as_f64)
                    .map(|v| v as f32)
                    .unwrap_or(512.0);
                let mut ref_blk = new_reference_block(ref_type, target_node, px, py);
                for p in &mut ref_blk.pins {
                    p.uuid = loxone_core::factory::generate_loxone_uuid();
                }
                (pu, ref_blk)
            };
            let block_uuid = block.uuid.clone();
            let ref_type = block.block_type.clone();
            let title = block.title.clone();
            match session.apply_edit(ProjectEdit::AddPageBlock {
                page_uuid: page_uuid.clone(),
                block,
            }) {
                Ok(()) => {
                    session.notify_select_or_page(Some(&page_uuid), Some(&block_uuid));
                    JsonRpcResponse::tool_ok(
                        id,
                        json!({
                            "block_uuid": block_uuid,
                            "ref_type": ref_type,
                            "target_uuid": target_uuid,
                            "title": title,
                            "page_uuid": page_uuid
                        }),
                    )
                }
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        "block_update" => {
            let Some(uuid) = args.get("uuid").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'uuid'");
            };
            let mut edits = Vec::new();
            let page_uuid_for_block = session.project().and_then(|proj| {
                proj.pages
                    .iter()
                    .find(|p| p.blocks.iter().any(|b| b.uuid == uuid))
                    .map(|p| p.uuid.clone())
            });

            for (arg_key, xml_attr) in [
                ("title", "Title"),
                ("desc", "Desc"),
                ("color", "Cl"),
                ("rating", "Rating"),
                ("visu_title", "VisuTitle"),
                ("visu_pwd", "VisuPwd"),
            ] {
                if let Some(val) = args.get(arg_key) {
                    let opt_str = val
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .map(str::to_string);
                    edits.push(ProjectEdit::SetControlAttr {
                        control_uuid: uuid.to_string(),
                        attr_name: xml_attr.to_string(),
                        value: opt_str,
                    });
                }
            }
            if let Some(v) = args.get("use_in_visu").and_then(Value::as_bool) {
                edits.push(ProjectEdit::SetControlAttr {
                    control_uuid: uuid.to_string(),
                    attr_name: "UseInVisu".to_string(),
                    value: Some(if v { "true" } else { "false" }.to_string()),
                });
            }
            if let Some(v) = args.get("use_in_fav").and_then(Value::as_bool) {
                edits.push(ProjectEdit::SetControlAttr {
                    control_uuid: uuid.to_string(),
                    attr_name: "UseInFav".to_string(),
                    value: Some(if v { "true" } else { "false" }.to_string()),
                });
            }
            if let Some(room_val) = args.get("room_uuid").and_then(Value::as_str) {
                edits.push(ProjectEdit::SetChildTagAttr {
                    control_uuid: uuid.to_string(),
                    child_tag: "IoData".to_string(),
                    attr_name: "Pr".to_string(),
                    value: (!room_val.is_empty()).then(|| room_val.to_string()),
                });
            }
            if let Some(cat_val) = args.get("category_uuid").and_then(Value::as_str) {
                edits.push(ProjectEdit::SetChildTagAttr {
                    control_uuid: uuid.to_string(),
                    child_tag: "IoData".to_string(),
                    attr_name: "Cr".to_string(),
                    value: (!cat_val.is_empty()).then(|| cat_val.to_string()),
                });
            }
            if let Some(pu) = &page_uuid_for_block {
                if let (Some(px), Some(py)) = (
                    args.get("px").and_then(Value::as_f64),
                    args.get("py").and_then(Value::as_f64),
                ) {
                    edits.push(ProjectEdit::MovePageBlock {
                        page_uuid: pu.clone(),
                        block_uuid: uuid.to_string(),
                        px: px as f32,
                        py: py as f32,
                    });
                }
                if let (Some(px2), Some(py2)) = (
                    args.get("px2").and_then(Value::as_f64),
                    args.get("py2").and_then(Value::as_f64),
                ) {
                    edits.push(ProjectEdit::ResizePageBlock {
                        page_uuid: pu.clone(),
                        block_uuid: uuid.to_string(),
                        px2: px2 as f32,
                        py2: py2 as f32,
                    });
                }
            }
            if let Some(attr_map) = args.get("attributes").and_then(Value::as_object) {
                for (k, v) in attr_map {
                    let val = v.as_str().map(str::to_string);
                    edits.push(ProjectEdit::SetControlAttr {
                        control_uuid: uuid.to_string(),
                        attr_name: k.clone(),
                        value: val,
                    });
                }
            }
            if edits.is_empty() {
                return JsonRpcResponse::tool_err(id, "No fields specified in block_update");
            }
            let count = edits.len();
            match session.apply_edit(ProjectEdit::Batch(edits)) {
                Ok(()) => JsonRpcResponse::tool_ok(
                    id,
                    json!({ "uuid": uuid, "updated_fields": count }),
                ),
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        "block_delete" => {
            let mut target_uuids = Vec::new();
            if let Some(u) = args.get("uuid").and_then(Value::as_str) {
                target_uuids.push(u.to_string());
            }
            if let Some(arr) = args.get("uuids").and_then(Value::as_array) {
                for item in arr {
                    if let Some(u) = item.as_str() {
                        target_uuids.push(u.to_string());
                    }
                }
            }
            if target_uuids.is_empty() {
                return JsonRpcResponse::tool_err(
                    id,
                    "Provide 'uuid' or 'uuids' in block_delete",
                );
            }
            let edits = {
                let Some(proj) = session.project() else {
                    return JsonRpcResponse::tool_err(id, "No project loaded");
                };
                let mut batch = Vec::new();
                for u in &target_uuids {
                    if let Some(page) = proj
                        .pages
                        .iter()
                        .find(|p| p.blocks.iter().any(|b| &b.uuid == u))
                    {
                        batch.push(ProjectEdit::RemovePageBlock {
                            page_uuid: page.uuid.clone(),
                            block_uuid: u.clone(),
                        });
                    } else if find_tree_node(proj, u).is_some() {
                        batch.push(ProjectEdit::RemoveTreeNode {
                            node_uuid: u.clone(),
                        });
                    }
                }
                batch
            };
            if edits.is_empty() {
                return JsonRpcResponse::tool_err(id, "None of the specified UUIDs were found");
            }
            let deleted = edits.len();
            match session.apply_edit(ProjectEdit::Batch(edits)) {
                Ok(()) => JsonRpcResponse::tool_ok(
                    id,
                    json!({ "deleted_count": deleted, "uuids": target_uuids }),
                ),
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        "wire_connect" => {
            let (
                Some(source_block_uuid),
                Some(source_pin_key),
                Some(target_block_uuid),
                Some(target_pin_key),
            ) = (
                args.get("source_block_uuid").and_then(Value::as_str),
                args.get("source_pin_key").and_then(Value::as_str),
                args.get("target_block_uuid").and_then(Value::as_str),
                args.get("target_pin_key").and_then(Value::as_str),
            ) else {
                return JsonRpcResponse::tool_err(
                    id,
                    "Missing source_block_uuid, source_pin_key, target_block_uuid, or target_pin_key",
                );
            };
            let source_pin_uuid = {
                let Some(proj) = session.project_mut() else {
                    return JsonRpcResponse::tool_err(id, "No project loaded");
                };
                for page in &mut proj.pages {
                    for blk in &mut page.blocks {
                        if blk.uuid == source_block_uuid || blk.uuid == target_block_uuid {
                            ensure_schema_pins_on_block(blk);
                        }
                    }
                }
                let src_pin = proj
                    .pages
                    .iter()
                    .flat_map(|p| p.blocks.iter())
                    .find(|b| b.uuid == source_block_uuid)
                    .and_then(|b| {
                        b.pins
                            .iter()
                            .find(|p| p.key.eq_ignore_ascii_case(source_pin_key))
                    })
                    .map(|p| p.uuid.clone());
                let Some(sp_uuid) = src_pin else {
                    return JsonRpcResponse::tool_err(
                        id,
                        format!(
                            "Output pin '{source_pin_key}' not found on block '{source_block_uuid}'"
                        ),
                    );
                };
                sp_uuid
            };
            match session.apply_edit(ProjectEdit::AddWireLink {
                target_block_uuid: target_block_uuid.to_string(),
                target_pin_key: target_pin_key.to_string(),
                source_pin_uuid: source_pin_uuid.clone(),
            }) {
                Ok(()) => JsonRpcResponse::tool_ok(
                    id,
                    json!({
                        "source_block_uuid": source_block_uuid,
                        "source_pin_key": source_pin_key,
                        "source_pin_uuid": source_pin_uuid,
                        "target_block_uuid": target_block_uuid,
                        "target_pin_key": target_pin_key
                    }),
                ),
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        "wire_disconnect" => {
            let (Some(target_block_uuid), Some(target_pin_key)) = (
                args.get("target_block_uuid").and_then(Value::as_str),
                args.get("target_pin_key").and_then(Value::as_str),
            ) else {
                return JsonRpcResponse::tool_err(
                    id,
                    "Missing target_block_uuid or target_pin_key",
                );
            };
            let edits = {
                let Some(proj) = session.project() else {
                    return JsonRpcResponse::tool_err(id, "No project loaded");
                };
                let explicit_src_pin = args.get("source_pin_uuid").and_then(Value::as_str);
                let src_block_filter = args.get("source_block_uuid").and_then(Value::as_str);
                let src_key_filter = args.get("source_pin_key").and_then(Value::as_str);

                let Some(target_block) = proj
                    .pages
                    .iter()
                    .flat_map(|p| p.blocks.iter())
                    .find(|b| b.uuid == target_block_uuid)
                else {
                    return JsonRpcResponse::tool_err(
                        id,
                        format!("Target block '{target_block_uuid}' not found"),
                    );
                };
                let Some(pin) = target_block
                    .pins
                    .iter()
                    .find(|p| p.key.eq_ignore_ascii_case(target_pin_key))
                else {
                    return JsonRpcResponse::tool_err(
                        id,
                        format!("Target pin '{target_pin_key}' not found on block"),
                    );
                };
                let mut batch = Vec::new();
                for link in &pin.incoming_links {
                    let matches_src = if let Some(sp) = explicit_src_pin {
                        link.source_pin_uuid == sp
                    } else if let Some(sb) = src_block_filter {
                        proj.xref
                            .pins_by_uuid
                            .get(&link.source_pin_uuid)
                            .is_some_and(|meta| {
                                meta.owner_uuid == sb
                                    && src_key_filter
                                        .map(|k| meta.pin_key.eq_ignore_ascii_case(k))
                                        .unwrap_or(true)
                            })
                    } else {
                        true
                    };
                    if matches_src {
                        batch.push(ProjectEdit::RemoveWireLink {
                            target_block_uuid: target_block_uuid.to_string(),
                            target_pin_key: pin.key.clone(),
                            source_pin_uuid: link.source_pin_uuid.clone(),
                        });
                    }
                }
                batch
            };
            if edits.is_empty() {
                return JsonRpcResponse::tool_err(id, "No matching wire links found to disconnect");
            }
            let count = edits.len();
            match session.apply_edit(ProjectEdit::Batch(edits)) {
                Ok(()) => JsonRpcResponse::tool_ok(id, json!({ "disconnected_wires": count })),
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        "cross_page_link_create" => {
            let (
                Some(source_page_uuid),
                Some(source_block_uuid),
                Some(source_pin_key),
                Some(target_page_uuid),
            ) = (
                args.get("source_page_uuid").and_then(Value::as_str),
                args.get("source_block_uuid").and_then(Value::as_str),
                args.get("source_pin_key").and_then(Value::as_str),
                args.get("target_page_uuid").and_then(Value::as_str),
            ) else {
                return JsonRpcResponse::tool_err(
                    id,
                    "Missing source_page_uuid, source_block_uuid, source_pin_key, or target_page_uuid",
                );
            };
            let target_px = args
                .get("target_px")
                .and_then(Value::as_f64)
                .map(|v| v as f32)
                .unwrap_or(512.0);
            let target_py = args
                .get("target_py")
                .and_then(Value::as_f64)
                .map(|v| v as f32)
                .unwrap_or(512.0);
            let batch_opt = {
                let Some(proj) = session.project_mut() else {
                    return JsonRpcResponse::tool_err(id, "No project loaded");
                };
                for page in &mut proj.pages {
                    for blk in &mut page.blocks {
                        if blk.uuid == source_block_uuid {
                            ensure_schema_pins_on_block(blk);
                        }
                    }
                }
                create_cross_page_link_edits(
                    proj,
                    source_page_uuid,
                    source_block_uuid,
                    source_pin_key,
                    target_page_uuid,
                    target_px,
                    target_py,
                )
            };
            let Some(batch) = batch_opt else {
                return JsonRpcResponse::tool_err(
                    id,
                    "Failed to build cross-page link (check page/block/pin UUIDs)",
                );
            };
            match session.apply_edit(batch) {
                Ok(()) => JsonRpcResponse::tool_ok(
                    id,
                    json!({
                        "source_page_uuid": source_page_uuid,
                        "source_block_uuid": source_block_uuid,
                        "source_pin_key": source_pin_key,
                        "target_page_uuid": target_page_uuid
                    }),
                ),
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        "pin_configure" => {
            let (Some(control_uuid), Some(pin_key)) = (
                args.get("control_uuid").and_then(Value::as_str),
                args.get("pin_key").and_then(Value::as_str),
            ) else {
                return JsonRpcResponse::tool_err(id, "Missing control_uuid or pin_key");
            };
            let mut edits = Vec::new();
            if let Some(def_val) = args.get("default_value") {
                edits.push(ProjectEdit::SetPinDefault {
                    control_uuid: control_uuid.to_string(),
                    pin_key: pin_key.to_string(),
                    default_value: def_val.as_str().map(str::to_string),
                });
            }
            if let Some(inv) = args.get("inverted").and_then(Value::as_bool) {
                edits.push(ProjectEdit::SetPinInverted {
                    control_uuid: control_uuid.to_string(),
                    pin_key: pin_key.to_string(),
                    inverted: inv,
                });
            }
            if edits.is_empty() {
                return JsonRpcResponse::tool_err(
                    id,
                    "Provide 'default_value' and/or 'inverted' in pin_configure",
                );
            }
            match session.apply_edit(ProjectEdit::Batch(edits)) {
                Ok(()) => JsonRpcResponse::tool_ok(
                    id,
                    json!({ "control_uuid": control_uuid, "pin_key": pin_key }),
                ),
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        "tree_list" => {
            let Some(proj) = session.project() else {
                return JsonRpcResponse::tool_err(id, "No project loaded");
            };
            let section = args
                .get("section")
                .and_then(Value::as_str)
                .unwrap_or("all");
            let hw_inventory: Vec<Value> = collect_provisioned_hardware(proj)
                .into_iter()
                .map(|d| {
                    json!({
                        "uuid": d.uuid,
                        "title": d.title,
                        "node_type": d.node_type,
                        "serial": d.serial,
                        "parent_label": d.parent_label,
                        "io_count": d.io_count
                    })
                })
                .collect();
            if section == "hardware_inventory" {
                return JsonRpcResponse::tool_ok(id, json!({ "hardware": hw_inventory }));
            }
            let periphery: Vec<Value> = if section == "all" || section == "periphery" {
                proj.periphery_tree
                    .iter()
                    .map(|n| tree_node_to_json(n, proj))
                    .collect()
            } else {
                Vec::new()
            };
            let system: Vec<Value> = if section == "all" || section == "system" {
                proj.system_tree
                    .iter()
                    .map(|n| tree_node_to_json(n, proj))
                    .collect()
            } else {
                Vec::new()
            };
            let templates: Vec<Value> = HardwareTemplateKind::ALL
                .iter()
                .map(|t| {
                    json!({
                        "template": format!("{:?}", t),
                        "display_name": t.display_name(),
                        "category": t.category().label(),
                        "summary": t.summary()
                    })
                })
                .collect();
            JsonRpcResponse::tool_ok(
                id,
                json!({
                    "periphery_tree": periphery,
                    "system_tree": system,
                    "hardware_inventory": hw_inventory,
                    "available_templates": templates
                }),
            )
        }
        "tree_add_device" => {
            let Some(template_str) = args.get("template").and_then(Value::as_str) else {
                return JsonRpcResponse::tool_err(id, "Missing required parameter 'template'");
            };
            let title = args.get("title").and_then(Value::as_str).unwrap_or("");
            let serial = args.get("serial").and_then(Value::as_str).unwrap_or("");
            let room_uuid = args.get("room_uuid").and_then(Value::as_str);
            let category_uuid = args.get("category_uuid").and_then(Value::as_str);
            let explicit_parent = args
                .get("parent_uuid")
                .and_then(Value::as_str)
                .map(str::to_string);

            let hw_kind = HardwareTemplateKind::ALL.iter().copied().find(|k| {
                format!("{:?}", k).eq_ignore_ascii_case(template_str)
                    || k.display_name().eq_ignore_ascii_case(template_str)
                    || k.type_id().eq_ignore_ascii_case(template_str)
            });

            let (section, parent_uuid, node) = {
                let Some(proj) = session.project() else {
                    return JsonRpcResponse::tool_err(id, "No project loaded");
                };
                if let Some(kind) = hw_kind {
                    let parent = explicit_parent.or_else(|| {
                        collect_parent_bus_candidates(proj, kind.category())
                            .first()
                            .map(|c| c.uuid.clone())
                    });
                    let n = build_hardware_tree_node(kind, title, serial, room_uuid, category_uuid);
                    (TreeSectionKind::Periphery, parent, n)
                } else {
                    let mut n = new_tree_node(template_str, title);
                    n.room_uuid = room_uuid.map(str::to_string);
                    n.category_uuid = category_uuid.map(str::to_string);
                    let sec = n.section;
                    (sec, explicit_parent, n)
                }
            };
            let node_uuid = node.uuid.clone();
            let node_title = node.title.clone();
            let children_uuids: Vec<Value> = node
                .children
                .iter()
                .map(|c| json!({ "uuid": c.uuid, "title": c.title, "type": c.control_type }))
                .collect();
            match session.apply_edit(ProjectEdit::AddTreeNode {
                section,
                parent_uuid: parent_uuid.clone(),
                node,
            }) {
                Ok(()) => JsonRpcResponse::tool_ok(
                    id,
                    json!({
                        "uuid": node_uuid,
                        "title": node_title,
                        "parent_uuid": parent_uuid,
                        "children": children_uuids
                    }),
                ),
                Err(e) => JsonRpcResponse::tool_err(id, e),
            }
        }
        _ => call_advanced_tool(session, id, tool_name, args),
    }
}
