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
        self.pin_params.iter().find(|p| p.pin_key == pin_key)
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
        ],
        &[
            param!("Ta", "Pre-alarm delay", Some("s"), ValueKind::DurationSeconds, "30", "Delay before emergency alarm triggers"),
            param!("Tr", "Repeat interval", Some("s"), ValueKind::DurationSeconds, "60", "Alarm repeat interval"),
        ],
        &[
            pin_in!("T", "Trigger", "Emergency button trigger input"),
            pin_in!("Dis", "Disable", "Disables the emergency alarm"),
            pin_in!("R", "Reset", "Resets active alarm"),
            pin_out!("A", "Alarm", "Active emergency alarm output"),
            pin_out!("S", "Status", "Current alarm status"),
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
        ],
        &[
            param!("Tmo", "Monitoring time", Some("min"), ValueKind::DurationMinutes, "60", "Max allowed inactivity duration"),
        ],
        &[
            pin_in!("En", "Enable", "Enables smart monitoring"),
            pin_in!("Mo", "Motion", "Motion or activity detector input"),
            pin_in!("R", "Reset", "Resets triggered smart alarm"),
            pin_out!("A", "Alarm", "Smart alarm output"),
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
        ],
        &[
            param!("T1", "Stage 1 duration", Some("s"), ValueKind::DurationSeconds, "60", "Duration of escalation stage 1"),
            param!("T2", "Stage 2 duration", Some("s"), ValueKind::DurationSeconds, "60", "Duration of escalation stage 2"),
            param!("T3", "Stage 3 duration", Some("s"), ValueKind::DurationSeconds, "60", "Duration of escalation stage 3"),
        ],
        &[
            pin_in!("En", "Enable", "Starts the alarm escalation sequence"),
            pin_in!("Next", "Next Stage", "Advances immediately to the next stage"),
            pin_in!("Q", "Acknowledge", "Acknowledges and stops escalation"),
            pin_out!("O", "Stage Output", "Sequential escalation outputs O1..O10"),
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
        ],
        &[
            param!("Ta", "Buzzer duration", Some("s"), ValueKind::DurationSeconds, "60", "Duration of main alarm output"),
            param!("Ts", "Snooze time", Some("s"), ValueKind::DurationSeconds, "540", "Snooze interval in seconds"),
            param!("Tp", "Pre-alarm lead time", Some("s"), ValueKind::DurationSeconds, "600", "Lead time for pre-alarm output Qp"),
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
        ],
        &[],
        &[
            pin_in!("T", "Touch", "Central touch input (toggles linked lighting controllers)"),
            pin_in!("O", "On", "Turns all linked lighting controllers on"),
            pin_in!("R", "Reset / Off", "Turns all linked lighting controllers off"),
            pin_in!("S", "Scene", "Selects scene number across linked controllers"),
            pin_in!("Dis", "Selective Disable", "Disables central commands for specific linked block Dis1..Dis16"),
            pin_out!("Q", "Status Output", "High when any linked lighting controller is active"),
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
        ],
        &[],
        &[
            pin_in!("Play", "Play", "Starts playback on linked zones"),
            pin_in!("Pause", "Pause", "Pauses playback on linked zones"),
            pin_in!("R", "Off", "Turns off all linked audio zones"),
            pin_out!("Q", "Active Status", "High when any linked player is active"),
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
        ],
        &[
            param!("T", "Switchover delay", Some("s"), ValueKind::DurationSeconds, "300", "Minimum delay before switching heating/cooling"),
        ],
        &[
            pin_in!("En", "Enable", "Enables climate demand aggregation"),
            pin_in!("To", "Outdoor Temperature", "Outdoor ambient temperature (°C)"),
            pin_out!("H", "Heating Demand", "Active heating demand output"),
            pin_out!("C", "Cooling Demand", "Active cooling demand output"),
            pin_out!("Tf", "Flow Temperature Setpoint", "Required supply flow temperature (°C)"),
        ],
        &[ChildConfigKind::IoData, ChildConfigKind::CoHist]
    ),
    schema!(
        "Door",
        "Door & Window Monitor",
        "Security & Safety",
        "https://www.loxone.com/enen/kb/window-door-monitor/",
        "Monitors door and window reed contacts, tilt sensors, and lock bolts with visualization and open-duration alerts.",
        &[
            attr!("I", "Connected Contacts Mask", None, ValueKind::Int, Some("1"), "Active contact inputs"),
            attr!("N", "Inverted Contacts Mask", None, ValueKind::Int, Some("0"), "Normally closed/open polarity mask"),
        ],
        &[],
        &[
            pin_in!("I", "Contact Input", "Reed or tilt contact input I1..I8"),
            pin_out!("Q", "Any Open", "High when any monitored door or window is open"),
            pin_out!("N", "Number Open", "Count of currently open contacts"),
        ],
        &[ChildConfigKind::IoData, ChildConfigKind::LoggerMailer]
    ),
    schema!(
        "Doorcontroller",
        "Door & Gate Controller",
        "Access",
        "https://www.loxone.com/enen/kb/gate-controller/",
        "Controls garage doors, swing gates, sliding gates, and electric door strikes with end-stop and safety monitoring.",
        &[
            attr!("Type", "Door/Gate Type", None, ValueKind::Int, Some("0"), "0=Door opener, 1=Gate, 2=Up-and-over garage door"),
        ],
        &[
            param!("To", "Opening run time", Some("s"), ValueKind::DurationSeconds, "15", "Full gate opening duration"),
            param!("Tc", "Closing run time", Some("s"), ValueKind::DurationSeconds, "15", "Full gate closing duration"),
            param!("Tp", "Pulse duration", Some("s"), ValueKind::DurationSeconds, "1", "Relay pulse duration for impulse drives"),
            param!("Tw", "Pre-warning time", Some("s"), ValueKind::DurationSeconds, "0", "Warning flash duration before movement"),
        ],
        &[
            pin_in!("Ui", "Trigger", "Single-button open/stop/close trigger"),
            pin_in!("O", "Open", "Commands door/gate to open"),
            pin_in!("C", "Close", "Commands door/gate to close"),
            pin_in!("Co", "Contact Open", "Limit switch indicating fully open"),
            pin_in!("Cc", "Contact Closed", "Limit switch indicating fully closed"),
            pin_in!("Sp", "Safety Barrier", "Photocell or safety edge input"),
            pin_out!("Qo", "Output Open", "Motor/strike open relay output"),
            pin_out!("Qc", "Output Close", "Motor close relay output"),
            pin_out!("Po", "Position", "Estimated door/gate position (0..100%)"),
        ],
        &[ChildConfigKind::IoData, ChildConfigKind::CoHist]
    ),
    schema!(
        "EFM",
        "Energy Flow Monitor",
        "Energy",
        "https://www.loxone.com/enen/kb/energy-flow-monitor/",
        "Visualizes real-time power flow between grid, solar PV, battery storage, and household consumers.",
        &[
            attr!("UO", "Linked Meters", None, ValueKind::Text, None, "UUIDs of linked Meter blocks"),
        ],
        &[],
        &[
            pin_in!("P", "Power Input", "Active power input (kW)"),
            pin_out!("Pg", "Grid Power", "Current grid import/export power (kW)"),
            pin_out!("Pc", "Consumption", "Total household power consumption (kW)"),
        ],
        &[ChildConfigKind::IoData]
    ),
    schema!(
        "Equal",
        "Equals Comparator",
        "Logic",
        "https://www.loxone.com/enen/kb/comparators/",
        "Outputs 1 when input V1 equals input V2.",
        &[],
        &[
            param!("V1", "Value 1", None, ValueKind::Float, "0", "First comparison value"),
            param!("V2", "Value 2", None, ValueKind::Float, "0", "Second comparison value"),
        ],
        &[
            pin_in!("V1", "Value 1", "First analog value"),
            pin_in!("V2", "Value 2", "Second analog value"),
            pin_out!("Q", "Equal Output", "1 when V1 == V2"),
            pin_out!("Qn", "Not Equal Output", "1 when V1 != V2"),
        ],
        &[]
    ),
    schema!(
        "HVACController",
        "HVAC Controller",
        "Climate",
        "https://www.loxone.com/enen/kb/hvac-controller/",
        "Controls heat pumps, boilers, chillers, and buffer tanks based on building heating and cooling requests.",
        &[],
        &[
            param!("To", "Outdoor cutoff temp", Some("°C"), ValueKind::TemperatureCelsius, "18", "Heating cutoff outdoor temperature"),
        ],
        &[
            pin_in!("H", "Heating Request", "Heating demand input"),
            pin_in!("C", "Cooling Request", "Cooling demand input"),
            pin_in!("To", "Outdoor Temp", "Outdoor temperature sensor (°C)"),
            pin_out!("Qh", "Heating Enable", "Enables heat source"),
            pin_out!("Qc", "Cooling Enable", "Enables cooling source"),
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
        ],
        &[ChildConfigKind::IoData, ChildConfigKind::CoHist]
    ),
    schema!(
        "Heatmixer2",
        "Mixing Valve Controller",
        "Climate",
        "https://www.loxone.com/enen/kb/mixing-valve-controller/",
        "PI controller regulating a 3-way motorised mixing valve and circulation pump to maintain target flow temperature.",
        &[],
        &[
            param!("Runtime", "Actuator run time", Some("s"), ValueKind::DurationSeconds, "120", "Full 0..100% travel time of motorised valve"),
            param!("Min", "Min flow temperature", Some("°C"), ValueKind::TemperatureCelsius, "20", "Minimum allowed supply flow temperature"),
            param!("Max", "Max flow temperature", Some("°C"), ValueKind::TemperatureCelsius, "45", "Maximum allowed supply flow temperature"),
            param!("MaxOff", "Overtemp shutdown", Some("°C"), ValueKind::TemperatureCelsius, "55", "Safety shutoff temperature for pump"),
            param!("Inter", "Control interval", Some("s"), ValueKind::DurationSeconds, "20", "PI sampling and adjustment interval"),
            param!("Gain", "Proportional gain Kp", None, ValueKind::Float, "1.0", "PI proportional gain factor"),
            param!("Tint", "Integral time Ti", Some("s"), ValueKind::DurationSeconds, "120", "PI integral reset time"),
        ],
        &[
            pin_in!("T", "Target Flow Temp", "Desired supply flow temperature (°C)"),
            pin_in!("ti", "Actual Flow Temp", "Measured supply flow temperature sensor (°C)"),
            pin_in!("Dis", "Disable", "Disables mixer and closes valve"),
            pin_out!("Qu", "Mixer Open", "Relay output driving mixing valve open (warmer)"),
            pin_out!("Qd", "Mixer Close", "Relay output driving mixing valve closed (cooler)"),
            pin_out!("Qp", "Circulation Pump", "Enables heating circuit circulation pump"),
            pin_out!("AQ", "Analog Valve Pos", "Calculated 0..100% mixing valve position"),
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
        ],
        &[],
        &[
            pin_out!("Q", "Digital Output", "Digital state of the referenced input"),
            pin_out!("AQ", "Analog Output", "Analog value of the referenced sensor/input"),
            pin_out!("On", "Online Status", "Device online state"),
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
        ],
        &[
            param!("Tb", "Bell pulse duration", Some("s"), ValueKind::DurationSeconds, "3", "Duration of bell output Qbell"),
        ],
        &[
            pin_in!("Bell", "Doorbell Input", "Triggers doorbell ring and app push notification"),
            pin_in!("Px", "Proximity", "Wakes up Intercom display / illumination"),
            pin_out!("Qb", "Bell Output", "Active when doorbell button is pressed"),
            pin_out!("Q1", "Gen. Button 1 / Door", "Programmable UI button 1 (typically door opener)"),
            pin_out!("Q2", "Gen. Button 2 / Light", "Programmable UI button 2"),
            pin_out!("Q3", "Gen. Button 3 / Gate", "Programmable UI button 3"),
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
            pin_in!(" DisM", "Disable Motion", "Temporarily disables motion automation"),
            pin_in!("Al", "Alarm", "Flashes room lights during burglar/fire alarm"),
            pin_out!("AQ", "Lighting Circuit Output", "Circuit output AQ1..AQ20 (0..100%, digital 0/1, or Smart Actuator)"),
            pin_out!("ActiveScene", "Active Mood", "Currently active mood number (0 = Off)"),
            pin_out!("Chg", "Mood Changed", "Pulses when active mood changes"),
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
        &[],
        &[
            param!("Tl", "Long-click threshold", Some("s"), ValueKind::DurationSeconds, "0.8", "Minimum press time to trigger Ql"),
            param!("Tp", "Output pulse duration", Some("s"), ValueKind::DurationSeconds, "0.5", "Duration of output pulse on Q/Ql"),
        ],
        &[
            pin_in!("T", "Trigger", "Pushbutton digital input"),
            pin_out!("Q", "Short Click Output", "Pulses when button is released before Tl"),
            pin_out!("Ql", "Long Click Output", "Triggers when button is held longer than Tl"),
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
        ],
        &[],
        &[
            pin_in!("I", "Input", "Writes value to the memory flag"),
            pin_out!("Q", "Digital Output", "Reads digital value of the memory flag"),
            pin_out!("AQ", "Analog Output", "Reads analog value of the memory flag"),
        ],
        &[ChildConfigKind::IoData]
    ),
    schema!(
        "MeterAbsBi",
        "Bidirectional Energy Meter",
        "Energy",
        "https://www.loxone.com/enen/kb/energy-meter/",
        "Records bidirectional energy import/export (kWh) and active power (kW) with tariff and statistics logging.",
        &[],
        &[
            param!("Ir", "Impulse rate", None, ValueKind::Float, "1000", "Pulses per kWh for S0 pulse inputs"),
        ],
        &[
            pin_in!("P", "Active Power", "Instantaneous power (+import / -export in kW)"),
            pin_in!("Mr", "Meter Reading Import", "Cumulative imported energy reading (kWh)"),
            pin_in!("Mrd", "Meter Reading Export", "Cumulative exported energy reading (kWh)"),
            pin_in!("R", "Reset Statistics", "Resets period counters"),
            pin_out!("AQp", "Power Output", "Current active power (kW)"),
            pin_out!("AQc", "Total Import", "Total imported energy (kWh)"),
            pin_out!("AQd", "Total Export", "Total exported energy (kWh)"),
        ],
        &[ChildConfigKind::IoData]
    ),
    schema!(
        "MeterPUni",
        "Unidirectional Utility Meter",
        "Energy",
        "https://www.loxone.com/enen/kb/utility-meter/",
        "Records unidirectional electricity, water, gas, or heat consumption and flow/power rate.",
        &[],
        &[
            param!("Ir", "Impulse rate", None, ValueKind::Float, "1000", "Pulses per unit (kWh / m³)"),
        ],
        &[
            pin_in!("P", "Power / Flow", "Current power (kW) or flow rate (m³/h)"),
            pin_in!("Mr", "Meter Reading", "Absolute cumulative meter reading"),
            pin_in!("R", "Reset", "Resets day/month/year statistics"),
            pin_out!("AQp", "Power / Flow Output", "Current power or flow rate"),
            pin_out!("AQc", "Total Consumption", "Total cumulative consumption"),
        ],
        &[ChildConfigKind::IoData]
    ),
    schema!(
        "MultiClick",
        "Multi-Click",
        "Pushbuttons",
        "https://www.loxone.com/enen/kb/multi-click/",
        "Counts rapid consecutive button presses (1x, 2x, 3x, 4x) and activates the corresponding output Q1..Q4.",
        &[],
        &[
            param!("T", "Multi-click window", Some("s"), ValueKind::DurationSeconds, "0.6", "Max time window for multiple clicks"),
            param!("Tp", "Output pulse duration", Some("s"), ValueKind::DurationSeconds, "0.5", "Duration of output pulse on Q1..Q4"),
        ],
        &[
            pin_in!("Tr", "Trigger", "Pushbutton input"),
            pin_out!("Q1", "Single Click", "Pulses on 1x click"),
            pin_out!("Q2", "Double Click", "Pulses on 2x double-click"),
            pin_out!("Q3", "Triple Click", "Pulses on 3x click"),
            pin_out!("Q4", "Quadruple Click", "Pulses on 4x click"),
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
        ],
        &[
            param!("V", "Default volume", Some("%"), ValueKind::Percent, "35", "Startup / default playback volume"),
            param!("MaxV", "Max volume", Some("%"), ValueKind::Percent, "80", "Maximum allowed zone volume"),
            param!("Step", "Volume step", Some("%"), ValueKind::Percent, "5", "Volume adjustment step per click"),
            param!("Vb", "Bell event volume", Some("%"), ValueKind::Percent, "50", "Volume for doorbell event chime"),
            param!("Va", "Alarm event volume", Some("%"), ValueKind::Percent, "70", "Volume for alarm event tone"),
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
        ],
        &[
            param!("Tq", "Output pulse duration", Some("s"), ValueKind::DurationSeconds, "3", "Pulse duration on Q1..Q6 upon valid authentication"),
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
        ],
        &[ChildConfigKind::IoData, ChildConfigKind::CoHist]
    ),
    schema!(
        "Not",
        "NOT Inverter",
        "Logic",
        "https://www.loxone.com/enen/kb/not/",
        "Inverts a digital signal: outputs 1 when input I is 0, and 0 when input I is 1.",
        &[],
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
        &[],
        &[
            param!("Td", "Switch-on delay", Some("s"), ValueKind::DurationSeconds, "1", "Delay before output pulse begins"),
            param!("Tp", "Pulse duration", Some("s"), ValueKind::DurationSeconds, "1", "Duration of the output pulse on Q"),
        ],
        &[
            pin_in!("Tr", "Trigger", "Starts delay timer on rising edge"),
            pin_in!("R", "Reset", "Cancels pending delay or active pulse"),
            pin_out!("Q", "Pulse Output", "High for Tp seconds after delay Td expires"),
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
        ],
        &[],
        &[
            pin_in!("I", "Actuator Input", "Drives the referenced digital or analog output"),
            pin_out!("Q", "Feedback Output", "Current state of the referenced output"),
        ],
        &[ChildConfigKind::IoData]
    ),
    schema!(
        "OutputRefLM",
        "Logger / Mailer / Notification Output",
        "Periphery References",
        "https://www.loxone.com/enen/kb/logger-mailer-caller-tracker/",
        "Sends push notifications, emails,语音 calls, or log entries when triggered by digital or analog events.",
        &[
            attr!("Ref", "Referenced Service UUID", None, ValueKind::UuidRef, None, "UUID of referenced Logger/Mailer/Notification node"),
        ],
        &[],
        &[
            pin_in!("I", "Trigger / Value", "Triggers message transmission on rising/falling edge or value change"),
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
        ],
        &[],
        &[
            pin_in!("T", "Trigger / Toggle", "Toggles output Q on rising edge"),
            pin_in!("O", "On", "Sets output Q to 1"),
            pin_in!("R", "Reset / Off", "Sets output Q to 0"),
            pin_in!("Dis", "Disable", "Disables button operation"),
            pin_out!("Q", "Output", "Current switched state (0 or 1)"),
            pin_out!("Qn", "Inverted Output", "Inverted switched state"),
        ],
        &[ChildConfigKind::IoData]
    ),
    schema!(
        "PresenceDetector",
        "Presence Controller",
        "Presence",
        "https://www.loxone.com/enen/kb/presence-controller/",
        "Combines motion, acoustic, and door sensors to determine room occupancy with configurable overrun time.",
        &[],
        &[
            param!("To", "Overrun time", Some("s"), ValueKind::DurationSeconds, "180", "Duration presence remains active after last motion"),
        ],
        &[
            pin_in!("Mo", "Motion / Presence", "Motion or acoustic presence sensor input"),
            pin_in!("R", "Reset", "Immediately clears presence state"),
            pin_in!("Dis", "Disable", "Disables presence detection"),
            pin_out!("Q", "Presence Active", "High while room is occupied"),
        ],
        &[ChildConfigKind::IoData]
    ),
    schema!(
        "PulseAt",
        "Edge Trigger (Pulse At)",
        "Times",
        "https://www.loxone.com/enen/kb/edge-trigger/",
        "Generates a fixed-duration pulse on rising edge, falling edge, or both edges of input Tr.",
        &[],
        &[
            param!("Tp", "Pulse duration", Some("s"), ValueKind::DurationSeconds, "0.5", "Length of generated output pulse"),
            param!("M", "Edge mode", None, ValueKind::Int, "1", "1=Rising edge, 2=Falling edge, 3=Both edges"),
        ],
        &[
            pin_in!("Tr", "Trigger", "Monitored digital signal"),
            pin_out!("Q", "Pulse Output", "Emits pulse of duration Tp on configured edge"),
        ],
        &[]
    ),
    schema!(
        "PulseBy",
        "Interval Pulse Generator",
        "Times",
        "https://www.loxone.com/enen/kb/pulse-generator/",
        "Generates periodic pulses (clock signal) with configurable high time Th and low time Tl while enabled.",
        &[],
        &[
            param!("Th", "Pulse high duration", Some("s"), ValueKind::DurationSeconds, "1", "Duration output Q stays 1"),
            param!("Tl", "Pause low duration", Some("s"), ValueKind::DurationSeconds, "1", "Duration output Q stays 0 between pulses"),
        ],
        &[
            pin_in!("En", "Enable", "Enables cyclic pulse generation while high"),
            pin_out!("Q", "Clock Output", "Periodic pulse output"),
        ],
        &[]
    ),
    schema!(
        "PushButton",
        "Virtual Pushbutton",
        "Pushbuttons",
        "https://www.loxone.com/enen/kb/pushbutton/",
        "Momentary or toggle pushbutton for app visualization and logic triggering.",
        &[],
        &[
            param!("Tp", "Pulse duration", Some("s"), ValueKind::DurationSeconds, "0.5", "Momentary pulse width"),
        ],
        &[
            pin_in!("T", "Trigger", "Triggers pushbutton pulse"),
            pin_in!("Dis", "Disable", "Disables button"),
            pin_out!("Q", "Output", "Momentary pulse output"),
            pin_out!("Qp", "Plus Output", "Up / Plus button output"),
            pin_out!("Qm", "Minus Output", "Down / Minus button output"),
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
        ],
        &[
            param!("Ta", "Pre-alarm duration", Some("s"), ValueKind::DurationSeconds, "30", "Delay before full fire/water alarm escalates"),
        ],
        &[
            pin_in!("I", "Detector Input", "Smoke or water leak sensor input I1..I8"),
            pin_in!("Q", "Acknowledge", "Acknowledges and silences active alarm"),
            pin_in!("Dis", "Disable", "Temporarily disables alarm inputs"),
            pin_out!("O", "Alarm Stage Output", "Staged emergency outputs O1..O6 (Sirens, Escape Light, Shades Up, Main Water Valve)"),
            pin_out!("Al", "Alarm Level", "Active alarm severity level"),
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
        ],
        &[],
        &[
            pin_in!("AI", "Condition Input", "Digital or analog input AI1..AI8 evaluated by status rules"),
            pin_out!("AQ", "Status Value", "Numeric value of the highest-priority matching status rule"),
            pin_out!("TQ", "Status Text", "Formatted text of the active status rule"),
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
        ],
        &[],
        &[
            pin_out!("Q", "Active Warning", "High when at least one monitored status is active"),
            pin_out!("TQ", "Status Summary", "Combined status text"),
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
        &[attr!("Chn", "Hardware Channel", None, ValueKind::Int, Some("1"), "Relay output channel number")],
        &[pin_in!("I", "Actuator Input", "Digital switch signal (0/1)")]
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
        "Cloud text-to-speech telephone call service for critical alarm notifications."
    ),
    tree_schema!(
        "CallerVirtualIn",
        "Call-In Virtual Input",
        "Virtual I/O",
        "https://www.loxone.com/enen/kb/virtual-inputs-outputs/",
        "Virtual input triggered via incoming cloud caller authentication."
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
        &[attr!("Chn", "Input Channel", None, ValueKind::Int, Some("1"), "Hardware digital input channel")],
        &[pin_out!("Q", "Input State", "Digital 0/1 state of the input")]
    ),
    tree_schema!(
        "Dimmer",
        "Dimmer Channel",
        "Periphery",
        "https://www.loxone.com/enen/kb/dimmer-extension/",
        "Leading/trailing-edge phase dimmer output channel (0..100%) on a Dimmer Extension.",
        &[attr!("Chn", "Dimmer Channel", None, ValueKind::Int, Some("1"), "Hardware dimmer channel")],
        &[pin_in!("AQ", "Brightness Input", "Target dimmer brightness (0..100%)")]
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
        "Analog actuator output channel on a Loxone Air wireless device."
    ),
    tree_schema!(
        "LoxAIRAsensor",
        "Loxone Air Analog Sensor",
        "Periphery",
        "https://www.loxone.com/enen/kb/air-base-extension/",
        "Analog sensor reading (temperature, humidity, brightness, battery) from a Loxone Air device."
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
        "Digital relay or switch channel on a Loxone Air device."
    ),
    tree_schema!(
        "LoxAIRsensor",
        "Loxone Air Digital Sensor",
        "Periphery",
        "https://www.loxone.com/enen/kb/air-base-extension/",
        "Digital input, touch button, or reed contact on a Loxone Air device."
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
        "Polled Modbus RTU/TCP holding or input register reading."
    ),
    tree_schema!(
        "ModbusDev",
        "Modbus Slave Device",
        "Periphery",
        "https://www.loxone.com/enen/kb/modbus-extension/",
        "External Modbus RTU or Modbus TCP slave device (energy meter, inverter, heat pump, ventilation)."
    ),
    tree_schema!(
        "Mode",
        "Operating Mode",
        "Times",
        "https://www.loxone.com/enen/kb/operating-modes/",
        "Global building operating mode (e.g. Holiday, Public Holiday, Heating Season, Party, Vacation)."
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
        "Sends push notifications to iOS/Android Loxone App users."
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
        "Registered mobile device token receiving push notifications."
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
        "Time-based switching schedule entry."
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
        "Built-in Miniserver diagnostic or status variable."
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
        "Analog actuator output channel on a Loxone Tree device (RGBW 24V Dimmer Tree, Valve Actuator Tree)."
    ),
    tree_schema!(
        "TreeActor",
        "Loxone Tree Digital Actuator",
        "Periphery",
        "https://www.loxone.com/enen/kb/tree-extension/",
        "Digital actuator or relay channel on a Loxone Tree device (Nano 2 Relay Tree)."
    ),
    tree_schema!(
        "TreeAsensor",
        "Loxone Tree Analog Sensor",
        "Periphery",
        "https://www.loxone.com/enen/kb/tree-extension/",
        "Analog sensor channel on a Loxone Tree device (Temperature, Humidity, Brightness, CO2, Power)."
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
        "Digital touch point, motion detector, or contact input on a Loxone Tree device."
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
        &[attr!("Address", "Target URI / Address", None, ValueKind::Text, None, "Target TCP/UDP/HTTP endpoint")],
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
            attr!("CmdOn", "Command for ON", None, ValueKind::Text, None, "Command string or URI sent when triggered"),
            attr!("CmdOff", "Command for OFF", None, ValueKind::Text, None, "Command string or URI sent when switched off"),
        ],
        &[pin_in!("I", "Command Input", "Triggers or formats the outgoing command")]
    ),
    tree_schema!(
        "VoltageIn",
        "Analog Input (0-10V)",
        "Periphery",
        "https://www.loxone.com/enen/kb/miniserver/",
        "0-10V DC analog input channel on Miniserver or Extension with linear two-point scaling.",
        &[attr!("Chn", "Analog Channel", None, ValueKind::Int, Some("1"), "Hardware 0-10V channel number")],
        &[pin_out!("AQ", "Scaled Analog Value", "Scaled sensor value")]
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
        "Current or forecast weather value channel (Temperature, Wind Speed, Solar Radiation, Precipitation)."
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
        assert_eq!(
            lc2.find_pin_doc("I3").unwrap().label,
            "Direct Mood Input"
        );
    }
}
