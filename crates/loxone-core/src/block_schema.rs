//! Official Loxone Config Knowledge Base (KB) schema registry for functional blocks,
//! periphery tree nodes, and system tree nodes.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    Bool,
    Int,
    Float,
    DurationSeconds,
    DurationMinutes,
    Percent,
    TemperatureCelsius,
    Lux,
    Text,
    Enum(&'static [(&'static str, &'static str)]),
    Bitmask(&'static [(u32, &'static str)]),
    UuidRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildConfigKind {
    /// `<HCL ...>` Human Centric Lighting curve (`LightController2`)
    Hcl,
    /// `<PSD ...>` Sun direction / shading polygon (`AutoJalousie`, `LightController2`)
    Psd,
    /// `<Logger ...>` / `<Mailer ...>` / `<LoggerMailer ...>` child elements (`OutputRefLM`, `Door`, `AalEmergency`, `SmokeAlarm`)
    LoggerMailer,
    /// `<IoData ...>` child bindings (`LightController2`, `AutoJalousie`, `HeatIRoomController2`, etc.)
    IoData,
    /// `<CoHist ...>` connection history (`LightController2`, `AutoJalousie`, `HeatIRoomController2`, etc.)
    CoHist,
    /// `<LightsceneDescription ...>` scene table (`LightController2`)
    LightScenes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttributeSchema {
    pub xml_attr: &'static str,
    pub label: &'static str,
    pub unit: Option<&'static str>,
    pub kind: ValueKind,
    pub default_value: Option<&'static str>,
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinParamSchema {
    pub pin_key: &'static str,
    pub label: &'static str,
    pub unit: Option<&'static str>,
    pub kind: ValueKind,
    pub default_value: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinDoc {
    pub pin_key: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub is_output: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockSchema {
    pub type_name: &'static str,
    pub display_name_en: &'static str,
    pub category_group: &'static str,
    pub kb_url: &'static str,
    pub summary: &'static str,
    pub attributes: &'static [AttributeSchema],
    pub pin_params: &'static [PinParamSchema],
    pub pin_docs: &'static [PinDoc],
    pub child_configs: &'static [ChildConfigKind],
}

impl BlockSchema {
    pub fn find_attr(&self, xml_attr: &str) -> Option<&'static AttributeSchema> {
        self.attributes.iter().find(|a| a.xml_attr == xml_attr)
    }

    pub fn find_pin_param(&self, pin_key: &str) -> Option<&'static PinParamSchema> {
        if let Some(exact) = self.pin_params.iter().find(|p| p.pin_key == pin_key) {
            return Some(exact);
        }
        let prefix = pin_key.trim_end_matches(|c: char| c.is_ascii_digit());
        if !prefix.is_empty() && prefix.len() < pin_key.len() {
            if let Some(by_prefix) = self.pin_params.iter().find(|p| {
                p.pin_key == prefix
                    || p.pin_key.trim_end_matches(|c: char| c.is_ascii_digit()) == prefix
            }) {
                return Some(by_prefix);
            }
        }
        None
    }

    pub fn find_pin_doc(&self, pin_key: &str) -> Option<&'static PinDoc> {
        if let Some(exact) = self.pin_docs.iter().find(|p| p.pin_key == pin_key) {
            return Some(exact);
        }
        let prefix = pin_key.trim_end_matches(|c: char| c.is_ascii_digit());
        if !prefix.is_empty() && prefix.len() < pin_key.len() {
            if let Some(by_prefix) = self.pin_docs.iter().find(|p| {
                p.pin_key == prefix
                    || p.pin_key.trim_end_matches(|c: char| c.is_ascii_digit()) == prefix
            }) {
                return Some(by_prefix);
            }
        }
        None
    }
}

macro_rules! attr {
    ($xml_attr:expr, $label:expr, $unit:expr, $kind:expr, $def:expr, $desc:expr) => {
        AttributeSchema {
            xml_attr: $xml_attr,
            label: $label,
            unit: $unit,
            kind: $kind,
            default_value: $def,
            description: $desc,
        }
    };
}

macro_rules! param {
    ($pin_key:expr, $label:expr, $unit:expr, $kind:expr, $def:expr, $desc:expr) => {
        PinParamSchema {
            pin_key: $pin_key,
            label: $label,
            unit: $unit,
            kind: $kind,
            default_value: $def,
            description: $desc,
        }
    };
}

macro_rules! pin_in {
    ($pin_key:expr, $label:expr, $desc:expr) => {
        PinDoc {
            pin_key: $pin_key,
            label: $label,
            description: $desc,
            is_output: false,
        }
    };
}

macro_rules! pin_out {
    ($pin_key:expr, $label:expr, $desc:expr) => {
        PinDoc {
            pin_key: $pin_key,
            label: $label,
            description: $desc,
            is_output: true,
        }
    };
}

macro_rules! schema {
    ($ty:expr, $name:expr, $cat:expr, $url:expr, $sum:expr, $attrs:expr, $params:expr, $pins:expr, $children:expr) => {
        BlockSchema {
            type_name: $ty,
            display_name_en: $name,
            category_group: $cat,
            kb_url: $url,
            summary: $sum,
            attributes: $attrs,
            pin_params: $params,
            pin_docs: $pins,
            child_configs: $children,
        }
    };
}

static BLOCK_SCHEMAS: &[BlockSchema] = &[
    // --- 41 Page Block Types ---
    schema!(
        "AalEmergency",
        "AAL Emergency Alarm",
        "Security & Safety",
        "https://www.loxone.com/enen/kb/aal-emergency-alarm/",
        "Assisted living emergency button alarm with acoustic warning, escalation delay, and reset control.",
        &[
            attr!("V", "Version", None, ValueKind::Int, Some("11"), "Block version"),
            attr!("WF", "Visu Flags", None, ValueKind::Int, Some("1"), "Visualization flags"),

            attr!("TE", "Emergency Timeout", Some("s"), ValueKind::DurationSeconds, Some("20"), "Emergency escalation timeout"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("13"), "Number of connector pins on block"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
            attr!("UO", "UO", None, ValueKind::Text, None, "Loxone XML attribute UO"),
        ],
        &[
            param!("Ta", "Pre-alarm delay", Some("s"), ValueKind::DurationSeconds, "30", "Delay before emergency alarm triggers"),
            param!("Tr", "Repeat interval", Some("s"), ValueKind::DurationSeconds, "60", "Alarm repeat interval"),

            param!("ParTimeAlarm", "Pre-alarm delay", Some("s"), ValueKind::DurationSeconds, "4", "Delay before emergency alarm triggers"),
            param!("ParTimeConfirm", "Confirm timeout", Some("s"), ValueKind::DurationSeconds, "2", "Confirmation timeout duration"),
        ],
        &[
            pin_in!("T", "Trigger", "Emergency button trigger input"),
            pin_in!("Dis", "Disable", "Disables the emergency alarm"),
            pin_in!("R", "Reset", "Resets active alarm"),
            pin_out!("A", "Alarm", "Active emergency alarm output"),
            pin_out!("S", "Status", "Current alarm status"),

            pin_in!("InTrigger", "InTrigger", "Input pin InTrigger"),
            pin_in!("InAlarm", "InAlarm", "Input pin InAlarm"),
            pin_in!("InConfirm", "InConfirm", "Input pin InConfirm"),
            pin_in!("InReset", "InReset", "Input pin InReset"),
            pin_out!("OutAlarm", "OutAlarm", "Output pin OutAlarm"),
            pin_in!("InTriggerHidden", "InTriggerHidden", "Input pin InTriggerHidden"),
            pin_out!("OutAlarmSource", "OutAlarmSource", "Output pin OutAlarmSource"),
            pin_out!("OutAlarmStart", "OutAlarmStart", "Output pin OutAlarmStart"),
            pin_out!("OutAlarmStop", "OutAlarmStop", "Output pin OutAlarmStop"),
            pin_out!("OutConfirmSource", "OutConfirmSource", "Output pin OutConfirmSource"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
        ],
        &[ChildConfigKind::IoData, ChildConfigKind::LoggerMailer]
    ),
    schema!(
        "AalSmartAlarm",
        "AAL Smart Alarm",
        "Security & Safety",
        "https://www.loxone.com/enen/kb/aal-smart-alarm/",
        "Monitors activity and presence in assisted living areas to detect unusual inactivity or falls.",
        &[
            attr!("V", "Version", None, ValueKind::Int, Some("11"), "Block version"),
            attr!("WF", "Visu Flags", None, ValueKind::Int, Some("1"), "Visualization flags"),

            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("18"), "Number of connector pins on block"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
            attr!("UO", "UO", None, ValueKind::Text, None, "Loxone XML attribute UO"),
        ],
        &[
            param!("Tmo", "Monitoring time", Some("min"), ValueKind::DurationMinutes, "60", "Max allowed inactivity duration"),

            param!("Delay Alarm Level 2", "Escalation delay level 2", Some("s"), ValueKind::DurationSeconds, "60", "Delay before escalating to alarm level 2"),
            param!("Time Bedroom Alarm", "Bedroom max inactivity", Some("min"), ValueKind::DurationMinutes, "420", "Maximum allowed inactivity time in bedroom"),
            param!("Time Recreational Room Alarm", "Living area max inactivity", Some("min"), ValueKind::DurationMinutes, "60", "Maximum allowed inactivity time in recreational room"),
            param!("Time Throughfare Alarm", "Corridor max inactivity", Some("min"), ValueKind::DurationMinutes, "15", "Maximum allowed inactivity time in thoroughfare"),
        ],
        &[
            pin_in!("En", "Enable", "Enables smart monitoring"),
            pin_in!("Mo", "Motion", "Motion or activity detector input"),
            pin_in!("R", "Reset", "Resets triggered smart alarm"),
            pin_out!("A", "Alarm", "Smart alarm output"),

            pin_in!("Reset", "Reset", "Input pin Reset"),
            pin_in!("TestAlarm", "TestAlarm", "Input pin TestAlarm"),
            pin_out!("Status", "Status", "Output pin Status"),
            pin_out!("Quit", "Quit", "Output pin Quit"),
            pin_out!("Alarm", "Alarm", "Output pin Alarm"),
            pin_out!("Alarm Hidden", "Alarm Hidden", "Output pin Alarm Hidden"),
            pin_out!("Alarm Level 1", "Alarm Level 1", "Output pin Alarm Level 1"),
            pin_in!("Cause", "Cause", "Input pin Cause"),
            pin_in!("Confirm", "Confirm", "Input pin Confirm"),
            pin_in!("Leave", "Leave", "Input pin Leave"),
            pin_in!("Movement Bedroom", "Movement Bedroom", "Input pin Movement Bedroom"),
            pin_in!("Movement Hidden", "Movement Hidden", "Input pin Movement Hidden"),
            pin_in!("Movement Recreational Room", "Movement Recreational Room", "Input pin Movement Recreational Room"),
            pin_in!("Movement Throughfare", "Movement Throughfare", "Input pin Movement Throughfare"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_in!("Time", "Time", "Input pin Time"),
        ],
        &[ChildConfigKind::IoData]
    ),
    schema!(
        "Alarm",
        "Burglar Alarm",
        "Security & Safety",
        "https://www.loxone.com/enen/kb/burglar-alarm/",
        "Complete intrusion alarm system supporting delayed arming, silent pre-alarm, acoustical/visual alerts, and motion/contact zones.",
        &[
            attr!("AutoAck", "Auto Acknowledge", None, ValueKind::Bool, Some("false"), "Automatically acknowledge alarm on disarm"),
            attr!("I", "Inputs Mask", None, ValueKind::Int, Some("63"), "Enabled sensor input channels"),
            attr!("P", "Outputs Mask", None, ValueKind::Int, Some("2047"), "Enabled alarm output stages"),
            attr!("SpStates", "System State Refs", None, ValueKind::Text, None, "Linked system status references"),
            attr!("UO", "Linked Objects", None, ValueKind::Text, None, "Linked presence/window/door objects"),
            attr!("UiStatus", "UI Status Mode", None, ValueKind::Int, Some("1"), "Visualization status mode"),

            attr!("WaitAfterRestart", "Startup Delay After Reboot", Some("s"), ValueKind::DurationSeconds, Some("10"), "Wait time after Miniserver restart before arming"),
            attr!("CentralOut", "Central Output Mode", None, ValueKind::Int, Some("3"), "Central alarm output routing mode"),
            attr!("Entries", "History Entries Limit", None, ValueKind::Int, Some("40"), "Maximum number of event log entries"),
            attr!("Objects", "Linked Objects", None, ValueKind::Text, None, "UUIDs of linked functional blocks or periphery devices"),
            attr!("MCzones", "Music Zones Mask", None, ValueKind::Text, Some("01"), "Linked audio server alarm zones"),
            attr!("MServer", "Music Server Reference", None, ValueKind::UuidRef, None, "Referenced Audio Server / Music Server UUID"),
            attr!("MZoneNr", "Music Zone Number", None, ValueKind::Text, Some("01"), "Target audio zone number"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("48"), "Number of connector pins on block"),
            attr!("SType", "Server / Source Type", None, ValueKind::Int, Some("02"), "Audio or alarm source type"),
        ],
        &[
            param!("Ta", "Arming delay", Some("s"), ValueKind::DurationSeconds, "30", "Exit delay before system arms"),
            param!("Td", "Trigger delay", Some("s"), ValueKind::DurationSeconds, "15", "Entry delay before alarm stage 1"),
            param!("T1", "Stage 1 duration", Some("s"), ValueKind::DurationSeconds, "10", "Silent alarm duration"),
            param!("T2", "Stage 2 duration", Some("s"), ValueKind::DurationSeconds, "20", "Acoustic alarm duration"),
            param!("T3", "Stage 3 duration", Some("s"), ValueKind::DurationSeconds, "300", "Optical alarm duration"),
            param!("T4", "Stage 4 duration", Some("s"), ValueKind::DurationSeconds, "300", "External siren duration"),
            param!("T5", "Stage 5 duration", Some("s"), ValueKind::DurationSeconds, "300", "Escalation stage 5 duration"),
            param!("Ti", "Siren pulse interval", Some("s"), ValueKind::DurationSeconds, "2", "Pulse interval for intermittent output"),
            param!("M", "Operation mode", None, ValueKind::Int, "1", "Default alarm operating mode"),

            param!("Delay", "Arming / exit delay", Some("s"), ValueKind::DurationSeconds, "600", "Exit delay before burglar alarm arms"),
            param!("MaxDurOut", "Max alarm output duration", Some("s"), ValueKind::DurationSeconds, "900", "Maximum active duration for alarm outputs"),
            param!("TQ1", "Stage 1 delay", Some("s"), ValueKind::DurationSeconds, "20", "Delay before alarm stage 1"),
            param!("TQ2", "Stage 2 delay", Some("s"), ValueKind::DurationSeconds, "20", "Delay before alarm stage 2"),
            param!("TQ3", "Stage 3 delay", Some("s"), ValueKind::DurationSeconds, "40", "Delay before alarm stage 3"),
            param!("TQ4", "Stage 4 delay", Some("s"), ValueKind::DurationSeconds, "90", "Delay before alarm stage 4"),
            param!("TQ5", "Stage 5 delay", Some("s"), ValueKind::DurationSeconds, "150", "Delay before alarm stage 5"),
            param!("TQ6", "Stage 6 delay", Some("s"), ValueKind::DurationSeconds, "300", "Delay before alarm stage 6"),
            param!("WI2", "Re-trigger window", Some("s"), ValueKind::DurationSeconds, "900", "Time window for repeated alarm trigger"),
        ],
        &[
            pin_in!("En", "Arm", "Arms the burglar alarm"),
            pin_in!("Ep", "Arm with Presence", "Arms the alarm with interior motion detectors disabled"),
            pin_in!("Da", "Delayed Arm", "Arms after exit delay Ta"),
            pin_in!("Dap", "Delayed Arm Presence", "Arms with presence after exit delay Ta"),
            pin_in!("Dis", "Disarm", "Disarms the alarm and acknowledges active alerts"),
            pin_in!("Q", "Acknowledge", "Acknowledges active alarm outputs without disarming"),
            pin_in!("I", "Sensor Input", "Door/window contact or Tamper input"),
            pin_in!("M", "Motion Input", "Interior motion sensor input"),
            pin_out!("O", "Alarm Stage Output", "Staged alarm outputs O1..O8"),
            pin_out!("Qa", "Armed Status", "High when alarm is armed"),
            pin_out!("Qd", "Delayed Arm Status", "High during exit countdown"),
            pin_out!("Qam", "Armed Mode", "Current arming mode"),
            pin_out!("Al", "Alarm Level", "Active alarm escalation level"),
            pin_out!("At", "Alarm Timestamp", "Time of last alarm trigger"),

            pin_in!("HI1", "HI1", "Input pin HI1"),
            pin_out!("AQ", "AQ", "Output pin AQ"),
            pin_out!("AQr", "AQr", "Output pin AQr"),
            pin_out!("Active", "Active", "Output pin Active"),
            pin_out!("ActiveDelay", "ActiveDelay", "Output pin ActiveDelay"),
            pin_out!("ActiveDelayP", "ActiveDelayP", "Output pin ActiveDelayP"),
            pin_out!("ActiveOO", "ActiveOO", "Output pin ActiveOO"),
            pin_out!("ActiveOOP", "ActiveOOP", "Output pin ActiveOOP"),
            pin_out!("ActiveP", "ActiveP", "Output pin ActiveP"),
            pin_in!("AutoConfirm", "AutoConfirm", "Input pin AutoConfirm"),
            pin_in!("Confirm", "Confirm", "Input pin Confirm"),
            pin_in!("DisMv", "DisMv", "Input pin DisMv"),
            pin_in!("IInit", "IInit", "Input pin IInit"),
            pin_in!("Inactive", "Inactive", "Input pin Inactive"),
            pin_in!("InputDisable", "InputDisable", "Input pin InputDisable"),
            pin_out!("OutTime", "OutTime", "Output pin OutTime"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_out!("QT", "QT", "Output pin QT"),
            pin_out!("Qo", "Qo", "Output pin Qo"),
            pin_in!("Remanence", "Remanence", "Input pin Remanence"),
            pin_in!("TI", "TI", "Input pin TI"),
            pin_in!("TM", "TM", "Input pin TM"),
            pin_out!("TQ", "TQ", "Output pin TQ"),
            pin_in!("TTS", "TTS", "Input pin TTS"),
        ],
        &[ChildConfigKind::IoData, ChildConfigKind::CoHist]
    ),
    schema!(
        "AlarmChain",
        "Alarm Sequence",
        "Security & Safety",
        "https://www.loxone.com/enen/kb/alarm-sequence/",
        "Escalates an alarm sequentially across up to 10 notification stages with configurable delay per stage.",
        &[
            attr!("V", "Version", None, ValueKind::Int, Some("11"), "Block version"),
            attr!("WF", "Visu Flags", None, ValueKind::Int, Some("1"), "Visualization flags"),

            attr!("AT", "Alarm Text Template", None, ValueKind::Text, None, "Notification message template for alarm escalation"),
            attr!("EST", "Emergency Service Text", None, ValueKind::Text, None, "Formatted emergency service notification text"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("26"), "Number of connector pins on block"),
        ],
        &[
            param!("T1", "Stage 1 duration", Some("s"), ValueKind::DurationSeconds, "60", "Duration of escalation stage 1"),
            param!("T2", "Stage 2 duration", Some("s"), ValueKind::DurationSeconds, "60", "Duration of escalation stage 2"),
            param!("T3", "Stage 3 duration", Some("s"), ValueKind::DurationSeconds, "60", "Duration of escalation stage 3"),

            param!("Time", "Stage escalation time", Some("s"), ValueKind::DurationSeconds, "30", "Duration of each alarm sequence stage"),
            param!("MaxRuns", "Max sequence repetitions", None, ValueKind::Int, "4", "Maximum number of escalation runs"),
        ],
        &[
            pin_in!("En", "Enable", "Starts the alarm escalation sequence"),
            pin_in!("Next", "Next Stage", "Advances immediately to the next stage"),
            pin_in!("Q", "Acknowledge", "Acknowledges and stops escalation"),
            pin_out!("O", "Stage Output", "Sequential escalation outputs O1..O10"),

            pin_in!("Level1", "Level1", "Input pin Level1"),
            pin_out!("Alarm", "Alarm", "Output pin Alarm"),
            pin_in!("Confirm", "Confirm", "Input pin Confirm"),
            pin_in!("Reset", "Reset", "Input pin Reset"),
            pin_out!("ActiveLevel", "ActiveLevel", "Output pin ActiveLevel"),
            pin_in!("ConfirmCause", "ConfirmCause", "Input pin ConfirmCause"),
            pin_in!("ConfirmTime", "ConfirmTime", "Input pin ConfirmTime"),
            pin_in!("InEmgService", "InEmgService", "Input pin InEmgService"),
            pin_in!("LastStart", "LastStart", "Input pin LastStart"),
            pin_in!("LastStop", "LastStop", "Input pin LastStop"),
            pin_out!("OutEmgService", "OutEmgService", "Output pin OutEmgService"),
            pin_in!("Text1", "Text1", "Input pin Text1"),
            pin_in!("Urgent", "Urgent", "Input pin Urgent"),
        ],
        &[ChildConfigKind::IoData]
    ),
    schema!(
        "AlarmClock",
        "Alarm Clock",
        "Times",
        "https://www.loxone.com/enen/kb/alarm-clock/",
        "Programmable alarm clock with snooze, night mode, pre-alarm ramping, and calendar integration.",
        &[
            attr!("AlarmTime", "Default Alarm Time", Some("s"), ValueKind::DurationSeconds, None, "Configured alarm time in seconds from midnight"),
            attr!("V", "Version", None, ValueKind::Int, Some("11"), "Block version"),

            attr!("Dev", "Hardware Device Reference", None, ValueKind::UuidRef, None, "Linked periphery device UUID"),
            attr!("LtE", "Last Edit Timestamp", None, ValueKind::Int, Some("478389556"), "Internal last modification timestamp"),
            attr!("N", "N", None, ValueKind::Text, Some("2"), "Loxone XML attribute N"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("26"), "Number of connector pins on block"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
        ],
        &[
            param!("Ta", "Buzzer duration", Some("s"), ValueKind::DurationSeconds, "60", "Duration of main alarm output"),
            param!("Ts", "Snooze time", Some("s"), ValueKind::DurationSeconds, "540", "Snooze interval in seconds"),
            param!("Tp", "Pre-alarm lead time", Some("s"), ValueKind::DurationSeconds, "600", "Lead time for pre-alarm output Qp"),

            param!("AlarmTime", "Alarm ringing duration", Some("s"), ValueKind::DurationSeconds, "120", "Duration of main alarm output"),
            param!("PrepTime", "Pre-alarm preparation time", Some("s"), ValueKind::DurationSeconds, "180", "Lead time before alarm for preparation output"),
            param!("SnoozeTime", "Snooze duration", Some("s"), ValueKind::DurationSeconds, "300", "Snooze interval in seconds"),
            param!("BrightAct", "Active display brightness", Some("%"), ValueKind::Percent, "1", "Nightlight brightness when active"),
            param!("BrightInact", "Inactive display brightness", Some("%"), ValueKind::Percent, "1", "Nightlight brightness when inactive"),
            param!("ParWakeAlarmSound", "Wake-up sound ID", None, ValueKind::Int, "1", "Selected wake-up alarm tone"),
            param!("ParWakeAlarmVolume", "Wake-up alarm volume", Some("%"), ValueKind::Percent, "100", "Maximum wake-up alarm volume"),
        ],
        &[
            pin_in!("Dis", "Disable", "Disables the alarm clock"),
            pin_in!("Sn", "Snooze", "Activates snooze when ringing"),
            pin_in!("R", "Off / Reset", "Turns off active alarm for today"),
            pin_in!("Ni", "Night", "Night mode input"),
            pin_out!("Q", "Alarm Output", "Main alarm output"),
            pin_out!("Qp", "Pre-Alarm Output", "Triggers Tp seconds before alarm"),
            pin_out!("Qn", "Night Active", "High when night mode is active"),
            pin_out!("T", "Next Alarm Time", "Next scheduled alarm time"),

            pin_in!("Snooze", "Snooze", "Input pin Snooze"),
            pin_in!("Bell", "Bell", "Input pin Bell"),
            pin_in!("Light", "Light", "Input pin Light"),
            pin_in!("Blinds", "Blinds", "Input pin Blinds"),
            pin_in!("Music", "Music", "Input pin Music"),
            pin_in!("Prepare", "Prepare", "Input pin Prepare"),
            pin_in!("TNext", "TNext", "Input pin TNext"),
            pin_out!("AQs", "AQs", "Output pin AQs"),
            pin_in!("Acknowledge", "Acknowledge", "Input pin Acknowledge"),
            pin_in!("Deactivate", "Deactivate", "Input pin Deactivate"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_in!("ParWakeAlarmSloping", "ParWakeAlarmSloping", "Input pin ParWakeAlarmSloping"),
            pin_out!("QMe", "QMe", "Output pin QMe"),
            pin_out!("QTa", "QTa", "Output pin QTa"),
            pin_out!("QTe", "QTe", "Output pin QTe"),
            pin_out!("QTna", "QTna", "Output pin QTna"),
            pin_out!("QTp", "QTp", "Output pin QTp"),
            pin_out!("Qa", "Qa", "Output pin Qa"),
            pin_out!("Qat", "Qat", "Output pin Qat"),
            pin_in!("Remanence", "Remanence", "Input pin Remanence"),
            pin_in!("Reset", "Reset", "Input pin Reset"),
            pin_in!("RtD", "RtD", "Input pin RtD"),
            pin_in!("TMe", "TMe", "Input pin TMe"),
            pin_in!("TgMe", "TgMe", "Input pin TgMe"),
            pin_in!("VIN1", "VIN1", "Input pin VIN1"),
        ],
        &[ChildConfigKind::IoData, ChildConfigKind::CoHist]
    ),
    schema!(
        "And",
        "AND Logic",
        "Logic",
        "https://www.loxone.com/enen/kb/and-logic/",
        "Outputs 1 when all connected digital inputs I1..I8 are high (1).",
        &[
            attr!("I", "Connected Inputs Mask", None, ValueKind::Int, Some("3"), "Bitmask of active inputs"),
            attr!("N", "Inverted Inputs Mask", None, ValueKind::Int, Some("0"), "Bitmask of negated inputs"),

            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("3"), "Number of connector pins on block"),
        ],
        &[],
        &[
            pin_in!("I", "Input", "Digital logic input I1..I8"),
            pin_out!("Q", "Output", "1 when all active inputs are 1"),
            pin_out!("Qn", "Inverted Output", "1 when at least one active input is 0"),
        ],
        &[]
    ),
    schema!(
        "AutoJalousie",
        "Automatic Blinds",
        "Shading",
        "https://www.loxone.com/enen/kb/automatic-blinds/",
        "Controls Venetian blinds, roller shutters, awnings, and curtains including sun-position slat tracking and safety lockout.",
        &[
            attr!("An", "Analog Target Mode", None, ValueKind::Bool, Some("false"), "Enable analog position input"),
            attr!("Mo", "Shading Type", None, ValueKind::Int, Some("0"), "0=Venetian blind, 1=Roller shutter, 2=Curtain, 6=Awning"),
            attr!("Op", "Option Flags", None, ValueKind::Int, None, "Shading behavior flags"),
            attr!("Rad", "Location Coordinates", None, ValueKind::Bool, Some("true"), "Use project sun coordinates"),

            attr!("LtE", "Last Edit Timestamp", None, ValueKind::Int, Some("483124543"), "Internal last modification timestamp"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("49"), "Number of connector pins on block"),
            attr!("Ref", "Referenced Device UUID", None, ValueKind::UuidRef, None, "UUID of linked periphery device or service"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
            attr!("Sun", "Sun Position Automation", None, ValueKind::Bool, Some("true"), "Enable solar elevation and azimuth calculation"),
        ],
        &[
            param!("Tu", "Upward run time", Some("s"), ValueKind::DurationSeconds, "30", "Full travel duration upwards"),
            param!("Td", "Downward run time", Some("s"), ValueKind::DurationSeconds, "30", "Full travel duration downwards"),
            param!("TimeEnd", "Upward runtime (XML)", Some("s"), ValueKind::DurationSeconds, "36", "Full runtime moving up"),
            param!("TimeEndDown", "Downward runtime (XML)", Some("s"), ValueKind::DurationSeconds, "34", "Full runtime moving down"),
            param!("TurnOffset", "Slat turn time", Some("s"), ValueKind::DurationSeconds, "1.1", "Time for 100% slat rotation"),
            param!("Deadtime", "Motor reversing delay", Some("s"), ValueKind::DurationSeconds, "0.5", "Pause before changing direction"),
            param!("MinMove", "Min movement pulse", Some("s"), ValueKind::DurationSeconds, "0.3", "Minimum motor actuation time"),
            param!("Back", "Slat back-step", Some("%"), ValueKind::Percent, "0", "Slat angle after downward travel"),
            param!("MinPulse", "Short-click threshold", Some("s"), ValueKind::DurationSeconds, "0.3", "Threshold for step vs full travel"),
            param!("DblClk", "Double-click window", Some("s"), ValueKind::DurationSeconds, "0.6", "Window for double-click detection"),
            param!("Type", "Blind mechanism type", None, ValueKind::Int, "0", "0=Venetian, 1=Roller shutter, 6=Awning"),
            param!("Dir", "Facade compass orientation", Some("°"), ValueKind::Float, "180", "Window orientation (0=N, 90=E, 180=S, 270=W)"),
            param!("DirTol", "Sun entry angle offset", Some("°"), ValueKind::Float, "85", "Left azimuth offset from facade normal"),
            param!("DirTol2", "Sun exit angle offset", Some("°"), ValueKind::Float, "85", "Right azimuth offset from facade normal"),
            param!("Width", "Slat width", Some("mm"), ValueKind::Float, "80", "Venetian slat width"),
            param!("Space", "Slat spacing", Some("mm"), ValueKind::Float, "70", "Vertical spacing between slats"),
            param!("Rdd", "Auto-shade re-enable delay", Some("s"), ValueKind::DurationSeconds, "900", "Delay after sun leaves"),
            param!("TimeBlock", "Manual override lock time", Some("s"), ValueKind::DurationSeconds, "0", "Block auto-shading after manual move"),
            param!("WP", "Window open position", Some("%"), ValueKind::Percent, "0", "Target position when window opens"),
            param!("AutoShadeTime", "Shade activation delay", Some("s"), ValueKind::DurationSeconds, "0", "Delay before automatic shading starts"),
            param!("AutoShadeEnd", "End of shading position", Some("%"), ValueKind::Percent, "0", "Position after sun shading ends"),
            param!("AutMode", "Automatic mode config", None, ValueKind::Int, "1", "Automatic shading behavior"),
            param!("SRoff", "Sunrise offset", Some("min"), ValueKind::DurationMinutes, "0", "Offset from sunrise"),
            param!("SSoff", "Sunset offset", Some("min"), ValueKind::DurationMinutes, "0", "Offset from sunset"),
        ],
        &[
            pin_in!("Ui", "Up / Down Touch", "Combined single-button Up/Down input"),
            pin_in!("Up", "Up", "Manual upward movement"),
            pin_in!("Do", "Down", "Manual downward movement"),
            pin_in!("Cu", "Complete Up", "Drive completely open"),
            pin_in!("Cd", "Complete Down", "Drive completely closed"),
            pin_in!("S", "Shade", "Activate sun shading position"),
            pin_in!("So", "Sun Automation", "Enable automatic sun shading"),
            pin_in!("Sp", "Safety Lock", "Safety shutdown (wind/frost/fire); drives full up and locks"),
            pin_in!("Dis", "Disable", "Locks the blind in current position"),
            pin_in!("W", "Window Contact", "Window open/tilted contact"),
            pin_out!("Qu", "Output Up", "Relay output for upward motor direction"),
            pin_out!("Qd", "Output Down", "Relay output for downward motor direction"),
            pin_out!("Po", "Position", "Current blind position (0..100%)"),
            pin_out!("Sl", "Slat Position", "Current slat angle (0..100%)"),
            pin_out!("Qs", "Auto Shading Active", "High when automatic sun shading is active"),

            pin_in!("AutoShade", "AutoShade", "Input pin AutoShade"),
            pin_in!("EnAutoShade", "EnAutoShade", "Input pin EnAutoShade"),
            pin_in!("EndDown", "EndDown", "Input pin EndDown"),
            pin_in!("EndUp", "EndUp", "Input pin EndUp"),
            pin_in!("Gesture", "Gesture", "Input pin Gesture"),
            pin_in!("InputDisable", "InputDisable", "Input pin InputDisable"),
            pin_in!("InputTrigger", "InputTrigger", "Input pin InputTrigger"),
            pin_in!("InputTriggerDown", "InputTriggerDown", "Input pin InputTriggerDown"),
            pin_in!("InputTriggerUp", "InputTriggerUp", "Input pin InputTriggerUp"),
            pin_in!("ManualLamelle", "ManualLamelle", "Input pin ManualLamelle"),
            pin_in!("ManualPosition", "ManualPosition", "Input pin ManualPosition"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_out!("OutputAutoShade", "OutputAutoShade", "Output pin OutputAutoShade"),
            pin_out!("OutputCombined", "OutputCombined", "Output pin OutputCombined"),
            pin_out!("OutputDown", "OutputDown", "Output pin OutputDown"),
            pin_out!("OutputLPos", "OutputLPos", "Output pin OutputLPos"),
            pin_out!("OutputLock", "OutputLock", "Output pin OutputLock"),
            pin_out!("OutputPos", "OutputPos", "Output pin OutputPos"),
            pin_out!("OutputSafety", "OutputSafety", "Output pin OutputSafety"),
            pin_out!("OutputUp", "OutputUp", "Output pin OutputUp"),
            pin_in!("ReactAutoShade", "ReactAutoShade", "Input pin ReactAutoShade"),
            pin_in!("SO", "SO", "Input pin SO"),
            pin_in!("Safety", "Safety", "Input pin Safety"),
            pin_in!("Shade", "Shade", "Input pin Shade"),
            pin_in!("Stop", "Stop", "Input pin Stop"),
            pin_in!("TargetPos", "TargetPos", "Input pin TargetPos"),
            pin_in!("Window", "Window", "Input pin Window"),
        ],
        &[ChildConfigKind::IoData, ChildConfigKind::Psd, ChildConfigKind::CoHist]
    ),
];

static PAGE_SCHEMAS_2: &[BlockSchema] = &[
    schema!(
        "CentralLight",
        "Central Lighting Controller",
        "Central",
        "https://www.loxone.com/enen/kb/central/",
        "Aggregates and controls multiple Lighting Controller blocks simultaneously with selective room/controller masks.",
        &[
            attr!("UO", "Controlled Blocks", None, ValueKind::Text, None, "Comma-separated UUIDs of linked LightController2 blocks"),

            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("24"), "Number of connector pins on block"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
            attr!("rec", "Controlled Blocks List", None, ValueKind::Text, None, "Comma-separated UUIDs of blocks controlled by Central block"),
        ],
        &[],
        &[
            pin_in!("T", "Touch", "Central touch input (toggles linked lighting controllers)"),
            pin_in!("O", "On", "Turns all linked lighting controllers on"),
            pin_in!("R", "Reset / Off", "Turns all linked lighting controllers off"),
            pin_in!("S", "Scene", "Selects scene number across linked controllers"),
            pin_in!("Dis", "Selective Disable", "Disables central commands for specific linked block Dis1..Dis16"),
            pin_out!("Q", "Status Output", "High when any linked lighting controller is active"),

            pin_out!("Alarm", "Alarm", "Output pin Alarm"),
            pin_out!("AlarmClock", "AlarmClock", "Output pin AlarmClock"),
            pin_in!("EnMove", "EnMove", "Input pin EnMove"),
            pin_in!("I1", "I1", "Input pin I1"),
            pin_in!("InputDisable", "InputDisable", "Input pin InputDisable"),
            pin_in!("InputTriggerDown", "InputTriggerDown", "Input pin InputTriggerDown"),
            pin_in!("InputTriggerUp", "InputTriggerUp", "Input pin InputTriggerUp"),
            pin_in!("On", "On", "Input pin On"),
            pin_out!("OutActive", "OutActive", "Output pin OutActive"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_in!("Reset", "Reset", "Input pin Reset"),
            pin_in!("RtD", "RtD", "Input pin RtD"),
            pin_in!("Sel1", "Sel1", "Input pin Sel1"),
            pin_in!("Select", "Select", "Input pin Select"),
        ],
        &[ChildConfigKind::IoData, ChildConfigKind::CoHist]
    ),
    schema!(
        "CentralMusic",
        "Central Audio",
        "Central",
        "https://www.loxone.com/enen/kb/central/",
        "Controls multiple Music Player / Audio zones simultaneously for central pause, off, or alarm mute.",
        &[
            attr!("UO", "Controlled Players", None, ValueKind::Text, None, "Comma-separated UUIDs of linked MusicPlayer blocks"),

            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("36"), "Number of connector pins on block"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
            attr!("plid", "Player Zone ID", None, ValueKind::Text, Some("01"), "Audio player zone identifier"),
            attr!("rec", "Controlled Blocks List", None, ValueKind::Text, None, "Comma-separated UUIDs of blocks controlled by Central block"),
        ],
        &[
            param!("DblClk", "Double-click threshold", Some("s"), ValueKind::DurationSeconds, "0.35", "Double-click recognition window"),
        ],
        &[
            pin_in!("Play", "Play", "Starts playback on linked zones"),
            pin_in!("Pause", "Pause", "Pauses playback on linked zones"),
            pin_in!("R", "Off", "Turns off all linked audio zones"),
            pin_out!("Q", "Active Status", "High when any linked player is active"),

            pin_out!("Alarm", "Alarm", "Output pin Alarm"),
            pin_in!("Bell", "Bell", "Input pin Bell"),
            pin_in!("Buzzer", "Buzzer", "Input pin Buzzer"),
            pin_in!("Cs", "Cs", "Input pin Cs"),
            pin_in!("DisMv", "DisMv", "Input pin DisMv"),
            pin_in!("Firealarm", "Firealarm", "Input pin Firealarm"),
            pin_in!("Gesture", "Gesture", "Input pin Gesture"),
            pin_in!("IgnoreROff", "IgnoreROff", "Input pin IgnoreROff"),
            pin_in!("InputDisable", "InputDisable", "Input pin InputDisable"),
            pin_in!("Mute", "Mute", "Input pin Mute"),
            pin_in!("NextSong", "NextSong", "Input pin NextSong"),
            pin_in!("NextSource", "NextSource", "Input pin NextSource"),
            pin_out!("OutActive", "OutActive", "Output pin OutActive"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_in!("PowerOff", "PowerOff", "Input pin PowerOff"),
            pin_in!("PowerOn", "PowerOn", "Input pin PowerOn"),
            pin_in!("PrevSong", "PrevSong", "Input pin PrevSong"),
            pin_in!("Progress", "Progress", "Input pin Progress"),
            pin_in!("Repeat", "Repeat", "Input pin Repeat"),
            pin_in!("Reset", "Reset", "Input pin Reset"),
            pin_in!("RtD", "RtD", "Input pin RtD"),
            pin_in!("Shuffle", "Shuffle", "Input pin Shuffle"),
            pin_in!("Sleeptimer", "Sleeptimer", "Input pin Sleeptimer"),
            pin_in!("Source", "Source", "Input pin Source"),
            pin_in!("Stop", "Stop", "Input pin Stop"),
            pin_in!("SvPower", "SvPower", "Input pin SvPower"),
            pin_in!("TTS", "TTS", "Input pin TTS"),
            pin_in!("Trigger", "Trigger", "Input pin Trigger"),
            pin_in!("VolMinus", "VolMinus", "Input pin VolMinus"),
            pin_in!("VolPlus", "VolPlus", "Input pin VolPlus"),
            pin_in!("Volume", "Volume", "Input pin Volume"),
            pin_in!("voldown", "voldown", "Input pin voldown"),
            pin_in!("volup", "volup", "Input pin volup"),
        ],
        &[ChildConfigKind::IoData]
    ),
    schema!(
        "CentralShade",
        "Central Shading",
        "Central",
        "https://www.loxone.com/enen/kb/central/",
        "Controls multiple Automatic Blinds blocks simultaneously for central up, down, sun shading, or storm protection.",
        &[
            attr!("UO", "Controlled Blinds", None, ValueKind::Text, None, "Comma-separated UUIDs of linked AutoJalousie blocks"),

            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("18"), "Number of connector pins on block"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
            attr!("rec", "Controlled Blocks List", None, ValueKind::Text, None, "Comma-separated UUIDs of blocks controlled by Central block"),
        ],
        &[],
        &[
            pin_in!("Ui", "Central Up/Down", "Single-button central shading control"),
            pin_in!("Cu", "Complete Up", "Drives all linked blinds completely up"),
            pin_in!("Cd", "Complete Down", "Drives all linked blinds completely down"),
            pin_in!("S", "Shade", "Activates shading across all linked blinds"),
            pin_in!("So", "Sun Automation", "Enables automatic sun shading on linked blinds"),
            pin_in!("Sp", "Safety Shutdown", "Locks all linked blinds in safety position"),
            pin_in!("Dis", "Selective Disable", "Excludes specific blind Dis1..Dis16 from central commands"),
            pin_out!("Po", "Average Position", "Average blind position across linked blinds"),

            pin_in!("AutoShade", "AutoShade", "Input pin AutoShade"),
            pin_in!("EnAutoShade", "EnAutoShade", "Input pin EnAutoShade"),
            pin_in!("EndDown", "EndDown", "Input pin EndDown"),
            pin_in!("EndUp", "EndUp", "Input pin EndUp"),
            pin_in!("Gesture", "Gesture", "Input pin Gesture"),
            pin_in!("InputDisable", "InputDisable", "Input pin InputDisable"),
            pin_in!("InputTrigger", "InputTrigger", "Input pin InputTrigger"),
            pin_in!("InputTriggerDown", "InputTriggerDown", "Input pin InputTriggerDown"),
            pin_in!("InputTriggerUp", "InputTriggerUp", "Input pin InputTriggerUp"),
            pin_in!("ManualLamelle", "ManualLamelle", "Input pin ManualLamelle"),
            pin_in!("ManualPosition", "ManualPosition", "Input pin ManualPosition"),
            pin_out!("OutClose", "OutClose", "Output pin OutClose"),
            pin_out!("OutOpen", "OutOpen", "Output pin OutOpen"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_in!("ReactAutoShade", "ReactAutoShade", "Input pin ReactAutoShade"),
            pin_in!("Safety", "Safety", "Input pin Safety"),
            pin_in!("Shade", "Shade", "Input pin Shade"),
            pin_in!("Stop", "Stop", "Input pin Stop"),
        ],
        &[ChildConfigKind::IoData, ChildConfigKind::CoHist]
    ),
    schema!(
        "ClimateControllerUS",
        "Climate Controller",
        "Climate",
        "https://www.loxone.com/enen/kb/climate-controller/",
        "Coordinates building heating and cooling demand across multiple Intelligent Room Controllers.",
        &[
            attr!("UO", "Linked Room Controllers", None, ValueKind::Text, None, "Comma-separated UUIDs of linked HeatIRoomController2 blocks"),

            attr!("DType", "Demand Calculation Mode", None, ValueKind::Int, Some("2"), "Heating/cooling demand aggregation mode"),
            attr!("cCap", "Climate Capabilities", None, ValueKind::Int, Some("3"), "Supported heating and cooling capabilities"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("32"), "Number of connector pins on block"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
        ],
        &[
            param!("T", "Switchover delay", Some("s"), ValueKind::DurationSeconds, "300", "Minimum delay before switching heating/cooling"),

            param!("FanDelay", "Fan activation delay", Some("s"), ValueKind::DurationSeconds, "90", "Delay before fan stage engages"),
            param!("HTarget", "Target humidity", Some("%"), ValueKind::Percent, "45", "Target relative humidity setpoint"),
            param!("OnThreshold", "Demand activation threshold", Some("%"), ValueKind::Percent, "30", "Minimum valve opening to request heating/cooling"),
            param!("TOm", "Operating mode switch delay", Some("min"), ValueKind::DurationMinutes, "3", "Minimum switch delay"),
            param!("TPMax", "Max dew-point / supply temp", Some("°C"), ValueKind::TemperatureCelsius, "19", "Maximum temperature threshold"),
            param!("TPMin", "Min cooling supply temp", Some("°C"), ValueKind::TemperatureCelsius, "12", "Minimum cooling temperature"),
            param!("Temp", "Outdoor temperature override", Some("°C"), ValueKind::TemperatureCelsius, "-1000", "Outdoor temperature input/default"),
            param!("TempDelta", "Heating/cooling deadband", Some("°C"), ValueKind::TemperatureCelsius, "2", "Temperature deadband between modes"),
            param!("TempLimitC", "Cooling outdoor cutoff", Some("°C"), ValueKind::TemperatureCelsius, "15", "Outdoor temperature above which cooling is enabled"),
            param!("TempLimitH", "Heating outdoor cutoff", Some("°C"), ValueKind::TemperatureCelsius, "18", "Outdoor temperature below which heating is enabled"),
            param!("TimeAddional", "Additional stage delay", Some("s"), ValueKind::DurationSeconds, "300", "Delay before activating auxiliary stage"),
            param!("TimeOffPump", "Pump overrun time", Some("s"), ValueKind::DurationSeconds, "300", "Circulation pump post-run time"),
        ],
        &[
            pin_in!("En", "Enable", "Enables climate demand aggregation"),
            pin_in!("To", "Outdoor Temperature", "Outdoor ambient temperature (°C)"),
            pin_out!("H", "Heating Demand", "Active heating demand output"),
            pin_out!("C", "Cooling Demand", "Active cooling demand output"),
            pin_out!("Tf", "Flow Temperature Setpoint", "Required supply flow temperature (°C)"),

            pin_in!("Boost", "Boost", "Input pin Boost"),
            pin_in!("CoolAvailable", "CoolAvailable", "Input pin CoolAvailable"),
            pin_in!("DirV", "DirV", "Input pin DirV"),
            pin_in!("Emergency", "Emergency", "Input pin Emergency"),
            pin_in!("Fan", "Fan", "Input pin Fan"),
            pin_in!("HeatAvailable", "HeatAvailable", "Input pin HeatAvailable"),
            pin_in!("InHumid", "InHumid", "Input pin InHumid"),
            pin_in!("Mode", "Mode", "Input pin Mode"),
            pin_out!("OutCompressor", "OutCompressor", "Output pin OutCompressor"),
            pin_out!("OutCool2", "OutCool2", "Output pin OutCool2"),
            pin_out!("OutEmergency", "OutEmergency", "Output pin OutEmergency"),
            pin_out!("OutFan", "OutFan", "Output pin OutFan"),
            pin_out!("OutHeat1", "OutHeat1", "Output pin OutHeat1"),
            pin_out!("OutHumid", "OutHumid", "Output pin OutHumid"),
            pin_out!("OutValve", "OutValve", "Output pin OutValve"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_in!("ServiceMode", "ServiceMode", "Input pin ServiceMode"),
            pin_in!("Stop", "Stop", "Input pin Stop"),
            pin_in!("TPMinH", "TPMinH", "Input pin TPMinH"),
        ],
        &[ChildConfigKind::IoData, ChildConfigKind::CoHist]
    ),
    schema!(
        "Door",
        "Gate & Door Controller",
        "Access",
        "https://www.loxone.com/enen/kb/gate-controller/",
        "Controls motorized gates, garage doors, and barriers with opening/closing relays, limit switches, and pulse/run times.",
        &[
            attr!("CtrlType", "Drive Controller Type", None, ValueKind::Int, Some("1"), "Gate/door actuator drive mode"),
            attr!("T5Side", "T5 Input Side", None, ValueKind::Int, Some("0"), "T5 button orientation"),

            attr!("LtE", "Last Edit Timestamp", None, ValueKind::Int, Some("488609646"), "Internal last modification timestamp"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("28"), "Number of connector pins on block"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
        ],
        &[
            param!("PulseOpen", "Open Pulse Duration", Some("s"), ValueKind::DurationSeconds, "0.5", "Relay pulse duration for open command"),
            param!("PulseClose", "Close Pulse Duration", Some("s"), ValueKind::DurationSeconds, "0.5", "Relay pulse duration for close command"),
            param!("PulseHigh", "Pulse High Duration", Some("s"), ValueKind::DurationSeconds, "0.5", "High pulse duration"),
            param!("PulseLow", "Pulse Low Duration", Some("s"), ValueKind::DurationSeconds, "0.5", "Low pulse duration"),
            param!("TimeBlock", "Dead/Lockout Time", Some("s"), ValueKind::DurationSeconds, "1", "Motor direction reversal lockout delay"),
            param!("TimeHigh", "Opening Run Time", Some("s"), ValueKind::DurationSeconds, "20", "Full gate opening run time"),
            param!("TimeLow", "Closing Run Time", Some("s"), ValueKind::DurationSeconds, "20", "Full gate closing run time"),
            param!("PoPos", "Partial Open Position", Some("%"), ValueKind::Percent, "0", "Target partial opening position"),
        ],
        &[
            pin_in!("Trigger", "Trigger", "Single-button open/stop/close trigger"),
            pin_in!("Open", "Open", "Commands gate/door to open"),
            pin_in!("Close", "Close", "Commands gate/door to close"),
            pin_in!("InputStop", "Safety Stop", "Safety barrier or emergency stop input"),
            pin_in!("EndOpen", "Limit Switch Open", "End-stop contact indicating fully open"),
            pin_in!("EndClose", "Limit Switch Closed", "End-stop contact indicating fully closed"),
            pin_out!("OutputOpen", "Output Open", "Motor/relay open output"),
            pin_out!("OutputClose", "Output Close", "Motor/relay close output"),
            pin_out!("OutputPos", "Position Output", "Estimated door/gate position (0..100%)"),
            pin_out!("IsOpen", "Is Open", "High when gate/door is open"),
            pin_out!("IsClosed", "Is Closed", "High when gate/door is closed"),

            pin_in!("InputDisable", "InputDisable", "Input pin InputDisable"),
            pin_in!("InputPo", "InputPo", "Input pin InputPo"),
            pin_in!("Light", "Light", "Input pin Light"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_out!("OutputActive", "OutputActive", "Output pin OutputActive"),
            pin_out!("OutputTrigger", "OutputTrigger", "Output pin OutputTrigger"),
            pin_in!("Remanence", "Remanence", "Input pin Remanence"),
            pin_in!("T5", "T5", "Input pin T5"),
            pin_in!("Type", "Type", "Input pin Type"),
        ],
        &[ChildConfigKind::IoData, ChildConfigKind::CoHist]
    ),
    schema!(
        "Doorcontroller",
        "Access Door Controller",
        "Access",
        "https://www.loxone.com/enen/kb/door-controller/",
        "Controls door access strikes, intercom audio/video integration, bell output, and programmable door/light buttons.",
        &[
            attr!("DoorNumber", "Door Number", None, ValueKind::Int, Some("1"), "Assigned door controller number"),
            attr!("OutName1", "Output Q1 Label", None, ValueKind::Text, None, "UI button label for output Q1"),
            attr!("OutName2", "Output Q2 Label", None, ValueKind::Text, None, "UI button label for output Q2"),
            attr!("OutName3", "Output Q3 Label", None, ValueKind::Text, None, "UI button label for output Q3"),
            attr!("Ref", "Referenced Audio/Video Device", None, ValueKind::Text, None, "UUID of linked Intercom or audio zone"),

            attr!("LtE", "Last Edit Timestamp", None, ValueKind::Int, Some("487021189"), "Internal last modification timestamp"),
            attr!("MCzones", "Music Zones Mask", None, ValueKind::Text, Some("01"), "Linked audio server alarm zones"),
            attr!("MServer", "Music Server Reference", None, ValueKind::UuidRef, None, "Referenced Audio Server / Music Server UUID"),
            attr!("MZoneNr", "Music Zone Number", None, ValueKind::Text, Some("01"), "Target audio zone number"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("8"), "Number of connector pins on block"),
            attr!("Objects", "Linked Objects", None, ValueKind::Text, None, "UUIDs of linked functional blocks or periphery devices"),
            attr!("SType", "Server / Source Type", None, ValueKind::Int, Some("02"), "Audio or alarm source type"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
        ],
        &[
            param!("Timeout", "Ring Timeout", Some("s"), ValueKind::DurationSeconds, "60", "Bell/intercom ring timeout"),
        ],
        &[
            pin_in!("TriggerFan", "Ring / Bell Input", "Doorbell trigger input"),
            pin_in!("Reset", "Reset", "Ends active ring/call session"),
            pin_out!("Qb", "Bell Output", "Active bell / chime output"),
            pin_out!("Q1", "Output 1 (Door Opener)", "Programmable door strike output Q1"),
            pin_out!("Q2", "Output 2 (Light)", "Programmable light/aux output Q2"),
            pin_out!("Q3", "Output 3 (Aux)", "Programmable auxiliary output Q3"),

            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
        ],
        &[ChildConfigKind::IoData, ChildConfigKind::LoggerMailer]
    ),
    schema!(
        "EFM",
        "Energy Flow Monitor",
        "Energy",
        "https://www.loxone.com/enen/kb/energy-flow-monitor/",
        "Visualizes real-time power flow between grid, solar PV, battery storage, and household consumers.",
        &[
            attr!("UO", "Linked Meters", None, ValueKind::Text, None, "UUIDs of linked Meter blocks"),

            attr!("StatsType", "Statistics Recording Mode", None, ValueKind::Int, Some("8"), "Meter statistics interval and retention mode"),
            attr!("Un1", "Power / Flow Unit Format", None, ValueKind::Text, Some("<v.3>kW"), "Format string and unit for instantaneous power/flow"),
            attr!("Un2", "Energy / Volume Unit Format", None, ValueKind::Text, Some("<v.1>kWh"), "Format string and unit for cumulative reading"),
            attr!("UnSt", "Self-Consumption Unit Format", None, ValueKind::Text, Some("<v>%"), "Format string for percentage display"),
            attr!("LtE", "Last Edit Timestamp", None, ValueKind::Int, Some("486888452"), "Internal last modification timestamp"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("19"), "Number of connector pins on block"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
        ],
        &[
            param!("CO2", "CO2 emission factor", Some("kg/kWh"), ValueKind::Float, "0.42", "CO2 savings factor per kWh"),
            param!("Pre", "Export feed-in tariff", None, ValueKind::Float, "0.2", "Revenue per exported kWh"),
            param!("Pri", "Import electricity price", None, ValueKind::Float, "0.2", "Cost per imported kWh"),
        ],
        &[
            pin_in!("P", "Power Input", "Active power input (kW)"),
            pin_out!("Pg", "Grid Power", "Current grid import/export power (kW)"),
            pin_out!("Pc", "Consumption", "Total household power consumption (kW)"),

            pin_out!("OCd", "OCd", "Output pin OCd"),
            pin_out!("OCo2d", "OCo2d", "Output pin OCo2d"),
            pin_out!("OCpwr", "OCpwr", "Output pin OCpwr"),
            pin_out!("OEd", "OEd", "Output pin OEd"),
            pin_out!("OGpwr", "OGpwr", "Output pin OGpwr"),
            pin_out!("OId", "OId", "Output pin OId"),
            pin_out!("OPd", "OPd", "Output pin OPd"),
            pin_out!("OPpwr", "OPpwr", "Output pin OPpwr"),
            pin_out!("OSc", "OSc", "Output pin OSc"),
            pin_out!("OScd", "OScd", "Output pin OScd"),
            pin_out!("OSpwr", "OSpwr", "Output pin OSpwr"),
            pin_out!("OYd", "OYd", "Output pin OYd"),
            pin_out!("OYt", "OYt", "Output pin OYt"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_in!("R", "R", "Input pin R"),
            pin_in!("Rest", "Rest", "Input pin Rest"),
        ],
        &[ChildConfigKind::IoData]
    ),
    schema!(
        "Equal",
        "Equals Comparator",
        "Logic",
        "https://www.loxone.com/enen/kb/comparators/",
        "Outputs 1 when input V1 equals input V2.",
        &[
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("3"), "Number of connector pins on block"),
        ],
        &[
            param!("V1", "Value 1", None, ValueKind::Float, "0", "First comparison value"),
            param!("V2", "Value 2", None, ValueKind::Float, "0", "Second comparison value"),
        ],
        &[
            pin_in!("V1", "Value 1", "First analog value"),
            pin_in!("V2", "Value 2", "Second analog value"),
            pin_out!("Q", "Equal Output", "1 when V1 == V2"),
            pin_out!("Qn", "Not Equal Output", "1 when V1 != V2"),

            pin_in!("Input1", "Input1", "Input pin Input1"),
        ],
        &[]
    ),
    schema!(
        "HVACController",
        "HVAC Controller",
        "Climate",
        "https://www.loxone.com/enen/kb/hvac-controller/",
        "Controls heat pumps, boilers, chillers, and buffer tanks based on building heating and cooling requests.",
        &[
            attr!("cCap", "Climate Capabilities", None, ValueKind::Int, Some("3"), "Supported heating and cooling capabilities"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("38"), "Number of connector pins on block"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
        ],
        &[
            param!("To", "Outdoor cutoff temp", Some("°C"), ValueKind::TemperatureCelsius, "18", "Heating cutoff outdoor temperature"),

            param!("Average", "Smoothing average window", None, ValueKind::Int, "1", "Temperature averaging factor"),
            param!("FanDelay", "Fan delay", Some("s"), ValueKind::DurationSeconds, "120", "Fan startup delay"),
            param!("OnThreshold", "Activation threshold", Some("%"), ValueKind::Percent, "30", "Demand threshold to start HVAC source"),
            param!("Temp", "Outdoor temperature", Some("°C"), ValueKind::TemperatureCelsius, "-1000", "Outdoor temperature sensor value"),
            param!("TempAdditional", "Bivalent stage threshold", Some("°C"), ValueKind::TemperatureCelsius, "-6", "Outdoor temp below which auxiliary heater enables"),
            param!("TempDelta", "Temperature hysteresis", Some("°C"), ValueKind::TemperatureCelsius, "2", "Switching hysteresis"),
            param!("TempLimitC", "Cooling cutoff temperature", Some("°C"), ValueKind::TemperatureCelsius, "15", "Min outdoor temperature for cooling"),
            param!("TempLimitH", "Heating cutoff temperature", Some("°C"), ValueKind::TemperatureCelsius, "10", "Max outdoor temperature for heating"),
            param!("TempMin", "Min operating temperature", Some("°C"), ValueKind::TemperatureCelsius, "-22", "Low ambient safety cutout temperature"),
            param!("TimeAddional", "Auxiliary stage delay", Some("s"), ValueKind::DurationSeconds, "60", "Delay before engaging second stage"),
            param!("TimePulseOff", "Minimum off time", Some("s"), ValueKind::DurationSeconds, "300", "Compressor minimum off duration"),
            param!("TimePulseOn", "Minimum run time", Some("s"), ValueKind::DurationSeconds, "750", "Compressor minimum run duration"),
        ],
        &[
            pin_in!("H", "Heating Request", "Heating demand input"),
            pin_in!("C", "Cooling Request", "Cooling demand input"),
            pin_in!("To", "Outdoor Temp", "Outdoor temperature sensor (°C)"),
            pin_out!("Qh", "Heating Enable", "Enables heat source"),
            pin_out!("Qc", "Cooling Enable", "Enables cooling source"),

            pin_in!("Boost", "Boost", "Input pin Boost"),
            pin_in!("CoolAvailable", "CoolAvailable", "Input pin CoolAvailable"),
            pin_in!("Error", "Error", "Input pin Error"),
            pin_in!("Fan", "Fan", "Input pin Fan"),
            pin_in!("FilterAck", "FilterAck", "Input pin FilterAck"),
            pin_in!("HeatAvailable", "HeatAvailable", "Input pin HeatAvailable"),
            pin_in!("ManualHeat", "ManualHeat", "Input pin ManualHeat"),
            pin_in!("Mode", "Mode", "Input pin Mode"),
            pin_out!("OutAverageTemp", "OutAverageTemp", "Output pin OutAverageTemp"),
            pin_out!("OutCool1", "OutCool1", "Output pin OutCool1"),
            pin_out!("OutError", "OutError", "Output pin OutError"),
            pin_out!("OutFan", "OutFan", "Output pin OutFan"),
            pin_out!("OutFilter", "OutFilter", "Output pin OutFilter"),
            pin_out!("OutHeat1", "OutHeat1", "Output pin OutHeat1"),
            pin_out!("OutSecHeat", "OutSecHeat", "Output pin OutSecHeat"),
            pin_out!("OutValve", "OutValve", "Output pin OutValve"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_in!("PulseThreshold", "PulseThreshold", "Input pin PulseThreshold"),
            pin_in!("SecHeat", "SecHeat", "Input pin SecHeat"),
            pin_in!("ServiceMode", "ServiceMode", "Input pin ServiceMode"),
            pin_in!("Stop", "Stop", "Input pin Stop"),
            pin_in!("TempAvg", "TempAvg", "Input pin TempAvg"),
            pin_in!("TimeFilter", "TimeFilter", "Input pin TimeFilter"),
            pin_in!("TimePump", "TimePump", "Input pin TimePump"),
            pin_in!("ValveDelay", "ValveDelay", "Input pin ValveDelay"),
        ],
        &[ChildConfigKind::IoData]
    ),
    schema!(
        "HeatIRoomController2",
        "Intelligent Room Controller",
        "Climate",
        "https://www.loxone.com/enen/kb/intelligent-room-controller/",
        "Self-learning room temperature controller managing comfort/eco setpoints, heating/cooling valves, and schedules.",
        &[
            attr!("UseH", "Heating Target Offset", Some("°C"), ValueKind::TemperatureCelsius, Some("0"), "Heating temperature configuration / offset"),
            attr!("AQDig", "Digital Valve Outputs", None, ValueKind::Bool, Some("false"), "Use 2-point digital PWM instead of 0-100% analog"),
            attr!("DMtc", "Manual Mode Duration", Some("min"), ValueKind::DurationMinutes, Some("120"), "Default manual override duration"),
            attr!("DefFan", "Default Fan Speed", None, ValueKind::Int, Some("0"), "Default ventilation stage"),
            attr!("DefFanNP", "Unoccupied Fan Speed", None, ValueKind::Int, Some("0"), "Fan stage when no presence"),
            attr!("ExT", "Excess Temp Mode", None, ValueKind::Int, Some("0"), "Use excess solar/thermal energy"),
            attr!("Cap", "Heating/Cooling Capability", None, ValueKind::Int, Some("3"), "1=Heating, 2=Cooling, 3=Both"),
            attr!("SourceCap", "Heat Source Binding", None, ValueKind::Int, Some("1"), "Climate controller integration mode"),
            attr!("PrioH", "Heating Priority", None, ValueKind::Int, Some("5"), "Priority of heating source 1 vs 2"),
            attr!("PrioC", "Cooling Priority", None, ValueKind::Int, Some("5"), "Priority of cooling source 1 vs 2"),
            attr!("srcName", "Source Names", None, ValueKind::Text, None, "Custom actuator circuit labels"),

            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("57"), "Number of connector pins on block"),
            attr!("Objects", "Linked Objects", None, ValueKind::Text, None, "UUIDs of linked functional blocks or periphery devices"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
            attr!("UUIDTimer", "Schedule Timer UUID", None, ValueKind::UuidRef, None, "UUID of internal heating/cooling schedule timer"),
            attr!("custRI", "Custom Room Icon", None, ValueKind::Text, Some(""), "Custom visualization icon"),
            attr!("custRU", "Custom Room Unit", None, ValueKind::Text, Some(""), "Custom temperature display unit"),
        ],
        &[
            param!("Tc", "Comfort temperature", Some("°C"), ValueKind::TemperatureCelsius, "21.5", "Target comfort heating temperature"),
            param!("TComfort", "Comfort Heating Temp", Some("°C"), ValueKind::TemperatureCelsius, "21.5", "Comfort heating setpoint"),
            param!("TComfortC", "Comfort Cooling Temp", Some("°C"), ValueKind::TemperatureCelsius, "24.0", "Comfort cooling setpoint"),
            param!("TComfortHC", "Comfort Deadband", Some("°C"), ValueKind::TemperatureCelsius, "1.0", "Deadband between heating and cooling"),
            param!("TDiff", "Allowed Deviation", Some("°C"), ValueKind::TemperatureCelsius, "1.0", "Allowed comfort range bandwidth"),
            param!("TShadeHeat", "Shading Heat Threshold", Some("°C"), ValueKind::TemperatureCelsius, "22.5", "Enable sun shading above this temp"),
            param!("TShadeCool", "Shading Cool Threshold", Some("°C"), ValueKind::TemperatureCelsius, "23.0", "Enable cooling shading above this temp"),
            param!("TSaveL", "Frost Protection", Some("°C"), ValueKind::TemperatureCelsius, "10.0", "Minimum building protection temperature"),
            param!("TSaveU", "Overheat Protection", Some("°C"), ValueKind::TemperatureCelsius, "32.0", "Maximum building protection temperature"),
            param!("TSave", "Economy Offset", Some("°C"), ValueKind::TemperatureCelsius, "2.0", "Eco setback offset from comfort"),
            param!("TDeepSleep", "Deep Sleep Offset", Some("°C"), ValueKind::TemperatureCelsius, "4.0", "Deep setback offset"),
            param!("TMax", "Max Floor Temperature", Some("°C"), ValueKind::TemperatureCelsius, "29.0", "Floor screed temperature limit"),
            param!("TimeMove", "Presence Extend Time", Some("s"), ValueKind::DurationSeconds, "3600", "Comfort extension on presence"),
            param!("TimeC", "Manual Timer Duration", Some("min"), ValueKind::DurationMinutes, "120", "Duration of manual comfort timer"),
            param!("TimeS", "Eco Timer Duration", Some("min"), ValueKind::DurationMinutes, "240", "Duration of manual eco timer"),
            param!("TimeMv", "Valve Exercise Interval", Some("s"), ValueKind::DurationSeconds, "0", "Periodic valve protection cycle"),
            param!("THCelvin", "Stage 2 Heat Threshold", Some("°C"), ValueKind::TemperatureCelsius, "1.5", "Delta to activate stage 2 heating"),
            param!("TCCelvin", "Stage 2 Cool Threshold", Some("°C"), ValueKind::TemperatureCelsius, "1.5", "Delta to activate stage 2 cooling"),
            param!("TPWM", "PWM Cycle Period", Some("s"), ValueKind::DurationSeconds, "900", "Thermal actuator PWM cycle period"),
            param!("TWin", "Open Window Delay", Some("s"), ValueKind::DurationSeconds, "300", "Delay before window open pauses heating"),
            param!("TExcess", "Excess Heat Offset", Some("°C"), ValueKind::TemperatureCelsius, "1.5", "Allowed overheat during solar surplus"),

            param!("TempO", "Outdoor temperature", Some("°C"), ValueKind::TemperatureCelsius, "-1000", "Outdoor reference temperature"),
            param!("inAirDir", "Airflow direction mode", None, ValueKind::Int, "1", "Ventilation louver direction"),
            param!("inFan", "Fan speed stage", None, ValueKind::Int, "1", "Room fan speed stage"),
        ],
        &[
            pin_in!("Tr", "Room Temperature", "Measured actual room temperature (°C)"),
            pin_in!("Tf", "Floor Temperature", "Floor sensor temperature for screed protection (°C)"),
            pin_in!("W", "Window Contact", "Pauses heating/cooling while window is open"),
            pin_in!("Mo", "Presence", "Activates comfort temperature on occupancy"),
            pin_in!("Vac", "Vacation Mode", "Switches room to frost/building protection"),
            pin_in!("Dis", "Disable", "Disables controller outputs"),
            pin_out!("AQh1", "Heating Valve 1", "Primary heating valve actuator output (0..100%)"),
            pin_out!("AQh2", "Heating Valve 2", "Secondary heating valve actuator output (0..100%)"),
            pin_out!("AQc1", "Cooling Valve 1", "Primary cooling valve actuator output (0..100%)"),
            pin_out!("AQc2", "Cooling Valve 2", "Secondary cooling valve actuator output (0..100%)"),
            pin_out!("Tsoll", "Target Temperature", "Currently active target room temperature (°C)"),
            pin_out!("Oh", "Heating Active", "High when room requests heating"),
            pin_out!("Oc", "Cooling Active", "High when room requests cooling"),
            pin_out!("Qs", "Shading Request", "High when room requests solar shading to prevent overheating"),

            pin_in!("AMode", "AMode", "Input pin AMode"),
            pin_out!("AQc", "AQc", "Output pin AQc"),
            pin_out!("AQh", "AQh", "Output pin AQh"),
            pin_out!("AQhc", "AQhc", "Output pin AQhc"),
            pin_out!("AQhm", "AQhm", "Output pin AQhm"),
            pin_out!("AQs", "AQs", "Output pin AQs"),
            pin_out!("AQt", "AQt", "Output pin AQt"),
            pin_out!("AQtm", "AQtm", "Output pin AQtm"),
            pin_in!("Comfort", "Comfort", "Input pin Comfort"),
            pin_in!("DisMv", "DisMv", "Input pin DisMv"),
            pin_in!("InCo2", "InCo2", "Input pin InCo2"),
            pin_in!("InHumid", "InHumid", "Input pin InHumid"),
            pin_in!("Input", "Input", "Input pin Input"),
            pin_in!("Move", "Move", "Input pin Move"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_out!("Qa", "Qa", "Output pin Qa"),
            pin_out!("Qb", "Qb", "Output pin Qb"),
            pin_out!("Qe", "Qe", "Output pin Qe"),
            pin_in!("Reset", "Reset", "Input pin Reset"),
            pin_in!("RtD", "RtD", "Input pin RtD"),
            pin_in!("Save", "Save", "Input pin Save"),
            pin_in!("Temp", "Temp", "Input pin Temp"),
            pin_in!("Window", "Window", "Input pin Window"),
        ],
        &[ChildConfigKind::IoData, ChildConfigKind::CoHist]
    ),
    schema!(
        "Heatmixer2",
        "Mixing Valve Controller",
        "Climate",
        "https://www.loxone.com/enen/kb/mixing-valve-controller/",
        "PI controller regulating a 3-way motorised mixing valve and circulation pump to maintain target flow temperature.",
        &[
            attr!("ControllerMode", "Mixing Valve Mode", None, ValueKind::Int, Some("1"), "Operating mode (Heating, Cooling, Return temperature)"),
            attr!("Cool", "Cooling Behaviour", None, ValueKind::Int, Some("2"), "Mixer behavior during cooling operation"),
            attr!("HVAC", "Linked HVAC Controller", None, ValueKind::UuidRef, None, "UUID of linked HVAC / Climate Controller"),
            attr!("Irr", "Linked Room Controllers", None, ValueKind::Text, None, "UUIDs of linked Intelligent Room Controllers"),
            attr!("Room", "Assigned Rooms", None, ValueKind::Text, None, "UUIDs of rooms served by this heating circuit"),
            attr!("UseHVAc", "Use HVAC Request", None, ValueKind::Int, Some("1"), "Follow heating/cooling mode from linked controller"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("23"), "Number of connector pins on block"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
        ],
        &[
            param!("Runtime", "Actuator run time", Some("s"), ValueKind::DurationSeconds, "120", "Full 0..100% travel time of motorised valve"),
            param!("Min", "Min flow temperature", Some("°C"), ValueKind::TemperatureCelsius, "20", "Minimum allowed supply flow temperature"),
            param!("Max", "Max flow temperature", Some("°C"), ValueKind::TemperatureCelsius, "45", "Maximum allowed supply flow temperature"),
            param!("MaxOff", "Overtemp shutdown", Some("°C"), ValueKind::TemperatureCelsius, "55", "Safety shutoff temperature for pump"),
            param!("Inter", "Control interval", Some("s"), ValueKind::DurationSeconds, "20", "PI sampling and adjustment interval"),
            param!("Gain", "Proportional gain Kp", None, ValueKind::Float, "1.0", "PI proportional gain factor"),
            param!("Tint", "Integral time Ti", Some("s"), ValueKind::DurationSeconds, "120", "PI integral reset time"),

            param!("BufferInc", "Buffer temperature boost", Some("°C"), ValueKind::TemperatureCelsius, "5", "Buffer target offset above flow setpoint"),
            param!("Outer", "Outdoor temperature", Some("°C"), ValueKind::TemperatureCelsius, "-1000", "Outdoor ambient temperature"),
            param!("PrevInc", "Pre-run boost offset", Some("°C"), ValueKind::TemperatureCelsius, "2", "Initial flow temperature boost"),
            param!("TMax", "Max flow temperature", Some("°C"), ValueKind::TemperatureCelsius, "40", "Maximum supply flow temperature"),
            param!("TMin", "Min flow temperature", Some("°C"), ValueKind::TemperatureCelsius, "5", "Minimum supply flow temperature"),
            param!("Transconductance", "Heating curve slope", None, ValueKind::Float, "0.5", "Weather-compensation curve steepness"),
            param!("Treshold", "Overtemp threshold", Some("°C"), ValueKind::TemperatureCelsius, "35", "Threshold temperature"),
        ],
        &[
            pin_in!("T", "Target Flow Temp", "Desired supply flow temperature (°C)"),
            pin_in!("ti", "Actual Flow Temp", "Measured supply flow temperature sensor (°C)"),
            pin_in!("Dis", "Disable", "Disables mixer and closes valve"),
            pin_out!("Qu", "Mixer Open", "Relay output driving mixing valve open (warmer)"),
            pin_out!("Qd", "Mixer Close", "Relay output driving mixing valve closed (cooler)"),
            pin_out!("Qp", "Circulation Pump", "Enables heating circuit circulation pump"),
            pin_out!("AQ", "Analog Valve Pos", "Calculated 0..100% mixing valve position"),

            pin_out!("AQb", "AQb", "Output pin AQb"),
            pin_out!("AQf", "AQf", "Output pin AQf"),
            pin_out!("AQi", "AQi", "Output pin AQi"),
            pin_out!("AQl", "AQl", "Output pin AQl"),
            pin_out!("AQr", "AQr", "Output pin AQr"),
            pin_out!("AQt", "AQt", "Output pin AQt"),
            pin_in!("Boost", "Boost", "Input pin Boost"),
            pin_in!("InBuffer", "InBuffer", "Input pin InBuffer"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_out!("Qe", "Qe", "Output pin Qe"),
            pin_in!("Stop", "Stop", "Input pin Stop"),
            pin_in!("TimeActive", "TimeActive", "Input pin TimeActive"),
            pin_in!("Translate", "Translate", "Input pin Translate"),
            pin_out!("TxQr", "TxQr", "Output pin TxQr"),
        ],
        &[ChildConfigKind::IoData, ChildConfigKind::CoHist]
    ),
    schema!(
        "InputRef",
        "Input Reference",
        "Periphery References",
        "https://www.loxone.com/enen/kb/periphery-references/",
        "Places a reference to a physical or virtual periphery input, sensor, or system variable on a logic page.",
        &[
            attr!("Ref", "Referenced Periphery UUID", None, ValueKind::UuidRef, None, "UUID of referenced TreeNode in periphery/system tree"),
            attr!("Analog", "Analog Mode", None, ValueKind::Bool, Some("false"), "True if referenced input produces an analog value"),

            attr!("LinkRefType", "Periphery Reference Type ID", None, ValueKind::Int, Some("182"), "Internal type ID of referenced periphery item"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("4"), "Number of connector pins on block"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("2048"), "Visualization and web interface bitflags"),
        ],
        &[],
        &[
            pin_out!("Q", "Digital Output", "Digital state of the referenced input"),
            pin_out!("AQ", "Analog Output", "Analog value of the referenced sensor/input"),
            pin_out!("On", "Online Status", "Device online state"),

            pin_in!("AI", "AI", "Input pin AI"),
            pin_in!("I", "I", "Input pin I"),
        ],
        &[ChildConfigKind::IoData]
    ),
    schema!(
        "Intercom",
        "Intercom",
        "Access",
        "https://www.loxone.com/enen/kb/intercom/",
        "Video door station controller managing doorbell events, SIP audio/video stream, proximity wake, and door strike triggers.",
        &[
            attr!("AudioInfo", "Audio Config", None, ValueKind::Text, None, "SIP / audio stream configuration"),
            attr!("VideoInfo", "Video Stream URL", None, ValueKind::Text, None, "External or internal camera stream URL"),

            attr!("OutName1", "Button 1 Label", None, ValueKind::Text, Some("Otwórz furtkę"), "Custom UI label for programmable output Q1"),
            attr!("OutName2", "Button 2 Label", None, ValueKind::Text, Some("Otwórz/zamknij bramę"), "Custom UI label for programmable output Q2"),
            attr!("OutName3", "Button 3 Label", None, ValueKind::Text, Some("Otwórz/zamknij garaż"), "Custom UI label for programmable output Q3"),
            attr!("ans", "Audio Answer Texts", None, ValueKind::Text, None, "Pre-recorded TTS quick-reply messages for Intercom"),
            attr!("Ref", "Referenced Device UUID", None, ValueKind::UuidRef, None, "UUID of linked periphery device or service"),
            attr!("VMode", "Video Stream Mode", None, ValueKind::Int, None, "Intercom video stream mode"),
            attr!("LtE", "Last Edit Timestamp", None, ValueKind::Int, Some("523626104"), "Internal last modification timestamp"),
            attr!("MCzones", "Music Zones Mask", None, ValueKind::Text, Some("01"), "Linked audio server alarm zones"),
            attr!("MServer", "Music Server Reference", None, ValueKind::UuidRef, None, "Referenced Audio Server / Music Server UUID"),
            attr!("MZoneNr", "Music Zone Number", None, ValueKind::Text, Some("01"), "Target audio zone number"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("14"), "Number of connector pins on block"),
            attr!("Objects", "Linked Objects", None, ValueKind::Text, None, "UUIDs of linked functional blocks or periphery devices"),
            attr!("SType", "Server / Source Type", None, ValueKind::Int, Some("02"), "Audio or alarm source type"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
        ],
        &[
            param!("Tb", "Bell pulse duration", Some("s"), ValueKind::DurationSeconds, "3", "Duration of bell output Qbell"),

            param!("Lbr", "LED status brightness", Some("%"), ValueKind::Percent, "20", "Backlight brightness of Intercom buttons"),
            param!("Timeout", "Call / stream timeout", Some("s"), ValueKind::DurationSeconds, "60", "Timeout for active doorbell call"),
            param!("Tp", "Output pulse width", Some("s"), ValueKind::DurationSeconds, "1", "Pulse duration for Q1..Q3 outputs"),
        ],
        &[
            pin_in!("Bell", "Doorbell Input", "Triggers doorbell ring and app push notification"),
            pin_in!("Px", "Proximity", "Wakes up Intercom display / illumination"),
            pin_out!("Qb", "Bell Output", "Active when doorbell button is pressed"),
            pin_out!("Q1", "Gen. Button 1 / Door", "Programmable UI button 1 (typically door opener)"),
            pin_out!("Q2", "Gen. Button 2 / Light", "Programmable UI button 2"),
            pin_out!("Q3", "Gen. Button 3 / Gate", "Programmable UI button 3"),

            pin_in!("API", "API", "Input pin API"),
            pin_in!("Ib", "Ib", "Input pin Ib"),
            pin_in!("Lon", "Lon", "Input pin Lon"),
            pin_in!("Mute", "Mute", "Input pin Mute"),
            pin_out!("Qmute", "Qmute", "Output pin Qmute"),
            pin_in!("Reset", "Reset", "Input pin Reset"),
            pin_in!("TTS", "TTS", "Input pin TTS"),
        ],
        &[ChildConfigKind::IoData, ChildConfigKind::CoHist]
    ),
    schema!(
        "LightController2",
        "Lighting Controller",
        "Lighting",
        "https://www.loxone.com/enen/kb/lighting-controller/",
        "Combines dimmers, relays, RGBW, and Tunable White circuits into room lighting moods with motion and daylight automation.",
        &[
            attr!("LightsceneDescription", "Mood Definitions", None, ValueKind::Text, None, "Configured room lighting moods and circuit mixtures"),
            attr!("MasterType", "Circuit Types", None, ValueKind::Text, None, "Types of output circuits AQ1..AQ20 (Switch, Dimmer, Smart Actuator)"),
            attr!("MasterTitle", "Circuit Names", None, ValueKind::Text, None, "Custom display names for output circuits AQ1..AQ20"),
            attr!("Remanence", "Restore After Power Loss", None, ValueKind::Bool, Some("true"), "Restore last active mood after Miniserver reboot"),
            attr!("Rad", "Use Sun Coordinates", None, ValueKind::Bool, Some("true"), "Use solar elevation for HCL curve"),

            attr!("COName", "Configured Circuit Mask", None, ValueKind::Int, Some("7"), "Bitmask of named lighting circuits"),
            attr!("CapAI0", "Circuit 1 Capability", None, ValueKind::Int, Some("24"), "Actuator type capability for circuit 1"),
            attr!("CapAI1", "Circuit 2 Capability", None, ValueKind::Int, Some("24"), "Actuator type capability for circuit 2"),
            attr!("CapAI2", "Circuit 3 Capability", None, ValueKind::Int, Some("24"), "Actuator type capability for circuit 3"),
            attr!("CapAI3", "Circuit 4 Capability", None, ValueKind::Int, Some("24"), "Actuator type capability for circuit 4"),
            attr!("LtE", "Last Edit Timestamp", None, ValueKind::Int, Some("523627544"), "Internal last modification timestamp"),
            attr!("NameAI0", "Circuit 1 Name", None, ValueKind::Text, Some("gabinet: podcięcie"), "Custom label for lighting circuit AQ1"),
            attr!("NameAI1", "Circuit 2 Name", None, ValueKind::Text, Some("Gabinet: Światło OK"), "Custom label for lighting circuit AQ2"),
            attr!("NameAI2", "Circuit 3 Name", None, ValueKind::Text, Some("Oświetlenie orientacyjne"), "Custom label for lighting circuit AQ3"),
            attr!("NameAI3", "Circuit 4 Name", None, ValueKind::Text, Some("ledy"), "Custom label for lighting circuit AQ4"),
            attr!("NameAI4", "Circuit 5 Name", None, ValueKind::Text, Some("Drzwi"), "Custom label for lighting circuit AQ5"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("75"), "Number of connector pins on block"),
            attr!("PresM0", "Mood 1 Presence Mask", None, ValueKind::Int, Some("65535"), "Circuit mask for mood 1"),
            attr!("PresM1", "Mood 2 Presence Mask", None, ValueKind::Int, Some("65535"), "Circuit mask for mood 2"),
            attr!("PresM2", "Mood 3 Presence Mask", None, ValueKind::Int, Some("65535"), "Circuit mask for mood 3"),
            attr!("PresM3", "Mood 4 Presence Mask", None, ValueKind::Int, Some("65535"), "Circuit mask for mood 4"),
            attr!("PresM4", "Mood 5 Presence Mask", None, ValueKind::Int, Some("65535"), "Circuit mask for mood 5"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
            attr!("T5P", "Touch T5 Assignment", None, ValueKind::Int, Some("2"), "Loxone Touch 5-button layout mode"),
            attr!("uuidSeqenceIx", "Color Sequence Index UUID", None, ValueKind::Text, None, "UUIDs of color sequence index states"),
            attr!("uuidSeqencing", "Color Sequence UUID", None, ValueKind::Text, None, "UUIDs of dynamic color sequences"),
        ],
        &[
            param!("Step", "Dimming step width", Some("%"), ValueKind::Percent, "10", "Brightness step percentage per click"),
            param!("Mt", "Motion active duration", Some("s"), ValueKind::DurationSeconds, "180", "Keep motion mood active for this duration"),
            param!("MoveTimeout", "Motion timeout (XML)", Some("s"), ValueKind::DurationSeconds, "180", "Duration before motion-triggered light turns off"),
            param!("BrightnessLimit", "Daylight threshold", Some("lx"), ValueKind::Lux, "150", "Ambient brightness below which motion activates light"),
            param!("Steptime", "Dimming ramp speed", Some("s"), ValueKind::DurationSeconds, "1", "Time per dimming step while holding button"),
            param!("Min", "Min dimmer output", Some("%"), ValueKind::Percent, "1", "Minimum brightness limit for dimmers"),
            param!("Max", "Max dimmer output", Some("%"), ValueKind::Percent, "100", "Maximum brightness limit for dimmers"),
            param!("MoveOn", "Motion activation mode", None, ValueKind::Int, "1", "How motion input Mv activates moods"),
            param!("MoveIgnore", "Ignore motion after manual off", Some("s"), ValueKind::DurationSeconds, "10", "Lockout period after manual switch off"),
            param!("MoveScene", "Motion mood ID", None, ValueKind::Int, "1", "Mood number activated by motion sensor"),
            param!("Wrap", "Wrap mood cycling", None, ValueKind::Bool, "1", "Cycle back to first mood after last mood"),
            param!("NoLast", "Reset mood selection", None, ValueKind::Bool, "0", "Always start at first mood instead of last active"),
            param!("MaxP", "Max circuit power", Some("W"), ValueKind::Float, "0", "Nominal wattage for energy estimation"),
            param!("SceneMixTime", "Mood crossfade time", Some("s"), ValueKind::DurationSeconds, "3", "Transition duration when switching moods"),
            param!("FadingTime", "Soft on/off fade time", Some("s"), ValueKind::DurationSeconds, "1", "Soft fade duration when switching on/off"),
            param!("DayMinTemp", "HCL min color temp", Some("K"), ValueKind::Int, "2700", "Warmest color temperature in HCL curve"),
            param!("DayMaxTemp", "HCL max color temp", Some("K"), ValueKind::Int, "6500", "Coolest color temperature in HCL curve"),
            param!("AlarmClockPeriod", "Wake-up ramp time", Some("s"), ValueKind::DurationSeconds, "600", "Sunrise simulation duration"),
            param!("AlarmPeriod", "Burglar flash interval", Some("s"), ValueKind::DurationSeconds, "2", "Flash period during alarm input"),
            param!("AlarmBrightness", "Burglar flash brightness", Some("%"), ValueKind::Percent, "100", "Brightness during alarm flashing"),
            param!("RGBalt", "Legacy RGB encoding", None, ValueKind::Bool, "0", "Use legacy 9-digit RGB value format"),
        ],
        &[
            pin_in!("T", "Touch", "Main room light button (cycles moods, hold or double-click for Off)"),
            pin_in!("I", "Direct Mood Input", "Direct trigger for mood I1..I8"),
            pin_in!("Mv", "Motion", "Motion or presence detector input"),
            pin_in!("Br", "Brightness", "Ambient room brightness in Lux (lx)"),
            pin_in!("P", "Plus", "Next mood / dim up"),
            pin_in!("M", "Minus", "Previous mood / dim down"),
            pin_in!("O", "On", "Switch on to default/last mood"),
            pin_in!("R", "Off / Reset", "Switch off all lighting circuits"),
            pin_in!("DisM", "Disable Motion", "Temporarily disables motion automation"),
            pin_in!("Al", "Alarm", "Flashes room lights during burglar/fire alarm"),
            pin_out!("AQ", "Lighting Circuit Output", "Circuit output AQ1..AQ20 (0..100%, digital 0/1, or Smart Actuator)"),
            pin_out!("ActiveScene", "Active Mood", "Currently active mood number (0 = Off)"),
            pin_out!("Chg", "Mood Changed", "Pulses when active mood changes"),

            pin_out!("Alarm", "Alarm", "Output pin Alarm"),
            pin_out!("AlarmClock", "AlarmClock", "Output pin AlarmClock"),
            pin_in!("Brightness", "Brightness", "Input pin Brightness"),
            pin_in!("EnMove", "EnMove", "Input pin EnMove"),
            pin_in!("InputDisable", "InputDisable", "Input pin InputDisable"),
            pin_in!("InputTriggerDown", "InputTriggerDown", "Input pin InputTriggerDown"),
            pin_in!("InputTriggerUp", "InputTriggerUp", "Input pin InputTriggerUp"),
            pin_in!("MasterBr", "MasterBr", "Input pin MasterBr"),
            pin_in!("Move", "Move", "Input pin Move"),
            pin_in!("On", "On", "Input pin On"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_out!("OutputReset", "OutputReset", "Output pin OutputReset"),
            pin_out!("OutputResetAll", "OutputResetAll", "Output pin OutputResetAll"),
            pin_in!("Presence", "Presence", "Input pin Presence"),
            pin_in!("Remanence", "Remanence", "Input pin Remanence"),
            pin_in!("Reset", "Reset", "Input pin Reset"),
            pin_in!("RtD", "RtD", "Input pin RtD"),
            pin_in!("Scene", "Scene", "Input pin Scene"),
            pin_in!("Sel1", "Sel1", "Input pin Sel1"),
            pin_in!("Select", "Select", "Input pin Select"),
        ],
        &[
            ChildConfigKind::IoData,
            ChildConfigKind::Hcl,
            ChildConfigKind::Psd,
            ChildConfigKind::CoHist,
            ChildConfigKind::LightScenes,
        ]
    ),
];

static PAGE_SCHEMAS_3: &[BlockSchema] = &[
    schema!(
        "LongClick",
        "Long-Click",
        "Pushbuttons",
        "https://www.loxone.com/enen/kb/long-click/",
        "Distinguishes between a short button press (output Q) and a long press/hold (output Ql).",
        &[
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("14"), "Number of connector pins on block"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("131072"), "Visualization and web interface bitflags"),
        ],
        &[
            param!("Tl", "Long-click threshold", Some("s"), ValueKind::DurationSeconds, "0.8", "Minimum press time to trigger Ql"),
            param!("Tp", "Output pulse duration", Some("s"), ValueKind::DurationSeconds, "0.5", "Duration of output pulse on Q/Ql"),

            param!("Time", "Long-click threshold", Some("s"), ValueKind::DurationSeconds, "0.1", "Hold time before long-click output triggers"),
            param!("Max", "Max click window", Some("s"), ValueKind::DurationSeconds, "0.35", "Maximum pulse/click duration"),
            param!("V1", "Output 1 analog value", None, ValueKind::Float, "1", "Analog value emitted on click 1"),
            param!("V2", "Output 2 analog value", None, ValueKind::Float, "2", "Analog value emitted on click 2"),
            param!("V3", "Output 3 analog value", None, ValueKind::Float, "3", "Analog value emitted on click 3"),
            param!("V4", "Output 4 analog value", None, ValueKind::Float, "4", "Analog value emitted on click 4"),
        ],
        &[
            pin_in!("T", "Trigger", "Pushbutton digital input"),
            pin_out!("Q", "Short Click Output", "Pulses when button is released before Tl"),
            pin_out!("Ql", "Long Click Output", "Triggers when button is held longer than Tl"),

            pin_out!("AQ", "AQ", "Output pin AQ"),
            pin_in!("InputTrigger", "InputTrigger", "Input pin InputTrigger"),
            pin_in!("Remanence", "Remanence", "Input pin Remanence"),
            pin_in!("Reset", "Reset", "Input pin Reset"),
        ],
        &[]
    ),
    schema!(
        "Memory",
        "Memory Flag",
        "Periphery References",
        "https://www.loxone.com/enen/kb/memory-flags/",
        "Named digital, analog, or text flag used to share state across multiple logic pages.",
        &[
            attr!("Analog", "Analog Flag", None, ValueKind::Bool, Some("false"), "True if memory flag stores an analog value"),
            attr!("Digital", "Digital Flag", None, ValueKind::Bool, Some("true"), "True if memory flag stores a 0/1 digital state"),
            attr!("Rem", "Remanence", None, ValueKind::Bool, Some("false"), "Persist flag value across Miniserver reboots"),
            attr!("Unit", "Display Unit", None, ValueKind::Text, None, "Format string / engineering unit"),

            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("3"), "Number of connector pins on block"),
            attr!("Tp", "Memory Flag Type", None, ValueKind::Int, Some("0"), "0=Digital, 1=Analog, 2=Text, 3=Pushbutton"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("16"), "Visualization and web interface bitflags"),
        ],
        &[],
        &[
            pin_in!("I", "Input", "Writes value to the memory flag"),
            pin_out!("Q", "Digital Output", "Reads digital value of the memory flag"),
            pin_out!("AQ", "Analog Output", "Reads analog value of the memory flag"),

            pin_in!("Input", "Input", "Input pin Input"),
        ],
        &[ChildConfigKind::IoData]
    ),
    schema!(
        "MeterAbsBi",
        "Bidirectional Energy Meter",
        "Energy",
        "https://www.loxone.com/enen/kb/energy-meter/",
        "Records bidirectional energy import/export (kWh) and active power (kW) with tariff and statistics logging.",
        &[
            attr!("StatsType", "Statistics Recording Mode", None, ValueKind::Int, Some("8"), "Meter statistics interval and retention mode"),
            attr!("Un1", "Power / Flow Unit Format", None, ValueKind::Text, Some("<v.3>kW"), "Format string and unit for instantaneous power/flow"),
            attr!("Un2", "Energy / Volume Unit Format", None, ValueKind::Text, Some("<v.1>kWh"), "Format string and unit for cumulative reading"),
            attr!("PwrN", "Power Channel Label", None, ValueKind::Text, Some("Moc"), "Display label for instantaneous power"),
            attr!("InvPf", "Invert Power Polarity", None, ValueKind::Bool, Some("true"), "Invert import/export sign of power reading"),
            attr!("Rec", "Linked Energy Flow Monitor", None, ValueKind::UuidRef, None, "UUID of parent EFM / Energy recording group"),
            attr!("IName", "IName", None, ValueKind::Text, None, "Loxone XML attribute IName"),
            attr!("LtE", "Last Edit Timestamp", None, ValueKind::Int, Some("486826437"), "Internal last modification timestamp"),
            attr!("NOEFM", "NOEFM", None, ValueKind::Text, Some("true"), "Loxone XML attribute NOEFM"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("26"), "Number of connector pins on block"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("1073741824"), "Visualization and web interface bitflags"),
        ],
        &[
            param!("Ir", "Impulse rate", None, ValueKind::Float, "1000", "Pulses per kWh for S0 pulse inputs"),

            param!("Mro1", "Meter offset import", Some("kWh"), ValueKind::Float, "0", "Calibration offset for import meter reading"),
            param!("Mro2", "Meter offset export", Some("kWh"), ValueKind::Float, "0", "Calibration offset for export meter reading"),
        ],
        &[
            pin_in!("P", "Active Power", "Instantaneous power (+import / -export in kW)"),
            pin_in!("Mr", "Meter Reading Import", "Cumulative imported energy reading (kWh)"),
            pin_in!("Mrd", "Meter Reading Export", "Cumulative exported energy reading (kWh)"),
            pin_in!("R", "Reset Statistics", "Resets period counters"),
            pin_out!("AQp", "Power Output", "Current active power (kW)"),
            pin_out!("AQc", "Total Import", "Total imported energy (kWh)"),
            pin_out!("AQd", "Total Export", "Total exported energy (kWh)"),

            pin_out!("OMr1", "OMr1", "Output pin OMr1"),
            pin_out!("OPf", "OPf", "Output pin OPf"),
            pin_out!("ORd1", "ORd1", "Output pin ORd1"),
            pin_out!("ORld1", "ORld1", "Output pin ORld1"),
            pin_out!("ORlm1", "ORlm1", "Output pin ORlm1"),
            pin_out!("ORlw1", "ORlw1", "Output pin ORlw1"),
            pin_out!("ORly1", "ORly1", "Output pin ORly1"),
            pin_out!("ORm1", "ORm1", "Output pin ORm1"),
            pin_out!("ORw1", "ORw1", "Output pin ORw1"),
            pin_out!("ORy1", "ORy1", "Output pin ORy1"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_in!("Pf", "Pf", "Input pin Pf"),
        ],
        &[ChildConfigKind::IoData]
    ),
    schema!(
        "MeterPUni",
        "Unidirectional Utility Meter",
        "Energy",
        "https://www.loxone.com/enen/kb/utility-meter/",
        "Records unidirectional electricity, water, gas, or heat consumption and flow/power rate.",
        &[
            attr!("StatsType", "Statistics Recording Mode", None, ValueKind::Int, Some("8"), "Meter statistics interval and retention mode"),
            attr!("Un1", "Power / Flow Unit Format", None, ValueKind::Text, Some("<v.3>kW"), "Format string and unit for instantaneous power/flow"),
            attr!("Un2", "Energy / Volume Unit Format", None, ValueKind::Text, Some("<v.1>kWh"), "Format string and unit for cumulative reading"),
            attr!("PwrN", "Power Channel Label", None, ValueKind::Text, Some("Moc"), "Display label for instantaneous power"),
            attr!("Rec", "Linked Energy Flow Monitor", None, ValueKind::UuidRef, None, "UUID of parent EFM / Energy recording group"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("16"), "Number of connector pins on block"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("1073741824"), "Visualization and web interface bitflags"),
        ],
        &[
            param!("Ir", "Impulse rate", None, ValueKind::Float, "1000", "Pulses per unit (kWh / m³)"),

            param!("Np", "Pulses per unit", None, ValueKind::Float, "1000", "Number of S0 pulses per kWh/m³"),
            param!("Mro", "Meter reading offset", None, ValueKind::Float, "0", "Calibration offset added to meter reading"),
        ],
        &[
            pin_in!("P", "Power / Flow", "Current power (kW) or flow rate (m³/h)"),
            pin_in!("Mr", "Meter Reading", "Absolute cumulative meter reading"),
            pin_in!("R", "Reset", "Resets day/month/year statistics"),
            pin_out!("AQp", "Power / Flow Output", "Current power or flow rate"),
            pin_out!("AQc", "Total Consumption", "Total cumulative consumption"),

            pin_in!("F", "F", "Input pin F"),
            pin_out!("OMr", "OMr", "Output pin OMr"),
            pin_out!("OPf", "OPf", "Output pin OPf"),
            pin_out!("ORd", "ORd", "Output pin ORd"),
            pin_out!("ORld", "ORld", "Output pin ORld"),
            pin_out!("ORlm", "ORlm", "Output pin ORlm"),
            pin_out!("ORlw", "ORlw", "Output pin ORlw"),
            pin_out!("ORly", "ORly", "Output pin ORly"),
            pin_out!("ORm", "ORm", "Output pin ORm"),
            pin_out!("ORw", "ORw", "Output pin ORw"),
            pin_out!("ORy", "ORy", "Output pin ORy"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
        ],
        &[ChildConfigKind::IoData]
    ),
    schema!(
        "MultiClick",
        "Multi-Click",
        "Pushbuttons",
        "https://www.loxone.com/enen/kb/multi-click/",
        "Counts rapid consecutive button presses (1x, 2x, 3x, 4x) and activates the corresponding output Q1..Q4.",
        &[
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("14"), "Number of connector pins on block"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("131072"), "Visualization and web interface bitflags"),
        ],
        &[
            param!("T", "Multi-click window", Some("s"), ValueKind::DurationSeconds, "0.6", "Max time window for multiple clicks"),
            param!("Tp", "Output pulse duration", Some("s"), ValueKind::DurationSeconds, "0.5", "Duration of output pulse on Q1..Q4"),

            param!("Time", "Pulse duration", Some("s"), ValueKind::DurationSeconds, "0.1", "Output pulse duration"),
            param!("Max", "Multi-click window", Some("s"), ValueKind::DurationSeconds, "0.35", "Maximum interval between consecutive clicks"),
            param!("V1", "Single-click value", None, ValueKind::Float, "1", "Value output on 1x click"),
            param!("V2", "Double-click value", None, ValueKind::Float, "2", "Value output on 2x click"),
            param!("V3", "Triple-click value", None, ValueKind::Float, "3", "Value output on 3x click"),
            param!("V4", "Quadruple-click value", None, ValueKind::Float, "4", "Value output on 4x click"),
        ],
        &[
            pin_in!("Tr", "Trigger", "Pushbutton input"),
            pin_out!("Q1", "Single Click", "Pulses on 1x click"),
            pin_out!("Q2", "Double Click", "Pulses on 2x double-click"),
            pin_out!("Q3", "Triple Click", "Pulses on 3x click"),
            pin_out!("Q4", "Quadruple Click", "Pulses on 4x click"),

            pin_out!("AQ", "AQ", "Output pin AQ"),
            pin_in!("InputTrigger", "InputTrigger", "Input pin InputTrigger"),
            pin_in!("Remanence", "Remanence", "Input pin Remanence"),
            pin_in!("Reset", "Reset", "Input pin Reset"),
        ],
        &[]
    ),
    schema!(
        "MusicPlayer",
        "Audio Player",
        "Audio",
        "https://www.loxone.com/enen/kb/audio-player/",
        "Controls an Audio Server zone with favorites, volume ramping, TTS announcements, bell chimes, and presence automation.",
        &[
            attr!("Ref", "Audio Zone Reference", None, ValueKind::UuidRef, None, "UUID of linked AudioOut periphery device"),

            attr!("PID", "Player Zone ID", None, ValueKind::Int, Some("1"), "Internal Audio Server zone ID"),
            attr!("APL", "Autoplay Enabled", None, ValueKind::Bool, Some("true"), "Automatically start playback on presence"),
            attr!("SPC", "Speaker Protection", None, ValueKind::Bool, Some("true"), "Enable thermal/clipping speaker protection"),
            attr!("HPF", "High-Pass Filter", None, ValueKind::Bool, Some("true"), "Enable speaker high-pass filter"),
            attr!("PR", "Presence Automation", None, ValueKind::Bool, Some("true"), "Enable presence-linked audio playback"),
            attr!("BZM", "Buzzer Mode", None, ValueKind::Int, Some("1"), "Alarm clock buzzer behavior"),
            attr!("SpCfg", "Speaker Configuration", None, ValueKind::Int, Some("4"), "Stereo / Mono / Subwoofer output configuration"),
            attr!("LtE", "Last Edit Timestamp", None, ValueKind::Int, Some("486821341"), "Internal last modification timestamp"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("47"), "Number of connector pins on block"),
            attr!("PGrP", "Player Group", None, ValueKind::Text, Some(""), "Linked multi-room audio group"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
        ],
        &[
            param!("V", "Default volume", Some("%"), ValueKind::Percent, "35", "Startup / default playback volume"),
            param!("MaxV", "Max volume", Some("%"), ValueKind::Percent, "80", "Maximum allowed zone volume"),
            param!("Step", "Volume step", Some("%"), ValueKind::Percent, "5", "Volume adjustment step per click"),
            param!("Vb", "Bell event volume", Some("%"), ValueKind::Percent, "50", "Volume for doorbell event chime"),
            param!("Va", "Alarm event volume", Some("%"), ValueKind::Percent, "70", "Volume for alarm event tone"),

            param!("DefVol", "Default startup volume", Some("%"), ValueKind::Percent, "20", "Default playback volume when switched on"),
            param!("MaxVol", "Maximum volume limit", Some("%"), ValueKind::Percent, "100", "Upper volume limit for the zone"),
            param!("AlarmVol", "Alarm event volume", Some("%"), ValueKind::Percent, "75", "Playback volume during alarm events"),
            param!("BellVol", "Doorbell chime volume", Some("%"), ValueKind::Percent, "10", "Playback volume during doorbell events"),
            param!("BuzzVol", "Wake-up buzzer volume", Some("%"), ValueKind::Percent, "50", "Playback volume for alarm clock buzzer"),
            param!("BuzzerFav", "Wake-up favorite ID", None, ValueKind::Int, "1", "Zone favorite played for wake-up alarm"),
            param!("TTSVol", "Text-to-speech volume", Some("%"), ValueKind::Percent, "40", "Playback volume for TTS announcements"),
            param!("DblClk", "Double-click time", Some("s"), ValueKind::DurationSeconds, "0.35", "Double-click detection window"),
            param!("Volume", "Current / initial volume", Some("%"), ValueKind::Percent, "20", "Zone volume percentage"),
        ],
        &[
            pin_in!("T", "Touch", "Room Touch surface audio control (Play/Next, Vol+/Vol-)"),
            pin_in!("Pl", "Play / Pause", "Toggles playback"),
            pin_in!("O", "On", "Starts playback"),
            pin_in!("R", "Off / Reset", "Stops playback and turns zone off"),
            pin_in!("Vp", "Volume +", "Increases zone volume"),
            pin_in!("Vm", "Volume -", "Decreases zone volume"),
            pin_in!("V", "Volume Setpoint", "Sets analog volume (0..100%)"),
            pin_in!("B", "Bell Event", "Plays doorbell chime overlay"),
            pin_in!("Al", "Alarm Event", "Plays alarm sound overlay"),
            pin_out!("Q", "Playing Status", "High while zone is actively playing"),
            pin_out!("AQv", "Current Volume", "Current zone volume (0..100%)"),

            pin_out!("Alarm", "Alarm", "Output pin Alarm"),
            pin_in!("Bell", "Bell", "Input pin Bell"),
            pin_in!("Buzzer", "Buzzer", "Input pin Buzzer"),
            pin_in!("Cs", "Cs", "Input pin Cs"),
            pin_in!("Disable", "Disable", "Input pin Disable"),
            pin_in!("DisableMv", "DisableMv", "Input pin DisableMv"),
            pin_in!("EnBT", "EnBT", "Input pin EnBT"),
            pin_in!("Fav", "Fav", "Input pin Fav"),
            pin_in!("FireAlarm", "FireAlarm", "Input pin FireAlarm"),
            pin_in!("HouseOff", "HouseOff", "Input pin HouseOff"),
            pin_in!("IgnoreROff", "IgnoreROff", "Input pin IgnoreROff"),
            pin_in!("LineIn", "LineIn", "Input pin LineIn"),
            pin_in!("Move", "Move", "Input pin Move"),
            pin_in!("Next", "Next", "Input pin Next"),
            pin_out!("OutActive", "OutActive", "Output pin OutActive"),
            pin_out!("OutAudio", "OutAudio", "Output pin OutAudio"),
            pin_out!("OutAudioL", "OutAudioL", "Output pin OutAudioL"),
            pin_out!("OutAudioR", "OutAudioR", "Output pin OutAudioR"),
            pin_out!("OutAudioSub", "OutAudioSub", "Output pin OutAudioSub"),
            pin_out!("OutBluetooth", "OutBluetooth", "Output pin OutBluetooth"),
            pin_out!("OutVolDownExternal", "OutVolDownExternal", "Output pin OutVolDownExternal"),
            pin_out!("OutVolUpExternal", "OutVolUpExternal", "Output pin OutVolUpExternal"),
            pin_out!("OutVolume", "OutVolume", "Output pin OutVolume"),
            pin_out!("OutZoneFav", "OutZoneFav", "Output pin OutZoneFav"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_in!("Pause", "Pause", "Input pin Pause"),
            pin_in!("Play", "Play", "Input pin Play"),
            pin_in!("Prev", "Prev", "Input pin Prev"),
            pin_in!("Reset", "Reset", "Input pin Reset"),
            pin_in!("RoomOff", "RoomOff", "Input pin RoomOff"),
            pin_in!("RtD", "RtD", "Input pin RtD"),
            pin_in!("TTS", "TTS", "Input pin TTS"),
            pin_in!("Tg", "Tg", "Input pin Tg"),
            pin_in!("VolDown", "VolDown", "Input pin VolDown"),
            pin_in!("VolUp", "VolUp", "Input pin VolUp"),
            pin_in!("volFading", "volFading", "Input pin volFading"),
        ],
        &[ChildConfigKind::IoData, ChildConfigKind::CoHist]
    ),
    schema!(
        "NfcCodeTouch",
        "NFC Code Touch",
        "Access",
        "https://www.loxone.com/enen/kb/nfc-code-touch/",
        "Authenticates users via encrypted NFC tags and numeric PIN codes to grant access to up to 6 outputs.",
        &[
            attr!("Ref", "Hardware Reference", None, ValueKind::UuidRef, None, "Linked NFC Code Touch Tree/Air device"),

            attr!("AP", "Access Protection", None, ValueKind::Bool, Some("true"), "Protect block actions in visualization"),
            attr!("HSize", "History Log Size", None, ValueKind::Int, Some("10"), "Number of access events stored in history"),
            attr!("OStr", "Output Names (Q1..Q6)", None, ValueKind::Text, None, "Semicolon-separated labels for outputs Q1..Q6"),
            attr!("LtE", "Last Edit Timestamp", None, ValueKind::Int, Some("487280839"), "Internal last modification timestamp"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("33"), "Number of connector pins on block"),
            attr!("SpCdUuid", "Special Code UUID", None, ValueKind::Text, Some(""), "UUID of linked special access code"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
        ],
        &[
            param!("Tq", "Output pulse duration", Some("s"), ValueKind::DurationSeconds, "3", "Pulse duration on Q1..Q6 upon valid authentication"),

            param!("Au", "Audible beep feedback", None, ValueKind::Bool, "1", "Enable acoustic confirmation beep"),
            param!("Lbr", "LED indicator brightness", Some("%"), ValueKind::Percent, "50", "Brightness of NFC Code Touch status LEDs"),
            param!("PulseTime", "Access output pulse time", Some("s"), ValueKind::DurationSeconds, "5", "Pulse duration for Q1..Q6 outputs"),
        ],
        &[
            pin_in!("Dis", "Disable", "Locks out authentication"),
            pin_out!("Q1", "Output 1 (Main Access)", "Pulses when authorized user triggers output 1"),
            pin_out!("Q2", "Output 2", "Pulses when authorized user triggers output 2"),
            pin_out!("Q3", "Output 3", "Pulses when authorized user triggers output 3"),
            pin_out!("Q4", "Output 4", "Pulses when authorized user triggers output 4"),
            pin_out!("Q5", "Output 5", "Pulses when authorized user triggers output 5"),
            pin_out!("Q6", "Output 6", "Pulses when authorized user triggers output 6"),
            pin_out!("Qd", "Access Denied", "Pulses when an invalid code or unauthorized NFC tag is presented"),

            pin_in!("Be", "Be", "Input pin Be"),
            pin_in!("BeSel", "BeSel", "Input pin BeSel"),
            pin_in!("Bfb", "Bfb", "Input pin Bfb"),
            pin_in!("Bg", "Bg", "Input pin Bg"),
            pin_in!("Disable", "Disable", "Input pin Disable"),
            pin_in!("ExA", "ExA", "Input pin ExA"),
            pin_in!("ExARem", "ExARem", "Input pin ExARem"),
            pin_in!("La", "La", "Input pin La"),
            pin_in!("Lb", "Lb", "Input pin Lb"),
            pin_in!("Lg", "Lg", "Input pin Lg"),
            pin_in!("Lon", "Lon", "Input pin Lon"),
            pin_in!("Lr", "Lr", "Input pin Lr"),
            pin_in!("Lw", "Lw", "Input pin Lw"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_in!("PUDel", "PUDel", "Input pin PUDel"),
            pin_out!("Qa", "Qa", "Output pin Qa"),
            pin_out!("Qn", "Qn", "Output pin Qn"),
            pin_out!("TQ", "TQ", "Output pin TQ"),
            pin_out!("TQU", "TQU", "Output pin TQU"),
            pin_out!("TQo", "TQo", "Output pin TQo"),
            pin_out!("TQt", "TQt", "Output pin TQt"),
        ],
        &[ChildConfigKind::IoData, ChildConfigKind::CoHist]
    ),
    schema!(
        "Not",
        "NOT Inverter",
        "Logic",
        "https://www.loxone.com/enen/kb/not/",
        "Inverts a digital signal: outputs 1 when input I is 0, and 0 when input I is 1.",
        &[
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("2"), "Number of connector pins on block"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("131072"), "Visualization and web interface bitflags"),
        ],
        &[],
        &[
            pin_in!("I", "Input", "Digital input to invert"),
            pin_out!("Q", "Inverted Output", "1 when I == 0, 0 when I == 1"),
        ],
        &[]
    ),
    schema!(
        "OnPulseDelay",
        "Switch-On Delayed Pulse",
        "Times",
        "https://www.loxone.com/enen/kb/delayed-pulse/",
        "Waits for delay Td after a rising edge on Tr, then emits a pulse of duration Tp on output Q.",
        &[
            attr!("LtE", "Last Edit Timestamp", None, ValueKind::Int, Some("489338320"), "Internal last modification timestamp"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("6"), "Number of connector pins on block"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("131072"), "Visualization and web interface bitflags"),
        ],
        &[
            param!("Td", "Switch-on delay", Some("s"), ValueKind::DurationSeconds, "1", "Delay before output pulse begins"),
            param!("Tp", "Pulse duration", Some("s"), ValueKind::DurationSeconds, "1", "Duration of the output pulse on Q"),

            param!("Delay", "Switch-on delay", Some("s"), ValueKind::DurationSeconds, "15", "Delay before output pulse triggers"),
            param!("Time", "Output pulse duration", Some("s"), ValueKind::DurationSeconds, "0.5", "Duration of output pulse Q"),
        ],
        &[
            pin_in!("Tr", "Trigger", "Starts delay timer on rising edge"),
            pin_in!("R", "Reset", "Cancels pending delay or active pulse"),
            pin_out!("Q", "Pulse Output", "High for Tp seconds after delay Td expires"),

            pin_in!("InputTrigger", "InputTrigger", "Input pin InputTrigger"),
            pin_in!("Remanence", "Remanence", "Input pin Remanence"),
            pin_in!("Reset", "Reset", "Input pin Reset"),
        ],
        &[]
    ),
    schema!(
        "OutputRef",
        "Output Reference",
        "Periphery References",
        "https://www.loxone.com/enen/kb/periphery-references/",
        "Places a reference to a physical or virtual periphery actuator, relay, dimmer, or memory flag output on a logic page.",
        &[
            attr!("Ref", "Referenced Actuator UUID", None, ValueKind::UuidRef, None, "UUID of referenced actuator TreeNode"),
            attr!("Analog", "Analog Actuator", None, ValueKind::Bool, Some("false"), "True if actuator accepts an analog value"),

            attr!("LinkRefType", "Periphery Reference Type ID", None, ValueKind::Int, Some("21"), "Internal type ID of referenced periphery item"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("2"), "Number of connector pins on block"),
            attr!("Text", "Text", None, ValueKind::Text, Some("true"), "Loxone XML attribute Text"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("131072"), "Visualization and web interface bitflags"),
        ],
        &[],
        &[
            pin_in!("I", "Actuator Input", "Drives the referenced digital or analog output"),
            pin_out!("Q", "Feedback Output", "Current state of the referenced output"),

            pin_in!("AI", "AI", "Input pin AI"),
            pin_out!("AQ", "AQ", "Output pin AQ"),
        ],
        &[ChildConfigKind::IoData]
    ),
    schema!(
        "OutputRefLM",
        "Logger / Mailer / Notification Output",
        "Periphery References",
        "https://www.loxone.com/enen/kb/logger-mailer-caller-tracker/",
        "Sends push notifications, emails,voice calls, or log entries when triggered by digital or analog events.",
        &[
            attr!("Ref", "Referenced Service UUID", None, ValueKind::UuidRef, None, "UUID of referenced Logger/Mailer/Notification node"),

            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("2"), "Number of connector pins on block"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("131072"), "Visualization and web interface bitflags"),
        ],
        &[],
        &[
            pin_in!("I", "Trigger / Value", "Triggers message transmission on rising/falling edge or value change"),

            pin_in!("AI", "AI", "Input pin AI"),
            pin_out!("AQ", "AQ", "Output pin AQ"),
        ],
        &[ChildConfigKind::LoggerMailer, ChildConfigKind::IoData]
    ),
    schema!(
        "PButtonT",
        "Pushbutton (Toggle)",
        "Pushbuttons",
        "https://www.loxone.com/enen/kb/pushbutton/",
        "UI and logic toggle pushbutton with separate On (O), Off/Reset (R), and Toggle (T) inputs.",
        &[
            attr!("Rem", "Remanence", None, ValueKind::Bool, Some("false"), "Retain state across Miniserver reboots"),

            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("7"), "Number of connector pins on block"),
        ],
        &[
            param!("Time", "Pulse / auto-off duration", Some("s"), ValueKind::DurationSeconds, "2", "Output pulse duration in seconds"),
        ],
        &[
            pin_in!("T", "Trigger / Toggle", "Toggles output Q on rising edge"),
            pin_in!("O", "On", "Sets output Q to 1"),
            pin_in!("R", "Reset / Off", "Sets output Q to 0"),
            pin_in!("Dis", "Disable", "Disables button operation"),
            pin_out!("Q", "Output", "Current switched state (0 or 1)"),
            pin_out!("Qn", "Inverted Output", "Inverted switched state"),

            pin_in!("InputDisable", "InputDisable", "Input pin InputDisable"),
            pin_in!("InputTrigger", "InputTrigger", "Input pin InputTrigger"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_in!("Remanence", "Remanence", "Input pin Remanence"),
            pin_in!("Reset", "Reset", "Input pin Reset"),
        ],
        &[ChildConfigKind::IoData]
    ),
    schema!(
        "PresenceDetector",
        "Presence Controller",
        "Presence",
        "https://www.loxone.com/enen/kb/presence-controller/",
        "Combines motion, acoustic, and door sensors to determine room occupancy with configurable overrun time.",
        &[
            attr!("DEVS", "Linked Presence Sensors", None, ValueKind::Text, None, "UUIDs of linked Tree/Air presence detectors"),
            attr!("TRK_MAX", "Max History Entries", None, ValueKind::Int, Some("50"), "Maximum tracked events"),
            attr!("TRK_NE", "Displayed History Entries", None, ValueKind::Int, Some("50"), "Number of visible history entries"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("19"), "Number of connector pins on block"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
        ],
        &[
            param!("To", "Overrun time", Some("s"), ValueKind::DurationSeconds, "180", "Duration presence remains active after last motion"),

            param!("ParamTOn", "Presence overrun time", Some("s"), ValueKind::DurationSeconds, "900", "Duration presence remains active after motion stops"),
            param!("ParamTWarn", "Pre-off warning time", Some("s"), ValueKind::DurationSeconds, "15", "Warning time before presence expires"),
        ],
        &[
            pin_in!("Mo", "Motion / Presence", "Motion or acoustic presence sensor input"),
            pin_in!("R", "Reset", "Immediately clears presence state"),
            pin_in!("Dis", "Disable", "Disables presence detection"),
            pin_out!("Q", "Presence Active", "High while room is occupied"),

            pin_in!("Activation", "Activation", "Input pin Activation"),
            pin_out!("Active", "Active", "Output pin Active"),
            pin_in!("Locked", "Locked", "Input pin Locked"),
            pin_in!("LockState", "LockState", "Input pin LockState"),
            pin_in!("Verify", "Verify", "Input pin Verify"),
            pin_in!("ControlInput", "ControlInput", "Input pin ControlInput"),
            pin_in!("DeviceActivate", "DeviceActivate", "Input pin DeviceActivate"),
            pin_in!("DeviceExtend", "DeviceExtend", "Input pin DeviceExtend"),
            pin_in!("DeviceTrigger", "DeviceTrigger", "Input pin DeviceTrigger"),
            pin_in!("InputActivate", "InputActivate", "Input pin InputActivate"),
            pin_in!("InputExtend", "InputExtend", "Input pin InputExtend"),
            pin_in!("InputReset", "InputReset", "Input pin InputReset"),
            pin_in!("InputTrigger", "InputTrigger", "Input pin InputTrigger"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_out!("OutputActive", "OutputActive", "Output pin OutputActive"),
            pin_out!("OutputDevActiveTime", "OutputDevActiveTime", "Output pin OutputDevActiveTime"),
            pin_out!("OutputDevOnTime", "OutputDevOnTime", "Output pin OutputDevOnTime"),
            pin_out!("OutputOff", "OutputOff", "Output pin OutputOff"),
            pin_out!("OutputOn", "OutputOn", "Output pin OutputOn"),
            pin_out!("OutputOnTime", "OutputOnTime", "Output pin OutputOnTime"),
            pin_out!("OutputPresence", "OutputPresence", "Output pin OutputPresence"),
            pin_out!("OutputWarn", "OutputWarn", "Output pin OutputWarn"),
        ],
        &[ChildConfigKind::IoData]
    ),
    schema!(
        "PulseAt",
        "Edge Trigger (Pulse At)",
        "Times",
        "https://www.loxone.com/enen/kb/edge-trigger/",
        "Generates a fixed-duration pulse on rising edge, falling edge, or both edges of input Tr.",
        &[
            attr!("Sec", "Trigger Time / Offset", Some("s"), ValueKind::DurationSeconds, Some("28800"), "Scheduled timestamp or offset in seconds"),
            attr!("Typ", "Trigger Mode", None, ValueKind::Int, Some("0"), "Pulse trigger type"),
            attr!("NTXT", "Notification / Display Text", None, ValueKind::Text, Some("Wyłącz Noc"), "Custom text label"),
            attr!("LtE", "Last Edit Timestamp", None, ValueKind::Int, Some("556034033"), "Internal last modification timestamp"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("5"), "Number of connector pins on block"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("131072"), "Visualization and web interface bitflags"),
        ],
        &[
            param!("Tp", "Pulse duration", Some("s"), ValueKind::DurationSeconds, "0.5", "Length of generated output pulse"),
            param!("M", "Edge mode", None, ValueKind::Int, "1", "1=Rising edge, 2=Falling edge, 3=Both edges"),

            param!("Time", "Output pulse duration", Some("s"), ValueKind::DurationSeconds, "1", "Duration of the generated pulse"),
        ],
        &[
            pin_in!("Tr", "Trigger", "Monitored digital signal"),
            pin_out!("Q", "Pulse Output", "Emits pulse of duration Tp on configured edge"),

            pin_in!("InputDisable", "InputDisable", "Input pin InputDisable"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_in!("Remanence", "Remanence", "Input pin Remanence"),
        ],
        &[]
    ),
    schema!(
        "PulseBy",
        "Interval Pulse Generator",
        "Times",
        "https://www.loxone.com/enen/kb/pulse-generator/",
        "Generates periodic pulses (clock signal) with configurable high time Th and low time Tl while enabled.",
        &[
            attr!("WTA", "Watchdog Text / Filter", None, ValueKind::Text, Some("*"), "Monitored text pattern"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("3"), "Number of connector pins on block"),
        ],
        &[
            param!("Th", "Pulse high duration", Some("s"), ValueKind::DurationSeconds, "1", "Duration output Q stays 1"),
            param!("Tl", "Pause low duration", Some("s"), ValueKind::DurationSeconds, "1", "Duration output Q stays 0 between pulses"),

            param!("Time", "Pulse width", Some("s"), ValueKind::DurationSeconds, "1", "Duration of the generated pulse"),
        ],
        &[
            pin_in!("En", "Enable", "Enables cyclic pulse generation while high"),
            pin_out!("Q", "Clock Output", "Periodic pulse output"),

            pin_in!("Pulse", "Pulse", "Input pin Pulse"),
            pin_in!("Text", "Text", "Input pin Text"),
        ],
        &[]
    ),
    schema!(
        "PushButton",
        "Virtual Pushbutton",
        "Pushbuttons",
        "https://www.loxone.com/enen/kb/pushbutton/",
        "Momentary or toggle pushbutton for app visualization and logic triggering.",
        &[
            attr!("LtE", "Last Edit Timestamp", None, ValueKind::Int, Some("480447591"), "Internal last modification timestamp"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("9"), "Number of connector pins on block"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
        ],
        &[
            param!("Tp", "Pulse duration", Some("s"), ValueKind::DurationSeconds, "0.5", "Momentary pulse width"),
        ],
        &[
            pin_in!("T", "Trigger", "Triggers pushbutton pulse"),
            pin_in!("Dis", "Disable", "Disables button"),
            pin_out!("Q", "Output", "Momentary pulse output"),
            pin_out!("Qp", "Plus Output", "Up / Plus button output"),
            pin_out!("Qm", "Minus Output", "Down / Minus button output"),

            pin_in!("InputDisable", "InputDisable", "Input pin InputDisable"),
            pin_in!("InputTrigger", "InputTrigger", "Input pin InputTrigger"),
            pin_in!("On", "On", "Input pin On"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_out!("Qoff", "Qoff", "Output pin Qoff"),
            pin_out!("Qon", "Qon", "Output pin Qon"),
            pin_in!("Remanence", "Remanence", "Input pin Remanence"),
            pin_in!("Reset", "Reset", "Input pin Reset"),
        ],
        &[ChildConfigKind::IoData]
    ),
    schema!(
        "SmokeAlarm",
        "Fire & Water Alarm",
        "Security & Safety",
        "https://www.loxone.com/enen/kb/fire-water-alarm/",
        "Monitors smoke, temperature, and water leak sensors with pre-alarm, main alarm escalation, and emergency actions.",
        &[
            attr!("UO", "Monitored Sensors", None, ValueKind::Text, None, "Linked smoke/water detector devices"),

            attr!("ST", "Smoke / Water Mode", None, ValueKind::Int, Some("1"), "1=Smoke/Fire alarm, 2=Water leak alarm"),
            attr!("CentralOut", "Central Output Mode", None, ValueKind::Int, Some("3"), "Central alarm output routing mode"),
            attr!("Entries", "History Entries Limit", None, ValueKind::Int, Some("40"), "Maximum number of event log entries"),
            attr!("Objects", "Linked Objects", None, ValueKind::Text, None, "UUIDs of linked functional blocks or periphery devices"),
            attr!("MCzones", "Music Zones Mask", None, ValueKind::Text, Some("01"), "Linked audio server alarm zones"),
            attr!("MServer", "Music Server Reference", None, ValueKind::UuidRef, None, "Referenced Audio Server / Music Server UUID"),
            attr!("MZoneNr", "Music Zone Number", None, ValueKind::Text, Some("01"), "Target audio zone number"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("27"), "Number of connector pins on block"),
            attr!("SType", "Server / Source Type", None, ValueKind::Int, Some("02"), "Audio or alarm source type"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
        ],
        &[
            param!("Ta", "Pre-alarm duration", Some("s"), ValueKind::DurationSeconds, "30", "Delay before full fire/water alarm escalates"),

            param!("AlarmDelay", "Pre-alarm delay", Some("s"), ValueKind::DurationSeconds, "120", "Delay between pre-alarm and main fire alarm"),
            param!("MaxDuration", "Max siren duration", Some("s"), ValueKind::DurationSeconds, "300", "Maximum duration of acoustic alarm outputs"),
            param!("MaxTemp", "Overtemperature threshold", Some("°C"), ValueKind::TemperatureCelsius, "43", "Temperature threshold triggering thermal fire alarm"),
        ],
        &[
            pin_in!("I", "Detector Input", "Smoke or water leak sensor input I1..I8"),
            pin_in!("Q", "Acknowledge", "Acknowledges and silences active alarm"),
            pin_in!("Dis", "Disable", "Temporarily disables alarm inputs"),
            pin_out!("O", "Alarm Stage Output", "Staged emergency outputs O1..O6 (Sirens, Escape Light, Shades Up, Main Water Valve)"),
            pin_out!("Al", "Alarm Level", "Active alarm severity level"),

            pin_in!("Confirm", "Confirm", "Input pin Confirm"),
            pin_in!("Reset", "Reset", "Input pin Reset"),
            pin_in!("TestAlarm", "TestAlarm", "Input pin TestAlarm"),
            pin_in!("Disable", "Disable", "Input pin Disable"),
            pin_in!("Water", "Water", "Input pin Water"),
            pin_in!("Smoke", "Smoke", "Input pin Smoke"),
            pin_in!("Temperature", "Temperature", "Input pin Temperature"),
            pin_in!("ServiceAlarm", "ServiceAlarm", "Input pin ServiceAlarm"),
            pin_out!("TimePreAlarm", "TimePreAlarm", "Output pin TimePreAlarm"),
            pin_out!("AlarmLevel", "AlarmLevel", "Output pin AlarmLevel"),
            pin_out!("Quit", "Quit", "Output pin Quit"),
            pin_out!("Status", "Status", "Output pin Status"),
            pin_in!("Autoconfirm", "Autoconfirm", "Input pin Autoconfirm"),
            pin_in!("InputAirAnalog", "InputAirAnalog", "Input pin InputAirAnalog"),
            pin_in!("InputAirDigitalS", "InputAirDigitalS", "Input pin InputAirDigitalS"),
            pin_in!("InputAirDigitalW", "InputAirDigitalW", "Input pin InputAirDigitalW"),
            pin_in!("InputAlarm", "InputAlarm", "Input pin InputAlarm"),
            pin_in!("InputAlarmS", "InputAlarmS", "Input pin InputAlarmS"),
            pin_in!("InputAlarmW", "InputAlarmW", "Input pin InputAlarmW"),
            pin_in!("InputTemp", "InputTemp", "Input pin InputTemp"),
            pin_in!("Mute", "Mute", "Input pin Mute"),
            pin_out!("OutActive", "OutActive", "Output pin OutActive"),
            pin_out!("OutAlarm1", "OutAlarm1", "Output pin OutAlarm1"),
            pin_out!("OutAlarmTest", "OutAlarmTest", "Output pin OutAlarmTest"),
            pin_out!("OutHorn", "OutHorn", "Output pin OutHorn"),
            pin_out!("OutHornSpd", "OutHornSpd", "Output pin OutHornSpd"),
            pin_out!("OutNumAlarms", "OutNumAlarms", "Output pin OutNumAlarms"),
            pin_out!("OutSilent", "OutSilent", "Output pin OutSilent"),
            pin_out!("OutText", "OutText", "Output pin OutText"),
            pin_out!("OutTime", "OutTime", "Output pin OutTime"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
            pin_in!("Remanence", "Remanence", "Input pin Remanence"),
            pin_in!("Servicemode", "Servicemode", "Input pin Servicemode"),
        ],
        &[ChildConfigKind::IoData, ChildConfigKind::LoggerMailer, ChildConfigKind::CoHist]
    ),
    schema!(
        "State",
        "Status",
        "Status",
        "https://www.loxone.com/enen/kb/status/",
        "Evaluates conditional rules across inputs AI1..AI8 to output custom status text, icon, color, and numeric value.",
        &[
            attr!("StateText", "Status Condition Table", None, ValueKind::Text, None, "Rule table mapping input conditions to text/color/value"),

            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("11"), "Number of connector pins on block"),
        ],
        &[],
        &[
            pin_in!("AI", "Condition Input", "Digital or analog input AI1..AI8 evaluated by status rules"),
            pin_out!("AQ", "Status Value", "Numeric value of the highest-priority matching status rule"),
            pin_out!("TQ", "Status Text", "Formatted text of the active status rule"),

            pin_in!("I1", "I1", "Input pin I1"),
            pin_out!("OutputAPI", "OutputAPI", "Output pin OutputAPI"),
        ],
        &[ChildConfigKind::IoData, ChildConfigKind::CoHist]
    ),
    schema!(
        "StateV",
        "System Status Monitor",
        "Status",
        "https://www.loxone.com/enen/kb/status/",
        "Aggregates system-wide status messages and warnings across functional blocks.",
        &[
            attr!("SpStates", "Monitored Status Refs", None, ValueKind::Text, None, "Linked system status sources"),

            attr!("Analog", "Analog", None, ValueKind::Text, Some("true"), "Loxone XML attribute Analog"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("2"), "Number of connector pins on block"),
            attr!("Tx", "Text Mode", None, ValueKind::Bool, Some("false"), "Output formatted text instead of numeric value"),
        ],
        &[],
        &[
            pin_out!("Q", "Active Warning", "High when at least one monitored status is active"),
            pin_out!("TQ", "Status Summary", "Combined status text"),

            pin_out!("AQ", "AQ", "Output pin AQ"),
            pin_in!("I", "I", "Input pin I"),
        ],
        &[ChildConfigKind::IoData]
    ),
    schema!(
        "Text",
        "Page Comment / Note",
        "Documentation",
        "https://www.loxone.com/enen/kb/documenting-projects/",
        "Canvas annotation box displaying documentation notes and section headings on a logic page.",
        &[
            attr!("Text", "Comment Text", None, ValueKind::Text, None, "Text displayed inside the annotation box"),

            attr!("Td", "Show Border / Todo", None, ValueKind::Bool, Some("true"), "Mark comment as todo or framed note"),
        ],
        &[],
        &[],
        &[]
    ),
];

macro_rules! tree_schema {
    ($ty:expr, $name:expr, $cat:expr, $url:expr, $sum:expr) => {
        BlockSchema {
            type_name: $ty,
            display_name_en: $name,
            category_group: $cat,
            kb_url: $url,
            summary: $sum,
            attributes: &[],
            pin_params: &[],
            pin_docs: &[],
            child_configs: &[ChildConfigKind::IoData],
        }
    };
    ($ty:expr, $name:expr, $cat:expr, $url:expr, $sum:expr, $attrs:expr, $pins:expr) => {
        BlockSchema {
            type_name: $ty,
            display_name_en: $name,
            category_group: $cat,
            kb_url: $url,
            summary: $sum,
            attributes: $attrs,
            pin_params: &[],
            pin_docs: $pins,
            child_configs: &[ChildConfigKind::IoData],
        }
    };
}

static TREE_SCHEMAS_1: &[BlockSchema] = &[
    tree_schema!(
        "Actor",
        "Digital Output (Relay)",
        "Periphery",
        "https://www.loxone.com/enen/kb/miniserver/",
        "Physical relay output channel on Miniserver or Extension switching 230V/24V loads.",
        &[
            attr!("IName", "IName", None, ValueKind::Text, Some("Q1"), "Periphery XML attribute IName"),
            attr!("Idx", "Idx", None, ValueKind::Text, Some("1"), "Periphery XML attribute Idx"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("1"), "Number of connector pins on block"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("16400"), "Visualization and web interface bitflags"),
        ],
        &[
            pin_in!("I", "I", "Connector pin I"),
        ]
    ),
    tree_schema!(
        "AnalogInputCaption",
        "Analog Inputs Folder",
        "Periphery",
        "https://www.loxone.com/enen/kb/miniserver/",
        "Container folder for analog 0-10V inputs."
    ),
    tree_schema!(
        "AnalogOutputCaption",
        "Analog Outputs Folder",
        "Periphery",
        "https://www.loxone.com/enen/kb/miniserver/",
        "Container folder for analog 0-10V actuator outputs."
    ),
    tree_schema!(
        "ApiActor",
        "API Virtual Actuator",
        "Virtual I/O",
        "https://www.loxone.com/enen/kb/virtual-inputs-outputs/",
        "Software API actuator endpoint controlled or queried via WebSocket/HTTP web services."
    ),
    tree_schema!(
        "AudioOut",
        "Audio Server Speaker Output",
        "Audio",
        "https://www.loxone.com/enen/kb/audioserver/",
        "Amplified speaker channel or line output on an Audio Server or Stereo Extension.",
        &[attr!("Chn", "Speaker Channel", None, ValueKind::Int, Some("1"), "Output channel index")],
        &[]
    ),
    tree_schema!(
        "AudioServer",
        "Audioserver",
        "Audio",
        "https://www.loxone.com/enen/kb/audioserver/",
        "Multi-room Loxone Audioserver hardware device with integrated amplifiers and Tree Turbo bus.",
        &[
            attr!("Serial", "Serial Number", None, ValueKind::Text, None, "Audioserver hardware serial"),
            attr!("HoN", "Hostname", None, ValueKind::Text, None, "Network hostname"),
        ],
        &[pin_out!("On", "Online Status", "1 when Audioserver is online")]
    ),
    tree_schema!(
        "AutoPilot",
        "AutoPilot Rule",
        "System",
        "https://www.loxone.com/enen/kb/autopilot/",
        "User-configurable rule created in the Loxone App combining triggers, conditions, and actions."
    ),
    tree_schema!(
        "Calendar",
        "Operating Mode Schedule",
        "Times",
        "https://www.loxone.com/enen/kb/operating-modes/",
        "Calendar schedule defining recurring dates, holidays, or time windows for operating modes."
    ),
    tree_schema!(
        "CalendarCaption",
        "Schedules & Calendars Folder",
        "Times",
        "https://www.loxone.com/enen/kb/operating-modes/",
        "Container folder for operating mode calendars and holidays."
    ),
    tree_schema!(
        "CalendarEntry",
        "Calendar Entry",
        "Times",
        "https://www.loxone.com/enen/kb/operating-modes/",
        "Single date, range, or recurrence rule inside an Operating Mode Calendar."
    ),
    tree_schema!(
        "Caller",
        "Phone Caller Service",
        "Messages",
        "https://www.loxone.com/enen/kb/logger-mailer-caller-tracker/",
        "Cloud text-to-speech telephone call service for critical alarm notifications.",
        &[
            attr!("Callback", "Callback", None, ValueKind::Text, Some("true"), "Periphery XML attribute Callback"),
            attr!("ES", "ES", None, ValueKind::Text, Some("1"), "Periphery XML attribute ES"),
            attr!("EnableSys", "EnableSys", None, ValueKind::Text, Some("false"), "Periphery XML attribute EnableSys"),
            attr!("Phone", "Phone", None, ValueKind::Text, None, "Periphery XML attribute Phone"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("16400"), "Visualization and web interface bitflags"),
        ],
        &[]
    ),
    tree_schema!(
        "CallerVirtualIn",
        "Call-In Virtual Input",
        "Virtual I/O",
        "https://www.loxone.com/enen/kb/virtual-inputs-outputs/",
        "Virtual input triggered via incoming cloud caller authentication.",
        &[
            attr!("CallerKey", "CallerKey", None, ValueKind::Text, Some("1"), "Periphery XML attribute CallerKey"),
            attr!("IName", "IName", None, ValueKind::Text, Some("CVI0"), "Periphery XML attribute IName"),
            attr!("Idx", "Idx", None, ValueKind::Text, Some("1"), "Periphery XML attribute Idx"),
            attr!("MinTime", "MinTime", None, ValueKind::Text, Some("10"), "Periphery XML attribute MinTime"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("2"), "Number of connector pins on block"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("16384"), "Visualization and web interface bitflags"),
        ],
        &[
            pin_out!("Qe", "Qe", "Connector pin Qe"),
            pin_out!("Qm", "Qm", "Connector pin Qm"),
        ]
    ),
    tree_schema!(
        "Category",
        "Category",
        "Project Organization",
        "https://www.loxone.com/enen/kb/rooms-categories/",
        "Functional category (Lighting, Shading, Climate, Audio, Security) used for grouping and app navigation."
    ),
    tree_schema!(
        "CategoryCaption",
        "Categories Folder",
        "Project Organization",
        "https://www.loxone.com/enen/kb/rooms-categories/",
        "Root container for project functional categories."
    ),
    tree_schema!(
        "Comm485",
        "RS485 / Modbus Extension",
        "Periphery",
        "https://www.loxone.com/enen/kb/modbus-extension/",
        "RS485 / Modbus RTU serial bus interface extension.",
        &[attr!("Baud", "Baud Rate", Some("bps"), ValueKind::Int, Some("9600"), "Serial communication speed")],
        &[]
    ),
    tree_schema!(
        "Constant",
        "Constant Value",
        "System",
        "https://www.loxone.com/enen/kb/constants/",
        "Named constant value available across logic pages.",
        &[attr!("Val", "Value", None, ValueKind::Float, Some("0"), "Constant numeric or text value")],
        &[pin_out!("AQ", "Constant Output", "Outputs the configured constant value")]
    ),
    tree_schema!(
        "ConstantCaption",
        "Constants Folder",
        "System",
        "https://www.loxone.com/enen/kb/constants/",
        "Container folder for global project constants."
    ),
    tree_schema!(
        "DateTime",
        "Current Date & Time",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "System variable outputting seconds elapsed since 01.01.2009 00:00:00."
    ),
    tree_schema!(
        "Day",
        "Day of Month",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "System variable outputting current day of month (1..31)."
    ),
    tree_schema!(
        "Day2009",
        "Days Since 2009",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "System variable outputting full days elapsed since 1st January 2009."
    ),
    tree_schema!(
        "DayOfWeek",
        "Day of Week",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "System variable outputting current weekday (1=Monday .. 7=Sunday)."
    ),
    tree_schema!(
        "Daylight",
        "Daylight Indicator",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "Outputs 1 between sunrise and sunset at the project geographic coordinates."
    ),
    tree_schema!(
        "Daylight2",
        "Civil Daylight Indicator",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "Outputs 1 between civil morning twilight and civil evening twilight."
    ),
    tree_schema!(
        "Devicemonitor",
        "Device Status Monitor",
        "System",
        "https://www.loxone.com/enen/kb/device-status/",
        "Monitors online status, battery level, and communication health of periphery devices."
    ),
    tree_schema!(
        "DigitalIn",
        "Digital Input",
        "Periphery",
        "https://www.loxone.com/enen/kb/miniserver/",
        "24V DC digital input channel on Miniserver or Extension with optional frequency/pulse counting.",
        &[
            attr!("EnoceanSubType", "EnoceanSubType", None, ValueKind::Text, Some("1"), "Periphery XML attribute EnoceanSubType"),
            attr!("IName", "IName", None, ValueKind::Text, Some("I1"), "Periphery XML attribute IName"),
            attr!("Idx", "Idx", None, ValueKind::Text, Some("1"), "Periphery XML attribute Idx"),
            attr!("MinTime", "MinTime", None, ValueKind::Text, Some("10"), "Periphery XML attribute MinTime"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("2"), "Number of connector pins on block"),
            attr!("ValOT", "ValOT", None, ValueKind::Text, Some("1"), "Periphery XML attribute ValOT"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("16384"), "Visualization and web interface bitflags"),
        ],
        &[
            pin_out!("Q", "Q", "Connector pin Q"),
            pin_out!("Qe", "Qe", "Connector pin Qe"),
        ]
    ),
    tree_schema!(
        "Dimmer",
        "Dimmer Channel",
        "Periphery",
        "https://www.loxone.com/enen/kb/dimmer-extension/",
        "Leading/trailing-edge phase dimmer output channel (0..100%) on a Dimmer Extension.",
        &[
            attr!("CCap", "CCap", None, ValueKind::Text, Some("24"), "Periphery XML attribute CCap"),
            attr!("Channel", "Channel", None, ValueKind::Text, Some("1"), "Periphery XML attribute Channel"),
            attr!("DestValHigh", "DestValHigh", None, ValueKind::Text, Some("100"), "Periphery XML attribute DestValHigh"),
            attr!("DimmType", "DimmType", None, ValueKind::Text, Some("2"), "Periphery XML attribute DimmType"),
            attr!("Dimmer", "Dimmer", None, ValueKind::Text, Some("true"), "Periphery XML attribute Dimmer"),
            attr!("Gamma", "Gamma", None, ValueKind::Text, Some("2.2"), "Periphery XML attribute Gamma"),
            attr!("IName", "IName", None, ValueKind::Text, Some("AQ1"), "Periphery XML attribute IName"),
            attr!("Idx", "Idx", None, ValueKind::Text, Some("1"), "Periphery XML attribute Idx"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("1"), "Number of connector pins on block"),
            attr!("Percent", "Percent", None, ValueKind::Text, Some("true"), "Periphery XML attribute Percent"),
            attr!("Slewrate", "Slewrate", None, ValueKind::Text, Some("100"), "Periphery XML attribute Slewrate"),
            attr!("SourceValHigh", "SourceValHigh", None, ValueKind::Text, Some("100"), "Periphery XML attribute SourceValHigh"),
            attr!("SubType", "SubType", None, ValueKind::Text, Some("159"), "Periphery XML attribute SubType"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("16400"), "Visualization and web interface bitflags"),
        ],
        &[
            pin_in!("I", "I", "Connector pin I"),
        ]
    ),
    tree_schema!(
        "Document",
        "Loxone Project Document",
        "Project Organization",
        "https://www.loxone.com/enen/kb/project-configuration/",
        "Root Loxone Config project document storing location coordinates, Miniserver settings, and project metadata."
    ),
    tree_schema!(
        "Eveningtwilight",
        "Civil Dusk Time",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "Outputs time of civil dusk in minutes from midnight."
    ),
    tree_schema!(
        "GenAsensor",
        "Generic Analog Sensor",
        "Periphery",
        "https://www.loxone.com/enen/kb/periphery-references/",
        "Generic analog sensor input channel."
    ),
    tree_schema!(
        "GenSensor",
        "Generic Digital Sensor",
        "Periphery",
        "https://www.loxone.com/enen/kb/periphery-references/",
        "Generic digital sensor input channel."
    ),
    tree_schema!(
        "GlobalStates",
        "Global System States",
        "System",
        "https://www.loxone.com/enen/kb/system-states/",
        "Container for global building states (Frost, Heat, Storm, Burglar Alarm, Fire Alarm)."
    ),
    tree_schema!(
        "Hour",
        "Current Hour",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "System variable outputting the current hour of the day (0..23)."
    ),
    tree_schema!(
        "Icon",
        "Custom Icon",
        "Project Organization",
        "https://www.loxone.com/enen/kb/rooms-categories/",
        "Icon definition referenced by categories, rooms, or status rules."
    ),
    tree_schema!(
        "ImpulseDay",
        "Daily Pulse",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "Emits a pulse every day at 00:00 midnight."
    ),
    tree_schema!(
        "ImpulseEveningtwilight",
        "Dusk Pulse",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "Emits a single pulse at the moment of civil dusk."
    ),
    tree_schema!(
        "ImpulseHour",
        "Hourly Pulse",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "Emits a single pulse at the start of every full hour."
    ),
    tree_schema!(
        "ImpulseMinute",
        "Minute Pulse",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "Emits a single pulse at the start of every full minute."
    ),
    tree_schema!(
        "ImpulseMonth",
        "Monthly Pulse",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "Emits a single pulse at 00:00 on the 1st of every month."
    ),
    tree_schema!(
        "ImpulseMorningtwilight",
        "Dawn Pulse",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "Emits a single pulse at the moment of civil dawn."
    ),
    tree_schema!(
        "ImpulseSecond",
        "1-Second Clock Pulse",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "Emits a pulse every second."
    ),
    tree_schema!(
        "ImpulseSunrise",
        "Sunrise Pulse",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "Emits a single pulse at sunrise."
    ),
    tree_schema!(
        "ImpulseSunset",
        "Sunset Pulse",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "Emits a single pulse at sunset."
    ),
    tree_schema!(
        "ImpulseYear",
        "Yearly Pulse",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "Emits a single pulse at 00:00 on January 1st."
    ),
    tree_schema!(
        "InputCaption",
        "Digital Inputs Folder",
        "Periphery",
        "https://www.loxone.com/enen/kb/miniserver/",
        "Container folder for hardware digital inputs."
    ),
    tree_schema!(
        "IntercomDevice",
        "Intercom Hardware Device",
        "Access",
        "https://www.loxone.com/enen/kb/intercom/",
        "Loxone Intercom hardware unit on Tree / LAN."
    ),
    tree_schema!(
        "KeycodeCaption",
        "Authentication & NFC Folder",
        "Access",
        "https://www.loxone.com/enen/kb/nfc-code-touch/",
        "Container folder for NFC tags and access keycodes."
    ),
    tree_schema!(
        "LanInt",
        "Network / LAN Interface",
        "Periphery",
        "https://www.loxone.com/enen/kb/virtual-inputs-outputs/",
        "LAN interface container for IP-connected devices and virtual outputs."
    ),
    tree_schema!(
        "Logger",
        "Event Logger",
        "Messages",
        "https://www.loxone.com/enen/kb/logger-mailer-caller-tracker/",
        "Logs formatted state transitions and values to a file, syslog server, or HTTP stream."
    ),
    tree_schema!(
        "LoggerOutCaption",
        "Messages & Loggers Folder",
        "Messages",
        "https://www.loxone.com/enen/kb/logger-mailer-caller-tracker/",
        "Container folder for Loggers, Mailers, Push Notifications, and Callers."
    ),
];

static TREE_SCHEMAS_2: &[BlockSchema] = &[
    tree_schema!(
        "LoxAIR",
        "Air Base Extension",
        "Periphery",
        "https://www.loxone.com/enen/kb/air-base-extension/",
        "Loxone Air wireless mesh transceiver extension."
    ),
    tree_schema!(
        "LoxAIRAactor",
        "Loxone Air Analog Actuator",
        "Periphery",
        "https://www.loxone.com/enen/kb/air-base-extension/",
        "Analog actuator output channel on a Loxone Air wireless device.",
        &[
            attr!("CCap", "CCap", None, ValueKind::Text, Some("31"), "Periphery XML attribute CCap"),
            attr!("Channel", "Channel", None, ValueKind::Text, Some("1"), "Periphery XML attribute Channel"),
            attr!("DestValHigh", "DestValHigh", None, ValueKind::Text, Some("100"), "Periphery XML attribute DestValHigh"),
            attr!("DimmType", "DimmType", None, ValueKind::Text, Some("1"), "Periphery XML attribute DimmType"),
            attr!("IName", "IName", None, ValueKind::Text, Some("AQ1"), "Periphery XML attribute IName"),
            attr!("Idx", "Idx", None, ValueKind::Text, Some("1"), "Periphery XML attribute Idx"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("1"), "Number of connector pins on block"),
            attr!("Percent", "Percent", None, ValueKind::Text, Some("true"), "Periphery XML attribute Percent"),
            attr!("Slewrate", "Slewrate", None, ValueKind::Text, Some("100"), "Periphery XML attribute Slewrate"),
            attr!("SourceValHigh", "SourceValHigh", None, ValueKind::Text, Some("100"), "Periphery XML attribute SourceValHigh"),
            attr!("SubType", "SubType", None, ValueKind::Text, Some("158"), "Periphery XML attribute SubType"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("16400"), "Visualization and web interface bitflags"),
        ],
        &[
            pin_in!("I", "I", "Connector pin I"),
        ]
    ),
    tree_schema!(
        "LoxAIRAsensor",
        "Loxone Air Analog Sensor",
        "Periphery",
        "https://www.loxone.com/enen/kb/air-base-extension/",
        "Analog sensor reading (temperature, humidity, brightness, battery) from a Loxone Air device.",
        &[
            attr!("Analog", "Analog", None, ValueKind::Text, Some("true"), "Periphery XML attribute Analog"),
            attr!("DestValHigh", "DestValHigh", None, ValueKind::Text, Some("100"), "Periphery XML attribute DestValHigh"),
            attr!("EnoceanSubType", "EnoceanSubType", None, ValueKind::Text, Some("69"), "Periphery XML attribute EnoceanSubType"),
            attr!("IName", "IName", None, ValueKind::Text, Some("AI1"), "Periphery XML attribute IName"),
            attr!("Idx", "Idx", None, ValueKind::Text, Some("1"), "Periphery XML attribute Idx"),
            attr!("MaxVal", "MaxVal", None, ValueKind::Text, Some("100"), "Periphery XML attribute MaxVal"),
            attr!("MinChange", "MinChange", None, ValueKind::Text, Some("0.25"), "Periphery XML attribute MinChange"),
            attr!("MinTime", "MinTime", None, ValueKind::Text, Some("1000"), "Periphery XML attribute MinTime"),
            attr!("MinVal", "MinVal", None, ValueKind::Text, Some("-40"), "Periphery XML attribute MinVal"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("2"), "Number of connector pins on block"),
            attr!("ResetTime", "ResetTime", None, ValueKind::Text, Some("4000"), "Periphery XML attribute ResetTime"),
            attr!("SourceValHigh", "SourceValHigh", None, ValueKind::Text, Some("100"), "Periphery XML attribute SourceValHigh"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("20480"), "Visualization and web interface bitflags"),
        ],
        &[
            pin_out!("AQ", "AQ", "Connector pin AQ"),
            pin_out!("Q", "Q", "Connector pin Q"),
        ]
    ),
    tree_schema!(
        "LoxAIRDevice",
        "Loxone Air Device",
        "Periphery",
        "https://www.loxone.com/enen/kb/air-base-extension/",
        "Wireless Loxone Air periphery device (Touch Air, Valve Actuator Air, Smoke Detector Air, Multi Extension Air)."
    ),
    tree_schema!(
        "LoxAIRactor",
        "Loxone Air Digital Actuator",
        "Periphery",
        "https://www.loxone.com/enen/kb/air-base-extension/",
        "Digital relay or switch channel on a Loxone Air device.",
        &[
            attr!("IName", "IName", None, ValueKind::Text, Some("Q1"), "Periphery XML attribute IName"),
            attr!("Idx", "Idx", None, ValueKind::Text, Some("1"), "Periphery XML attribute Idx"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("1"), "Number of connector pins on block"),
            attr!("SubType", "SubType", None, ValueKind::Text, Some("70"), "Periphery XML attribute SubType"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("16384"), "Visualization and web interface bitflags"),
        ],
        &[
            pin_in!("I", "I", "Connector pin I"),
        ]
    ),
    tree_schema!(
        "LoxAIRsensor",
        "Loxone Air Digital Sensor",
        "Periphery",
        "https://www.loxone.com/enen/kb/air-base-extension/",
        "Digital input, touch button, or reed contact on a Loxone Air device.",
        &[
            attr!("EnoceanSubType", "EnoceanSubType", None, ValueKind::Text, Some("67"), "Periphery XML attribute EnoceanSubType"),
            attr!("IName", "IName", None, ValueKind::Text, Some("I1"), "Periphery XML attribute IName"),
            attr!("Idx", "Idx", None, ValueKind::Text, Some("1"), "Periphery XML attribute Idx"),
            attr!("MinTime", "MinTime", None, ValueKind::Text, Some("10"), "Periphery XML attribute MinTime"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("2"), "Number of connector pins on block"),
            attr!("ResetTime", "ResetTime", None, ValueKind::Text, Some("4000"), "Periphery XML attribute ResetTime"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("134234128"), "Visualization and web interface bitflags"),
        ],
        &[
            pin_out!("Q", "Q", "Connector pin Q"),
            pin_out!("Qe", "Qe", "Connector pin Qe"),
        ]
    ),
    tree_schema!(
        "LoxCaption",
        "Extension Bus Folder",
        "Periphery",
        "https://www.loxone.com/enen/kb/miniserver/",
        "Container folder for Loxone Link extensions, Tree branches, or Air devices."
    ),
    tree_schema!(
        "LoxDIMM",
        "Dimmer Extension",
        "Periphery",
        "https://www.loxone.com/enen/kb/dimmer-extension/",
        "Loxone Link 4-channel universal Dimmer Extension with 8 digital inputs."
    ),
    tree_schema!(
        "LoxLIVE",
        "Miniserver",
        "Periphery",
        "https://www.loxone.com/enen/kb/miniserver/",
        "Central Loxone Miniserver controller managing extensions, Tree/Air buses, network interfaces, and logic execution.",
        &[
            attr!("IntAddr", "Internal IP Address", None, ValueKind::Text, None, "LAN IPv4 address of the Miniserver"),
            attr!("Serial", "Serial Number", None, ValueKind::Text, None, "Miniserver MAC / serial number"),
            attr!("HoN", "Hostname", None, ValueKind::Text, None, "Network hostname of the Miniserver"),
        ],
        &[]
    ),
    tree_schema!(
        "LoxREL",
        "Relay Extension",
        "Periphery",
        "https://www.loxone.com/enen/kb/relay-extension/",
        "Loxone Link 14-channel high-power Relay Extension (16A per relay)."
    ),
    tree_schema!(
        "LoxTree",
        "Tree Extension",
        "Periphery",
        "https://www.loxone.com/enen/kb/tree-extension/",
        "Loxone Tree Extension providing Left and Right Tree bus branches for up to 100 Tree devices."
    ),
    tree_schema!(
        "MTablet",
        "Mobile / Tablet Client",
        "Access",
        "https://www.loxone.com/enen/kb/app/",
        "Registered mobile or wall-mounted tablet visualization client."
    ),
    tree_schema!(
        "Mailer",
        "Email Notification Service",
        "Messages",
        "https://www.loxone.com/enen/kb/logger-mailer-caller-tracker/",
        "Sends SMTP or Loxone Cloud emails when triggered by logic or alarm events."
    ),
    tree_schema!(
        "MemoryCaption",
        "Memory Flags Folder",
        "System",
        "https://www.loxone.com/enen/kb/memory-flags/",
        "Container folder for global project Memory Flags."
    ),
    tree_schema!(
        "MessageCenter",
        "System Message Center",
        "Messages",
        "https://www.loxone.com/enen/kb/system-status/",
        "Central hub for system diagnostics, device warnings, and user notifications."
    ),
    tree_schema!(
        "Minute",
        "Current Minute",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "System variable outputting the current minute of the hour (0..59)."
    ),
    tree_schema!(
        "ModbusASensor",
        "Modbus Analog Sensor",
        "Periphery",
        "https://www.loxone.com/enen/kb/modbus-extension/",
        "Polled Modbus RTU/TCP holding or input register reading.",
        &[
            attr!("Analog", "Analog", None, ValueKind::Text, Some("true"), "Periphery XML attribute Analog"),
            attr!("DestValHigh", "DestValHigh", None, ValueKind::Text, Some("0.01"), "Periphery XML attribute DestValHigh"),
            attr!("EnoceanSubType", "EnoceanSubType", None, ValueKind::Text, Some("17"), "Periphery XML attribute EnoceanSubType"),
            attr!("IName", "IName", None, ValueKind::Text, Some("AMQ-1"), "Periphery XML attribute IName"),
            attr!("Idx", "Idx", None, ValueKind::Text, Some("-1"), "Periphery XML attribute Idx"),
            attr!("MaxTime", "MaxTime", None, ValueKind::Text, Some("300000"), "Periphery XML attribute MaxTime"),
            attr!("MaxVal", "MaxVal", None, ValueKind::Text, Some("10000"), "Periphery XML attribute MaxVal"),
            attr!("MinChange", "MinChange", None, ValueKind::Text, Some("0.25"), "Periphery XML attribute MinChange"),
            attr!("MinTime", "MinTime", None, ValueKind::Text, Some("1000"), "Periphery XML attribute MinTime"),
            attr!("MinVal", "MinVal", None, ValueKind::Text, Some("-10000"), "Periphery XML attribute MinVal"),
            attr!("ModbusAddress", "ModbusAddress", None, ValueKind::Text, Some("52"), "Periphery XML attribute ModbusAddress"),
            attr!("ModbusCmd", "ModbusCmd", None, ValueKind::Text, Some("4"), "Periphery XML attribute ModbusCmd"),
            attr!("ModbusDataType", "ModbusDataType", None, ValueKind::Text, Some("36"), "Periphery XML attribute ModbusDataType"),
            attr!("ModbusPollingCycle", "ModbusPollingCycle", None, ValueKind::Text, Some("5"), "Periphery XML attribute ModbusPollingCycle"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("2"), "Number of connector pins on block"),
            attr!("SourceValHigh", "SourceValHigh", None, ValueKind::Text, Some("10"), "Periphery XML attribute SourceValHigh"),
            attr!("ValOT", "ValOT", None, ValueKind::Text, Some("1"), "Periphery XML attribute ValOT"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("20480"), "Visualization and web interface bitflags"),
        ],
        &[
            pin_out!("Q", "Q", "Connector pin Q"),
            pin_out!("Qe", "Qe", "Connector pin Qe"),
        ]
    ),
    tree_schema!(
        "ModbusDev",
        "Modbus Slave Device",
        "Periphery",
        "https://www.loxone.com/enen/kb/modbus-extension/",
        "External Modbus RTU or Modbus TCP slave device (energy meter, inverter, heat pump, ventilation).",
        &[
            attr!("Channel", "Channel", None, ValueKind::Text, Some("1"), "Periphery XML attribute Channel"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("1"), "Number of connector pins on block"),
            attr!("NumHeatBoilers", "NumHeatBoilers", None, ValueKind::Text, Some("1"), "Periphery XML attribute NumHeatBoilers"),
            attr!("NumHeatBuffers", "NumHeatBuffers", None, ValueKind::Text, Some("1"), "Periphery XML attribute NumHeatBuffers"),
            attr!("NumHeatCircuits", "NumHeatCircuits", None, ValueKind::Text, Some("1"), "Periphery XML attribute NumHeatCircuits"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("20480"), "Visualization and web interface bitflags"),
        ],
        &[
            pin_out!("AQ", "AQ", "Connector pin AQ"),
        ]
    ),
    tree_schema!(
        "Mode",
        "Operating Mode",
        "Times",
        "https://www.loxone.com/enen/kb/operating-modes/",
        "Global building operating mode (e.g. Holiday, Public Holiday, Heating Season, Party, Vacation).",
        &[
            attr!("Mode", "Mode", None, ValueKind::Text, Some("1"), "Periphery XML attribute Mode"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("2"), "Number of connector pins on block"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("16384"), "Visualization and web interface bitflags"),
        ],
        &[
            pin_in!("I", "I", "Connector pin I"),
            pin_out!("Q", "Q", "Connector pin Q"),
        ]
    ),
    tree_schema!(
        "ModeCaption",
        "Operating Modes Folder",
        "Times",
        "https://www.loxone.com/enen/kb/operating-modes/",
        "Container folder for building operating modes."
    ),
    tree_schema!(
        "Month",
        "Current Month",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "System variable outputting the current month (1..12)."
    ),
    tree_schema!(
        "Morningtwilight",
        "Civil Dawn Time",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "Outputs time of civil dawn in minutes from midnight."
    ),
    tree_schema!(
        "NFCIDCaption",
        "NFC Tags Folder",
        "Access",
        "https://www.loxone.com/enen/kb/nfc-code-touch/",
        "Container folder for learned NFC key fobs and cards."
    ),
    tree_schema!(
        "NetworkDevice",
        "Network / Wake-on-LAN Device",
        "Periphery",
        "https://www.loxone.com/enen/kb/network-devices/",
        "Monitored IP host or Wake-on-LAN target on the local network."
    ),
    tree_schema!(
        "NightTime",
        "Night Indicator",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "Outputs 1 between sunset and sunrise."
    ),
    tree_schema!(
        "Notification",
        "Push Notification",
        "Messages",
        "https://www.loxone.com/enen/kb/logger-mailer-caller-tracker/",
        "Sends push notifications to iOS/Android Loxone App users.",
        &[
            attr!("Entries", "History Entries Limit", None, ValueKind::Int, Some("10"), "Maximum number of event log entries"),
            attr!("Sys", "Sys", None, ValueKind::Text, Some("true"), "Periphery XML attribute Sys"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("16384"), "Visualization and web interface bitflags"),
        ],
        &[]
    ),
    tree_schema!(
        "Online",
        "Internet Connection Status",
        "System",
        "https://www.loxone.com/enen/kb/system-variables/",
        "Outputs 1 when the Miniserver has active internet connectivity."
    ),
    tree_schema!(
        "OutputCaption",
        "Digital Outputs Folder",
        "Periphery",
        "https://www.loxone.com/enen/kb/miniserver/",
        "Container folder for hardware relay outputs."
    ),
    tree_schema!(
        "OvertempShutdown",
        "Overtemperature Shutdown",
        "System",
        "https://www.loxone.com/enen/kb/dimmer-extension/",
        "Thermal overload indicator for Dimmer or power extensions."
    ),
    tree_schema!(
        "Page",
        "Logic Page",
        "Project Organization",
        "https://www.loxone.com/enen/kb/documenting-projects/",
        "Programming page containing functional blocks, periphery references, and wiring."
    ),
    tree_schema!(
        "Permission",
        "Access Permission",
        "Users",
        "https://www.loxone.com/enen/kb/user-management/",
        "Permission entry linking users or groups to protected objects."
    ),
    tree_schema!(
        "Place",
        "Room / Place",
        "Project Organization",
        "https://www.loxone.com/enen/kb/rooms-categories/",
        "Physical room or zone in the building with area (m²) and room type metadata."
    ),
    tree_schema!(
        "PlaceCaption",
        "Rooms Folder",
        "Project Organization",
        "https://www.loxone.com/enen/kb/rooms-categories/",
        "Root container for building rooms and zones."
    ),
    tree_schema!(
        "Plugin",
        "Communication Plugin",
        "Periphery",
        "https://www.loxone.com/enen/kb/periphery-references/",
        "Integrated driver plugin for third-party or internal interfaces."
    ),
    tree_schema!(
        "Program",
        "Logic Program Container",
        "Project Organization",
        "https://www.loxone.com/enen/kb/documenting-projects/",
        "Container holding all logic pages of a Miniserver."
    ),
    tree_schema!(
        "PuDe",
        "Push Notification Target Device",
        "Messages",
        "https://www.loxone.com/enen/kb/logger-mailer-caller-tracker/",
        "Registered mobile device token receiving push notifications.",
        &[
            attr!("DevOs", "DevOs", None, ValueKind::Text, Some("ios"), "Periphery XML attribute DevOs"),
            attr!("DevT", "DevT", None, ValueKind::Text, Some("Apple iPad12,1"), "Periphery XML attribute DevT"),
            attr!("PCfg", "PCfg", None, ValueKind::Text, Some("-130"), "Periphery XML attribute PCfg"),
            attr!("Usr", "Usr", None, ValueKind::Text, None, "Periphery XML attribute Usr"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("4210688"), "Visualization and web interface bitflags"),
            attr!("pTkn", "pTkn", None, ValueKind::Text, None, "Periphery XML attribute pTkn"),
        ],
        &[]
    ),
    tree_schema!(
        "RemoteControls",
        "Media / IR Remote Controls",
        "Audio",
        "https://www.loxone.com/enen/kb/ir-control-air/",
        "Container for media and infrared remote control definitions."
    ),
    tree_schema!(
        "RightGroup",
        "Permission Group",
        "Users",
        "https://www.loxone.com/enen/kb/user-management/",
        "Access control permission group."
    ),
    tree_schema!(
        "Second",
        "Current Second",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "System variable outputting the current second of the minute (0..59)."
    ),
    tree_schema!(
        "SecondsBoot",
        "Time Since Boot",
        "System",
        "https://www.loxone.com/enen/kb/system-variables/",
        "Outputs seconds elapsed since the last Miniserver start."
    ),
    tree_schema!(
        "StartPulse",
        "Boot Impulse",
        "System",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "Outputs a single pulse when the Miniserver finishes booting and starts the program."
    ),
    tree_schema!(
        "SubDevices",
        "Sub-Devices Container",
        "Periphery",
        "https://www.loxone.com/enen/kb/tree-extension/",
        "Container grouping channels or sub-devices of an extension or bus branch."
    ),
    tree_schema!(
        "SunAltitude",
        "Solar Elevation Angle",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "Outputs current sun altitude above the horizon in degrees (°)."
    ),
    tree_schema!(
        "SunAzimuth",
        "Solar Compass Direction",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "Outputs current sun compass azimuth in degrees (0°=North, 90°=East, 180°=South, 270°=West)."
    ),
    tree_schema!(
        "Sunrise",
        "Sunrise Time",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "Outputs today's sunrise time in minutes from midnight."
    ),
    tree_schema!(
        "Sunset",
        "Sunset Time",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "Outputs today's sunset time in minutes from midnight."
    ),
    tree_schema!(
        "SwitchingTimer",
        "Scheduled Switching Timer",
        "Times",
        "https://www.loxone.com/enen/kb/schedule/",
        "Time-based switching schedule entry.",
        &[
            attr!("FIX", "FIX", None, ValueKind::Text, Some("1"), "Periphery XML attribute FIX"),
            attr!("M", "M", None, ValueKind::Text, Some("3"), "Periphery XML attribute M"),
            attr!("Modes", "Modes", None, ValueKind::Text, None, "Periphery XML attribute Modes"),
            attr!("N", "N", None, ValueKind::Text, Some("7"), "Periphery XML attribute N"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("16384"), "Visualization and web interface bitflags"),
        ],
        &[]
    ),
    tree_schema!(
        "SysTemp",
        "Miniserver Internal Temperature",
        "System",
        "https://www.loxone.com/enen/kb/system-variables/",
        "Outputs the internal enclosure temperature of the Miniserver (°C)."
    ),
    tree_schema!(
        "SysVar",
        "System Variable",
        "System",
        "https://www.loxone.com/enen/kb/system-variables/",
        "Built-in Miniserver diagnostic or status variable.",
        &[
            attr!("DestValHigh", "DestValHigh", None, ValueKind::Text, Some("10"), "Periphery XML attribute DestValHigh"),
            attr!("EU", "EU", None, ValueKind::Text, Some("1"), "Periphery XML attribute EU"),
            attr!("Fch", "Fch", None, ValueKind::Text, Some("24"), "Periphery XML attribute Fch"),
            attr!("IName", "IName", None, ValueKind::Text, Some("SYS1"), "Periphery XML attribute IName"),
            attr!("MaxVal", "MaxVal", None, ValueKind::Text, Some("150"), "Periphery XML attribute MaxVal"),
            attr!("MinVal", "MinVal", None, ValueKind::Text, Some("-80"), "Periphery XML attribute MinVal"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("3"), "Number of connector pins on block"),
            attr!("OM", "OM", None, ValueKind::Text, Some("false"), "Periphery XML attribute OM"),
            attr!("SourceValHigh", "SourceValHigh", None, ValueKind::Text, Some("10"), "Periphery XML attribute SourceValHigh"),
            attr!("SpStates", "System State References", None, ValueKind::Text, None, "Linked system status UUIDs"),
            attr!("SubType", "SubType", None, ValueKind::Text, Some("1"), "Periphery XML attribute SubType"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("6307856"), "Visualization and web interface bitflags"),
            attr!("source", "source", None, ValueKind::Text, None, "Periphery XML attribute source"),
        ],
        &[
            pin_in!("AI", "AI", "Connector pin AI"),
            pin_out!("AQ", "AQ", "Connector pin AQ"),
            pin_out!("Q", "Q", "Connector pin Q"),
        ]
    ),
    tree_schema!(
        "TaskCaption",
        "System Tasks & Schedules Folder",
        "Times",
        "https://www.loxone.com/enen/kb/schedule/",
        "Container folder for scheduled tasks and switching timers."
    ),
    tree_schema!(
        "Time",
        "Current Time of Day",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "System variable outputting minutes elapsed since midnight (0..1439)."
    ),
    tree_schema!(
        "TimeCaption",
        "Times & Impulses Folder",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "Container folder for time variables, clock pulses, and solar position."
    ),
    tree_schema!(
        "TreeAactor",
        "Loxone Tree Analog Actuator",
        "Periphery",
        "https://www.loxone.com/enen/kb/tree-extension/",
        "Analog actuator output channel on a Loxone Tree device (RGBW 24V Dimmer Tree, Valve Actuator Tree).",
        &[
            attr!("CCap", "CCap", None, ValueKind::Text, Some("24"), "Periphery XML attribute CCap"),
            attr!("Channel", "Channel", None, ValueKind::Text, Some("1"), "Periphery XML attribute Channel"),
            attr!("DestValHigh", "DestValHigh", None, ValueKind::Text, Some("100"), "Periphery XML attribute DestValHigh"),
            attr!("DimmType", "DimmType", None, ValueKind::Text, Some("3"), "Periphery XML attribute DimmType"),
            attr!("IName", "IName", None, ValueKind::Text, Some("AQ0"), "Periphery XML attribute IName"),
            attr!("Idx", "Idx", None, ValueKind::Text, Some("1"), "Periphery XML attribute Idx"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("1"), "Number of connector pins on block"),
            attr!("Percent", "Percent", None, ValueKind::Text, Some("true"), "Periphery XML attribute Percent"),
            attr!("SourceValHigh", "SourceValHigh", None, ValueKind::Text, Some("100"), "Periphery XML attribute SourceValHigh"),
            attr!("SubType", "SubType", None, ValueKind::Text, Some("150"), "Periphery XML attribute SubType"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("20480"), "Visualization and web interface bitflags"),
        ],
        &[
            pin_in!("I", "I", "Connector pin I"),
        ]
    ),
    tree_schema!(
        "TreeActor",
        "Loxone Tree Digital Actuator",
        "Periphery",
        "https://www.loxone.com/enen/kb/tree-extension/",
        "Digital actuator or relay channel on a Loxone Tree device (Nano 2 Relay Tree).",
        &[
            attr!("IName", "IName", None, ValueKind::Text, Some("Q0"), "Periphery XML attribute IName"),
            attr!("Idx", "Idx", None, ValueKind::Text, Some("1"), "Periphery XML attribute Idx"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("1"), "Number of connector pins on block"),
            attr!("SubType", "SubType", None, ValueKind::Text, Some("147"), "Periphery XML attribute SubType"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("20480"), "Visualization and web interface bitflags"),
        ],
        &[
            pin_in!("I", "I", "Connector pin I"),
        ]
    ),
    tree_schema!(
        "TreeAsensor",
        "Loxone Tree Analog Sensor",
        "Periphery",
        "https://www.loxone.com/enen/kb/tree-extension/",
        "Analog sensor channel on a Loxone Tree device (Temperature, Humidity, Brightness, CO2, Power).",
        &[
            attr!("Analog", "Analog", None, ValueKind::Text, Some("true"), "Periphery XML attribute Analog"),
            attr!("DestValHigh", "DestValHigh", None, ValueKind::Text, Some("10"), "Periphery XML attribute DestValHigh"),
            attr!("EnoceanSubType", "EnoceanSubType", None, ValueKind::Text, Some("57"), "Periphery XML attribute EnoceanSubType"),
            attr!("IName", "IName", None, ValueKind::Text, Some("AI1"), "Periphery XML attribute IName"),
            attr!("Idx", "Idx", None, ValueKind::Text, Some("1"), "Periphery XML attribute Idx"),
            attr!("MaxVal", "MaxVal", None, ValueKind::Text, Some("10"), "Periphery XML attribute MaxVal"),
            attr!("MinChange", "MinChange", None, ValueKind::Text, Some("0.25"), "Periphery XML attribute MinChange"),
            attr!("MinTime", "MinTime", None, ValueKind::Text, Some("1000"), "Periphery XML attribute MinTime"),
            attr!("MinVal", "MinVal", None, ValueKind::Text, Some("-40"), "Periphery XML attribute MinVal"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("2"), "Number of connector pins on block"),
            attr!("SourceValHigh", "SourceValHigh", None, ValueKind::Text, Some("10"), "Periphery XML attribute SourceValHigh"),
            attr!("ValOT", "ValOT", None, ValueKind::Text, Some("1"), "Periphery XML attribute ValOT"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("20480"), "Visualization and web interface bitflags"),
        ],
        &[
            pin_out!("AQ", "AQ", "Connector pin AQ"),
            pin_out!("Q", "Q", "Connector pin Q"),
        ]
    ),
    tree_schema!(
        "TreeDevice",
        "Loxone Tree Device",
        "Periphery",
        "https://www.loxone.com/enen/kb/tree-extension/",
        "Hardware device connected to a Loxone Tree branch (Touch Pure Tree, Presence Sensor Tree, RGBW Dimmer Tree, NFC Code Touch Tree)."
    ),
    tree_schema!(
        "TreeSensor",
        "Loxone Tree Digital Sensor",
        "Periphery",
        "https://www.loxone.com/enen/kb/tree-extension/",
        "Digital touch point, motion detector, or contact input on a Loxone Tree device.",
        &[
            attr!("EnoceanSubType", "EnoceanSubType", None, ValueKind::Text, Some("149"), "Periphery XML attribute EnoceanSubType"),
            attr!("IName", "IName", None, ValueKind::Text, Some("I0"), "Periphery XML attribute IName"),
            attr!("Idx", "Idx", None, ValueKind::Text, Some("1"), "Periphery XML attribute Idx"),
            attr!("MaxTime", "MaxTime", None, ValueKind::Text, Some("45000"), "Periphery XML attribute MaxTime"),
            attr!("MinTime", "MinTime", None, ValueKind::Text, Some("10"), "Periphery XML attribute MinTime"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("2"), "Number of connector pins on block"),
            attr!("ValMsgs", "ValMsgs", None, ValueKind::Text, Some("true"), "Periphery XML attribute ValMsgs"),
            attr!("ValOT", "ValOT", None, ValueKind::Text, Some("1"), "Periphery XML attribute ValOT"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("20480"), "Visualization and web interface bitflags"),
        ],
        &[
            pin_out!("Q", "Q", "Connector pin Q"),
            pin_out!("Qe", "Qe", "Connector pin Qe"),
        ]
    ),
    tree_schema!(
        "TreeTextActor",
        "Loxone Tree Text Actuator",
        "Periphery",
        "https://www.loxone.com/enen/kb/touch-nightlight/",
        "Text display actuator on a Loxone Tree device (e.g. Touch Nightlight display)."
    ),
    tree_schema!(
        "User",
        "User Account",
        "Users",
        "https://www.loxone.com/enen/kb/user-management/",
        "Loxone user profile with credentials, access PIN codes, NFC tags, and group memberships."
    ),
    tree_schema!(
        "UserCaption",
        "Users Folder",
        "Users",
        "https://www.loxone.com/enen/kb/user-management/",
        "Container folder for user accounts."
    ),
    tree_schema!(
        "UserGroup",
        "User Group",
        "Users",
        "https://www.loxone.com/enen/kb/user-management/",
        "Role group assigning visualization and access permissions to users."
    ),
    tree_schema!(
        "UserGroupCaption",
        "User Groups Folder",
        "Users",
        "https://www.loxone.com/enen/kb/user-management/",
        "Container folder for user groups."
    ),
    tree_schema!(
        "VirtualInCaption",
        "Virtual Inputs Folder",
        "Virtual I/O",
        "https://www.loxone.com/enen/kb/virtual-inputs-outputs/",
        "Container folder for Virtual Inputs, UDP/HTTP inputs, and Call-In triggers."
    ),
    tree_schema!(
        "VirtualOut",
        "Virtual Output",
        "Virtual I/O",
        "https://www.loxone.com/enen/kb/virtual-inputs-outputs/",
        "IP destination endpoint (HTTP, TCP, UDP, RS232/RS485) for sending commands to external network devices.",
        &[
            attr!("Address", "Address", None, ValueKind::Text, None, "Periphery XML attribute Address"),
            attr!("CloseAfterSend", "CloseAfterSend", None, ValueKind::Text, Some("true"), "Periphery XML attribute CloseAfterSend"),
            attr!("CmdSep", "CmdSep", None, ValueKind::Text, Some(";"), "Periphery XML attribute CmdSep"),
            attr!("IName", "IName", None, ValueKind::Text, Some("VQ2"), "Periphery XML attribute IName"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("16384"), "Visualization and web interface bitflags"),
        ],
        &[]
    ),
    tree_schema!(
        "VirtualOutCaption",
        "Virtual Outputs Folder",
        "Virtual I/O",
        "https://www.loxone.com/enen/kb/virtual-inputs-outputs/",
        "Container folder for Virtual Output endpoints."
    ),
    tree_schema!(
        "VirtualOutCmd",
        "Virtual Output Command",
        "Virtual I/O",
        "https://www.loxone.com/enen/kb/virtual-inputs-outputs/",
        "HTTP/TCP/UDP command template under a Virtual Output endpoint.",
        &[
            attr!("Analog", "Analog", None, ValueKind::Text, Some("true"), "Periphery XML attribute Analog"),
            attr!("CmdOffMethod", "CmdOffMethod", None, ValueKind::Text, Some("1"), "Periphery XML attribute CmdOffMethod"),
            attr!("CmdOn", "CmdOn", None, ValueKind::Text, None, "Periphery XML attribute CmdOn"),
            attr!("CmdOnMethod", "CmdOnMethod", None, ValueKind::Text, Some("1"), "Periphery XML attribute CmdOnMethod"),
            attr!("CmdOnPost", "CmdOnPost", None, ValueKind::Text, Some("<v.3>"), "Periphery XML attribute CmdOnPost"),
            attr!("DestValHigh", "DestValHigh", None, ValueKind::Text, Some("10"), "Periphery XML attribute DestValHigh"),
            attr!("DestValLow", "DestValLow", None, ValueKind::Text, Some("1"), "Periphery XML attribute DestValLow"),
            attr!("IName", "IName", None, ValueKind::Text, Some("VQC3"), "Periphery XML attribute IName"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("1"), "Number of connector pins on block"),
            attr!("Repeat", "Repeat", None, ValueKind::Text, Some("5"), "Periphery XML attribute Repeat"),
            attr!("SourceValHigh", "SourceValHigh", None, ValueKind::Text, Some("10"), "Periphery XML attribute SourceValHigh"),
            attr!("SourceValLow", "SourceValLow", None, ValueKind::Text, Some("1"), "Periphery XML attribute SourceValLow"),
            attr!("Tx", "Text Mode", None, ValueKind::Bool, Some("false"), "Output formatted text instead of numeric value"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("16400"), "Visualization and web interface bitflags"),
        ],
        &[
            pin_in!("I", "I", "Connector pin I"),
        ]
    ),
    tree_schema!(
        "VoltageIn",
        "Analog Input (0-10V)",
        "Periphery",
        "https://www.loxone.com/enen/kb/miniserver/",
        "0-10V DC analog input channel on Miniserver or Extension with linear two-point scaling.",
        &[
            attr!("Analog", "Analog", None, ValueKind::Text, Some("true"), "Periphery XML attribute Analog"),
            attr!("DestValHigh", "DestValHigh", None, ValueKind::Text, Some("10"), "Periphery XML attribute DestValHigh"),
            attr!("IName", "IName", None, ValueKind::Text, Some("AI0"), "Periphery XML attribute IName"),
            attr!("Idx", "Idx", None, ValueKind::Text, Some("1"), "Periphery XML attribute Idx"),
            attr!("MaxVal", "MaxVal", None, ValueKind::Text, Some("10"), "Periphery XML attribute MaxVal"),
            attr!("MinChange", "MinChange", None, ValueKind::Text, Some("0.25"), "Periphery XML attribute MinChange"),
            attr!("MinTime", "MinTime", None, ValueKind::Text, Some("1000"), "Periphery XML attribute MinTime"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("2"), "Number of connector pins on block"),
            attr!("SourceValHigh", "SourceValHigh", None, ValueKind::Text, Some("10"), "Periphery XML attribute SourceValHigh"),
            attr!("ValOT", "ValOT", None, ValueKind::Text, Some("1"), "Periphery XML attribute ValOT"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("20480"), "Visualization and web interface bitflags"),
        ],
        &[
            pin_out!("AQ", "AQ", "Connector pin AQ"),
            pin_out!("Q", "Q", "Connector pin Q"),
        ]
    ),
    tree_schema!(
        "WeatherCaption",
        "Weather Service Folder",
        "Weather",
        "https://www.loxone.com/enen/kb/weather-service/",
        "Container folder for Weather Server and forecast channels."
    ),
    tree_schema!(
        "WeatherData",
        "Weather Data Channel",
        "Weather",
        "https://www.loxone.com/enen/kb/weather-service/",
        "Current or forecast weather value channel (Temperature, Wind Speed, Solar Radiation, Precipitation).",
        &[
            attr!("IName", "IName", None, ValueKind::Text, Some("WDC34"), "Periphery XML attribute IName"),
            attr!("Nio", "Connected Pins Count", None, ValueKind::Int, Some("1"), "Number of connector pins on block"),
            attr!("SLs", "SLs", None, ValueKind::Text, Some("600"), "Periphery XML attribute SLs"),
            attr!("SLw", "SLw", None, ValueKind::Text, Some("120"), "Periphery XML attribute SLw"),
            attr!("WF", "Visualization Flags", None, ValueKind::Int, Some("16384"), "Visualization and web interface bitflags"),
            attr!("WType", "WType", None, ValueKind::Text, Some("34"), "Periphery XML attribute WType"),
        ],
        &[
            pin_out!("AQ", "AQ", "Connector pin AQ"),
        ]
    ),
    tree_schema!(
        "WeatherServer",
        "Weather Service",
        "Weather",
        "https://www.loxone.com/enen/kb/weather-service/",
        "Loxone Cloud Weather Service or Weather Station providing local meteorological data and forecasts."
    ),
    tree_schema!(
        "Week",
        "Calendar Week",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "System variable outputting the current ISO calendar week number (1..53)."
    ),
    tree_schema!(
        "Year",
        "Current Year",
        "Times",
        "https://www.loxone.com/enen/kb/times-impulses/",
        "System variable outputting the current 4-digit year."
    ),
];

pub fn lookup_block_schema(type_name: &str) -> Option<&'static BlockSchema> {
    for slice in [
        BLOCK_SCHEMAS,
        PAGE_SCHEMAS_2,
        PAGE_SCHEMAS_3,
        TREE_SCHEMAS_1,
        TREE_SCHEMAS_2,
    ] {
        if let Some(found) = slice.iter().find(|s| s.type_name == type_name) {
            return Some(found);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::load_loxone_file;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    #[test]
    fn test_klimczaki_100_percent_block_and_tree_schema_coverage() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("parse Klimczaki.Loxone");

        let mut page_types = BTreeSet::new();
        for page in &project.pages {
            for block in &page.blocks {
                page_types.insert(block.block_type.clone());
            }
        }
        assert_eq!(
            page_types.len(),
            41,
            "Klimczaki.Loxone has 41 unique page block types"
        );

        for ty in &page_types {
            let schema = lookup_block_schema(ty)
                .unwrap_or_else(|| panic!("Missing KB schema for page block type: {ty}"));
            assert_eq!(schema.type_name, ty);
            assert!(
                !schema.display_name_en.is_empty(),
                "Empty display name for {ty}"
            );
            assert!(
                schema.kb_url.starts_with("https://www.loxone.com/enen/kb/"),
                "Invalid KB URL for {ty}: {}",
                schema.kb_url
            );
            assert!(!schema.summary.is_empty(), "Empty summary for {ty}");
        }

        fn collect_tree_types(nodes: &[crate::model::TreeNode], out: &mut BTreeSet<String>) {
            for n in nodes {
                if !n.control_type.is_empty() {
                    out.insert(n.control_type.clone());
                }
                collect_tree_types(&n.children, out);
            }
        }
        let mut tree_types = BTreeSet::new();
        collect_tree_types(&project.periphery_tree, &mut tree_types);
        collect_tree_types(&project.system_tree, &mut tree_types);

        for ty in &tree_types {
            let schema = lookup_block_schema(ty).unwrap_or_else(|| {
                panic!("Missing KB schema for periphery/system tree type: {ty}")
            });
            assert_eq!(schema.type_name, ty);
            assert!(
                schema.kb_url.starts_with("https://www.loxone.com/enen/kb/"),
                "Invalid KB URL for tree type {ty}: {}",
                schema.kb_url
            );
        }
    }

    #[test]
    fn test_kb_parameter_and_attribute_metadata_details() {
        let lc2 = lookup_block_schema("LightController2").unwrap();
        assert_eq!(lc2.display_name_en, "Lighting Controller");
        assert!(lc2
            .pin_params
            .iter()
            .any(|p| p.pin_key == "Step" && p.unit == Some("%")));
        assert!(lc2.pin_params.iter().any(|p| p.pin_key == "Mt"));
        assert!(lc2.child_configs.contains(&ChildConfigKind::Hcl));

        let jal = lookup_block_schema("AutoJalousie").unwrap();
        assert!(jal
            .pin_params
            .iter()
            .any(|p| p.pin_key == "Tu" && p.unit == Some("s")));
        assert!(jal
            .pin_params
            .iter()
            .any(|p| p.pin_key == "Td" && p.unit == Some("s")));
        assert!(jal.child_configs.contains(&ChildConfigKind::Psd));

        let irc = lookup_block_schema("HeatIRoomController2").unwrap();
        assert!(irc
            .attributes
            .iter()
            .any(|a| a.xml_attr == "UseH" && a.unit == Some("°C")));
        assert!(irc
            .pin_params
            .iter()
            .any(|p| p.pin_key == "Tc" && p.unit == Some("°C")));

        let lm = lookup_block_schema("OutputRefLM").unwrap();
        assert!(lm.child_configs.contains(&ChildConfigKind::LoggerMailer));

        // Verify helper methods find_attr, find_pin_param, and find_pin_doc (exact + prefix match)
        assert_eq!(irc.find_attr("UseH").unwrap().unit, Some("°C"));
        assert_eq!(lc2.find_pin_param("Step").unwrap().default_value, "10");
        assert_eq!(lc2.find_pin_doc("T").unwrap().label, "Touch");
        assert_eq!(
            lc2.find_pin_doc("AQ7").unwrap().label,
            "Lighting Circuit Output"
        );
        assert_eq!(lc2.find_pin_doc("I3").unwrap().label, "Direct Mood Input");
    }

    #[test]
    fn test_klimczaki_100_percent_pin_param_and_attr_coverage() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Klimczaki.Loxone");
        let project = load_loxone_file(&path).expect("parse Klimczaki.Loxone");

        let base_attrs = [
            "U", "Type", "V", "Title", "Desc", "Px", "Py", "Px2", "Py2", "Cl", "Visu", "VisuPwd",
            "UseInFav", "Rating",
        ];

        for page in &project.pages {
            for block in &page.blocks {
                let schema = lookup_block_schema(&block.block_type).unwrap();
                for (attr_k, _) in &block.raw_attributes {
                    if !base_attrs.contains(&attr_k.as_str()) {
                        assert!(
                            schema.find_attr(attr_k).is_some(),
                            "Missing AttributeSchema for attr `{attr_k}` on page block type `{}`",
                            block.block_type
                        );
                    }
                }
                for pin in &block.pins {
                    if pin.default_value.is_some() {
                        assert!(
                            schema.find_pin_param(&pin.key).is_some(),
                            "Missing PinParamSchema for `<Co K=\"{}\" Def=...>` on page block type `{}`",
                            pin.key,
                            block.block_type
                        );
                    } else {
                        assert!(
                            schema.find_pin_doc(&pin.key).is_some()
                                || schema.find_pin_param(&pin.key).is_some(),
                            "Missing PinDoc/PinParamSchema for `<Co K=\"{}\">` on page block type `{}`",
                            pin.key,
                            block.block_type
                        );
                    }
                }
            }
        }

        // Verify Door vs Doorcontroller swapped semantics & attributes/pins
        let door = lookup_block_schema("Door").unwrap();
        assert_eq!(door.display_name_en, "Gate & Door Controller");
        assert!(door.find_attr("CtrlType").is_some());
        assert!(door.find_attr("T5Side").is_some());
        assert!(door.find_pin_param("PulseOpen").is_some());
        assert!(door.find_pin_param("PulseClose").is_some());
        assert!(door.find_pin_doc("OutputOpen").is_some());
        assert!(door.find_pin_doc("OutputClose").is_some());

        let door_ctrl = lookup_block_schema("Doorcontroller").unwrap();
        assert_eq!(door_ctrl.display_name_en, "Access Door Controller");
        assert!(door_ctrl.find_attr("DoorNumber").is_some());
        assert!(door_ctrl.find_attr("OutName1").is_some());
        assert!(door_ctrl.find_attr("Ref").is_some());
        assert!(door_ctrl.find_pin_param("Timeout").is_some());
        assert!(door_ctrl.find_pin_doc("TriggerFan").is_some());
        assert!(door_ctrl.find_pin_doc("Qb").is_some());

        // Verify typo fixes
        let lc2 = lookup_block_schema("LightController2").unwrap();
        assert!(lc2.find_pin_doc("DisM").is_some());
        let lm = lookup_block_schema("OutputRefLM").unwrap();
        assert!(!lm.summary.contains("\u{8bed}\u{97f3}"));
        assert!(lm.summary.contains("voice"));
    }
}
