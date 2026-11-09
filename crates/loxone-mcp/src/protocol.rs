use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
pub const DEFAULT_GUI_MCP_ADDR: &str = "127.0.0.1:8765";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn ok(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn err(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }

    pub fn tool_ok(id: Option<Value>, payload: Value) -> Self {
        let text = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string());
        Self::ok(
            id,
            json!({
                "content": [
                    {
                        "type": "text",
                        "text": text
                    }
                ],
                "structuredContent": payload,
                "isError": false
            }),
        )
    }

    pub fn tool_err(id: Option<Value>, message: impl Into<String>) -> Self {
        let msg = message.into();
        Self::ok(
            id,
            json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("Error: {msg}")
                    }
                ],
                "isError": true
            }),
        )
    }
}

pub fn tool_definitions() -> Value {
    json!({
        "tools": [
            {
                "name": "project_status",
                "description": "Returns active operating mode (gui_live or headless), loaded .Loxone path, project metadata, counts of pages/rooms/categories/blocks/wires, and undo/redo state.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "project_open",
                "description": "Loads a .Loxone project file from disk into the active session.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Path to the .Loxone file to open." }
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "project_save",
                "description": "Saves the current Loxone project XML to disk (defaults to the currently loaded path if path is omitted).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Optional target .Loxone path." }
                    }
                }
            },
            {
                "name": "project_create",
                "description": "Creates a new clean Loxone Config 17 project with initial rooms, categories, and Page 1.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string", "description": "Project title." },
                        "path": { "type": "string", "description": "Optional .Loxone path to save immediately." }
                    }
                }
            },
            {
                "name": "project_undo",
                "description": "Undoes the most recent project edit.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "project_redo",
                "description": "Redoes the most recently undone project edit.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "project_search",
                "description": "Searches across all pages, function blocks, periphery devices, and system tree nodes by query, object_type, room_uuid, or category_uuid.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Case-insensitive substring matching title, type, or UUID." },
                        "object_type": { "type": "string", "description": "Optional exact block/control type filter (e.g. LightController2, HeatIRoomController2)." },
                        "room_uuid": { "type": "string", "description": "Optional room UUID filter." },
                        "category_uuid": { "type": "string", "description": "Optional category UUID filter." }
                    }
                }
            },
            {
                "name": "page_list",
                "description": "Lists all schematic pages in the project with index, UUID, title, room/category, block count, and wire count.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "page_get",
                "description": "Returns full details of a schematic page (by page_uuid, title, or index), including all placed blocks, coordinates, pins, and wire connections.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "page_uuid": { "type": "string", "description": "Page UUID." },
                        "title": { "type": "string", "description": "Page title (case-insensitive)." },
                        "index": { "type": "integer", "description": "Zero-based page index." }
                    }
                }
            },
            {
                "name": "page_create",
                "description": "Creates a new schematic page in the project.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string", "description": "Title of the new page." },
                        "room_uuid": { "type": "string", "description": "Optional default room UUID for the page." },
                        "category_uuid": { "type": "string", "description": "Optional default category UUID for the page." }
                    },
                    "required": ["title"]
                }
            },
            {
                "name": "page_rename",
                "description": "Renames an existing schematic page.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "page_uuid": { "type": "string", "description": "UUID of the page to rename." },
                        "title": { "type": "string", "description": "New page title." }
                    },
                    "required": ["page_uuid", "title"]
                }
            },
            {
                "name": "page_delete",
                "description": "Deletes a schematic page by UUID.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "page_uuid": { "type": "string", "description": "UUID of the page to delete." }
                    },
                    "required": ["page_uuid"]
                }
            },
            {
                "name": "room_list",
                "description": "Lists all rooms (Place) in the project.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "room_create",
                "description": "Creates a new room (Place) in the project.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string", "description": "Room name." },
                        "sqm": { "type": "string", "description": "Optional room area in square meters." },
                        "p_type": { "type": "string", "description": "Optional Loxone room type code (0..8)." }
                    },
                    "required": ["title"]
                }
            },
            {
                "name": "room_rename",
                "description": "Renames an existing room (Place).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "room_uuid": { "type": "string", "description": "UUID of the room." },
                        "title": { "type": "string", "description": "New room name." }
                    },
                    "required": ["room_uuid", "title"]
                }
            },
            {
                "name": "room_delete",
                "description": "Deletes a room (Place) by UUID.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "room_uuid": { "type": "string", "description": "UUID of the room to delete." }
                    },
                    "required": ["room_uuid"]
                }
            },
            {
                "name": "category_list",
                "description": "Lists all categories (Category) in the project.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "category_create",
                "description": "Creates a new category in the project.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string", "description": "Category title." },
                        "cat_type": { "type": "string", "description": "Optional category group (e.g. lights, shading, climate, multimedia, safety)." }
                    },
                    "required": ["title"]
                }
            },
            {
                "name": "category_rename",
                "description": "Renames an existing category.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "category_uuid": { "type": "string", "description": "UUID of the category." },
                        "title": { "type": "string", "description": "New category title." }
                    },
                    "required": ["category_uuid", "title"]
                }
            },
            {
                "name": "category_delete",
                "description": "Deletes a category by UUID.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "category_uuid": { "type": "string", "description": "UUID of the category to delete." }
                    },
                    "required": ["category_uuid"]
                }
            },
            {
                "name": "block_catalog",
                "description": "Lists documented Loxone function block schemas (type_name, display_name, category_group, summary, input/output pins, and attributes).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Optional substring filter on type_name, display_name, or summary." },
                        "category": { "type": "string", "description": "Optional category_group filter (e.g. Lighting, Shading, Climate, Logic, Security & Safety, Audio, Energy)." }
                    }
                }
            },
            {
                "name": "block_get",
                "description": "Returns complete inspection details for a specific schematic block or periphery/system tree node by UUID.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "uuid": { "type": "string", "description": "UUID of the block or tree node." }
                    },
                    "required": ["uuid"]
                }
            },
            {
                "name": "block_add",
                "description": "Adds a function block onto a schematic page with full schema pins and optional room/category assignment.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "page_uuid": { "type": "string", "description": "Target page UUID (or provide page_title)." },
                        "page_title": { "type": "string", "description": "Target page title if page_uuid is omitted." },
                        "block_type": { "type": "string", "description": "Loxone block type name (e.g. LightController2, AutoJalousie, HeatIRoomController2, Switch, And, Or, StairwellLight, Alarm)." },
                        "title": { "type": "string", "description": "Display title for the block." },
                        "px": { "type": "number", "description": "Optional X coordinate on page." },
                        "py": { "type": "number", "description": "Optional Y coordinate on page." },
                        "room_uuid": { "type": "string", "description": "Optional room UUID to assign (<IoData Pr=.../>)." },
                        "category_uuid": { "type": "string", "description": "Optional category UUID to assign (<IoData Cr=.../>)." }
                    },
                    "required": ["block_type"]
                }
            },
            {
                "name": "block_add_reference",
                "description": "Places an InputRef, OutputRef, OutputRefLM, or Memory reference bar on a schematic page referencing a periphery or system tree node.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "page_uuid": { "type": "string", "description": "Target page UUID (or provide page_title)." },
                        "page_title": { "type": "string", "description": "Target page title if page_uuid is omitted." },
                        "target_uuid": { "type": "string", "description": "UUID of the periphery/system TreeNode to reference." },
                        "ref_type": { "type": "string", "description": "Reference block type: InputRef, OutputRef, OutputRefLM, or Memory (auto-detected if omitted)." },
                        "px": { "type": "number", "description": "Optional X coordinate on page." },
                        "py": { "type": "number", "description": "Optional Y coordinate on page." }
                    },
                    "required": ["target_uuid"]
                }
            },
            {
                "name": "block_update",
                "description": "Updates a block or tree node's title, description, room, category, color, rating, visualization settings, coordinates, or arbitrary XML attributes.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "uuid": { "type": "string", "description": "UUID of the block or tree node." },
                        "title": { "type": "string", "description": "New Title." },
                        "desc": { "type": "string", "description": "New Desc (description)." },
                        "room_uuid": { "type": "string", "description": "Room UUID (pass empty string to clear)." },
                        "category_uuid": { "type": "string", "description": "Category UUID (pass empty string to clear)." },
                        "color": { "type": "string", "description": "RGB color string 'R,G,B'." },
                        "rating": { "type": "string", "description": "Complexity/star rating '0'..'5'." },
                        "use_in_visu": { "type": "boolean", "description": "Whether the block is shown in visualization." },
                        "use_in_fav": { "type": "boolean", "description": "Whether the block is marked as a favorite." },
                        "visu_title": { "type": "string", "description": "Alternative visualization title (VisuTitle)." },
                        "visu_pwd": { "type": "string", "description": "Visualization password / PIN protection (VisuPwd)." },
                        "px": { "type": "number", "description": "New X coordinate on page." },
                        "py": { "type": "number", "description": "New Y coordinate on page." },
                        "px2": { "type": "number", "description": "New right X2 coordinate on page." },
                        "py2": { "type": "number", "description": "New bottom Y2 coordinate on page." },
                        "attributes": { "type": "object", "description": "Map of arbitrary XML attribute key -> string or null to set/remove." }
                    },
                    "required": ["uuid"]
                }
            },
            {
                "name": "block_delete",
                "description": "Deletes one or more schematic blocks (or tree nodes) by UUID and cleans up dangling wire links.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "uuid": { "type": "string", "description": "Single UUID to delete." },
                        "uuids": { "type": "array", "items": { "type": "string" }, "description": "List of UUIDs to delete." }
                    }
                }
            },
            {
                "name": "wire_connect",
                "description": "Connects an output pin on source_block_uuid to an input pin on target_block_uuid.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source_block_uuid": { "type": "string", "description": "UUID of the source block." },
                        "source_pin_key": { "type": "string", "description": "Output pin key on the source block (e.g. Q, AQ1, O, OutHeat1)." },
                        "target_block_uuid": { "type": "string", "description": "UUID of the target block." },
                        "target_pin_key": { "type": "string", "description": "Input pin key on the target block (e.g. I, Tr, I1, AI1, Dis1)." }
                    },
                    "required": ["source_block_uuid", "source_pin_key", "target_block_uuid", "target_pin_key"]
                }
            },
            {
                "name": "wire_disconnect",
                "description": "Disconnects a wire link feeding target_block_uuid's target_pin_key from source_block_uuid (or source_pin_uuid).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "target_block_uuid": { "type": "string", "description": "UUID of the target block." },
                        "target_pin_key": { "type": "string", "description": "Input pin key on the target block." },
                        "source_block_uuid": { "type": "string", "description": "Optional source block UUID (if omitted and source_pin_uuid is omitted, removes all incoming links on target_pin_key)." },
                        "source_pin_key": { "type": "string", "description": "Optional output pin key on source_block_uuid." },
                        "source_pin_uuid": { "type": "string", "description": "Optional exact source pin UUID." }
                    },
                    "required": ["target_block_uuid", "target_pin_key"]
                }
            },
            {
                "name": "cross_page_link_create",
                "description": "Creates a cross-page Memory flag link from an output pin on source_page_uuid to a paired Memory reader block on target_page_uuid.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "source_page_uuid": { "type": "string", "description": "Source page UUID." },
                        "source_block_uuid": { "type": "string", "description": "Source block UUID." },
                        "source_pin_key": { "type": "string", "description": "Source output pin key." },
                        "target_page_uuid": { "type": "string", "description": "Target page UUID." },
                        "target_px": { "type": "number", "description": "Optional X coordinate on target page." },
                        "target_py": { "type": "number", "description": "Optional Y coordinate on target page." }
                    },
                    "required": ["source_page_uuid", "source_block_uuid", "source_pin_key", "target_page_uuid"]
                }
            },
            {
                "name": "pin_configure",
                "description": "Configures a pin's default parameter value (Def) and/or digital logic inversion (Inv).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "control_uuid": { "type": "string", "description": "UUID of the block or tree node." },
                        "pin_key": { "type": "string", "description": "Pin key (e.g. T, Delay, TH, TL, I1)." },
                        "default_value": { "type": ["string", "null"], "description": "Default parameter value (or null to clear)." },
                        "inverted": { "type": "boolean", "description": "Whether the digital pin is inverted." }
                    },
                    "required": ["control_uuid", "pin_key"]
                }
            },
            {
                "name": "tree_list",
                "description": "Lists the Periphery (Link/Tree/Air hardware) and System tree hierarchy, plus available hardware provisioning templates.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "section": { "type": "string", "enum": ["all", "periphery", "system", "hardware_inventory"], "description": "Which tree section to return (default: all)." }
                    }
                }
            },
            {
                "name": "tree_add_device",
                "description": "Provisions a new hardware extension/device (e.g. TreeExtension, RelayExtension, DimmerExtension, TouchPureTree, PresenceSensorTree, RgbwDimmerTree, TouchPureAir) or generic tree node into the Periphery/System tree.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "template": { "type": "string", "description": "Hardware template name (TreeExtension, AirBaseExtension, RelayExtension, DimmerExtension, AudioServer, ModbusExtension, TouchPureTree, PresenceSensorTree, RgbwDimmerTree, Nano2RelayTree, NfcCodeTouchTree, ValveActuatorTree, TouchPureAir, SmokeDetectorAir, WaterSensorAir, NanoDimmerAir) or control_type (DigitalIn, Actor, VirtualIn, VirtualOut, Memory)." },
                        "title": { "type": "string", "description": "Device or channel title." },
                        "serial": { "type": "string", "description": "Optional Loxone hex serial number." },
                        "parent_uuid": { "type": "string", "description": "Optional parent bus/caption UUID." },
                        "room_uuid": { "type": "string", "description": "Optional room UUID." },
                        "category_uuid": { "type": "string", "description": "Optional category UUID." }
                    },
                    "required": ["template"]
                }
            },
            {
                "name": "config_object_assignments_get",
                "description": "Reads candidate and linked I/O object assignments (Objects / UO) for Alarm, SmokeAlarm, FireEmergency, WaterCentral, CentralLightController, CentralJalousie, CentralGate, WindowMonitor, PresenceController, or LoadManager.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "block_uuid": { "type": "string", "description": "UUID of the target function block." },
                        "only_selected": { "type": "boolean", "description": "If true, returns only currently assigned inputs/outputs." }
                    },
                    "required": ["block_uuid"]
                }
            },
            {
                "name": "config_object_assignments_set",
                "description": "Updates linked input/output object assignments for a Central/Alarm/Monitor function block.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "block_uuid": { "type": "string", "description": "UUID of the target function block." },
                        "assignments": {
                            "type": "array",
                            "description": "List of target_uuid assignments to update.",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "target_uuid": { "type": "string" },
                                    "is_input": { "type": "boolean" },
                                    "is_output": { "type": "boolean" }
                                },
                                "required": ["target_uuid"]
                            }
                        }
                    },
                    "required": ["block_uuid", "assignments"]
                }
            },
            {
                "name": "config_light_controller_get",
                "description": "Reads circuits, lighting moods/scenes, automation rules, and HCL daylight curve from a LightController2 block.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "block_uuid": { "type": "string", "description": "UUID of the LightController2 block." }
                    },
                    "required": ["block_uuid"]
                }
            },
            {
                "name": "config_light_controller_set",
                "description": "Updates circuits, lighting scenes/moods, automation rules, or HCL settings on a LightController2 block.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "block_uuid": { "type": "string", "description": "UUID of the LightController2 block." },
                        "circuits": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "index": { "type": "integer", "description": "Circuit index 0..7." },
                                    "name": { "type": "string" },
                                    "circuit_type": { "type": "string", "description": "Switch, Dimmer0To100, Dimmer0To10V, Dimmer1To10V, Rgb, LumitechDmx, or SmartActuator." }
                                },
                                "required": ["index"]
                            }
                        },
                        "scenes": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "sid": { "type": "integer", "description": "Scene ID (1..99 or 778)." },
                                    "name": { "type": "string" },
                                    "short_name": { "type": "string" },
                                    "mix_mode": { "type": "boolean" },
                                    "add_mode": { "type": "boolean" },
                                    "move_default": { "type": "boolean" },
                                    "circuit_pcts": { "type": "array", "items": { "type": "integer" }, "description": "Up to 8 brightness percentages (0..100)." }
                                }
                            }
                        },
                        "hcl": {
                            "type": "object",
                            "properties": {
                                "mode": { "type": "string", "enum": ["SunriseSunset", "FixedPeriod"] },
                                "tmin_kelvin": { "type": "integer" },
                                "tmax_kelvin": { "type": "integer" },
                                "from_seconds": { "type": "integer" },
                                "to_seconds": { "type": "integer" }
                            }
                        }
                    },
                    "required": ["block_uuid"]
                }
            },
            {
                "name": "config_room_controller_get",
                "description": "Reads energy source groups, heating/cooling priorities, and Timer schedule entries from a HeatIRoomController2 block.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "block_uuid": { "type": "string", "description": "UUID of the HeatIRoomController2 block." }
                    },
                    "required": ["block_uuid"]
                }
            },
            {
                "name": "config_room_controller_set",
                "description": "Updates schedule intervals, default temperature mode, or energy source groups on a HeatIRoomController2 block.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "block_uuid": { "type": "string", "description": "UUID of the HeatIRoomController2 block." },
                        "default_mode": { "type": "string", "enum": ["Economy", "Comfort", "BuildingProtection", "FixedValue"] },
                        "set_intervals": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "row_ix": { "type": "integer" },
                                    "from_min": { "type": "integer", "description": "Minutes from midnight (0..1440)." },
                                    "to_min": { "type": "integer", "description": "Minutes from midnight (0..1440)." },
                                    "mode": { "type": "string", "enum": ["Economy", "Comfort", "BuildingProtection", "FixedValue"] }
                                },
                                "required": ["row_ix", "from_min", "to_min", "mode"]
                            }
                        },
                        "add_energy_source_uuids": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    },
                    "required": ["block_uuid"]
                }
            },
            {
                "name": "config_efm_get",
                "description": "Reads Energy Flow Monitor (EFM) tree nodes (Grid, Production, Battery, Consumer, Group), units, and statistics intervals.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "block_uuid": { "type": "string", "description": "UUID of the EFM block." }
                    },
                    "required": ["block_uuid"]
                }
            },
            {
                "name": "config_efm_set",
                "description": "Updates an Energy Flow Monitor (EFM) block's units, statistics intervals, and child meter nodes.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "block_uuid": { "type": "string", "description": "UUID of the EFM block." },
                        "unit_power": { "type": "string" },
                        "unit_energy": { "type": "string" },
                        "add_children": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "title": { "type": "string" },
                                    "node_type": { "type": "string", "enum": ["Grid", "Battery", "Consumer", "Production", "Group"] },
                                    "ctrl_uuid": { "type": "string" }
                                },
                                "required": ["title", "node_type"]
                            }
                        }
                    },
                    "required": ["block_uuid"]
                }
            },
            {
                "name": "config_audio_zone_get",
                "description": "Reads AudioZone / MusicPlayer / StereoExtension / MusicPlayerGroup configuration, volume presets, assigned Audioserver AudioOut channels, player group sync, and room favorites.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "block_uuid": { "type": "string", "description": "UUID of the AudioZone / MusicPlayer block." }
                    },
                    "required": ["block_uuid"]
                }
            },
            {
                "name": "config_audio_zone_set",
                "description": "Updates AudioZone / MusicPlayer / StereoExtension / MusicPlayerGroup parameters, volume presets, assigned AudioOut channels (with MxV/VOLL/Vg), player group synchronization, and room favorites.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "block_uuid": { "type": "string", "description": "UUID of the AudioZone / MusicPlayer block." },
                        "speaker_cfg": { "type": "integer", "description": "Speaker mode (1=Mono, 2=Stereo, 4=Quad)." },
                        "presence_enabled": { "type": "boolean" },
                        "airplay_enabled": { "type": "boolean" },
                        "spotify_enabled": { "type": "boolean" },
                        "high_pass_filter": { "type": "boolean" },
                        "default_vol": { "type": "integer" },
                        "max_vol": { "type": "integer" },
                        "step_vol": { "type": "integer" },
                        "alarm_vol": { "type": "integer" },
                        "bell_vol": { "type": "integer" },
                        "buzzer_vol": { "type": "integer" },
                        "tts_vol": { "type": "integer" },
                        "assigned_channel_uuids": { "type": "array", "items": { "type": "string" } },
                        "master_player_uuid": { "type": "string" },
                        "grouped_player_uuids": { "type": "array", "items": { "type": "string" } },
                        "favorites": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "slot": { "type": "integer" },
                                    "title": { "type": "string" },
                                    "source_uri": { "type": "string" }
                                },
                                "required": ["title", "source_uri"]
                            }
                        }
                    },
                    "required": ["block_uuid"]
                }
            },
            {
                "name": "tree_replace_device",
                "description": "Replaces a hardware device's serial number in-place and updates commissioning metadata (description, distribution_board, row_position, lighting_group) while preserving child I/O channels and schematic wire links.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "device_uuid": { "type": "string", "description": "UUID of the hardware device node in Periphery/System tree." },
                        "new_serial": { "type": "string", "description": "New Loxone serial number." },
                        "description": { "type": "string" },
                        "distribution_board": { "type": "string" },
                        "row_position": { "type": "string" },
                        "lighting_group": { "type": "string" }
                    },
                    "required": ["device_uuid", "new_serial"]
                }
            },
            {
                "name": "tree_resolve_serial_conflicts",
                "description": "Detects and optionally resolves duplicate or unassigned ('00000000') hardware serial numbers across all Periphery/System devices.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "preview_only": { "type": "boolean", "description": "If true, only lists conflicts and planned serial assignments without mutating the project." }
                    }
                }
            },
            {
                "name": "page_auto_layout",
                "description": "Runs the topological Sugiyama-style schematic auto-layout engine on a page, arranging InputRefs on the left, function blocks in signal-flow columns, and OutputRefs on the right snapped to the 64-unit grid.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "page_uuid": { "type": "string", "description": "UUID of the schematic page to auto-layout." }
                    },
                    "required": ["page_uuid"]
                }
            },
            {
                "name": "todo_list",
                "description": "Lists all To-Do / Memo notes across schematic pages.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "todo_add",
                "description": "Adds a new To-Do / Memo note block to a schematic page.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "page_uuid": { "type": "string" },
                        "title": { "type": "string" },
                        "text": { "type": "string" },
                        "px": { "type": "number" },
                        "py": { "type": "number" }
                    },
                    "required": ["page_uuid", "title"]
                }
            },
            {
                "name": "todo_resolve",
                "description": "Resolves (marks Done) or deletes a To-Do / Memo note by UUID.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "todo_uuid": { "type": "string" },
                        "delete_block": { "type": "boolean" }
                    },
                    "required": ["todo_uuid"]
                }
            },
            {
                "name": "autopilot_rule_create",
                "description": "Creates a Loxone Autopilot IF/AND/OR condition -> action rule on a schematic page.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string" },
                        "page_uuid": { "type": "string" },
                        "logic_op": { "type": "string", "enum": ["AND", "OR"] },
                        "condition_source_uuids": { "type": "array", "items": { "type": "string" } },
                        "target_block_uuid": { "type": "string" },
                        "target_pin_key": { "type": "string" },
                        "px": { "type": "number" },
                        "py": { "type": "number" }
                    },
                    "required": ["title", "page_uuid"]
                }
            },
            {
                "name": "project_diff",
                "description": "Compares the active project against another .Loxone file on disk (or compares two .Loxone files base_path vs target_path) and returns added, removed, and modified pages, blocks, and tree nodes.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "base_path": { "type": "string", "description": "Optional path to the baseline .Loxone file (defaults to loaded_path on disk)." },
                        "target_path": { "type": "string", "description": "Optional path to the target .Loxone file (defaults to the active in-memory project)." }
                    }
                }
            },
            {
                "name": "wizard_auto_configure_rooms",
                "description": "Previews or executes the Auto Configuration room logic wizard to generate standard LightController2, AutoJalousie, HeatIRoomController2, and/or AudioZone blocks across rooms.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "preview_only": { "type": "boolean", "description": "If true, returns current room coverage and planned blocks without mutating the project." },
                        "room_uuids": { "type": "array", "items": { "type": "string" }, "description": "Optional subset of room UUIDs to configure (defaults to all rooms missing standard controllers)." },
                        "gen_lighting": { "type": "boolean", "description": "Generate LightController2 when missing (default: true)." },
                        "gen_shading": { "type": "boolean", "description": "Generate AutoJalousie when missing (default: true)." },
                        "gen_climate": { "type": "boolean", "description": "Generate HeatIRoomController2 when missing (default: true)." },
                        "gen_audio": { "type": "boolean", "description": "Generate AudioZone when missing (default: false)." }
                    }
                }
            },
            {
                "name": "bulk_find_and_replace",
                "description": "Previews or applies bulk Find & Replace across block and tree node titles, descriptions, and XML attributes.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Substring to search for." },
                        "replacement": { "type": "string", "description": "Replacement string." },
                        "preview_only": { "type": "boolean", "description": "If true, only previews matches without modifying the project." },
                        "case_sensitive": { "type": "boolean", "description": "Case-sensitive search (default: false)." },
                        "in_titles": { "type": "boolean", "description": "Search in Title (default: true)." },
                        "in_descriptions": { "type": "boolean", "description": "Search in Desc (default: true)." },
                        "in_attributes": { "type": "boolean", "description": "Search in raw XML attributes (default: false)." }
                    },
                    "required": ["query", "replacement"]
                }
            },
            {
                "name": "analysis_run",
                "description": "Runs the 6-category Loxone Config 17 Project Analysis linter and returns all diagnostics plus category counts.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "category": { "type": "string", "description": "Optional category filter." },
                        "severity": { "type": "string", "enum": ["Error", "Warning", "Info"], "description": "Optional severity filter." }
                    }
                }
            },
            {
                "name": "sim_control",
                "description": "Controls the offline logic simulator (start, stop, reset, step, set_time, set_input, status) and returns live evaluated pin/block values.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["start", "stop", "reset", "step", "set_time", "set_input", "status"],
                            "description": "Simulation command to execute."
                        },
                        "seconds": { "type": "integer", "description": "Seconds to advance when action='step' (default: 1)." },
                        "iso_time": { "type": "string", "description": "Virtual timestamp 'YYYY-MM-DD HH:MM:SS' when action='set_time'." },
                        "control_uuid": { "type": "string", "description": "Block or tree node UUID when action='set_input'." },
                        "pin_key": { "type": "string", "description": "Optional pin key when action='set_input'." },
                        "value": { "type": "string", "description": "Signal value (e.g. '1', '0', '22.5') when action='set_input'." },
                        "page_uuid": { "type": "string", "description": "Optional page UUID whose evaluated block/pin values should be returned." }
                    },
                    "required": ["action"]
                }
            },
            {
                "name": "report_export",
                "description": "Generates a project report (page_svg, documentation_html, technical_report_html, project_analysis_csv, component_bom_csv, or terminal_schedule_csv) and optionally writes it to output_path.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "format": {
                            "type": "string",
                            "enum": ["page_svg", "documentation_html", "technical_report_html", "project_analysis_csv", "component_bom_csv", "terminal_schedule_csv"],
                            "description": "Report format to generate."
                        },
                        "page_uuid": { "type": "string", "description": "Page UUID (required when format='page_svg' unless page_index is provided)." },
                        "page_index": { "type": "integer", "description": "Zero-based page index for page_svg." },
                        "output_path": { "type": "string", "description": "Optional file path on disk to write the generated report." }
                    },
                    "required": ["format"]
                }
            },
            {
                "name": "project_bom_get",
                "description": "Returns the structured Bill of Materials (BOM) including all hardware devices, DIN distribution board/row placement, wired channel counts, and part summaries.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "distribution_board": { "type": "string", "description": "Optional filter by Distribution Board (DistBoard)." }
                    }
                }
            },
            {
                "name": "project_terminal_schedule_get",
                "description": "Returns the physical DIN-Rail & Terminal Wiring Schedule mapping each device terminal (I1, AI1, Q1, AQ1, etc.) to its room, category, and wired schematic pages & function blocks.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "device_uuid": { "type": "string", "description": "Optional filter by hardware device UUID." },
                        "wired_only": { "type": "boolean", "description": "If true, returns only terminals wired on a schematic page." }
                    }
                }
            },
            {
                "name": "block_pin_set_inverted",
                "description": "Sets or toggles logical negation (FLG / Inv red negation dot) on an input pin of a function block.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "block_uuid": { "type": "string", "description": "UUID of the function block." },
                        "pin_key": { "type": "string", "description": "Connector input pin key (e.g. 'I1', 'Tr', 'Dis', 'R')." },
                        "inverted": { "type": "boolean", "description": "Whether the input pin should be logically negated." }
                    },
                    "required": ["block_uuid", "pin_key", "inverted"]
                }
            },
            {
                "name": "page_align_blocks",
                "description": "Aligns, distributes, or grid-snaps selected blocks (or all blocks on a page) using 64-unit Loxone grid coordinates.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "page_uuid": { "type": "string", "description": "UUID of the schematic page." },
                        "mode": {
                            "type": "string",
                            "enum": ["align_left", "align_center_h", "align_right", "align_top", "align_middle_v", "align_bottom", "distribute_h", "distribute_v", "snap_to_grid"],
                            "description": "Alignment or distribution mode."
                        },
                        "block_uuids": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Optional list of block UUIDs to align (defaults to all positioned blocks on the page)."
                        }
                    },
                    "required": ["page_uuid", "mode"]
                }
            },
            {
                "name": "multiplier_units_list",
                "description": "Lists all configured Multiplier units (Miniserver serials, addresses, and per-device serial overrides) in the active project.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "multiplier_unit_upsert",
                "description": "Creates or updates a Multiplier unit in the project and marks the project as a Multiplier template.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "unit_uuid": { "type": "string", "description": "Optional existing Multiplier unit UUID to update." },
                        "title": { "type": "string", "description": "Unit name (e.g. 'Suite 201', 'Apartment 3B')." },
                        "miniserver_serial": { "type": "string", "description": "Miniserver serial number for this unit." },
                        "address": { "type": "string", "description": "IP or Cloud DNS address for this unit." },
                        "description": { "type": "string", "description": "Optional unit description." },
                        "device_serial_overrides": {
                            "type": "array",
                            "description": "Optional list of per-device serial overrides ({device_uuid, serial}).",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "device_uuid": { "type": "string" },
                                    "serial": { "type": "string" }
                                },
                                "required": ["device_uuid", "serial"]
                            }
                        }
                    },
                    "required": ["title", "miniserver_serial"]
                }
            },
            {
                "name": "multiplier_export_unit",
                "description": "Instantiates a standalone .Loxone XML project for a specific Multiplier unit (applying its Miniserver serial/address and device serial overrides) and optionally writes it to output_path.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "unit_uuid": { "type": "string", "description": "UUID of the Multiplier unit to instantiate." },
                        "output_path": { "type": "string", "description": "Optional file path on disk to write the instantiated .Loxone file." }
                    },
                    "required": ["unit_uuid"]
                }
            },
            {
                "name": "sim_weather_get",
                "description": "Returns the current simulated Weather Server environmental state and computed astronomical solar elevation & azimuth angles.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "sim_weather_set",
                "description": "Updates the simulated Weather Server environmental state (outdoor temperature °C, wind speed km/h, solar radiation W/m², rain, humidity %) and recomputes all schematic logic.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "outdoor_temp_c": { "type": "number", "description": "Outdoor temperature in °C." },
                        "wind_speed_kmh": { "type": "number", "description": "Wind speed in km/h." },
                        "solar_radiation_wm2": { "type": "number", "description": "Solar radiation in W/m²." },
                        "rain_active": { "type": "boolean", "description": "Whether rain / precipitation is active." },
                        "relative_humidity_pct": { "type": "number", "description": "Outdoor relative humidity %." }
                    }
                }
            },
            {
                "name": "sim_run_scenario",
                "description": "Executes a deterministic multi-step simulation test scenario (or the auto-generated project regression suite when 'steps' is omitted) and returns per-step pass/fail assertions.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Scenario name (default: 'MCP Test Scenario')." },
                        "description": { "type": "string", "description": "Optional scenario description." },
                        "steps": {
                            "type": "array",
                            "description": "Optional ordered list of simulation steps. If omitted, runs all auto-generated project regression scenarios.",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "label": { "type": "string", "description": "Step description." },
                                    "set_iso_time": { "type": "string", "description": "Optional virtual timestamp 'YYYY-MM-DD HH:MM:SS'." },
                                    "set_outdoor_temp_c": { "type": "number" },
                                    "set_wind_speed_kmh": { "type": "number" },
                                    "set_solar_radiation_wm2": { "type": "number" },
                                    "set_rain_active": { "type": "boolean" },
                                    "advance_seconds": { "type": "integer", "description": "Seconds to advance virtual clock (default: 0)." },
                                    "inputs": {
                                        "type": "array",
                                        "items": {
                                            "type": "object",
                                            "properties": {
                                                "control_uuid": { "type": "string" },
                                                "pin_key": { "type": "string" },
                                                "value": { "type": "string" }
                                            },
                                            "required": ["control_uuid", "value"]
                                        }
                                    },
                                    "assertions": {
                                        "type": "array",
                                        "items": {
                                            "type": "object",
                                            "properties": {
                                                "control_uuid": { "type": "string" },
                                                "pin_key": { "type": "string" },
                                                "comparison": { "type": "string", "enum": ["eq", "ne", "gt", "ge", "lt", "le"] },
                                                "expected": { "type": "string" }
                                            },
                                            "required": ["control_uuid", "expected"]
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        ]
    })
}

pub fn resource_definitions() -> Value {
    json!({
        "resources": [
            {
                "uri": "loxone://project/summary",
                "name": "Active Loxone Project Summary",
                "description": "High-level metadata, room/category/page counts, and status of the loaded .Loxone project.",
                "mimeType": "application/json"
            },
            {
                "uri": "loxone://project/pages",
                "name": "Schematic Pages Index",
                "description": "List of all schematic pages and block/wire counts.",
                "mimeType": "application/json"
            },
            {
                "uri": "loxone://project/analysis",
                "name": "Project Analysis Diagnostics",
                "description": "Loxone Config 17 6-category Project Analysis results.",
                "mimeType": "application/json"
            },
            {
                "uri": "loxone://project/xml",
                "name": "Serialized .Loxone XML",
                "description": "Current lossless XML serialization of the active project.",
                "mimeType": "application/xml"
            }
        ]
    })
}
