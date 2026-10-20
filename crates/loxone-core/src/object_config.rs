use std::collections::{HashMap, HashSet};

use crate::block_schema::lookup_block_schema;
use crate::factory::generate_loxone_uuid;
use crate::model::{ConnectorPin, LoxoneProject, PageBlock, PinKind, TreeNode, WireLink};
use crate::writer::ProjectEdit;

/// Identifies whether a row in the Object Configuration (`Konfiguracja obiektu`) dialog
/// refers to a periphery/system [`TreeNode`] or a schematic [`PageBlock`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectConfigTargetKind {
    PeripheryNode {
        node_uuid: String,
    },
    PageFunctionBlock {
        block_uuid: String,
        page_uuid: String,
    },
}

/// A single row in the Loxone Config–style "Konfiguracja obiektu" ("Object Configuration")
/// modal table (matching the Burglar Alarm / Fire & Water Central / Room Controller dialog).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectConfigRow {
    pub target_uuid: String,
    pub target_kind: ObjectConfigTargetKind,
    /// If this periphery node is currently linked via one or more unpositioned `InputRef`/`OutputRef`
    /// blocks on schematic pages, stores `(page_uuid, ref_block_uuid)` for each.
    pub existing_ref_blocks: Vec<(String, String)>,
    pub can_be_input: bool,
    pub can_be_output: bool,
    pub is_input: bool,
    pub is_output: bool,
    /// `Nazwa` (Object / Sensor / Block Name)
    pub name: String,
    /// `Wyświetlana nazwa` (Optional Display Name / Description)
    pub display_name: String,
    /// `Urządzenie` (Parent Periphery Device Title + Installation location)
    pub device_name: String,
    /// `Pomieszczenie` (Room Title)
    pub room_name: String,
    /// `Typ pomieszczenia` (Human-readable Room Type from `Place.p_type`)
    pub room_type: String,
    /// `Kategoria` (Category Title)
    pub category_name: String,
    /// `Typ obiektu` (Human-readable Device Model or Function Block Type)
    pub object_type: String,
}

/// Returns `true` if `block` supports the Loxone Config "Konfiguracja obiektu"
/// ("Object Configuration") modal for assigning multi-object inputs and outputs.
pub fn supports_object_config(block: &PageBlock) -> bool {
    if matches!(
        block.block_type.as_str(),
        "HeatIRoomController"
            | "HeatIRoomController2"
            | "EFM"
            | "LightController2"
            | "LightControllerV2"
            | "LightController"
    ) {
        return false;
    }
    if block
        .raw_attributes
        .iter()
        .any(|(k, _)| k == "Objects" || k == "UO")
    {
        return true;
    }
    matches!(
        block.block_type.as_str(),
        "Alarm"
            | "SmokeAlarm"
            | "FireEmergency"
            | "WaterCentral"
            | "Doorcontroller"
            | "Intercom"
            | "ClimateController"
            | "Central"
            | "CentralLightController"
            | "CentralJalousie"
            | "CentralGate"
            | "WindowMonitor"
            | "PresenceController"
            | "LoadManager"
    )
}

/// Maps Loxone `Place` `PType` code to the localized Polish label shown in Loxone Config.
pub fn format_room_type(p_type: Option<&str>) -> String {
    match p_type.map(str::trim) {
        Some("1") => "Sypialniane".to_string(),
        Some("2") => "Pobytowe".to_string(),
        Some("3") => "Komunikacyjne".to_string(),
        Some("4") => "Centralne".to_string(),
        Some("5") => "Inne".to_string(),
        Some("6") => "Sanitarne".to_string(),
        Some("7") => "Kuchnia".to_string(),
        Some("8") => "Garaż".to_string(),
        Some(other) if !other.is_empty() => format!("Typ {}", other),
        _ => String::new(),
    }
}

/// Maps a periphery device [`TreeNode`] (`TreeDevice`, `LoxAIRDevice`, `LoxAIR`, `Extension`, etc.)
/// to its human-readable Loxone model name (`Typ obiektu`).
pub fn resolve_periphery_object_type(device_node: Option<&TreeNode>, leaf_node: &TreeNode) -> String {
    if let Some(dev) = device_node {
        let tid = get_attr(&dev.raw_attributes, "TId").unwrap_or("");
        let sub_type = get_attr(&dev.raw_attributes, "SubType").unwrap_or("");
        match tid {
            "PresenceSensorTree" => return "Czujnik obecności Tree".to_string(),
            "TouchTree" => return "Touch Tree".to_string(),
            "TouchPureTree" => return "Touch Pure Tree".to_string(),
            "TouchStoneTree" => return "Touch Surface Tree".to_string(),
            "TouchFlexTree" => return "Touch Pure Flex Tree".to_string(),
            "NFCCodeTouchTree" => return "NFC Code Touch Tree".to_string(),
            "Nano2RelayTree" => return "Nano 2 Relay Tree".to_string(),
            "NoisemakerTree" => return "Syrena alarmowa Tree".to_string(),
            "RgbwDimmerTree" => return "RGBW 24V Dimmer Tree".to_string(),
            "FirealarmAir" => return "Czujnik dymu Air".to_string(),
            "WaterSensor" => return "Czujnik wody Air".to_string(),
            "AmbientLight" => return "Touch Nightlight Air".to_string(),
            "NanoDimmer" => return "Nano Dimmer Air".to_string(),
            "TouchPureAir_V2" | "TouchPureAir" => return "Touch Pure Air".to_string(),
            _ => {}
        }
        match sub_type {
            "32794" => return "Czujnik obecności Tree".to_string(),
            "32771" => return "Touch Tree".to_string(),
            "32773" => return "Touch Pure Tree".to_string(),
            "32781" => return "Touch Surface Tree".to_string(),
            "32799" => return "Touch Pure Flex Tree".to_string(),
            "32796" => return "NFC Code Touch Tree".to_string(),
            "32793" => return "Nano 2 Relay Tree".to_string(),
            "32786" => return "Syrena alarmowa Tree".to_string(),
            "32780" => return "RGBW 24V Dimmer Tree".to_string(),
            _ => {}
        }
        match dev.control_type.as_str() {
            "LoxAIR" => return "Air Base Extension".to_string(),
            "LoxTREE" | "TreeExtension" => return "Tree Extension".to_string(),
            "Extension" => return "Extension".to_string(),
            "RelayExtension" => return "Relay Extension".to_string(),
            "DimmerExtension" => return "Dimmer Extension".to_string(),
            "Comm485" => return "Modbus Extension".to_string(),
            _ => {
                if !dev.title.is_empty() {
                    return dev.title.clone();
                }
            }
        }
    }
    if let Some(schema) = lookup_block_schema(&leaf_node.control_type) {
        return schema.display_name_en.to_string();
    }
    leaf_node.control_type.clone()
}

/// Maps a schematic function block type to the Polish object type name displayed in Loxone Config.
pub fn resolve_function_block_object_type(block_type: &str) -> String {
    match block_type {
        "LightController2" | "LightControllerV2" | "LightController" => {
            "Sterownik oświetlenia".to_string()
        }
        "AutoJalousie" | "Jalousie" => "Automatyczne zacienianie".to_string(),
        "MusicPlayer" | "AudioZone" => "Odtwarzacz audio".to_string(),
        "HeatIRoomController2" | "IRoomControllerV2" | "IRoomController" => {
            "Sterownik pomieszczeniowy".to_string()
        }
        "ClimateController" => "Sterownik klimatu".to_string(),
        "Alarm" => "System alarmowy".to_string(),
        "SmokeAlarm" | "FireEmergency" => "Centrala pożarowa".to_string(),
        "WaterCentral" => "Centrala czujników wycieku wody".to_string(),
        "Doorcontroller" => "Sterownik drzwi i okien".to_string(),
        "Intercom" => "Intercom".to_string(),
        "WindowMonitor" => "Monitor okien i drzwi".to_string(),
        "Central" => "Obiekt centralny".to_string(),
        _ => lookup_block_schema(block_type)
            .map(|s| s.display_name_en.to_string())
            .unwrap_or_else(|| block_type.to_string()),
    }
}

fn get_attr<'a>(attrs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

fn is_candidate_function_block_for_target(target_block_type: &str, block_type: &str) -> bool {
    match target_block_type {
        "Alarm" | "SmokeAlarm" | "FireEmergency" | "WaterCentral" => matches!(
            block_type,
            "LightController2"
                | "LightControllerV2"
                | "LightController"
                | "AutoJalousie"
                | "Jalousie"
                | "MusicPlayer"
                | "AudioZone"
        ),
        "CentralLightController" => matches!(
            block_type,
            "LightController2"
                | "LightControllerV2"
                | "LightController"
                | "PresenceLight"
                | "StairwellLight"
                | "Switch"
        ),
        "CentralJalousie" => matches!(block_type, "AutoJalousie" | "Jalousie" | "RoofWindowShading"),
        "CentralGate" => matches!(block_type, "Gate" | "Doorcontroller" | "WindowMonitor"),
        _ => matches!(
            block_type,
            "LightController2"
                | "LightControllerV2"
                | "LightController"
                | "AutoJalousie"
                | "Jalousie"
                | "MusicPlayer"
                | "AudioZone"
                | "PresenceLight"
                | "StairwellLight"
                | "Switch"
                | "Pushbutton"
                | "PushButton"
                | "Gate"
                | "CentralGate"
                | "HeatIRoomController2"
                | "ClimateController"
                | "SaunaController"
                | "PoolController"
                | "CeilingFan"
                | "Dehumidifier"
                | "Wallbox2"
        ),
    }
}

fn classify_periphery_leaf_capabilities(control_type: &str) -> (bool, bool) {
    match control_type {
        "TreeSensor" | "LoxAIRsensor" | "Sensor" | "DigitalInput" | "AnalogInput"
        | "VirtualIn" | "VirtualInCmd" => (true, false),
        "TreeActor" | "LoxAIRactor" | "Actor" | "DigitalOutput" | "AnalogOutput"
        | "VirtualOut" | "VirtualOutCmd" => (false, true),
        "Merker" | "Memory" => (true, true),
        _ => (false, false),
    }
}

fn classify_periphery_leaf_for_block(
    target_block_type: &str,
    parent_dev: Option<&TreeNode>,
    leaf: &TreeNode,
) -> (bool, bool) {
    let sub_type = get_attr(&leaf.raw_attributes, "SubType").unwrap_or("");
    let enocean_sub_type = get_attr(&leaf.raw_attributes, "EnoceanSubType").unwrap_or("");
    let parent_tid = parent_dev
        .and_then(|d| get_attr(&d.raw_attributes, "TId"))
        .unwrap_or("");

    match target_block_type {
        "Alarm" => {
            let can_in = matches!(
                leaf.control_type.as_str(),
                "TreeSensor" | "LoxAIRsensor" | "Sensor" | "DigitalInput"
            ) && parent_tid != "NoisemakerTree"
                && parent_tid != "FirealarmAir"
                && matches!(enocean_sub_type, "4" | "67");
            let can_out = matches!(
                leaf.control_type.as_str(),
                "TreeActor" | "LoxAIRactor" | "Actor" | "DigitalOutput"
            ) && matches!(sub_type, "163" | "168" | "169");
            (can_in, can_out)
        }
        "SmokeAlarm" | "FireEmergency" => {
            let can_in = matches!(
                leaf.control_type.as_str(),
                "TreeSensor" | "LoxAIRsensor" | "Sensor" | "DigitalInput"
            ) && (parent_tid == "FirealarmAir" || leaf.title.to_lowercase().contains("dym"));
            let can_out = matches!(
                leaf.control_type.as_str(),
                "TreeActor" | "LoxAIRactor" | "Actor" | "DigitalOutput"
            ) && matches!(sub_type, "163" | "168" | "169");
            (can_in, can_out)
        }
        "WaterCentral" => {
            let can_in = matches!(
                leaf.control_type.as_str(),
                "TreeSensor" | "LoxAIRsensor" | "Sensor" | "DigitalInput"
            ) && (parent_tid == "WaterSensor" || leaf.title.to_lowercase().contains("wod"));
            let can_out = matches!(
                leaf.control_type.as_str(),
                "TreeActor" | "LoxAIRactor" | "Actor" | "DigitalOutput"
            ) && matches!(sub_type, "163" | "168" | "169");
            (can_in, can_out)
        }
        "CentralLightController" | "CentralJalousie" | "CentralGate" => (false, false),
        _ => classify_periphery_leaf_capabilities(&leaf.control_type),
    }
}

fn collect_periphery_leaves<'a>(
    nodes: &'a [TreeNode],
    parent_device: Option<&'a TreeNode>,
    out: &mut Vec<(Option<&'a TreeNode>, &'a TreeNode)>,
) {
    for node in nodes {
        let is_device_container = matches!(
            node.control_type.as_str(),
            "TreeDevice"
                | "LoxAIRDevice"
                | "LoxAIR"
                | "LoxTREE"
                | "TreeExtension"
                | "Extension"
                | "RelayExtension"
                | "DimmerExtension"
                | "Comm485"
                | "LoxServer"
        );
        let next_parent = if is_device_container {
            Some(node)
        } else {
            parent_device
        };
        let (can_in, can_out) = classify_periphery_leaf_capabilities(&node.control_type);
        if can_in || can_out || node.control_type == "Online" {
            out.push((parent_device, node));
        }
        if !node.children.is_empty() {
            collect_periphery_leaves(&node.children, next_parent, out);
        }
    }
}

/// Builds the full list of [`ObjectConfigRow`]s for a given configurable block (`block_uuid`).
///
/// Includes:
/// - All currently linked inputs (`Wejście`) and outputs (`Wyjście`) from the block's
///   `Objects` attribute and hidden `InputRef` / `OutputRef` blocks,
/// - All candidate periphery sensors/actors and controllable schematic function blocks in `project`.
pub fn build_object_config_rows(
    project: &LoxoneProject,
    _page_uuid: &str,
    block_uuid: &str,
) -> Vec<ObjectConfigRow> {
    let Some(target_block) = project
        .pages
        .iter()
        .flat_map(|p| p.blocks.iter())
        .find(|b| b.uuid == block_uuid)
    else {
        return Vec::new();
    };

    let objects_attr = get_attr(&target_block.raw_attributes, "Objects")
        .or_else(|| get_attr(&target_block.raw_attributes, "UO"))
        .unwrap_or("");
    let object_uuids: Vec<String> = objects_attr
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    let object_uuid_set: HashSet<&str> = object_uuids.iter().map(String::as_str).collect();

    // Also collect any pin UUIDs wired into this block's hidden input pins (HI1..HI4, InputAir*)
    let mut hi_source_pin_uuids: HashSet<&str> = HashSet::new();
    for pin in &target_block.pins {
        if pin.key.starts_with("HI") || pin.key.starts_with("InputAir") {
            for link in &pin.incoming_links {
                hi_source_pin_uuids.insert(link.source_pin_uuid.as_str());
            }
        }
    }
    let target_output_pin_uuids: HashSet<&str> = target_block
        .pins
        .iter()
        .map(|p| p.uuid.as_str())
        .collect();

    // Map periphery node UUID -> (vec of (page_uuid, ref_block_uuid), is_input, is_output)
    let mut linked_periphery_map: HashMap<String, (Vec<(String, String)>, bool, bool)> =
        HashMap::new();
    for page in &project.pages {
        for blk in &page.blocks {
            let in_objects = object_uuid_set.contains(blk.uuid.as_str());
            let wired_to_hi = blk
                .pins
                .iter()
                .any(|p| hi_source_pin_uuids.contains(p.uuid.as_str()));
            let wired_from_target = object_uuid_set.is_empty()
                && !blk.has_position
                && blk.pins.iter().any(|p| {
                    p.incoming_links
                        .iter()
                        .any(|l| target_output_pin_uuids.contains(l.source_pin_uuid.as_str()))
                });

            if (in_objects || wired_to_hi || wired_from_target) && blk.is_ref_bar() {
                if let Some(ref_u) = &blk.ref_uuid {
                    let is_in = blk.is_input_ref_bar() || wired_to_hi;
                    let is_out = blk.is_output_ref_bar() || wired_from_target;
                    let entry = linked_periphery_map
                        .entry(ref_u.clone())
                        .or_insert_with(|| (Vec::new(), false, false));
                    let pair = (page.uuid.clone(), blk.uuid.clone());
                    if !entry.0.contains(&pair) {
                        entry.0.push(pair);
                    }
                    entry.1 |= is_in;
                    entry.2 |= is_out;
                }
            }
        }
    }

    let resolve_room = |room_uuid: Option<&str>| -> (String, String) {
        if let Some(ru) = room_uuid {
            if let Some(place) = project.places.get(ru) {
                return (
                    place.title.clone(),
                    format_room_type(place.p_type.as_deref()),
                );
            }
        }
        (String::new(), String::new())
    };

    let resolve_category = |cat_uuid: Option<&str>| -> String {
        if let Some(cu) = cat_uuid {
            if let Some(cat) = project.categories.get(cu) {
                return cat.title.clone();
            }
        }
        String::new()
    };

    let mut rows = Vec::new();
    let mut seen_target_uuids = HashSet::new();

    // 1. Collect periphery & system leaves
    let mut leaves = Vec::new();
    collect_periphery_leaves(&project.periphery_tree, None, &mut leaves);
    collect_periphery_leaves(&project.system_tree, None, &mut leaves);

    for (parent_dev, leaf) in leaves {
        let (mut can_in, mut can_out) =
            classify_periphery_leaf_for_block(&target_block.block_type, parent_dev, leaf);
        let (existing_ref_blocks, is_input, is_output) =
            if let Some((ref_blks, is_in, is_out)) = linked_periphery_map.get(&leaf.uuid) {
                can_in |= *is_in;
                can_out |= *is_out;
                (ref_blks.clone(), *is_in, *is_out)
            } else {
                (Vec::new(), false, false)
            };
        if !can_in && !can_out && !is_input && !is_output {
            continue;
        }
        if !seen_target_uuids.insert(leaf.uuid.clone()) {
            continue;
        }

        let room_uuid = leaf
            .room_uuid
            .as_deref()
            .or_else(|| parent_dev.and_then(|d| d.room_uuid.as_deref()));
        let cat_uuid = if !is_input
            && !is_output
            && get_attr(&leaf.raw_attributes, "SubType") == Some("163")
        {
            parent_dev.and_then(|d| d.category_uuid.as_deref())
        } else {
            leaf.category_uuid
                .as_deref()
                .or_else(|| parent_dev.and_then(|d| d.category_uuid.as_deref()))
        };
        let (room_name, room_type) = resolve_room(room_uuid);
        let category_name = resolve_category(cat_uuid);

        let device_name = parent_dev.map(|d| d.title.clone()).unwrap_or_default();

        let display_name = get_attr(&leaf.raw_attributes, "DisplayTitle")
            .or_else(|| get_attr(&leaf.raw_attributes, "Desc"))
            .unwrap_or("")
            .to_string();

        let object_type = resolve_periphery_object_type(parent_dev, leaf);

        rows.push(ObjectConfigRow {
            target_uuid: leaf.uuid.clone(),
            target_kind: ObjectConfigTargetKind::PeripheryNode {
                node_uuid: leaf.uuid.clone(),
            },
            existing_ref_blocks,
            can_be_input: can_in,
            can_be_output: can_out,
            is_input,
            is_output,
            name: leaf.title.clone(),
            display_name,
            device_name,
            room_name,
            room_type,
            category_name,
            object_type,
        });
    }

    // 2. Collect controllable schematic function blocks across all pages
    for page in &project.pages {
        for blk in &page.blocks {
            if blk.uuid == block_uuid || blk.is_ref_bar() || blk.is_note() || !blk.has_position {
                continue;
            }
            let is_selected = object_uuid_set.contains(blk.uuid.as_str());
            if !is_selected
                && !is_candidate_function_block_for_target(
                    &target_block.block_type,
                    &blk.block_type,
                )
            {
                continue;
            }
            if !seen_target_uuids.insert(blk.uuid.clone()) {
                continue;
            }

            let room_uuid = blk.room_uuid.as_deref().or(page.room_uuid.as_deref());
            let cat_uuid = blk
                .category_uuid
                .as_deref()
                .or(page.category_uuid.as_deref());
            let (room_name, room_type) = resolve_room(room_uuid);
            let category_name = resolve_category(cat_uuid);
            let display_name = get_attr(&blk.raw_attributes, "DisplayTitle")
                .or_else(|| get_attr(&blk.raw_attributes, "Desc"))
                .unwrap_or("")
                .to_string();
            let object_type = resolve_function_block_object_type(&blk.block_type);

            rows.push(ObjectConfigRow {
                target_uuid: blk.uuid.clone(),
                target_kind: ObjectConfigTargetKind::PageFunctionBlock {
                    block_uuid: blk.uuid.clone(),
                    page_uuid: page.uuid.clone(),
                },
                existing_ref_blocks: Vec::new(),
                can_be_input: false,
                can_be_output: true,
                is_input: false,
                is_output: is_selected,
                name: blk.title.clone(),
                display_name,
                device_name: String::new(),
                room_name,
                room_type,
                category_name,
                object_type,
            });
        }
    }

    // Sort matching Loxone Config's default ordering:
    // 1. Selected Inputs first (0), Selected Outputs next (1), Unselected rows last (2)
    // 2. Within each group: Name descending, then Device descending, then Room descending
    rows.sort_by(|a, b| {
        let rank = |r: &ObjectConfigRow| -> u8 {
            if r.is_input {
                0
            } else if r.is_output {
                1
            } else {
                2
            }
        };
        rank(a)
            .cmp(&rank(b))
            .then_with(|| b.name.to_lowercase().cmp(&a.name.to_lowercase()))
            .then_with(|| b.device_name.to_lowercase().cmp(&a.device_name.to_lowercase()))
            .then_with(|| b.room_name.to_lowercase().cmp(&a.room_name.to_lowercase()))
    });

    rows
}

/// Computes the undoable [`ProjectEdit`] sequence required to apply changes made in the
/// "Konfiguracja obiektu" ("Object Configuration") dialog back to `project`.
pub fn build_object_config_edits(
    project: &LoxoneProject,
    page_uuid: &str,
    block_uuid: &str,
    updated_rows: &[ObjectConfigRow],
) -> Vec<ProjectEdit> {
    let Some(target_block) = project
        .pages
        .iter()
        .flat_map(|p| p.blocks.iter())
        .find(|b| b.uuid == block_uuid)
    else {
        return Vec::new();
    };

    let attr_key = if get_attr(&target_block.raw_attributes, "Objects").is_some()
        || get_attr(&target_block.raw_attributes, "UO").is_none()
    {
        "Objects"
    } else {
        "UO"
    };

    let mut edits = Vec::new();
    let mut new_object_uuids = Vec::new();

    // Determine primary output pin UUID on target_block for newly added hidden OutputRef blocks
    let target_out_pin_uuid = target_block
        .pins
        .iter()
        .find(|p| matches!(p.key.as_str(), "Q1" | "OutAlarm1" | "Q"))
        .map(|p| p.uuid.clone());

    for row in updated_rows {
        let is_selected = row.is_input || row.is_output;
        match &row.target_kind {
            ObjectConfigTargetKind::PageFunctionBlock {
                block_uuid: fn_block_uuid,
                ..
            } => {
                if is_selected {
                    new_object_uuids.push(fn_block_uuid.clone());
                }
            }
            ObjectConfigTargetKind::PeripheryNode { node_uuid } => {
                if is_selected {
                    if !row.existing_ref_blocks.is_empty() {
                        for (_, existing_ref_uuid) in &row.existing_ref_blocks {
                            new_object_uuids.push(existing_ref_uuid.clone());
                        }
                    } else {
                        // Create a new hidden InputRef / OutputRef on the block's page
                        let new_ref_uuid = generate_loxone_uuid();
                        let ref_type = if row.is_input {
                            "InputRef"
                        } else {
                            "OutputRef"
                        };
                        let wf = if row.is_input { "6144" } else { "4096" };
                        let mut pins = Vec::new();

                        if row.is_input {
                            let q_pin_uuid = generate_loxone_uuid();
                            pins.push(ConnectorPin {
                                uuid: q_pin_uuid.clone(),
                                key: "Q".to_string(),
                                default_value: None,
                                pin_kind: PinKind::Output,
                                incoming_links: Vec::new(),
                            });
                            let hidden_block = PageBlock {
                                uuid: new_ref_uuid.clone(),
                                block_type: ref_type.to_string(),
                                title: row.name.clone(),
                                has_position: false,
                                px: 0.0,
                                py: 0.0,
                                px2: 0.0,
                                py2: 0.0,
                                color_rgb: Some([0, 0, 0]),
                                ref_uuid: Some(node_uuid.clone()),
                                extra_ref_uuids: Vec::new(),
                                room_uuid: None,
                                category_uuid: None,
                                text_content: None,
                                pins,
                                raw_attributes: vec![
                                    ("Type".to_string(), ref_type.to_string()),
                                    ("U".to_string(), new_ref_uuid.clone()),
                                    ("Title".to_string(), row.name.clone()),
                                    ("Cl".to_string(), "0,0,0".to_string()),
                                    ("Ref".to_string(), node_uuid.clone()),
                                    ("WF".to_string(), wf.to_string()),
                                ],
                                extra_children: Vec::new(),
                            };
                            edits.push(ProjectEdit::AddPageBlock {
                                page_uuid: page_uuid.to_string(),
                                block: hidden_block,
                            });
                            let hi_pin_key = match target_block.block_type.as_str() {
                                "SmokeAlarm" => "InputAirDigitalS",
                                _ => {
                                    if row.object_type.contains("obecności")
                                        || row.name.to_lowercase().contains("ruch")
                                    {
                                        "HI1"
                                    } else {
                                        "HI4"
                                    }
                                }
                            };
                            edits.push(ProjectEdit::AddWireLink {
                                target_block_uuid: block_uuid.to_string(),
                                target_pin_key: hi_pin_key.to_string(),
                                source_pin_uuid: q_pin_uuid,
                            });
                        } else {
                            let ai_pin_uuid = generate_loxone_uuid();
                            let incoming_links = if let Some(src_pin) = &target_out_pin_uuid {
                                vec![WireLink {
                                    source_pin_uuid: src_pin.clone(),
                                    flags: Some("1".to_string()),
                                }]
                            } else {
                                Vec::new()
                            };
                            pins.push(ConnectorPin {
                                uuid: ai_pin_uuid,
                                key: "AI".to_string(),
                                default_value: None,
                                pin_kind: PinKind::Input,
                                incoming_links,
                            });
                            let hidden_block = PageBlock {
                                uuid: new_ref_uuid.clone(),
                                block_type: ref_type.to_string(),
                                title: row.name.clone(),
                                has_position: false,
                                px: 0.0,
                                py: 0.0,
                                px2: 0.0,
                                py2: 0.0,
                                color_rgb: Some([0, 0, 0]),
                                ref_uuid: Some(node_uuid.clone()),
                                extra_ref_uuids: Vec::new(),
                                room_uuid: None,
                                category_uuid: None,
                                text_content: None,
                                pins,
                                raw_attributes: vec![
                                    ("Type".to_string(), ref_type.to_string()),
                                    ("U".to_string(), new_ref_uuid.clone()),
                                    ("Title".to_string(), row.name.clone()),
                                    ("Cl".to_string(), "0,0,0".to_string()),
                                    ("Ref".to_string(), node_uuid.clone()),
                                    ("WF".to_string(), wf.to_string()),
                                ],
                                extra_children: Vec::new(),
                            };
                            edits.push(ProjectEdit::AddPageBlock {
                                page_uuid: page_uuid.to_string(),
                                block: hidden_block,
                            });
                        }
                        new_object_uuids.push(new_ref_uuid);
                    }
                } else {
                    // User unchecked a previously linked periphery sensor/actor
                    for (ref_page_uuid, existing_ref_uuid) in &row.existing_ref_blocks {
                        edits.push(ProjectEdit::RemovePageBlock {
                            page_uuid: ref_page_uuid.clone(),
                            block_uuid: existing_ref_uuid.clone(),
                        });
                    }
                }
            }
        }
    }

    let new_objects_val = if new_object_uuids.is_empty() {
        None
    } else {
        Some(new_object_uuids.join(","))
    };
    let old_objects_val = get_attr(&target_block.raw_attributes, attr_key).map(str::to_string);
    if new_objects_val != old_objects_val {
        edits.push(ProjectEdit::SetControlAttr {
            control_uuid: block_uuid.to_string(),
            attr_name: attr_key.to_string(),
            value: new_objects_val,
        });
    }

    edits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::load_loxone_file;
    use crate::writer::{apply_edit_with_inverse, ProjectEdit};
    use std::path::PathBuf;

    #[test]
    fn test_burglar_alarm_object_config_rows_match_klimczaki_loxone() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Klimczaki.Loxone");
        let proj = load_loxone_file(&path).expect("parse Klimczaki.Loxone");

        let alarm_page_uuid = "19eb19ad-02ad-eae0-ffff51535bcccc04";
        let alarm_block_uuid = "19eb19a9-024d-3a10-ffff51535bcccc04";

        let rows = build_object_config_rows(&proj, alarm_page_uuid, alarm_block_uuid);
        assert!(!rows.is_empty());

        let selected_inputs: Vec<&ObjectConfigRow> =
            rows.iter().filter(|r| r.is_input).collect();
        let selected_outputs: Vec<&ObjectConfigRow> =
            rows.iter().filter(|r| r.is_output).collect();

        // System alarmowy has 11 selected inputs (10 Ruch sensors + 1 Kanał wolny Air Base Extension)
        assert_eq!(selected_inputs.len(), 11);
        assert!(selected_inputs
            .iter()
            .any(|r| r.name == "Ruch" && r.room_name == "Wiatrołap" && r.room_type == "Komunikacyjne" && r.object_type == "Czujnik obecności Tree"));
        assert!(selected_inputs
            .iter()
            .any(|r| r.name == "Ruch" && r.room_name == "Gabinet" && r.device_name == "Czujka obecności" && r.object_type == "Czujnik obecności Tree"));
        assert!(selected_inputs
            .iter()
            .any(|r| r.name == "Kanał wolny Air Base Extension" && r.room_name == "Funkcje centralne" && r.category_name == "Bezpieczeństwo"));

        // And 24 unique output targets (10 Sterownik oświetlenia, 7 Automatyczna roleta, 1 Odtwarzacz audio, plus Touch/Nightlight acoustic/optical outputs)
        assert!(selected_outputs
            .iter()
            .any(|r| r.name == "Wiatrołap" && r.object_type == "Sterownik oświetlenia" && r.category_name == "Oświetlenie"));
        assert!(selected_outputs
            .iter()
            .any(|r| r.name == "Salon + jadalnia" && r.object_type == "Sterownik oświetlenia"));
    }

    #[test]
    fn test_burglar_alarm_object_config_toggle_and_undo_roundtrip() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Klimczaki.Loxone");
        let mut proj = load_loxone_file(&path).expect("parse Klimczaki.Loxone");

        let alarm_page_uuid = "19eb19ad-02ad-eae0-ffff51535bcccc04";
        let alarm_block_uuid = "19eb19a9-024d-3a10-ffff51535bcccc04";

        let mut rows = build_object_config_rows(&proj, alarm_page_uuid, alarm_block_uuid);
        let initial_input_count = rows.iter().filter(|r| r.is_input).count();
        let initial_output_count = rows.iter().filter(|r| r.is_output).count();

        // Uncheck one output (Wiatrołap Sterownik oświetlenia) and one input (Wiatrołap Ruch)
        for r in &mut rows {
            if r.is_output && r.name == "Wiatrołap" && r.object_type == "Sterownik oświetlenia" {
                r.is_output = false;
            }
            if r.is_input && r.name == "Ruch" && r.room_name == "Wiatrołap" {
                r.is_input = false;
            }
        }

        let edits = build_object_config_edits(&proj, alarm_page_uuid, alarm_block_uuid, &rows);
        assert!(!edits.is_empty());

        let inverse = apply_edit_with_inverse(&mut proj, &ProjectEdit::Batch(edits))
            .expect("apply object config batch");

        let rows_after = build_object_config_rows(&proj, alarm_page_uuid, alarm_block_uuid);
        assert_eq!(
            rows_after.iter().filter(|r| r.is_input).count(),
            initial_input_count - 1
        );
        assert_eq!(
            rows_after.iter().filter(|r| r.is_output).count(),
            initial_output_count - 1
        );

        // Undo restores the exact original input and output selections
        apply_edit_with_inverse(&mut proj, &inverse).expect("undo object config batch");
        let rows_restored = build_object_config_rows(&proj, alarm_page_uuid, alarm_block_uuid);
        assert_eq!(
            rows_restored.iter().filter(|r| r.is_input).count(),
            initial_input_count
        );
        assert_eq!(
            rows_restored.iter().filter(|r| r.is_output).count(),
            initial_output_count
        );
    }

    #[test]
    fn test_burglar_alarm_unselected_rows_match_bug01_exact_order() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Klimczaki.Loxone");
        let proj = load_loxone_file(&path).expect("parse Klimczaki.Loxone");

        let alarm_page_uuid = "19eb19ad-02ad-eae0-ffff51535bcccc04";
        let alarm_block_uuid = "19eb19a9-024d-3a10-ffff51535bcccc04";

        let rows = build_object_config_rows(&proj, alarm_page_uuid, alarm_block_uuid);
        let unselected: Vec<&ObjectConfigRow> = rows
            .iter()
            .filter(|r| !r.is_input && !r.is_output)
            .collect();

        // Must have the exact 13 unselected rows from bug01.png in exact order
        let actual: Vec<(&str, &str, &str, &str, &str, &str, &str)> = unselected
            .iter()
            .map(|r| {
                (
                    r.name.as_str(),
                    r.display_name.as_str(),
                    r.device_name.as_str(),
                    r.room_name.as_str(),
                    r.room_type.as_str(),
                    r.category_name.as_str(),
                    r.object_type.as_str(),
                )
            })
            .collect();

        let expected = vec![
            ("Taras", "", "", "Taras", "Pobytowe", "Oświetlenie", "Sterownik oświetlenia"),
            ("Strych", "", "", "Strych", "Pobytowe", "Oświetlenie", "Sterownik oświetlenia"),
            ("Roleta mala", "", "", "Gabinet", "Pobytowe", "Zacienianie", "Automatyczne zacienianie"),
            ("Roleta duża", "", "", "Gabinet", "Pobytowe", "Zacienianie", "Automatyczne zacienianie"),
            ("Oświetlenie przed domem", "Na zewnątrz", "", "Na zewnątrz", "Inne", "Oświetlenie", "Sterownik oświetlenia"),
            ("lewa strona", "", "", "Salon + jadalnia", "Pobytowe", "Zacienianie", "Automatyczne zacienianie"),
            ("Akustyczny sygnał kliknięcia", "", "Touch Tree", "Przedpokój_schody", "Komunikacyjne", "", "Touch Tree"),
            ("Akustyczny sygnał kliknięcia", "", "Touch Surface Tree", "Łazienka (piętro)", "Pobytowe", "", "Touch Surface Tree"),
            ("Akustyczny sygnał kliknięcia", "", "Touch Pure Tree", "Wiatrołap", "Komunikacyjne", "", "Touch Pure Tree"),
            ("Akustyczny sygnał kliknięcia", "", "Touch Pure Tree", "Pokój Agatki", "Sypialniane", "", "Touch Pure Tree"),
            ("Akustyczny sygnał kliknięcia", "", "sypialnia: Drzwi", "Sypialnia", "Sypialniane", "", "Touch Pure Tree"),
            ("Akustyczny sygnał kliknięcia", "", "Schody", "Salon + jadalnia", "Pobytowe", "", "Touch Pure Flex Tree"),
            ("Akustyczny sygnał kliknięcia", "", "Drzwi", "Gabinet", "Pobytowe", "", "Touch Tree"),
        ];

        assert_eq!(actual, expected);
    }
}

