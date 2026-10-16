use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::block_schema::lookup_block_schema;
use crate::model::{
    Category, ConnectorPin, Page, PageBlock, PinKind, Place, TreeNode, TreeSectionKind,
};

static UUID_COUNTER: AtomicU64 = AtomicU64::new(1);

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// Generates a Loxone-compatible UUID in `8-4-4-16` lowercase hex format
/// (`xxxxxxxx-xxxx-xxxx-xxxxxxxxxxxxxxxx`).
pub fn generate_loxone_uuid() -> String {
    let seq = UUID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x0196_36db_0000_0000);
    let pid = std::process::id() as u64;
    let hi = splitmix64(nanos ^ seq.rotate_left(17) ^ (pid << 32));
    let lo = splitmix64(hi ^ seq.wrapping_mul(0x517c_c1b7_2722_0a95) ^ nanos.rotate_right(13));

    let p0 = (hi >> 32) as u32;
    let p1 = ((hi >> 16) & 0xffff) as u16;
    let p2 = (hi & 0xffff) as u16;
    let p3 = lo;

    format!("{p0:08x}-{p1:04x}-{p2:04x}-{p3:016x}")
}

/// Returns a standard Loxone RGB color (`Cl`) for a given KB category group.
pub fn default_category_color(category: &str) -> [u8; 3] {
    match category {
        "Security & Safety" | "Access" => [255, 0, 84],
        "Audio" => [164, 112, 255],
        "Climate" => [105, 195, 80],
        "Periphery" | "Periphery References" | "Virtual I/O" => [238, 238, 238],
        "Documentation" => [94, 129, 255],
        _ => [141, 255, 112],
    }
}

/// Ensures every pin defined in `lookup_block_schema(&block.block_type)` (`pin_docs` + `pin_params`)
/// exists in `block.pins` so any block (including blocks loaded from XML that omitted unconnected
/// `<Co>` elements) exposes all connectible input and output pins.
pub fn ensure_schema_pins_on_block(block: &mut PageBlock) {
    if let Some(schema) = lookup_block_schema(&block.block_type) {
        for doc in schema.pin_docs {
            let default_val = schema
                .find_pin_param(doc.pin_key)
                .and_then(|pp| (!pp.default_value.is_empty()).then(|| pp.default_value.to_string()));
            let desired_kind = if doc.is_output {
                PinKind::Output
            } else {
                PinKind::Input
            };

            if let Some(existing) = block.pins.iter_mut().find(|p| p.key == doc.pin_key) {
                if doc.is_output {
                    existing.pin_kind = PinKind::Output;
                } else if existing.pin_kind == PinKind::Parameter {
                    existing.pin_kind = PinKind::Input;
                }
                if existing.default_value.is_none() && existing.incoming_links.is_empty() {
                    if let Some(def) = default_val {
                        existing.default_value = Some(def);
                    }
                }
            } else {
                block.pins.push(ConnectorPin {
                    uuid: generate_loxone_uuid(),
                    key: doc.pin_key.to_string(),
                    default_value: default_val,
                    pin_kind: desired_kind,
                    incoming_links: Vec::new(),
                });
            }
        }

        for param in schema.pin_params {
            let default_val =
                (!param.default_value.is_empty()).then(|| param.default_value.to_string());
            if let Some(existing) = block.pins.iter_mut().find(|p| p.key == param.pin_key) {
                if existing.default_value.is_none() && existing.incoming_links.is_empty() {
                    existing.default_value = default_val;
                }
            } else {
                block.pins.push(ConnectorPin {
                    uuid: generate_loxone_uuid(),
                    key: param.pin_key.to_string(),
                    default_value: default_val,
                    pin_kind: PinKind::Parameter,
                    incoming_links: Vec::new(),
                });
            }
        }
    }

    if block.is_input_ref_bar() && !block.pins.iter().any(|p| p.pin_kind == PinKind::Output) {
        block.pins.push(ConnectorPin {
            uuid: generate_loxone_uuid(),
            key: "Q".to_string(),
            default_value: None,
            pin_kind: PinKind::Output,
            incoming_links: Vec::new(),
        });
    } else if block.is_output_ref_bar() && !block.pins.iter().any(|p| p.pin_kind == PinKind::Input)
    {
        block.pins.push(ConnectorPin {
            uuid: generate_loxone_uuid(),
            key: "I".to_string(),
            default_value: None,
            pin_kind: PinKind::Input,
            incoming_links: Vec::new(),
        });
    }
}

/// Creates a new `PageBlock` at `(px, py)` initialized with schema defaults and pins.
pub fn new_page_block(type_name: &str, title: &str, px: f32, py: f32) -> PageBlock {
    let uuid = generate_loxone_uuid();
    let schema = lookup_block_schema(type_name);
    let effective_title = if title.is_empty() {
        schema
            .map(|s| s.display_name_en)
            .unwrap_or(type_name)
            .to_string()
    } else {
        title.to_string()
    };
    let category = schema.map(|s| s.category_group).unwrap_or("Logic");
    let color = default_category_color(category);
    let px2 = px + 2112.0;
    let py2 = py + 768.0;

    let mut raw_attributes = vec![
        ("Type".to_string(), type_name.to_string()),
        ("Title".to_string(), effective_title.clone()),
        ("U".to_string(), uuid.clone()),
        ("Px".to_string(), format!("{}", px.round() as i32)),
        ("Py".to_string(), format!("{}", py.round() as i32)),
        ("Px2".to_string(), format!("{}", px2.round() as i32)),
        ("Py2".to_string(), format!("{}", py2.round() as i32)),
        (
            "Cl".to_string(),
            format!("{},{},{}", color[0], color[1], color[2]),
        ),
    ];

    if let Some(s) = schema {
        for attr in s.attributes {
            if let Some(def) = attr.default_value {
                if !def.is_empty() && !raw_attributes.iter().any(|(k, _)| k == attr.xml_attr) {
                    raw_attributes.push((attr.xml_attr.to_string(), def.to_string()));
                }
            }
        }
    }

    let text_content = if type_name == "Text" {
        Some(effective_title.clone())
    } else {
        None
    };

    let mut block = PageBlock {
        uuid,
        block_type: type_name.to_string(),
        title: effective_title,
        has_position: true,
        px,
        py,
        px2,
        py2,
        color_rgb: Some(color),
        ref_uuid: None,
        extra_ref_uuids: Vec::new(),
        room_uuid: None,
        category_uuid: None,
        text_content,
        pins: Vec::new(),
        raw_attributes,
        extra_children: Vec::new(),
    };

    ensure_schema_pins_on_block(&mut block);
    block
}

/// Constructs an `InputRef`, `OutputRef`, `OutputRefLM`, or `Memory` `PageBlock` referencing `target_node`.
pub fn new_reference_block(
    ref_type: &str,
    target_node: &TreeNode,
    px: f32,
    py: f32,
) -> PageBlock {
    let uuid = generate_loxone_uuid();
    let title = if target_node.title.is_empty() {
        target_node.control_type.clone()
    } else {
        target_node.title.clone()
    };
    let desc = target_node
        .raw_attributes
        .iter()
        .find(|(k, _)| k == "Desc")
        .map(|(_, v)| v.clone());

    let px2 = px + 2112.0;
    let py2 = py + 256.0;
    let color = [238, 238, 238];

    let mut raw_attributes = vec![
        ("Type".to_string(), ref_type.to_string()),
        ("Title".to_string(), title.clone()),
        ("U".to_string(), uuid.clone()),
        ("Px".to_string(), format!("{}", px.round() as i32)),
        ("Py".to_string(), format!("{}", py.round() as i32)),
        ("Px2".to_string(), format!("{}", px2.round() as i32)),
        ("Py2".to_string(), format!("{}", py2.round() as i32)),
        ("Cl".to_string(), "238,238,238".to_string()),
        ("Ref".to_string(), target_node.uuid.clone()),
    ];
    if let Some(d) = desc {
        if !d.is_empty() {
            raw_attributes.push(("Desc".to_string(), d));
        }
    }

    let mut pins = Vec::new();
    for tp in &target_node.pins {
        let pin_kind = match ref_type {
            "InputRef" => PinKind::Output,
            "OutputRef" | "OutputRefLM" => PinKind::Input,
            _ => tp.pin_kind,
        };
        pins.push(ConnectorPin {
            uuid: if tp.uuid.is_empty() {
                generate_loxone_uuid()
            } else {
                tp.uuid.clone()
            },
            key: tp.key.clone(),
            default_value: None,
            pin_kind,
            incoming_links: Vec::new(),
        });
    }

    match ref_type {
        "InputRef" => {
            if !pins.iter().any(|p| p.pin_kind == PinKind::Output) {
                pins.push(ConnectorPin {
                    uuid: generate_loxone_uuid(),
                    key: "O".to_string(),
                    default_value: None,
                    pin_kind: PinKind::Output,
                    incoming_links: Vec::new(),
                });
            }
            if !pins.iter().any(|p| p.key == "Q") {
                pins.push(ConnectorPin {
                    uuid: generate_loxone_uuid(),
                    key: "Q".to_string(),
                    default_value: None,
                    pin_kind: PinKind::Output,
                    incoming_links: Vec::new(),
                });
            }
        }
        "OutputRef" | "OutputRefLM" => {
            if !pins.iter().any(|p| p.pin_kind == PinKind::Input) {
                pins.push(ConnectorPin {
                    uuid: generate_loxone_uuid(),
                    key: "I".to_string(),
                    default_value: None,
                    pin_kind: PinKind::Input,
                    incoming_links: Vec::new(),
                });
            }
        }
        "Memory" => {
            if !pins.iter().any(|p| p.pin_kind == PinKind::Input) {
                pins.push(ConnectorPin {
                    uuid: generate_loxone_uuid(),
                    key: "I".to_string(),
                    default_value: None,
                    pin_kind: PinKind::Input,
                    incoming_links: Vec::new(),
                });
            }
            if !pins.iter().any(|p| p.pin_kind == PinKind::Output) {
                pins.push(ConnectorPin {
                    uuid: generate_loxone_uuid(),
                    key: "Q".to_string(),
                    default_value: None,
                    pin_kind: PinKind::Output,
                    incoming_links: Vec::new(),
                });
            }
        }
        _ => {}
    }

    PageBlock {
        uuid,
        block_type: ref_type.to_string(),
        title,
        has_position: true,
        px,
        py,
        px2,
        py2,
        color_rgb: Some(color),
        ref_uuid: Some(target_node.uuid.clone()),
        extra_ref_uuids: Vec::new(),
        room_uuid: target_node.room_uuid.clone(),
        category_uuid: target_node.category_uuid.clone(),
        text_content: None,
        pins,
        raw_attributes,
        extra_children: Vec::new(),
    }
}

/// Creates a new logic `Page`.
pub fn new_page(title: &str) -> Page {
    let effective_title = if title.is_empty() {
        "New Page".to_string()
    } else {
        title.to_string()
    };
    Page {
        uuid: generate_loxone_uuid(),
        title: effective_title,
        room_uuid: None,
        category_uuid: None,
        blocks: Vec::new(),
    }
}

/// Creates a new `Place` (Room).
pub fn new_room(title: &str) -> Place {
    let effective_title = if title.is_empty() {
        "New Room".to_string()
    } else {
        title.to_string()
    };
    Place {
        uuid: generate_loxone_uuid(),
        title: effective_title,
        sqm: None,
        p_type: Some("0".to_string()),
        icon: None,
    }
}

/// Creates a new `Category`.
pub fn new_category(title: &str, cat_type: &str) -> Category {
    let effective_title = if title.is_empty() {
        "New Category".to_string()
    } else {
        title.to_string()
    };
    let effective_type = if cat_type.is_empty() {
        "0".to_string()
    } else {
        cat_type.to_string()
    };
    Category {
        uuid: generate_loxone_uuid(),
        title: effective_title,
        cat_group: Some(effective_type),
        bk_color: Some([105, 195, 80]),
        icon: None,
    }
}

/// Creates a new periphery or system `TreeNode` populated with schema defaults and connector pins.
pub fn new_tree_node(type_name: &str, title: &str) -> TreeNode {
    let uuid = generate_loxone_uuid();
    let schema = lookup_block_schema(type_name);
    let effective_title = if title.is_empty() {
        schema
            .map(|s| s.display_name_en)
            .unwrap_or(type_name)
            .to_string()
    } else {
        title.to_string()
    };

    let section = match schema.map(|s| s.category_group) {
        Some("System" | "Times" | "Messages" | "Weather" | "Users" | "Project Organization") => {
            TreeSectionKind::System
        }
        _ if matches!(
            type_name,
            "Memory"
                | "Constant"
                | "Logger"
                | "Mailer"
                | "Caller"
                | "Tracker"
                | "WeatherServer"
        ) =>
        {
            TreeSectionKind::System
        }
        _ => TreeSectionKind::Periphery,
    };

    let mut raw_attributes = vec![
        ("Type".to_string(), type_name.to_string()),
        ("Title".to_string(), effective_title.clone()),
        ("U".to_string(), uuid.clone()),
    ];

    let mut pins = Vec::new();
    if let Some(s) = schema {
        for attr in s.attributes {
            if let Some(def) = attr.default_value {
                if !def.is_empty() && !raw_attributes.iter().any(|(k, _)| k == attr.xml_attr) {
                    raw_attributes.push((attr.xml_attr.to_string(), def.to_string()));
                }
            }
        }
        for doc in s.pin_docs {
            pins.push(ConnectorPin {
                uuid: generate_loxone_uuid(),
                key: doc.pin_key.to_string(),
                default_value: None,
                pin_kind: if doc.is_output {
                    PinKind::Output
                } else {
                    PinKind::Input
                },
                incoming_links: Vec::new(),
            });
        }
    }

    if pins.is_empty() {
        let is_actuator = matches!(
            type_name,
            "Actor" | "AnalogOut" | "Dimmer" | "VirtualOut" | "VirtualOutCmd" | "Logger" | "Mailer" | "Caller" | "Tracker"
        );
        pins.push(ConnectorPin {
            uuid: generate_loxone_uuid(),
            key: if is_actuator { "I" } else { "Q" }.to_string(),
            default_value: None,
            pin_kind: if is_actuator {
                PinKind::Input
            } else {
                PinKind::Output
            },
            incoming_links: Vec::new(),
        });
    }

    TreeNode {
        uuid,
        control_type: type_name.to_string(),
        title: effective_title,
        section,
        room_uuid: None,
        category_uuid: None,
        pins,
        raw_attributes,
        extra_children: Vec::new(),
        children: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_schema::{lookup_block_schema, page_block_schemas};
    use crate::model::PinKind;

    #[test]
    fn test_uuid_format_matches_loxone_8_4_4_16_lowercase_hex() {
        let u1 = generate_loxone_uuid();
        let u2 = generate_loxone_uuid();
        assert_ne!(u1, u2);
        let parts: Vec<&str> = u1.split('-').collect();
        assert_eq!(
            parts.len(),
            4,
            "UUID must have 4 hyphen-separated segments: {u1}"
        );
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 16);
        assert!(u1.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));
        assert_eq!(u1, u1.to_ascii_lowercase());
    }

    #[test]
    fn test_expanded_page_block_catalog_has_over_75_types() {
        let count = page_block_schemas().count();
        assert_eq!(
            count, 83,
            "Expected 83 page block schemas in catalog, got {count}"
        );
        assert!(
            count >= 75,
            "Expected at least 75 page block schemas in catalog, got {count}"
        );
        for required in [
            "Switch",
            "Pushbutton",
            "StairwellLight",
            "OffDelay",
            "DoubleClick",
            "ThresholdSwitch",
            "Hysteresis",
            "Limiter",
            "Average",
            "MinMax",
            "ChangeRate",
            "ValueSelector",
            "Scale",
            "Linearity",
            "CentralJalousie",
            "CentralGate",
            "RoofWindowShading",
            "WindowMonitor",
            "PID",
            "HeatingCurve",
            "PWM",
            "TwoPointController",
            "ClimateController",
            "CeilingFan",
            "Dehumidifier",
            "MoodSwitch",
            "LightScene",
            "PresenceLight",
            "StereoExtension",
            "IntervalImpulse",
            "Stopwatch",
            "OperatingHoursCounter",
            "Schedule",
            "AlarmClock",
            "AccessCode",
            "FireEmergency",
            "WaterCentral",
            "PoolController",
            "SaunaController",
            "SolarController",
            "LoadManager",
            "Wallbox2",
            "SpotPriceOptimizer",
        ] {
            let schema = lookup_block_schema(required)
                .unwrap_or_else(|| panic!("Missing required KB block schema: {required}"));
            assert_eq!(schema.type_name, required);
            assert!(schema.kb_url.starts_with("https://www.loxone.com/enen/kb/"));
        }
    }

    #[test]
    fn test_new_page_block_populates_default_attrs_and_pins() {
        let block = new_page_block("StairwellLight", "Hallway Stair Light", 128.0, 256.0);
        assert_eq!(block.block_type, "StairwellLight");
        assert_eq!(block.title, "Hallway Stair Light");
        assert_eq!(block.px, 128.0);
        assert_eq!(block.py, 256.0);
        assert_eq!(block.px2, 2240.0);
        assert_eq!(block.py2, 1024.0);
        assert!(block
            .pins
            .iter()
            .any(|p| p.key == "Tr" && p.pin_kind == PinKind::Input));
        assert!(block
            .pins
            .iter()
            .any(|p| p.key == "Q" && p.pin_kind == PinKind::Output));
        assert!(block
            .pins
            .iter()
            .any(|p| p.key == "T" && p.default_value.is_some()));

        let tree_in = new_tree_node("DigitalIn", "Front Door Reed");
        let in_ref = new_reference_block("InputRef", &tree_in, 64.0, 64.0);
        assert_eq!(in_ref.ref_uuid.as_deref(), Some(tree_in.uuid.as_str()));
        assert_eq!(in_ref.title, "Front Door Reed");
        assert!(in_ref.pins.iter().any(|p| p.pin_kind == PinKind::Output));

        let page = new_page("Ground Floor");
        assert_eq!(page.title, "Ground Floor");
        let room = new_room("Kitchen");
        assert_eq!(room.title, "Kitchen");
        let cat = new_category("Lighting", "lights");
        assert_eq!(cat.title, "Lighting");
    }
}
