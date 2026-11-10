use std::sync::atomic::{AtomicU64, Ordering};

use crate::factory::generate_loxone_uuid;
use crate::model::{ConnectorPin, LoxoneProject, PinKind, TreeNode, TreeSectionKind};
use crate::writer::ProjectEdit;

static SERIAL_SEQ: AtomicU64 = AtomicU64::new(0x1040);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HardwareCategory {
    #[default]
    LinkExtension,
    TreeDevice,
    AirDevice,
}

impl HardwareCategory {
    pub const ALL: [HardwareCategory; 3] = [
        Self::LinkExtension,
        Self::TreeDevice,
        Self::AirDevice,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::LinkExtension => "Link Extensions & Audioserver",
            Self::TreeDevice => "Loxone Tree Devices",
            Self::AirDevice => "Loxone Air Devices",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HardwareTemplateKind {
    // Link Extensions
    #[default]
    TreeExtension,
    AirBaseExtension,
    RelayExtension,
    DimmerExtension,
    AudioServer,
    ModbusExtension,
    DaliExtension,
    DmxExtension,
    OneWireExtension,
    // Tree Devices
    TouchPureTree,
    PresenceSensorTree,
    RgbwDimmerTree,
    Nano2RelayTree,
    NfcCodeTouchTree,
    ValveActuatorTree,
    // Air Devices
    TouchPureAir,
    SmokeDetectorAir,
    WaterSensorAir,
    NanoDimmerAir,
}

impl HardwareTemplateKind {
    pub const ALL: [HardwareTemplateKind; 19] = [
        Self::TreeExtension,
        Self::AirBaseExtension,
        Self::RelayExtension,
        Self::DimmerExtension,
        Self::AudioServer,
        Self::ModbusExtension,
        Self::DaliExtension,
        Self::DmxExtension,
        Self::OneWireExtension,
        Self::TouchPureTree,
        Self::PresenceSensorTree,
        Self::RgbwDimmerTree,
        Self::Nano2RelayTree,
        Self::NfcCodeTouchTree,
        Self::ValveActuatorTree,
        Self::TouchPureAir,
        Self::SmokeDetectorAir,
        Self::WaterSensorAir,
        Self::NanoDimmerAir,
    ];

    pub fn category(self) -> HardwareCategory {
        match self {
            Self::TreeExtension
            | Self::AirBaseExtension
            | Self::RelayExtension
            | Self::DimmerExtension
            | Self::AudioServer
            | Self::ModbusExtension
            | Self::DaliExtension
            | Self::DmxExtension
            | Self::OneWireExtension => HardwareCategory::LinkExtension,
            Self::TouchPureTree
            | Self::PresenceSensorTree
            | Self::RgbwDimmerTree
            | Self::Nano2RelayTree
            | Self::NfcCodeTouchTree
            | Self::ValveActuatorTree => HardwareCategory::TreeDevice,
            Self::TouchPureAir
            | Self::SmokeDetectorAir
            | Self::WaterSensorAir
            | Self::NanoDimmerAir => HardwareCategory::AirDevice,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::TreeExtension => "Tree Extension",
            Self::AirBaseExtension => "Air Base Extension",
            Self::RelayExtension => "Relay Extension (14x DO)",
            Self::DimmerExtension => "Dimmer Extension (4x AQ, 8x DI)",
            Self::AudioServer => "Audioserver (4x Stereo Out)",
            Self::ModbusExtension => "Modbus Extension (RS485)",
            Self::DaliExtension => "DALI Extension (64 ECGs / 16 Groups)",
            Self::DmxExtension => "DMX Extension (128 Channels)",
            Self::OneWireExtension => "1-Wire Extension (Sensors & iButton)",
            Self::TouchPureTree => "Touch Pure Tree",
            Self::PresenceSensorTree => "Presence Sensor Tree",
            Self::RgbwDimmerTree => "RGBW 24V Dimmer Tree",
            Self::Nano2RelayTree => "Nano 2 Relay Tree",
            Self::NfcCodeTouchTree => "NFC Code Touch Tree",
            Self::ValveActuatorTree => "Valve Actuator Tree",
            Self::TouchPureAir => "Touch Pure Air",
            Self::SmokeDetectorAir => "Smoke Detector Air",
            Self::WaterSensorAir => "Water Sensor Air",
            Self::NanoDimmerAir => "Nano Dimmer Air",
        }
    }

    pub fn node_type(self) -> &'static str {
        match self {
            Self::TreeExtension => "LoxTree",
            Self::AirBaseExtension => "LoxAIR",
            Self::RelayExtension => "LoxREL",
            Self::DimmerExtension => "LoxDIMM",
            Self::AudioServer => "AudioServer",
            Self::ModbusExtension => "Comm485",
            Self::DaliExtension => "LoxDALI",
            Self::DmxExtension => "LoxDMX",
            Self::OneWireExtension => "Lox1Wire",
            Self::TouchPureTree
            | Self::PresenceSensorTree
            | Self::RgbwDimmerTree
            | Self::Nano2RelayTree
            | Self::NfcCodeTouchTree
            | Self::ValveActuatorTree => "TreeDevice",
            Self::TouchPureAir
            | Self::SmokeDetectorAir
            | Self::WaterSensorAir
            | Self::NanoDimmerAir => "LoxAIRDevice",
        }
    }

    pub fn type_id(self) -> &'static str {
        match self {
            Self::TreeExtension => "LoxCaption_2",
            Self::AirBaseExtension => "LoxAIR",
            Self::RelayExtension => "RelayExtension",
            Self::DimmerExtension => "LoxDIMM",
            Self::AudioServer => "AudioServer_1",
            Self::ModbusExtension => "ModbusExtension",
            Self::DaliExtension => "DaliExtension",
            Self::DmxExtension => "DMXExtension",
            Self::OneWireExtension => "1WireExtension",
            Self::TouchPureTree => "TouchPureTree",
            Self::PresenceSensorTree => "PresenceSensorTree",
            Self::RgbwDimmerTree => "RGBW24VDimmerTree",
            Self::Nano2RelayTree => "Nano2RelayTree",
            Self::NfcCodeTouchTree => "NFCCodeTouchTree",
            Self::ValveActuatorTree => "ValveActuatorTree",
            Self::TouchPureAir => "TouchPureAir_V2",
            Self::SmokeDetectorAir => "FirealarmAir",
            Self::WaterSensorAir => "WaterSensor",
            Self::NanoDimmerAir => "NanoDimmer",
        }
    }

    pub fn serial_prefix(self) -> &'static str {
        match self {
            Self::TreeExtension => "1300",
            Self::AirBaseExtension => "0B00",
            Self::RelayExtension => "0D00",
            Self::DimmerExtension => "0C00",
            Self::AudioServer => "504F94F0",
            Self::ModbusExtension => "0E00",
            Self::DaliExtension => "0A00",
            Self::DmxExtension => "0900",
            Self::OneWireExtension => "0600",
            Self::TouchPureTree
            | Self::PresenceSensorTree
            | Self::RgbwDimmerTree
            | Self::Nano2RelayTree
            | Self::NfcCodeTouchTree
            | Self::ValveActuatorTree => "B030",
            Self::TouchPureAir
            | Self::SmokeDetectorAir
            | Self::WaterSensorAir
            | Self::NanoDimmerAir => "2010",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Self::TreeExtension => "2x Loxone Tree branches (Left & Right, up to 50 Tree devices each)",
            Self::AirBaseExtension => "Wireless Loxone Air transceiver for up to 128 Air devices",
            Self::RelayExtension => "14x 16A high-power relay digital outputs (Q1..Q14)",
            Self::DimmerExtension => "4x universal 230V dimmer channels (AQ1..AQ4) + 8x digital inputs",
            Self::AudioServer => "4x amplified stereo outputs + Tree Turbo & network audio streaming",
            Self::ModbusExtension => "RS-485 Modbus RTU bus interface for meters, HVAC & inverters",
            Self::DaliExtension => "DALI bus master for up to 64 ECG ballasts (0..63) and 16 DALI groups (0..15)",
            Self::DmxExtension => "DMX512 lighting interface for up to 128 channels / RGBW fixtures",
            Self::OneWireExtension => "1-Wire bus master for DS18B20 temperature sensors and iButton readers",
            Self::TouchPureTree => "5-point capacitive touch switch + integrated temperature & humidity sensor",
            Self::PresenceSensorTree => "PIR + acoustic presence sensor + brightness lux meter",
            Self::RgbwDimmerTree => "4-channel PWM LED dimmer (Red, Green, Blue, White / Smart Actuator)",
            Self::Nano2RelayTree => "Compact flush-mount 2x relay actuator for shading or lighting",
            Self::NfcCodeTouchTree => "Encrypted NFC reader + backlit keypad with access outputs",
            Self::ValveActuatorTree => "Motorized 0-100% heating/cooling manifold valve actuator",
            Self::TouchPureAir => "Battery/24V wireless 5-point touch switch + temp/humidity sensor",
            Self::SmokeDetectorAir => "Optical wireless smoke detector with acoustic alarm & battery status",
            Self::WaterSensorAir => "Wireless flood & water leak sensor with acoustic alert",
            Self::NanoDimmerAir => "Flush-mount wireless phase-cut dimmer with 2x digital inputs",
        }
    }
}

pub fn normalize_loxone_serial(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_hexdigit())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

pub fn generate_mock_serial(kind: HardwareTemplateKind) -> String {
    let seq = SERIAL_SEQ.fetch_add(1, Ordering::Relaxed) & 0xFFFF;
    format!("{}{:04X}", kind.serial_prefix(), seq)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentBusCandidate {
    pub uuid: String,
    pub label: String,
    pub node_type: String,
}

fn collect_parent_buses_rec(
    node: &TreeNode,
    category: HardwareCategory,
    parent_label: &str,
    out: &mut Vec<ParentBusCandidate>,
) {
    match category {
        HardwareCategory::LinkExtension => {
            if node.control_type == "LoxLIVE" {
                out.push(ParentBusCandidate {
                    uuid: node.uuid.clone(),
                    label: format!("Miniserver Link ({})", node.title),
                    node_type: node.control_type.clone(),
                });
            }
        }
        HardwareCategory::TreeDevice => {
            if node.control_type == "LoxCaption"
                && node.raw_attributes.iter().any(|(k, v)| k == "CaptionType" && v == "3")
            {
                out.push(ParentBusCandidate {
                    uuid: node.uuid.clone(),
                    label: format!("{} ▸ {}", parent_label, node.title),
                    node_type: node.control_type.clone(),
                });
            } else if node.control_type == "LoxTree"
                && !node.children.iter().any(|c| c.control_type == "LoxCaption")
            {
                out.push(ParentBusCandidate {
                    uuid: node.uuid.clone(),
                    label: format!("Tree Extension ({})", node.title),
                    node_type: node.control_type.clone(),
                });
            }
        }
        HardwareCategory::AirDevice => {
            if node.control_type == "LoxAIR" {
                out.push(ParentBusCandidate {
                    uuid: node.uuid.clone(),
                    label: format!("Air Base ({})", node.title),
                    node_type: node.control_type.clone(),
                });
            }
        }
    }
    let next_parent = if !node.title.is_empty() {
        node.title.as_str()
    } else {
        parent_label
    };
    for child in &node.children {
        collect_parent_buses_rec(child, category, next_parent, out);
    }
}

pub fn collect_parent_bus_candidates(
    project: &LoxoneProject,
    category: HardwareCategory,
) -> Vec<ParentBusCandidate> {
    let mut out = Vec::new();
    for node in &project.periphery_tree {
        collect_parent_buses_rec(node, category, &node.title, &mut out);
    }
    out
}

fn make_io_leaf(
    control_type: &str,
    title: &str,
    pin_key: &str,
    pin_kind: PinKind,
    room_uuid: Option<&str>,
    category_uuid: Option<&str>,
    extra_attrs: &[(&str, &str)],
) -> TreeNode {
    let uuid = generate_loxone_uuid();
    let pin_uuid = generate_loxone_uuid();
    let mut raw_attributes = vec![
        ("Type".to_string(), control_type.to_string()),
        ("V".to_string(), "178".to_string()),
        ("U".to_string(), uuid.clone()),
        ("Title".to_string(), title.to_string()),
        ("WF".to_string(), "16384".to_string()),
    ];
    for (k, v) in extra_attrs {
        raw_attributes.push(((*k).to_string(), (*v).to_string()));
    }
    TreeNode {
        uuid,
        control_type: control_type.to_string(),
        title: title.to_string(),
        section: TreeSectionKind::Periphery,
        room_uuid: room_uuid.map(str::to_string),
        category_uuid: category_uuid.map(str::to_string),
        pins: vec![ConnectorPin {
            uuid: pin_uuid,
            key: pin_key.to_string(),
            default_value: None,
            inverted: false,
            pin_kind,
            incoming_links: Vec::new(),
        }],
        raw_attributes,
        extra_children: Vec::new(),
        children: Vec::new(),
    }
}

fn make_caption_node(
    control_type: &str,
    title: &str,
    extra_attrs: &[(&str, &str)],
    children: Vec<TreeNode>,
) -> TreeNode {
    let uuid = generate_loxone_uuid();
    let mut raw_attributes = vec![
        ("Type".to_string(), control_type.to_string()),
        ("V".to_string(), "178".to_string()),
        ("U".to_string(), uuid.clone()),
        ("Title".to_string(), title.to_string()),
        ("WF".to_string(), "16384".to_string()),
    ];
    for (k, v) in extra_attrs {
        raw_attributes.push(((*k).to_string(), (*v).to_string()));
    }
    TreeNode {
        uuid,
        control_type: control_type.to_string(),
        title: title.to_string(),
        section: TreeSectionKind::Periphery,
        room_uuid: None,
        category_uuid: None,
        pins: Vec::new(),
        raw_attributes,
        extra_children: Vec::new(),
        children,
    }
}

pub fn build_hardware_tree_node(
    kind: HardwareTemplateKind,
    title: &str,
    serial: &str,
    room_uuid: Option<&str>,
    category_uuid: Option<&str>,
) -> TreeNode {
    let uuid = generate_loxone_uuid();
    let clean_serial = {
        let norm = normalize_loxone_serial(serial);
        if norm.is_empty() {
            generate_mock_serial(kind)
        } else {
            norm
        }
    };
    let effective_title = if title.trim().is_empty() {
        kind.display_name()
    } else {
        title.trim()
    };

    let mut raw_attributes = vec![
        ("Type".to_string(), kind.node_type().to_string()),
        ("TId".to_string(), kind.type_id().to_string()),
        ("V".to_string(), "178".to_string()),
        ("U".to_string(), uuid.clone()),
        ("Title".to_string(), effective_title.to_string()),
        ("WF".to_string(), "16384".to_string()),
        ("Serial".to_string(), clean_serial),
    ];
    if kind == HardwareTemplateKind::AudioServer {
        raw_attributes.push(("MAC".to_string(), "504F94F09999".to_string()));
    }

    let mut children = Vec::new();
    match kind {
        HardwareTemplateKind::TreeExtension => {
            children.push(make_caption_node(
                "LoxCaption",
                "Gałąź Tree Lewo",
                &[("CaptionType", "3"), ("SubType", "3")],
                Vec::new(),
            ));
            children.push(make_caption_node(
                "LoxCaption",
                "Gałąź Tree Prawo",
                &[("CaptionType", "3"), ("SubType", "3")],
                Vec::new(),
            ));
        }
        HardwareTemplateKind::AirBaseExtension => {
            children.push(make_io_leaf(
                "Online",
                "Status Online Air Base",
                "Q",
                PinKind::Output,
                room_uuid,
                category_uuid,
                &[],
            ));
        }
        HardwareTemplateKind::RelayExtension => {
            let mut outputs = Vec::new();
            for idx in 1..=14 {
                outputs.push(make_io_leaf(
                    "Actor",
                    &format!("Wyjście Q{idx}"),
                    "I",
                    PinKind::Input,
                    room_uuid,
                    category_uuid,
                    &[],
                ));
            }
            children.push(make_caption_node("OutputCaption", "Wyjścia cyfrowe", &[], outputs));
        }
        HardwareTemplateKind::DimmerExtension => {
            let mut inputs = Vec::new();
            for idx in 1..=8 {
                inputs.push(make_io_leaf(
                    "DigitalIn",
                    &format!("Wejście I{idx}"),
                    "Q",
                    PinKind::Output,
                    room_uuid,
                    category_uuid,
                    &[],
                ));
            }
            let mut dimmers = Vec::new();
            for idx in 1..=4 {
                dimmers.push(make_io_leaf(
                    "Dimmer",
                    &format!("Ściemniacz AQ{idx}"),
                    "I",
                    PinKind::Input,
                    room_uuid,
                    category_uuid,
                    &[],
                ));
            }
            children.push(make_caption_node("InputCaption", "Wejścia cyfrowe", &[], inputs));
            children.push(make_caption_node(
                "AnalogOutputCaption",
                "Wyjścia ściemniacza",
                &[],
                dimmers,
            ));
        }
        HardwareTemplateKind::AudioServer => {
            for stereo_idx in 1..=2 {
                children.push(make_io_leaf(
                    "AudioOut",
                    &format!("Wyjście stereo {stereo_idx} - Lewo"),
                    "I",
                    PinKind::Input,
                    room_uuid,
                    category_uuid,
                    &[("MxV", "100"), ("VOLL", "30")],
                ));
                children.push(make_io_leaf(
                    "AudioOut",
                    &format!("Wyjście stereo {stereo_idx} - Prawa"),
                    "I",
                    PinKind::Input,
                    room_uuid,
                    category_uuid,
                    &[("MxV", "100"), ("VOLL", "30")],
                ));
            }
        }
        HardwareTemplateKind::ModbusExtension => {
            children.push(make_io_leaf(
                "Sensor",
                "Licznik energii (kW)",
                "Q",
                PinKind::Output,
                room_uuid,
                category_uuid,
                &[],
            ));
        }
        HardwareTemplateKind::DaliExtension => {
            children.push(make_io_leaf(
                "DALIDimmer",
                &format!("{effective_title} ECG #0"),
                "I",
                PinKind::Input,
                room_uuid,
                category_uuid,
                &[("Address", "0"), ("IName", "AQ1"), ("Percent", "true")],
            ));
            children.push(make_io_leaf(
                "DALIGroup",
                &format!("{effective_title} Group #0"),
                "I",
                PinKind::Input,
                room_uuid,
                category_uuid,
                &[("Address", "0"), ("IName", "AQG1"), ("Percent", "true")],
            ));
        }
        HardwareTemplateKind::DmxExtension => {
            children.push(make_io_leaf(
                "DMXActor",
                &format!("{effective_title} CH 1"),
                "I",
                PinKind::Input,
                room_uuid,
                category_uuid,
                &[("Channel", "1"), ("IName", "AQ1"), ("Percent", "true")],
            ));
            children.push(make_io_leaf(
                "DMXActor",
                &format!("{effective_title} RGBW CH 2..5"),
                "I",
                PinKind::Input,
                room_uuid,
                category_uuid,
                &[("Channel", "2"), ("SubType", "RGBW"), ("IName", "AQ2")],
            ));
        }
        HardwareTemplateKind::OneWireExtension => {
            children.push(make_io_leaf(
                "OneWireSensor",
                &format!("{effective_title} DS18B20 Temp #1"),
                "AQ",
                PinKind::Output,
                room_uuid,
                category_uuid,
                &[("Serial", "28.A1B2C3D40001"), ("IName", "AI1"), ("Analog", "true")],
            ));
        }
        HardwareTemplateKind::TouchPureTree | HardwareTemplateKind::TouchPureAir => {
            children.push(make_io_leaf(
                "Touch",
                &format!("{effective_title} Przyciski T5"),
                "O",
                PinKind::Output,
                room_uuid,
                category_uuid,
                &[],
            ));
            children.push(make_io_leaf(
                "Sensor",
                &format!("{effective_title} Temperatura"),
                "Q",
                PinKind::Output,
                room_uuid,
                category_uuid,
                &[],
            ));
            children.push(make_io_leaf(
                "Sensor",
                &format!("{effective_title} Wilgotność"),
                "Q",
                PinKind::Output,
                room_uuid,
                category_uuid,
                &[],
            ));
        }
        HardwareTemplateKind::PresenceSensorTree => {
            children.push(make_io_leaf(
                "DigitalIn",
                &format!("{effective_title} Obecność"),
                "Q",
                PinKind::Output,
                room_uuid,
                category_uuid,
                &[],
            ));
            children.push(make_io_leaf(
                "Sensor",
                &format!("{effective_title} Jasność (lx)"),
                "Q",
                PinKind::Output,
                room_uuid,
                category_uuid,
                &[],
            ));
        }
        HardwareTemplateKind::RgbwDimmerTree => {
            children.push(make_io_leaf(
                "Dimmer",
                &format!("{effective_title} RGBW Smart Actuator"),
                "I",
                PinKind::Input,
                room_uuid,
                category_uuid,
                &[],
            ));
        }
        HardwareTemplateKind::Nano2RelayTree => {
            children.push(make_io_leaf(
                "Actor",
                &format!("{effective_title} Przekaźnik Q1"),
                "I",
                PinKind::Input,
                room_uuid,
                category_uuid,
                &[],
            ));
            children.push(make_io_leaf(
                "Actor",
                &format!("{effective_title} Przekaźnik Q2"),
                "I",
                PinKind::Input,
                room_uuid,
                category_uuid,
                &[],
            ));
        }
        HardwareTemplateKind::NfcCodeTouchTree => {
            children.push(make_io_leaf(
                "DigitalIn",
                &format!("{effective_title} Autoryzacja Q1"),
                "Q",
                PinKind::Output,
                room_uuid,
                category_uuid,
                &[],
            ));
            children.push(make_io_leaf(
                "DigitalIn",
                &format!("{effective_title} Dzwonek"),
                "Q",
                PinKind::Output,
                room_uuid,
                category_uuid,
                &[],
            ));
        }
        HardwareTemplateKind::ValveActuatorTree => {
            children.push(make_io_leaf(
                "Actuator",
                &format!("{effective_title} Zawór AQ (0-100%)"),
                "I",
                PinKind::Input,
                room_uuid,
                category_uuid,
                &[],
            ));
        }
        HardwareTemplateKind::SmokeDetectorAir => {
            children.push(make_io_leaf(
                "DigitalIn",
                &format!("{effective_title} Alarm dymu"),
                "Q",
                PinKind::Output,
                room_uuid,
                category_uuid,
                &[],
            ));
        }
        HardwareTemplateKind::WaterSensorAir => {
            children.push(make_io_leaf(
                "DigitalIn",
                &format!("{effective_title} Wyciek wody"),
                "Q",
                PinKind::Output,
                room_uuid,
                category_uuid,
                &[],
            ));
        }
        HardwareTemplateKind::NanoDimmerAir => {
            children.push(make_io_leaf(
                "Dimmer",
                &format!("{effective_title} Ściemniacz AQ"),
                "I",
                PinKind::Input,
                room_uuid,
                category_uuid,
                &[],
            ));
        }
    }

    TreeNode {
        uuid,
        control_type: kind.node_type().to_string(),
        title: effective_title.to_string(),
        section: TreeSectionKind::Periphery,
        room_uuid: room_uuid.map(str::to_string),
        category_uuid: category_uuid.map(str::to_string),
        pins: Vec::new(),
        raw_attributes,
        extra_children: Vec::new(),
        children,
    }
}

pub fn build_provision_hardware_edit(
    project: &LoxoneProject,
    kind: HardwareTemplateKind,
    title: &str,
    serial: &str,
    parent_bus_uuid: Option<&str>,
    room_uuid: Option<&str>,
    category_uuid: Option<&str>,
) -> ProjectEdit {
    let node = build_hardware_tree_node(kind, title, serial, room_uuid, category_uuid);
    let resolved_parent = parent_bus_uuid
        .filter(|u| !u.is_empty())
        .map(str::to_string)
        .or_else(|| {
            collect_parent_bus_candidates(project, kind.category())
                .into_iter()
                .next()
                .map(|c| c.uuid)
        });
    ProjectEdit::AddTreeNode {
        section: TreeSectionKind::Periphery,
        parent_uuid: resolved_parent,
        node,
    }
}

// ============================================================================
// HARDWARE INVENTORY & SERIAL SWAP
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareDeviceSummary {
    pub uuid: String,
    pub title: String,
    pub node_type: String,
    pub type_id: String,
    pub serial: String,
    pub parent_label: String,
    pub io_count: usize,
    pub description: String,
    pub distribution_board: String,
    pub row_position: String,
    pub lighting_group: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialConflictGroup {
    pub serial: String,
    pub device_uuids: Vec<String>,
    pub device_titles: Vec<String>,
}

fn count_io_leaves(node: &TreeNode) -> usize {
    let self_io = usize::from(!node.pins.is_empty());
    self_io + node.children.iter().map(count_io_leaves).sum::<usize>()
}

fn is_hardware_device_type(control_type: &str) -> bool {
    matches!(
        control_type,
        "LoxLIVE"
            | "AudioServer"
            | "LoxTree"
            | "LoxAIR"
            | "LoxDIMM"
            | "LoxREL"
            | "Comm485"
            | "LoxDALI"
            | "LoxDMX"
            | "Lox1Wire"
            | "TreeDevice"
            | "LoxAIRDevice"
            | "IntercomDevice"
            | "NetworkDevice"
            | "MTablet"
    )
}

fn collect_hardware_rec(
    node: &TreeNode,
    parent_label: &str,
    out: &mut Vec<HardwareDeviceSummary>,
) {
    if is_hardware_device_type(&node.control_type) {
        let serial = node
            .attr("Serial")
            .or_else(|| node.attr("MAC"))
            .unwrap_or_default()
            .to_string();
        let type_id = node
            .attr("TId")
            .unwrap_or(node.control_type.as_str())
            .to_string();
        let description = node.attr("Desc").unwrap_or_default().to_string();
        let distribution_board = node
            .attr("DistBoard")
            .or_else(|| node.attr("DBoard"))
            .unwrap_or_default()
            .to_string();
        let row_position = node
            .attr("RowPos")
            .or_else(|| node.attr("BlinkPos"))
            .unwrap_or_default()
            .to_string();
        let lighting_group = node
            .attr("LightGroup")
            .or_else(|| node.attr("LGrp"))
            .unwrap_or_default()
            .to_string();

        out.push(HardwareDeviceSummary {
            uuid: node.uuid.clone(),
            title: node.title.clone(),
            node_type: node.control_type.clone(),
            type_id,
            serial,
            parent_label: parent_label.to_string(),
            io_count: count_io_leaves(node),
            description,
            distribution_board,
            row_position,
            lighting_group,
        });
    }
    let next_parent = if !node.title.is_empty() {
        node.title.as_str()
    } else {
        parent_label
    };
    for child in &node.children {
        collect_hardware_rec(child, next_parent, out);
    }
}

pub fn collect_provisioned_hardware(project: &LoxoneProject) -> Vec<HardwareDeviceSummary> {
    let mut out = Vec::new();
    for node in project.periphery_tree.iter().chain(project.system_tree.iter()) {
        collect_hardware_rec(node, "Periphery", &mut out);
    }
    out
}

pub fn build_serial_swap_edit(node_uuid: &str, new_serial: &str) -> ProjectEdit {
    let clean = normalize_loxone_serial(new_serial);
    ProjectEdit::SetControlAttr {
        control_uuid: node_uuid.to_string(),
        attr_name: "Serial".to_string(),
        value: (!clean.is_empty()).then_some(clean),
    }
}

/// Builds a batch edit to replace a hardware device's serial number and/or update its
/// commissioning metadata (`Desc`, `DistBoard`, `RowPos`, `LightGroup`).
pub fn build_replace_device_edit(
    node_uuid: &str,
    new_serial: &str,
    description: Option<&str>,
    distribution_board: Option<&str>,
    row_position: Option<&str>,
    lighting_group: Option<&str>,
) -> ProjectEdit {
    let mut edits = vec![build_serial_swap_edit(node_uuid, new_serial)];
    if let Some(desc) = description {
        let trimmed = desc.trim();
        edits.push(ProjectEdit::SetControlAttr {
            control_uuid: node_uuid.to_string(),
            attr_name: "Desc".to_string(),
            value: (!trimmed.is_empty()).then(|| trimmed.to_string()),
        });
    }
    if let Some(db) = distribution_board {
        let trimmed = db.trim();
        edits.push(ProjectEdit::SetControlAttr {
            control_uuid: node_uuid.to_string(),
            attr_name: "DistBoard".to_string(),
            value: (!trimmed.is_empty()).then(|| trimmed.to_string()),
        });
    }
    if let Some(rp) = row_position {
        let trimmed = rp.trim();
        edits.push(ProjectEdit::SetControlAttr {
            control_uuid: node_uuid.to_string(),
            attr_name: "RowPos".to_string(),
            value: (!trimmed.is_empty()).then(|| trimmed.to_string()),
        });
    }
    if let Some(lg) = lighting_group {
        let trimmed = lg.trim();
        edits.push(ProjectEdit::SetControlAttr {
            control_uuid: node_uuid.to_string(),
            attr_name: "LightGroup".to_string(),
            value: (!trimmed.is_empty()).then(|| trimmed.to_string()),
        });
    }
    ProjectEdit::Batch(edits)
}

/// Detects duplicate serial numbers or unassigned (`00000000`) serials across hardware devices.
pub fn detect_serial_number_conflicts(project: &LoxoneProject) -> Vec<SerialConflictGroup> {
    let devices = collect_provisioned_hardware(project);
    let mut by_serial: std::collections::BTreeMap<String, Vec<HardwareDeviceSummary>> =
        std::collections::BTreeMap::new();
    for dev in devices {
        if dev.node_type == "LoxLIVE" {
            continue;
        }
        let s = if dev.serial.trim().is_empty() {
            "00000000".to_string()
        } else {
            dev.serial.trim().to_ascii_uppercase()
        };
        by_serial.entry(s).or_default().push(dev);
    }
    let mut conflicts = Vec::new();
    for (serial, group) in by_serial {
        if serial == "00000000" || group.len() > 1 {
            conflicts.push(SerialConflictGroup {
                serial,
                device_uuids: group.iter().map(|d| d.uuid.clone()).collect(),
                device_titles: group.iter().map(|d| d.title.clone()).collect(),
            });
        }
    }
    conflicts
}

/// Generates an undoable `ProjectEdit::Batch` that assigns unique serial numbers to all
/// conflicting or unassigned (`00000000`) hardware devices in the project.
pub fn build_resolve_serial_conflicts_edit(
    project: &LoxoneProject,
) -> (Vec<(String, String, String)>, Option<ProjectEdit>) {
    let devices = collect_provisioned_hardware(project);
    let mut used_serials: std::collections::BTreeSet<String> = devices
        .iter()
        .map(|d| d.serial.trim().to_ascii_uppercase())
        .filter(|s| !s.is_empty() && s != "00000000")
        .collect();

    let mut seen_once: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut reassignments = Vec::new();
    let mut edits = Vec::new();
    let mut next_seq: u32 = 0x1001;

    for dev in devices {
        if dev.node_type == "LoxLIVE" {
            continue;
        }
        let norm = dev.serial.trim().to_ascii_uppercase();
        let needs_new = norm.is_empty() || norm == "00000000" || !seen_once.insert(norm.clone());
        if needs_new {
            let prefix = match dev.node_type.as_str() {
                "TreeDevice" => "B030",
                "LoxAIRDevice" => "504F",
                _ => "14D8",
            };
            let new_serial = loop {
                let cand = format!("{prefix}{next_seq:04X}");
                next_seq += 1;
                if used_serials.insert(cand.clone()) {
                    break cand;
                }
            };
            reassignments.push((dev.uuid.clone(), dev.serial.clone(), new_serial.clone()));
            edits.push(build_serial_swap_edit(&dev.uuid, &new_serial));
        }
    }

    let batch = (!edits.is_empty()).then_some(ProjectEdit::Batch(edits));
    (reassignments, batch)
}

// ============================================================================
// HARDWARE DEVICE IDENTIFICATION (LINK / TREE / AIR)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceBusKind {
    Link,
    Tree,
    Air,
    System,
}

impl DeviceBusKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Link => "Loxone Link",
            Self::Tree => "Loxone Tree",
            Self::Air => "Loxone Air",
            Self::System => "Miniserver",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceIdentifyTarget {
    pub uuid: String,
    pub title: String,
    pub control_type: String,
    pub serial: String,
    pub bus_kind: DeviceBusKind,
    /// Optional child I/O channel UUID (e.g. acoustic click feedback `SubType="163"` or LED channel).
    pub feedback_io_uuid: Option<String>,
}

/// Returns `true` if `node` represents an identifiable Loxone Link extension, Tree device,
/// Air device, or Miniserver hardware unit in the Periphery tree.
pub fn is_identifiable_hardware_node(node: &TreeNode) -> bool {
    if node.uuid.is_empty() || node.control_type == "LoxCaption" {
        return false;
    }
    matches!(
        node.control_type.as_str(),
        "LoxTree"
            | "LoxDIMM"
            | "LoxREL"
            | "Comm485"
            | "LoxAIR"
            | "AudioServer"
            | "TreeDevice"
            | "LoxAIRDevice"
            | "IntercomDevice"
            | "LoxLIVE"
    ) || node.attr("Serial").is_some_and(|s| !s.trim().is_empty())
        || node
            .attr("IDev")
            .is_some_and(|s| s.eq_ignore_ascii_case("true"))
}

fn classify_device_bus_kind(node: &TreeNode) -> DeviceBusKind {
    match node.control_type.as_str() {
        "TreeDevice" => DeviceBusKind::Tree,
        "LoxAIRDevice" => DeviceBusKind::Air,
        "LoxTree" | "LoxDIMM" | "LoxREL" | "Comm485" | "LoxAIR" | "AudioServer"
        | "IntercomDevice" => DeviceBusKind::Link,
        "LoxLIVE" => DeviceBusKind::System,
        _ => {
            if node.attr("BlinkPos").is_some() {
                DeviceBusKind::Tree
            } else if node.attr("ExtensionBlinkNum").is_some()
                || node
                    .attr("IDev")
                    .is_some_and(|s| s.eq_ignore_ascii_case("true"))
            {
                DeviceBusKind::Link
            } else {
                DeviceBusKind::System
            }
        }
    }
}

fn find_feedback_io_child(node: &TreeNode) -> Option<String> {
    for child in &node.children {
        if child.attr("SubType") == Some("163") && !child.uuid.is_empty() {
            return Some(child.uuid.clone());
        }
        let lower_title = child.title.to_lowercase();
        if matches!(child.control_type.as_str(), "Actor" | "Dimmer")
            && (lower_title.contains("led")
                || lower_title.contains("klik")
                || lower_title.contains("click")
                || lower_title.contains("akust"))
            && !child.uuid.is_empty()
        {
            return Some(child.uuid.clone());
        }
        if let Some(found) = find_feedback_io_child(child) {
            return Some(found);
        }
    }
    None
}

fn build_identify_target(node: &TreeNode) -> DeviceIdentifyTarget {
    let serial = node
        .attr("Serial")
        .or_else(|| node.attr("MAC"))
        .unwrap_or_default()
        .trim()
        .to_string();
    let title = if node.title.trim().is_empty() {
        node.control_type.clone()
    } else {
        node.title.clone()
    };
    DeviceIdentifyTarget {
        uuid: node.uuid.clone(),
        title,
        control_type: node.control_type.clone(),
        serial,
        bus_kind: classify_device_bus_kind(node),
        feedback_io_uuid: find_feedback_io_child(node),
    }
}

fn resolve_identify_target_rec<'a>(
    node: &'a TreeNode,
    target_uuid: &str,
    enclosing_hw: Option<&'a TreeNode>,
) -> Option<&'a TreeNode> {
    let current_hw = if is_identifiable_hardware_node(node) {
        Some(node)
    } else {
        enclosing_hw
    };
    if node.uuid == target_uuid {
        return current_hw;
    }
    for child in &node.children {
        if let Some(found) = resolve_identify_target_rec(child, target_uuid, current_hw) {
            return Some(found);
        }
    }
    None
}

/// Resolves the hardware device to identify when a user right-clicks or selects `node_uuid`
/// (supports clicking either the Link/Tree/Air device node itself or any of its child I/O channels).
pub fn resolve_device_identify_target(
    project: &LoxoneProject,
    node_uuid: &str,
) -> Option<DeviceIdentifyTarget> {
    if node_uuid.trim().is_empty() {
        return None;
    }
    for root in project
        .periphery_tree
        .iter()
        .chain(project.system_tree.iter())
    {
        if let Some(hw_node) = resolve_identify_target_rec(root, node_uuid, None) {
            return Some(build_identify_target(hw_node));
        }
    }
    None
}

// ============================================================================
// STAGE 18: DALI, DMX & 1-WIRE BUS DEVICE PROVISIONING & ADDRESSING
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BusDeviceProtocol {
    #[default]
    Dali,
    Dmx,
    OneWire,
    Modbus,
}

impl BusDeviceProtocol {
    pub const ALL: [BusDeviceProtocol; 4] = [
        Self::Dali,
        Self::Dmx,
        Self::OneWire,
        Self::Modbus,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Dali => "DALI Bus",
            Self::Dmx => "DMX512 Bus",
            Self::OneWire => "1-Wire Bus",
            Self::Modbus => "Modbus RS-485",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Dali => "dali",
            Self::Dmx => "dmx",
            Self::OneWire => "onewire",
            Self::Modbus => "modbus",
        }
    }

    pub fn from_code(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "dali" | "loxdali" => Some(Self::Dali),
            "dmx" | "dmx512" | "loxdmx" => Some(Self::Dmx),
            "onewire" | "1wire" | "1-wire" | "lox1wire" => Some(Self::OneWire),
            "modbus" | "rs485" | "comm485" => Some(Self::Modbus),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BusDeviceKind {
    #[default]
    DaliDimmerChannel,
    DaliGroupDimmer,
    DmxDimmerChannel,
    DmxRgbwActuator,
    OneWireTemperatureSensor,
    OneWireIButtonReader,
}

impl BusDeviceKind {
    pub const ALL: [BusDeviceKind; 6] = [
        Self::DaliDimmerChannel,
        Self::DaliGroupDimmer,
        Self::DmxDimmerChannel,
        Self::DmxRgbwActuator,
        Self::OneWireTemperatureSensor,
        Self::OneWireIButtonReader,
    ];

    pub fn protocol(self) -> BusDeviceProtocol {
        match self {
            Self::DaliDimmerChannel | Self::DaliGroupDimmer => BusDeviceProtocol::Dali,
            Self::DmxDimmerChannel | Self::DmxRgbwActuator => BusDeviceProtocol::Dmx,
            Self::OneWireTemperatureSensor | Self::OneWireIButtonReader => BusDeviceProtocol::OneWire,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::DaliDimmerChannel => "dali_ecg",
            Self::DaliGroupDimmer => "dali_group",
            Self::DmxDimmerChannel => "dmx_channel",
            Self::DmxRgbwActuator => "dmx_rgbw",
            Self::OneWireTemperatureSensor => "onewire_temp",
            Self::OneWireIButtonReader => "onewire_ibutton",
        }
    }

    pub fn from_code(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "dali_ecg" | "dalidimmer" | "dali_dimmer" => Some(Self::DaliDimmerChannel),
            "dali_group" | "daligroup" => Some(Self::DaliGroupDimmer),
            "dmx_channel" | "dmxactor" | "dmx_dimmer" => Some(Self::DmxDimmerChannel),
            "dmx_rgbw" | "dmxrgbw" => Some(Self::DmxRgbwActuator),
            "onewire_temp" | "onewiresensor" | "ds18b20" => Some(Self::OneWireTemperatureSensor),
            "onewire_ibutton" | "ibutton" => Some(Self::OneWireIButtonReader),
            _ => None,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::DaliDimmerChannel => "DALI ECG Dimmer (Short Addr 0..63)",
            Self::DaliGroupDimmer => "DALI Group Actuator (Group 0..15)",
            Self::DmxDimmerChannel => "DMX Single Channel Dimmer (CH 1..512)",
            Self::DmxRgbwActuator => "DMX 4-Ch RGBW Fixture (Start CH 1..509)",
            Self::OneWireTemperatureSensor => "1-Wire DS18B20 Temp Sensor (ROM ID)",
            Self::OneWireIButtonReader => "1-Wire iButton Reader (ROM ID)",
        }
    }

    pub fn control_type(self) -> &'static str {
        match self {
            Self::DaliDimmerChannel => "DALIDimmer",
            Self::DaliGroupDimmer => "DALIGroup",
            Self::DmxDimmerChannel | Self::DmxRgbwActuator => "DMXActor",
            Self::OneWireTemperatureSensor => "OneWireSensor",
            Self::OneWireIButtonReader => "OneWireiButton",
        }
    }

    pub fn default_address_hint(self) -> &'static str {
        match self {
            Self::DaliDimmerChannel => "0",
            Self::DaliGroupDimmer => "0",
            Self::DmxDimmerChannel => "1",
            Self::DmxRgbwActuator => "1",
            Self::OneWireTemperatureSensor => "28.A1B2C3D40001",
            Self::OneWireIButtonReader => "01.F0E1D2C30001",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvisionedBusDevice {
    pub uuid: String,
    pub parent_extension_uuid: String,
    pub parent_extension_title: String,
    pub protocol: BusDeviceProtocol,
    pub control_type: String,
    pub title: String,
    pub address_or_rom: String,
    pub room_uuid: Option<String>,
    pub category_uuid: Option<String>,
}

fn extension_protocol(node: &TreeNode) -> Option<BusDeviceProtocol> {
    match node.control_type.as_str() {
        "LoxDALI" => Some(BusDeviceProtocol::Dali),
        "LoxDMX" => Some(BusDeviceProtocol::Dmx),
        "Lox1Wire" => Some(BusDeviceProtocol::OneWire),
        "Comm485" => Some(BusDeviceProtocol::Modbus),
        _ => None,
    }
}

fn collect_bus_ext_parents_rec(
    node: &TreeNode,
    filter: Option<BusDeviceProtocol>,
    out: &mut Vec<ParentBusCandidate>,
) {
    if let Some(proto) = extension_protocol(node) {
        if filter.is_none() || filter == Some(proto) {
            out.push(ParentBusCandidate {
                uuid: node.uuid.clone(),
                label: format!("{} ({})", node.title, proto.label()),
                node_type: node.control_type.clone(),
            });
        }
    }
    for ch in &node.children {
        collect_bus_ext_parents_rec(ch, filter, out);
    }
}

pub fn collect_bus_extension_parents(
    project: &LoxoneProject,
    protocol: Option<BusDeviceProtocol>,
) -> Vec<ParentBusCandidate> {
    let mut out = Vec::new();
    for root in project
        .periphery_tree
        .iter()
        .chain(project.system_tree.iter())
    {
        collect_bus_ext_parents_rec(root, protocol, &mut out);
    }
    out
}

fn collect_bus_devices_rec(
    node: &TreeNode,
    enclosing_ext: Option<(&TreeNode, BusDeviceProtocol)>,
    out: &mut Vec<ProvisionedBusDevice>,
) {
    let current_ext = if let Some(proto) = extension_protocol(node) {
        Some((node, proto))
    } else {
        enclosing_ext
    };

    if matches!(
        node.control_type.as_str(),
        "DALIDimmer" | "DALIGroup" | "DMXActor" | "OneWireSensor" | "OneWireiButton"
    ) || (enclosing_ext.is_some()
        && matches!(node.control_type.as_str(), "Sensor" | "Actor" | "Dimmer" | "Actuator"))
    {
        let (parent_uuid, parent_title, proto) = match current_ext {
            Some((ext, p)) => (ext.uuid.clone(), ext.title.clone(), p),
            None => (
                String::new(),
                "Bus Extension".to_string(),
                match node.control_type.as_str() {
                    "DALIDimmer" | "DALIGroup" => BusDeviceProtocol::Dali,
                    "DMXActor" => BusDeviceProtocol::Dmx,
                    "OneWireSensor" | "OneWireiButton" => BusDeviceProtocol::OneWire,
                    _ => BusDeviceProtocol::Modbus,
                },
            ),
        };
        let address_or_rom = node
            .attr("Address")
            .or_else(|| node.attr("Channel"))
            .or_else(|| node.attr("Serial"))
            .unwrap_or("0")
            .to_string();
        out.push(ProvisionedBusDevice {
            uuid: node.uuid.clone(),
            parent_extension_uuid: parent_uuid,
            parent_extension_title: parent_title,
            protocol: proto,
            control_type: node.control_type.clone(),
            title: node.title.clone(),
            address_or_rom,
            room_uuid: node.room_uuid.clone(),
            category_uuid: node.category_uuid.clone(),
        });
    }

    for ch in &node.children {
        collect_bus_devices_rec(ch, current_ext, out);
    }
}

pub fn collect_bus_devices(project: &LoxoneProject) -> Vec<ProvisionedBusDevice> {
    let mut out = Vec::new();
    for root in project
        .periphery_tree
        .iter()
        .chain(project.system_tree.iter())
    {
        collect_bus_devices_rec(root, None, &mut out);
    }
    out
}

pub fn build_provision_bus_device_edit(
    project: &LoxoneProject,
    parent_extension_uuid: &str,
    kind: BusDeviceKind,
    title: &str,
    address_or_rom: &str,
    room_uuid: Option<&str>,
    category_uuid: Option<&str>,
) -> Result<(String, ProjectEdit), String> {
    let addr_trim = address_or_rom.trim();
    let effective_addr = if addr_trim.is_empty() {
        kind.default_address_hint().to_string()
    } else {
        addr_trim.to_string()
    };

    match kind {
        BusDeviceKind::DaliDimmerChannel => {
            let n: u32 = effective_addr
                .parse()
                .map_err(|_| format!("DALI ECG short address must be 0..63, got '{effective_addr}'"))?;
            if n > 63 {
                return Err(format!("DALI ECG short address must be in 0..=63 (got {n})"));
            }
        }
        BusDeviceKind::DaliGroupDimmer => {
            let n: u32 = effective_addr
                .parse()
                .map_err(|_| format!("DALI group address must be 0..15, got '{effective_addr}'"))?;
            if n > 15 {
                return Err(format!("DALI group address must be in 0..=15 (got {n})"));
            }
        }
        BusDeviceKind::DmxDimmerChannel => {
            let n: u32 = effective_addr
                .parse()
                .map_err(|_| format!("DMX channel must be 1..512, got '{effective_addr}'"))?;
            if !(1..=512).contains(&n) {
                return Err(format!("DMX channel must be in 1..=512 (got {n})"));
            }
        }
        BusDeviceKind::DmxRgbwActuator => {
            let n: u32 = effective_addr
                .parse()
                .map_err(|_| format!("DMX RGBW start channel must be 1..509, got '{effective_addr}'"))?;
            if !(1..=509).contains(&n) {
                return Err(format!("DMX RGBW start channel must be in 1..=509 (got {n})"));
            }
        }
        BusDeviceKind::OneWireTemperatureSensor | BusDeviceKind::OneWireIButtonReader => {
            if effective_addr.len() < 4 {
                return Err("1-Wire ROM ID must be at least 4 characters (e.g. 28.A1B2C3D40001)".to_string());
            }
        }
    }

    let resolved_parent = if !parent_extension_uuid.trim().is_empty() {
        parent_extension_uuid.trim().to_string()
    } else {
        collect_bus_extension_parents(project, Some(kind.protocol()))
            .into_iter()
            .next()
            .map(|c| c.uuid)
            .ok_or_else(|| {
                format!(
                    "No {} extension found in project; provision the extension first",
                    kind.protocol().label()
                )
            })?
    };

    let effective_title = if title.trim().is_empty() {
        format!("{} ({})", kind.display_name(), effective_addr)
    } else {
        title.trim().to_string()
    };

    let (pin_key, pin_kind, extra_attrs): (&str, PinKind, Vec<(&str, &str)>) = match kind {
        BusDeviceKind::DaliDimmerChannel => (
            "I",
            PinKind::Input,
            vec![("Address", &effective_addr), ("Percent", "true")],
        ),
        BusDeviceKind::DaliGroupDimmer => (
            "I",
            PinKind::Input,
            vec![("Address", &effective_addr), ("Percent", "true")],
        ),
        BusDeviceKind::DmxDimmerChannel => (
            "I",
            PinKind::Input,
            vec![("Channel", &effective_addr), ("Percent", "true")],
        ),
        BusDeviceKind::DmxRgbwActuator => (
            "I",
            PinKind::Input,
            vec![("Channel", &effective_addr), ("SubType", "RGBW")],
        ),
        BusDeviceKind::OneWireTemperatureSensor => (
            "AQ",
            PinKind::Output,
            vec![("Serial", &effective_addr), ("Analog", "true")],
        ),
        BusDeviceKind::OneWireIButtonReader => (
            "Q",
            PinKind::Output,
            vec![("Serial", &effective_addr)],
        ),
    };

    let node = make_io_leaf(
        kind.control_type(),
        &effective_title,
        pin_key,
        pin_kind,
        room_uuid,
        category_uuid,
        &extra_attrs,
    );
    let node_uuid = node.uuid.clone();
    Ok((
        node_uuid,
        ProjectEdit::AddTreeNode {
            section: TreeSectionKind::Periphery,
            parent_uuid: Some(resolved_parent),
            node,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::load_loxone_file;
    use crate::writer::{apply_edit_with_inverse, serialize_loxone_xml};
    use std::path::PathBuf;

    #[test]
    fn test_klimczaki_hardware_inventory_and_provisioning_roundtrip() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Sample.Loxone");
        let mut proj = load_loxone_file(&path).expect("load Sample.Loxone");

        // 1. Hardware discovery
        let inventory = collect_provisioned_hardware(&proj);
        assert!(
            inventory.len() >= 20,
            "Expected >= 20 hardware devices in Sample.Loxone, found {}",
            inventory.len()
        );
        assert!(inventory.iter().any(|d| d.node_type == "LoxLIVE" && d.serial == "00000000"));
        assert!(inventory.iter().any(|d| d.node_type == "TreeDevice"));
        assert!(inventory.iter().any(|d| d.node_type == "LoxAIRDevice"));

        // 2. Parent bus discovery
        let link_buses = collect_parent_bus_candidates(&proj, HardwareCategory::LinkExtension);
        assert!(!link_buses.is_empty());
        let tree_buses = collect_parent_bus_candidates(&proj, HardwareCategory::TreeDevice);
        assert!(!tree_buses.is_empty());
        let air_buses = collect_parent_bus_candidates(&proj, HardwareCategory::AirDevice);
        assert!(!air_buses.is_empty());

        // 3. Provision a new Touch Pure Tree device on the first Tree branch
        let prev_count = inventory.len();
        let edit = build_provision_hardware_edit(
            &proj,
            HardwareTemplateKind::TouchPureTree,
            "Salon Touch Pure Nowy",
            "B030CAFE",
            Some(&tree_buses[0].uuid),
            None,
            None,
        );
        let inv = apply_edit_with_inverse(&mut proj, &edit).expect("apply provision edit");
        let new_inventory = collect_provisioned_hardware(&proj);
        assert_eq!(new_inventory.len(), prev_count + 1);
        let added = new_inventory
            .iter()
            .find(|d| d.title == "Salon Touch Pure Nowy")
            .expect("find newly provisioned Touch Pure Tree");
        assert_eq!(added.serial, "B030CAFE");
        assert_eq!(added.io_count, 3);

        // 4. Serial number swap on the newly provisioned device
        let swap_edit = build_serial_swap_edit(&added.uuid, "b030-beef");
        let swap_inv = apply_edit_with_inverse(&mut proj, &swap_edit).expect("apply serial swap");
        let swapped = collect_provisioned_hardware(&proj)
            .into_iter()
            .find(|d| d.uuid == added.uuid)
            .unwrap();
        assert_eq!(swapped.serial, "B030BEEF");

        // 5. Undo both edits and verify XML roundtrip
        apply_edit_with_inverse(&mut proj, &swap_inv).unwrap();
        apply_edit_with_inverse(&mut proj, &inv).unwrap();
        assert_eq!(collect_provisioned_hardware(&proj).len(), prev_count);
        let xml = serialize_loxone_xml(&proj).unwrap();
        assert!(xml.contains("00000000"));
    }

    #[test]
    fn test_klimczaki_link_and_tree_device_identification_resolution() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Sample.Loxone");
        let proj = load_loxone_file(&path).expect("load Sample.Loxone");

        let inventory = collect_provisioned_hardware(&proj);
        let link_tree_ext = inventory
            .iter()
            .find(|d| d.node_type == "LoxTree")
            .expect("Klimczaki should have a LoxTree Link Extension");
        let link_target = resolve_device_identify_target(&proj, &link_tree_ext.uuid)
            .expect("LoxTree extension should resolve as identifiable");
        assert_eq!(link_target.bus_kind, DeviceBusKind::Link);
        assert_eq!(link_target.serial, "00000000");
        assert_eq!(link_target.serial, link_tree_ext.serial);

        let tree_dev = inventory
            .iter()
            .find(|d| d.node_type == "TreeDevice" && !d.serial.is_empty())
            .expect("Klimczaki should have a TreeDevice with serial");
        let tree_target = resolve_device_identify_target(&proj, &tree_dev.uuid)
            .expect("TreeDevice should resolve as identifiable");
        assert_eq!(tree_target.bus_kind, DeviceBusKind::Tree);
        assert_eq!(tree_target.serial, tree_dev.serial);
    }

    #[test]
    fn test_dali_dmx_and_onewire_bus_provisioning_and_addressing() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Sample.Loxone");
        let mut proj = load_loxone_file(&path).expect("load Sample.Loxone");

        // Provision DALI, DMX, and 1-Wire extensions
        for (kind, title, serial) in [
            (HardwareTemplateKind::DaliExtension, "DALI Line 1", "0A001001"),
            (HardwareTemplateKind::DmxExtension, "DMX Stage 1", "09001002"),
            (HardwareTemplateKind::OneWireExtension, "1-Wire Boiler", "06001003"),
        ] {
            let edit = build_provision_hardware_edit(&proj, kind, title, serial, None, None, None);
            apply_edit_with_inverse(&mut proj, &edit).expect("provision bus extension");
        }

        let dali_parents = collect_bus_extension_parents(&proj, Some(BusDeviceProtocol::Dali));
        assert_eq!(dali_parents.len(), 1);
        let dmx_parents = collect_bus_extension_parents(&proj, Some(BusDeviceProtocol::Dmx));
        assert_eq!(dmx_parents.len(), 1);
        let ow_parents = collect_bus_extension_parents(&proj, Some(BusDeviceProtocol::OneWire));
        assert_eq!(ow_parents.len(), 1);

        // Address validation checks
        assert!(build_provision_bus_device_edit(
            &proj,
            &dali_parents[0].uuid,
            BusDeviceKind::DaliDimmerChannel,
            "Invalid ECG",
            "64",
            None,
            None
        )
        .is_err());
        assert!(build_provision_bus_device_edit(
            &proj,
            &dmx_parents[0].uuid,
            BusDeviceKind::DmxDimmerChannel,
            "Invalid DMX",
            "513",
            None,
            None
        )
        .is_err());

        // Provision valid bus devices
        let (_, dali_edit) = build_provision_bus_device_edit(
            &proj,
            &dali_parents[0].uuid,
            BusDeviceKind::DaliDimmerChannel,
            "Kitchen Island Spot ECG #7",
            "7",
            None,
            None,
        )
        .unwrap();
        apply_edit_with_inverse(&mut proj, &dali_edit).unwrap();

        let (_, dmx_edit) = build_provision_bus_device_edit(
            &proj,
            &dmx_parents[0].uuid,
            BusDeviceKind::DmxRgbwActuator,
            "Cove Strip RGBW CH 10..13",
            "10",
            None,
            None,
        )
        .unwrap();
        apply_edit_with_inverse(&mut proj, &dmx_edit).unwrap();

        let (_, ow_edit) = build_provision_bus_device_edit(
            &proj,
            &ow_parents[0].uuid,
            BusDeviceKind::OneWireTemperatureSensor,
            "Return Pipe DS18B20",
            "28.FFEEDDCC0011",
            None,
            None,
        )
        .unwrap();
        apply_edit_with_inverse(&mut proj, &ow_edit).unwrap();

        let bus_devs = collect_bus_devices(&proj);
        assert!(bus_devs.iter().any(|d| d.title == "Kitchen Island Spot ECG #7" && d.address_or_rom == "7"));
        assert!(bus_devs.iter().any(|d| d.title == "Cove Strip RGBW CH 10..13" && d.address_or_rom == "10"));
        assert!(bus_devs.iter().any(|d| d.title == "Return Pipe DS18B20" && d.address_or_rom == "28.FFEEDDCC0011"));
    }
}
