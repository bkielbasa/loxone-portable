use egui::{Align2, Color32, FontFamily, FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2};
use loxone_core::block_schema::lookup_block_schema;
use loxone_core::model::TreeNode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoxoneIcon {
    PushNotifications,
    WeatherServer,
    AudioFolder,
    AudioServer,
    Trusts,
    Miniserver,
    DigitalInput,
    AnalogInput,
    DigitalOutput,
    AnalogOutput,
    VirtualInput,
    VirtualOutput,
    Intercom,
    NetworkPeriphery,
    MessagesLog,
    LightingGroup,
    SdCard,
    LoxoneLink,
    LoxoneTree,
    ExtensionBadge(&'static str),
    MemoryFlag,
    Constant,
    OperatingMode,
    TimeFunction,
    Shading,
    Climate,
    Security,
    Energy,
    LogicGate,
    Page,
    Room,
    Category(Option<[u8; 3]>),
    StatusOnline,
    GenericBlock,
}

/// Looks up an attribute value from `TreeNode::raw_attributes`.
pub fn tree_node_attr<'a>(node: &'a TreeNode, key: &str) -> Option<&'a str> {
    node.raw_attributes
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

/// Resolves the exact Loxone Config–style icon for a `TreeNode` in the Periphery or System tree.
pub fn icon_for_tree_node(node: &TreeNode) -> LoxoneIcon {
    let ct = node.control_type.as_str();
    match ct {
        "LoxLIVE" | "Miniserver" | "Document" | "Program" => LoxoneIcon::Miniserver,
        "WeatherServer" | "WeatherData" => LoxoneIcon::WeatherServer,
        "AudioServer" | "MusicPlayer" | "AudioOut" | "StereoExtension" => LoxoneIcon::AudioServer,
        "InputCaption" | "DigitalIn" | "TreeSensor" | "LoxAIRsensor" | "GenSensor"
        | "CallerVirtualIn" => LoxoneIcon::DigitalInput,
        "AnalogInputCaption" | "VoltageIn" | "TreeAsensor" | "LoxAIRAsensor" | "GenAsensor"
        | "ModbusASensor" | "SysTemp" | "SysVar" => LoxoneIcon::AnalogInput,
        "OutputCaption" | "Actor" | "TreeActor" | "TreeTextActor" | "LoxAIRactor" | "ApiActor" => {
            LoxoneIcon::DigitalOutput
        }
        "AnalogOutputCaption" | "Dimmer" | "TreeAactor" | "LoxAIRAactor" => {
            LoxoneIcon::AnalogOutput
        }
        "VirtualInCaption" | "VirtualIn" => LoxoneIcon::VirtualInput,
        "VirtualOutCaption" | "VirtualOut" | "VirtualOutCmd" => LoxoneIcon::VirtualOutput,
        "Intercom" | "IntercomDevice" => LoxoneIcon::Intercom,
        "WeatherCaption" | "LanInt" | "NetworkDevice" | "Plugin" | "MTablet" => {
            LoxoneIcon::NetworkPeriphery
        }
        "LoggerOutCaption" | "Logger" | "Mailer" | "Caller" | "Notification" | "MessageCenter" => {
            LoxoneIcon::MessagesLog
        }
        "LoxTree" => LoxoneIcon::LoxoneTree,
        "LoxAIR" | "LoxAIRDevice" => LoxoneIcon::ExtensionBadge("Air"),
        "LoxDIMM" => LoxoneIcon::ExtensionBadge("Dim"),
        "Comm485" | "SubDevices" | "ModbusDev" => LoxoneIcon::ExtensionBadge("MB"),
        "LoxREL" => LoxoneIcon::ExtensionBadge("Rel"),
        "TreeDevice" => LoxoneIcon::ExtensionBadge("Tr"),
        "PuDe" => LoxoneIcon::PushNotifications,
        "Permission" | "RightGroup" | "UserCaption" | "UserGroupCaption" | "User" | "UserGroup" => {
            LoxoneIcon::Trusts
        }
        "MemoryCaption" | "Memory" | "Merker" => LoxoneIcon::MemoryFlag,
        "ConstantCaption" | "Constant" | "State" | "StateV" => LoxoneIcon::Constant,
        "ModeCaption" | "Mode" | "CalendarCaption" | "Calendar" | "CalendarEntry"
        | "SwitchingTimer" => LoxoneIcon::OperatingMode,
        "TimeCaption"
        | "Time"
        | "DateTime"
        | "PulseAt"
        | "Day"
        | "Hour"
        | "Minute"
        | "Second"
        | "Month"
        | "Year"
        | "Week"
        | "DayOfWeek"
        | "Day2009"
        | "SecondsBoot"
        | "Sunrise"
        | "Sunset"
        | "Daylight"
        | "Daylight2"
        | "Morningtwilight"
        | "Eveningtwilight"
        | "SunAltitude"
        | "SunAzimuth"
        | "ImpulseDay"
        | "ImpulseHour"
        | "ImpulseMinute"
        | "ImpulseSecond"
        | "ImpulseMonth"
        | "ImpulseYear"
        | "ImpulseSunrise"
        | "ImpulseSunset"
        | "ImpulseMorningtwilight"
        | "ImpulseEveningtwilight"
        | "StartPulse" => LoxoneIcon::TimeFunction,
        "KeycodeCaption" | "NFCIDCaption" | "NfcCodeTouch" => LoxoneIcon::Security,
        "PlaceCaption" | "Place" => LoxoneIcon::Room,
        "CategoryCaption" | "Category" => LoxoneIcon::Category(None),
        "Online" | "Devicemonitor" | "GlobalStates" | "AutoPilot" | "TaskCaption"
        | "RemoteControls" | "OvertempShutdown" => LoxoneIcon::StatusOnline,
        "LoxCaption" => {
            if let Some(cap_type) =
                tree_node_attr(node, "CaptionType").or_else(|| tree_node_attr(node, "SubType"))
            {
                match cap_type {
                    "3" | "8" => return LoxoneIcon::LoxoneTree,
                    "4" | "10" => return LoxoneIcon::SdCard,
                    "6" | "34" => return LoxoneIcon::NetworkPeriphery,
                    "7" => return LoxoneIcon::LightingGroup,
                    "11" => return LoxoneIcon::OperatingMode,
                    "12" | "13" | "19" | "20" => return LoxoneIcon::Trusts,
                    "14" => return LoxoneIcon::AudioFolder,
                    "21" => return LoxoneIcon::Intercom,
                    "24" => return LoxoneIcon::LoxoneLink,
                    "28" => return LoxoneIcon::PushNotifications,
                    "33" => return LoxoneIcon::NetworkPeriphery,
                    _ => {}
                }
            }
            let title_lower = node.title.to_lowercase();
            if title_lower.contains("push") {
                LoxoneIcon::PushNotifications
            } else if title_lower.contains("audio") {
                LoxoneIcon::AudioFolder
            } else if title_lower.contains("trust")
                || title_lower.contains("uprawnie")
                || title_lower.contains("użytkown")
            {
                LoxoneIcon::Trusts
            } else if title_lower.contains("intercom") {
                LoxoneIcon::Intercom
            } else if title_lower.contains("karta sd") || title_lower.contains("sd card") {
                LoxoneIcon::SdCard
            } else if title_lower.contains("oświetlen") || title_lower.contains("light") {
                LoxoneIcon::LightingGroup
            } else if title_lower.contains("link") {
                LoxoneIcon::LoxoneLink
            } else if title_lower.contains("tree") {
                LoxoneIcon::LoxoneTree
            } else if title_lower.contains("sieci") || title_lower.contains("network") {
                LoxoneIcon::NetworkPeriphery
            } else {
                LoxoneIcon::GenericBlock
            }
        }
        _ => icon_for_block_type(ct, None),
    }
}

/// Resolves the Loxone icon for a schematic `PageBlock` type (and optional referenced `TreeNode` type).
pub fn icon_for_block_type(
    block_type: &str,
    referenced_tree_node_type: Option<&str>,
) -> LoxoneIcon {
    match block_type {
        "InputRef" => match referenced_tree_node_type {
            Some("VirtualIn" | "VirtualInCaption") => LoxoneIcon::VirtualInput,
            Some(
                "VoltageIn" | "AnalogInputCaption" | "TreeAsensor" | "LoxAIRAsensor" | "GenAsensor"
                | "ModbusASensor" | "SysTemp" | "SysVar",
            ) => LoxoneIcon::AnalogInput,
            Some("WeatherData" | "WeatherServer") => LoxoneIcon::WeatherServer,
            Some("Memory" | "Merker") => LoxoneIcon::MemoryFlag,
            _ => LoxoneIcon::DigitalInput,
        },
        "OutputRef" => match referenced_tree_node_type {
            Some("VirtualOut" | "VirtualOutCmd" | "VirtualOutCaption") => LoxoneIcon::VirtualOutput,
            Some("Dimmer" | "AnalogOutputCaption" | "TreeAactor" | "LoxAIRAactor") => {
                LoxoneIcon::AnalogOutput
            }
            Some("Memory" | "Merker") => LoxoneIcon::MemoryFlag,
            _ => LoxoneIcon::DigitalOutput,
        },
        "OutputRefLM" => LoxoneIcon::MessagesLog,
        "Memory" | "Merker" => LoxoneIcon::MemoryFlag,
        "Constant" | "State" | "StateV" => LoxoneIcon::Constant,
        "PButtonT" | "PushButton" | "Pushbutton" => LoxoneIcon::DigitalInput,
        "LightController2" | "CentralLight" | "MoodSwitch" | "LightScene" | "PresenceLight"
        | "StairwellLight" | "PresenceDetector" => LoxoneIcon::LightingGroup,
        "AutoJalousie" | "CentralShade" | "CentralJalousie" | "RoofWindowShading" => {
            LoxoneIcon::Shading
        }
        "HeatIRoomController2" | "HVACController" | "ClimateControllerUS" | "Heatmixer2"
        | "ClimateController" | "PID" | "HeatingCurve" | "PWM" | "TwoPointController"
        | "CeilingFan" | "Dehumidifier" | "PoolController" | "SaunaController" => {
            LoxoneIcon::Climate
        }
        "MusicPlayer" | "CentralMusic" | "StereoExtension" | "AudioServer" => {
            LoxoneIcon::AudioFolder
        }
        "Alarm" | "SmokeAlarm" | "AalEmergency" | "AalSmartAlarm" | "AlarmChain" | "Door"
        | "Doorcontroller" | "CentralGate" | "WindowMonitor" | "AccessCode" | "FireEmergency"
        | "WaterCentral" | "Intercom" => LoxoneIcon::Security,
        "EFM" | "MeterAbsBi" | "MeterPUni" | "SolarController" | "LoadManager" | "Wallbox2"
        | "SpotPriceOptimizer" => LoxoneIcon::Energy,
        "OnPulseDelay" | "OffDelay" | "IntervalImpulse" | "Stopwatch"
        | "OperatingHoursCounter" | "Schedule" | "AlarmClock" => LoxoneIcon::TimeFunction,
        "And" | "Not" | "Equal" | "PulseBy" | "LongClick" | "MultiClick" | "DoubleClick"
        | "ThresholdSwitch" | "Hysteresis" | "Limiter" | "Average" | "MinMax" | "ChangeRate"
        | "ValueSelector" | "Scale" | "Linearity" => LoxoneIcon::LogicGate,
        _ => {
            if let Some(schema) = lookup_block_schema(block_type) {
                let cat = schema.category_group.to_lowercase();
                if cat.contains("light") {
                    return LoxoneIcon::LightingGroup;
                } else if cat.contains("shad") {
                    return LoxoneIcon::Shading;
                } else if cat.contains("climat") || cat.contains("hvac") {
                    return LoxoneIcon::Climate;
                } else if cat.contains("audio") || cat.contains("multimedia") {
                    return LoxoneIcon::AudioFolder;
                } else if cat.contains("secur") || cat.contains("access") {
                    return LoxoneIcon::Security;
                } else if cat.contains("energ") {
                    return LoxoneIcon::Energy;
                } else if cat.contains("time") {
                    return LoxoneIcon::TimeFunction;
                } else if cat.contains("logic") || cat.contains("analog") {
                    return LoxoneIcon::LogicGate;
                }
            }
            LoxoneIcon::GenericBlock
        }
    }
}

/// Allocates an `egui` icon widget of `size x size` and paints the vector `LoxoneIcon`.
pub fn ui_icon(ui: &mut Ui, icon: LoxoneIcon, size: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    if ui.is_rect_visible(rect) {
        paint_icon(ui.painter(), rect, icon);
    }
    response
}

/// Paints a crisp Loxone Config–style vector icon inside `rect`.
pub fn paint_icon(painter: &Painter, rect: Rect, icon: LoxoneIcon) {
    let r = rect.shrink(1.0);
    let c = r.center();
    let w = r.width();
    let h = r.height();
    let p = |nx: f32, ny: f32| Pos2::new(r.left() + nx * w, r.top() + ny * h);

    let lox_green = Color32::from_rgb(0x69, 0xC3, 0x32);
    let lox_green_soft = Color32::from_rgb(0x8C, 0xE0, 0x58);
    let lox_green_dark = Color32::from_rgb(0x3D, 0x7A, 0x1A);
    let link_cyan = Color32::from_rgb(0x27, 0xB2, 0xF0);
    let audio_purple = Color32::from_rgb(0x88, 0x47, 0xFF);
    let analog_orange = Color32::from_rgb(0xF0, 0x78, 0x18);
    let output_crimson = Color32::from_rgb(0xE0, 0x24, 0x5E);
    let sun_gold = Color32::from_rgb(0xFF, 0xCA, 0x28);
    let sky_blue = Color32::from_rgb(0x42, 0xB4, 0xE6);
    let din_dark = Color32::from_rgb(0x3E, 0x43, 0x4C);
    let wire_stroke = Color32::from_rgb(0xC8, 0xD0, 0xDC);

    match icon {
        LoxoneIcon::PushNotifications => {
            // Bell body + bottom clapper + bright green notification badge dot on top-right
            let bell_pts = vec![
                p(0.20, 0.76),
                p(0.26, 0.44),
                p(0.38, 0.24),
                p(0.62, 0.24),
                p(0.74, 0.44),
                p(0.80, 0.76),
            ];
            painter.add(Shape::closed_line(
                bell_pts,
                Stroke::new(1.5_f32, wire_stroke),
            ));
            painter.line_segment(
                [p(0.14, 0.76), p(0.86, 0.76)],
                Stroke::new(1.6_f32, wire_stroke),
            );
            painter.circle_filled(p(0.50, 0.86), w * 0.08, wire_stroke);
            painter.circle_filled(p(0.76, 0.24), w * 0.20, lox_green);
        }
        LoxoneIcon::WeatherServer => {
            // Yellow sun top-right + blue cloud foreground
            painter.circle_filled(p(0.68, 0.30), w * 0.18, sun_gold);
            let cloud_rect = Rect::from_min_max(p(0.14, 0.48), p(0.84, 0.82));
            painter.rect_filled(cloud_rect, w * 0.16, sky_blue);
            painter.circle_filled(p(0.38, 0.48), w * 0.20, sky_blue);
            painter.circle_filled(p(0.60, 0.52), w * 0.16, sky_blue);
        }
        LoxoneIcon::AudioFolder => {
            // Purple double-beamed musical note (♫)
            painter.circle_filled(p(0.28, 0.76), w * 0.14, audio_purple);
            painter.circle_filled(p(0.72, 0.68), w * 0.14, audio_purple);
            painter.line_segment(
                [p(0.38, 0.74), p(0.38, 0.24)],
                Stroke::new(1.8_f32, audio_purple),
            );
            painter.line_segment(
                [p(0.82, 0.66), p(0.82, 0.16)],
                Stroke::new(1.8_f32, audio_purple),
            );
            painter.line_segment(
                [p(0.38, 0.24), p(0.82, 0.16)],
                Stroke::new(2.6_f32, audio_purple),
            );
        }
        LoxoneIcon::AudioServer => {
            // Purple DIN box with top connectors + white musical note inside
            let box_r = Rect::from_min_max(p(0.08, 0.22), p(0.92, 0.86));
            painter.rect_filled(box_r, 2.5, audio_purple);
            painter.line_segment(
                [p(0.30, 0.12), p(0.30, 0.22)],
                Stroke::new(1.5_f32, wire_stroke),
            );
            painter.line_segment(
                [p(0.70, 0.12), p(0.70, 0.22)],
                Stroke::new(1.5_f32, wire_stroke),
            );
            painter.circle_filled(p(0.42, 0.66), w * 0.10, Color32::WHITE);
            painter.line_segment(
                [p(0.50, 0.66), p(0.50, 0.36)],
                Stroke::new(1.5_f32, Color32::WHITE),
            );
            painter.line_segment(
                [p(0.50, 0.36), p(0.68, 0.42)],
                Stroke::new(1.8_f32, Color32::WHITE),
            );
        }
        LoxoneIcon::Trusts => {
            // Dark charcoal shield with two horizontal bars inside
            let shield = vec![
                p(0.50, 0.10),
                p(0.86, 0.22),
                p(0.82, 0.58),
                p(0.50, 0.90),
                p(0.18, 0.58),
                p(0.14, 0.22),
            ];
            painter.add(Shape::convex_polygon(
                shield,
                din_dark,
                Stroke::new(1.2_f32, wire_stroke),
            ));
            painter.line_segment(
                [p(0.30, 0.40), p(0.70, 0.40)],
                Stroke::new(1.6_f32, Color32::WHITE),
            );
            painter.line_segment(
                [p(0.34, 0.56), p(0.66, 0.56)],
                Stroke::new(1.6_f32, Color32::WHITE),
            );
        }
        LoxoneIcon::Miniserver => {
            // Bright Loxone green DIN-rail unit with top/bottom terminal dots & status LEDs
            let body = Rect::from_min_max(p(0.08, 0.18), p(0.92, 0.84));
            painter.rect_filled(body, 2.5, lox_green);
            painter.rect_stroke(
                body,
                2.5,
                Stroke::new(1.2_f32, lox_green_dark),
                StrokeKind::Outside,
            );
            painter.line_segment(
                [p(0.10, 0.42), p(0.90, 0.42)],
                Stroke::new(1.2_f32, lox_green_dark),
            );
            painter.line_segment(
                [p(0.10, 0.64), p(0.90, 0.64)],
                Stroke::new(1.2_f32, lox_green_dark),
            );
            painter.circle_filled(
                p(0.28, 0.53),
                w * 0.06,
                Color32::from_rgb(0xFF, 0x3B, 0x30),
            );
            painter.circle_filled(p(0.46, 0.53), w * 0.06, sun_gold);
        }
        LoxoneIcon::DigitalInput => {
            // Square-wave pulse at top + bright green right-pointing input arrow at bottom
            let wave = vec![
                p(0.10, 0.36),
                p(0.26, 0.36),
                p(0.26, 0.14),
                p(0.54, 0.14),
                p(0.54, 0.36),
                p(0.86, 0.36),
                p(0.86, 0.14),
            ];
            painter.add(Shape::line(wave, Stroke::new(1.5_f32, wire_stroke)));
            let tri = vec![p(0.18, 0.50), p(0.56, 0.70), p(0.18, 0.90)];
            painter.add(Shape::convex_polygon(
                tri,
                lox_green,
                Stroke::new(1.0_f32, lox_green_dark),
            ));
        }
        LoxoneIcon::AnalogInput => {
            // Sine-wave curve at top + orange right-pointing input arrow at bottom
            let wave = vec![
                p(0.10, 0.28),
                p(0.24, 0.12),
                p(0.38, 0.12),
                p(0.52, 0.28),
                p(0.66, 0.42),
                p(0.78, 0.42),
                p(0.90, 0.26),
            ];
            painter.add(Shape::line(wave, Stroke::new(1.5_f32, wire_stroke)));
            let tri = vec![p(0.18, 0.50), p(0.56, 0.70), p(0.18, 0.90)];
            painter.add(Shape::convex_polygon(
                tri,
                analog_orange,
                Stroke::new(1.0_f32, Color32::from_rgb(0xB8, 0x52, 0x0A)),
            ));
        }
        LoxoneIcon::DigitalOutput => {
            // Square-wave pulse at top + crimson right-pointing output arrow at bottom
            let wave = vec![
                p(0.10, 0.36),
                p(0.26, 0.36),
                p(0.26, 0.14),
                p(0.54, 0.14),
                p(0.54, 0.36),
                p(0.86, 0.36),
                p(0.86, 0.14),
            ];
            painter.add(Shape::line(wave, Stroke::new(1.5_f32, wire_stroke)));
            let tri = vec![p(0.36, 0.50), p(0.76, 0.70), p(0.36, 0.90)];
            painter.add(Shape::convex_polygon(
                tri,
                output_crimson,
                Stroke::new(1.0_f32, Color32::from_rgb(0x9E, 0x12, 0x3E)),
            ));
        }
        LoxoneIcon::AnalogOutput => {
            // Sine-wave curve at top + crimson right-pointing output arrow at bottom
            let wave = vec![
                p(0.10, 0.28),
                p(0.24, 0.12),
                p(0.38, 0.12),
                p(0.52, 0.28),
                p(0.66, 0.42),
                p(0.78, 0.42),
                p(0.90, 0.26),
            ];
            painter.add(Shape::line(wave, Stroke::new(1.5_f32, wire_stroke)));
            let tri = vec![p(0.36, 0.50), p(0.76, 0.70), p(0.36, 0.90)];
            painter.add(Shape::convex_polygon(
                tri,
                output_crimson,
                Stroke::new(1.0_f32, Color32::from_rgb(0x9E, 0x12, 0x3E)),
            ));
        }
        LoxoneIcon::VirtualInput => {
            // Light green rounded box with dark right-arrow entering
            let box_r = Rect::from_min_max(p(0.08, 0.22), p(0.92, 0.80));
            painter.rect_filled(box_r, 3.0, lox_green_soft);
            painter.rect_stroke(
                box_r,
                3.0,
                Stroke::new(1.2_f32, lox_green_dark),
                StrokeKind::Outside,
            );
            let arrow_col = Color32::from_rgb(0x1E, 0x26, 0x1A);
            painter.line_segment(
                [p(0.18, 0.51), p(0.66, 0.51)],
                Stroke::new(1.8_f32, arrow_col),
            );
            let head = vec![p(0.48, 0.34), p(0.72, 0.51), p(0.48, 0.68)];
            painter.add(Shape::convex_polygon(head, arrow_col, Stroke::NONE));
        }
        LoxoneIcon::VirtualOutput => {
            // Light green rounded box with dark right-arrow exiting
            let box_r = Rect::from_min_max(p(0.08, 0.22), p(0.92, 0.80));
            painter.rect_filled(box_r, 3.0, lox_green_soft);
            painter.rect_stroke(
                box_r,
                3.0,
                Stroke::new(1.2_f32, lox_green_dark),
                StrokeKind::Outside,
            );
            let arrow_col = Color32::from_rgb(0x1E, 0x26, 0x1A);
            painter.line_segment(
                [p(0.28, 0.51), p(0.76, 0.51)],
                Stroke::new(1.8_f32, arrow_col),
            );
            let head = vec![p(0.58, 0.34), p(0.82, 0.51), p(0.58, 0.68)];
            painter.add(Shape::convex_polygon(head, arrow_col, Stroke::NONE));
        }
        LoxoneIcon::Intercom => {
            // Dark square intercom housing with white bell/camera ring inside
            let box_r = Rect::from_min_max(p(0.12, 0.12), p(0.88, 0.88));
            painter.rect_filled(box_r, 2.5, din_dark);
            painter.rect_stroke(
                box_r,
                2.5,
                Stroke::new(1.1_f32, wire_stroke),
                StrokeKind::Outside,
            );
            painter.circle_stroke(c, w * 0.20, Stroke::new(1.4_f32, Color32::WHITE));
            painter.circle_filled(c, w * 0.07, Color32::WHITE);
        }
        LoxoneIcon::NetworkPeriphery => {
            // Three green network nodes connected in a hub triangle
            let n1 = p(0.24, 0.26);
            let n2 = p(0.78, 0.42);
            let n3 = p(0.38, 0.78);
            painter.line_segment([n1, n2], Stroke::new(1.5_f32, lox_green));
            painter.line_segment([n2, n3], Stroke::new(1.5_f32, lox_green));
            painter.line_segment([n1, n3], Stroke::new(1.5_f32, lox_green));
            for pt in [n1, n2, n3] {
                painter.circle_filled(pt, w * 0.13, lox_green);
                painter.circle_stroke(pt, w * 0.13, Stroke::new(1.0_f32, lox_green_dark));
            }
        }
        LoxoneIcon::MessagesLog => {
            // Document page with folded corner and bold "LOG" text at the bottom
            let page = vec![
                p(0.18, 0.10),
                p(0.64, 0.10),
                p(0.84, 0.30),
                p(0.84, 0.90),
                p(0.18, 0.90),
            ];
            painter.add(Shape::convex_polygon(
                page,
                Color32::from_rgb(0xEC, 0xF0, 0xF6),
                Stroke::new(1.2_f32, Color32::from_rgb(0x6E, 0x78, 0x88)),
            ));
            painter.text(
                p(0.51, 0.68),
                Align2::CENTER_CENTER,
                "LOG",
                FontId::new((h * 0.34).max(6.5), FontFamily::Proportional),
                Color32::from_rgb(0x24, 0x2A, 0x34),
            );
        }
        LoxoneIcon::LightingGroup => {
            // Bright green lightbulb with gray screw base
            painter.circle_filled(p(0.52, 0.40), w * 0.26, lox_green_soft);
            painter.circle_stroke(p(0.52, 0.40), w * 0.26, Stroke::new(1.2_f32, lox_green_dark));
            let base = Rect::from_min_max(p(0.38, 0.66), p(0.66, 0.86));
            painter.rect_filled(base, 1.5, wire_stroke);
        }
        LoxoneIcon::SdCard => {
            // Dark charcoal SD memory card with angled top-right corner & contact pins
            let card = vec![
                p(0.20, 0.12),
                p(0.66, 0.12),
                p(0.82, 0.28),
                p(0.82, 0.88),
                p(0.20, 0.88),
            ];
            painter.add(Shape::convex_polygon(
                card,
                din_dark,
                Stroke::new(1.1_f32, wire_stroke),
            ));
            for x in [0.34, 0.46, 0.58] {
                painter.line_segment(
                    [p(x, 0.20), p(x, 0.34)],
                    Stroke::new(1.4_f32, sun_gold),
                );
            }
        }
        LoxoneIcon::LoxoneLink => {
            // Split square: Left half cyan-blue, right half white, cyan border (matches Link in icons.png)
            let left_r = Rect::from_min_max(p(0.12, 0.12), p(0.50, 0.88));
            let right_r = Rect::from_min_max(p(0.50, 0.12), p(0.88, 0.88));
            let full_r = Rect::from_min_max(p(0.12, 0.12), p(0.88, 0.88));
            painter.rect_filled(left_r, 1.5, link_cyan);
            painter.rect_filled(right_r, 1.5, Color32::WHITE);
            painter.rect_stroke(
                full_r,
                1.5,
                Stroke::new(1.3_f32, link_cyan),
                StrokeKind::Outside,
            );
            painter.line_segment([p(0.50, 0.12), p(0.50, 0.88)], Stroke::new(1.0_f32, link_cyan));
        }
        LoxoneIcon::LoxoneTree => {
            // Split square: Left half Loxone green, right half white, green border (matches Tree in icons.png)
            let left_r = Rect::from_min_max(p(0.12, 0.12), p(0.50, 0.88));
            let right_r = Rect::from_min_max(p(0.50, 0.12), p(0.88, 0.88));
            let full_r = Rect::from_min_max(p(0.12, 0.12), p(0.88, 0.88));
            painter.rect_filled(left_r, 1.5, lox_green);
            painter.rect_filled(right_r, 1.5, Color32::WHITE);
            painter.rect_stroke(
                full_r,
                1.5,
                Stroke::new(1.3_f32, lox_green),
                StrokeKind::Outside,
            );
            painter.line_segment([p(0.50, 0.12), p(0.50, 0.88)], Stroke::new(1.0_f32, lox_green));
        }
        LoxoneIcon::ExtensionBadge(code) => {
            // Dark charcoal DIN module badge with top/bottom terminal teeth and white label ("Air", "Dim", "MB", "Rel")
            for x in [0.22, 0.40, 0.60, 0.78] {
                painter.line_segment([p(x, 0.12), p(x, 0.22)], Stroke::new(1.4_f32, wire_stroke));
                painter.line_segment([p(x, 0.78), p(x, 0.88)], Stroke::new(1.4_f32, wire_stroke));
            }
            let body = Rect::from_min_max(p(0.06, 0.20), p(0.94, 0.80));
            painter.rect_filled(body, 2.5, din_dark);
            painter.rect_stroke(
                body,
                2.5,
                Stroke::new(1.0_f32, wire_stroke),
                StrokeKind::Outside,
            );
            painter.text(
                body.center(),
                Align2::CENTER_CENTER,
                code,
                FontId::new((h * 0.42).max(7.5), FontFamily::Proportional),
                Color32::WHITE,
            );
        }
        LoxoneIcon::MemoryFlag => {
            // Amber/gold flag icon
            let amber = Color32::from_rgb(0xF5, 0xA6, 0x23);
            painter.line_segment([p(0.24, 0.14), p(0.24, 0.88)], Stroke::new(1.8_f32, amber));
            let flag = vec![p(0.24, 0.16), p(0.82, 0.32), p(0.24, 0.54)];
            painter.add(Shape::convex_polygon(flag, amber, Stroke::NONE));
        }
        LoxoneIcon::Constant => {
            let box_r = Rect::from_min_max(p(0.12, 0.18), p(0.88, 0.82));
            painter.rect_filled(box_r, 3.0, sky_blue);
            painter.text(
                box_r.center(),
                Align2::CENTER_CENTER,
                "C",
                FontId::new((h * 0.52).max(8.0), FontFamily::Proportional),
                Color32::WHITE,
            );
        }
        LoxoneIcon::OperatingMode | LoxoneIcon::TimeFunction => {
            // Clock / timer circle with hands
            painter.circle_stroke(c, w * 0.38, Stroke::new(1.5_f32, sky_blue));
            painter.line_segment([c, p(0.50, 0.26)], Stroke::new(1.6_f32, Color32::WHITE));
            painter.line_segment([c, p(0.72, 0.54)], Stroke::new(1.6_f32, sun_gold));
        }
        LoxoneIcon::Shading => {
            // Blue Venetian blinds horizontal slats
            let frame = Rect::from_min_max(p(0.14, 0.16), p(0.86, 0.84));
            painter.rect_stroke(frame, 2.0, Stroke::new(1.3_f32, sky_blue), StrokeKind::Outside);
            for y in [0.34, 0.52, 0.70] {
                painter.line_segment([p(0.20, y), p(0.80, y)], Stroke::new(1.6_f32, sky_blue));
            }
        }
        LoxoneIcon::Climate => {
            // Orange/red thermometer
            painter.line_segment(
                [p(0.50, 0.16), p(0.50, 0.68)],
                Stroke::new(3.2_f32, analog_orange),
            );
            painter.circle_filled(p(0.50, 0.74), w * 0.18, output_crimson);
        }
        LoxoneIcon::Security => {
            // Red security shield
            let shield = vec![
                p(0.50, 0.10),
                p(0.86, 0.22),
                p(0.82, 0.58),
                p(0.50, 0.90),
                p(0.18, 0.58),
                p(0.14, 0.22),
            ];
            painter.add(Shape::convex_polygon(
                shield,
                output_crimson,
                Stroke::new(1.2_f32, Color32::WHITE),
            ));
        }
        LoxoneIcon::Energy => {
            // Lightning bolt
            let bolt = vec![
                p(0.56, 0.10),
                p(0.24, 0.54),
                p(0.48, 0.54),
                p(0.40, 0.90),
                p(0.76, 0.44),
                p(0.52, 0.44),
            ];
            painter.add(Shape::convex_polygon(bolt, sun_gold, Stroke::NONE));
        }
        LoxoneIcon::LogicGate => {
            let box_r = Rect::from_min_max(p(0.14, 0.16), p(0.86, 0.84));
            painter.rect_filled(box_r, 2.5, din_dark);
            painter.rect_stroke(
                box_r,
                2.5,
                Stroke::new(1.2_f32, lox_green),
                StrokeKind::Outside,
            );
            painter.text(
                box_r.center(),
                Align2::CENTER_CENTER,
                "&",
                FontId::new((h * 0.52).max(8.0), FontFamily::Proportional),
                lox_green_soft,
            );
        }
        LoxoneIcon::Page => {
            // Schematic page with two mini blocks connected by a wire
            let b1 = Rect::from_min_max(p(0.12, 0.20), p(0.44, 0.48));
            let b2 = Rect::from_min_max(p(0.56, 0.52), p(0.88, 0.80));
            painter.rect_filled(b1, 1.5, link_cyan);
            painter.rect_filled(b2, 1.5, lox_green);
            painter.line_segment(
                [b1.right_center(), b2.left_center()],
                Stroke::new(1.4_f32, wire_stroke),
            );
        }
        LoxoneIcon::Room => {
            // House / room outline
            let roof = vec![p(0.12, 0.48), p(0.50, 0.14), p(0.88, 0.48)];
            painter.add(Shape::line(roof, Stroke::new(1.6_f32, sky_blue)));
            let walls = Rect::from_min_max(p(0.22, 0.48), p(0.78, 0.86));
            painter.rect_stroke(walls, 1.0, Stroke::new(1.4_f32, sky_blue), StrokeKind::Outside);
        }
        LoxoneIcon::Category(rgb) => {
            let col = rgb
                .map(|[r, g, b]| Color32::from_rgb(r, g, b))
                .unwrap_or(lox_green);
            let diamond = vec![p(0.50, 0.14), p(0.86, 0.50), p(0.50, 0.86), p(0.14, 0.50)];
            painter.add(Shape::convex_polygon(
                diamond,
                col,
                Stroke::new(1.0_f32, Color32::WHITE),
            ));
        }
        LoxoneIcon::StatusOnline => {
            painter.circle_filled(c, w * 0.28, lox_green);
            painter.circle_stroke(c, w * 0.36, Stroke::new(1.2_f32, lox_green_dark));
        }
        LoxoneIcon::GenericBlock => {
            let box_r = Rect::from_min_max(p(0.16, 0.20), p(0.84, 0.80));
            painter.rect_filled(box_r, 2.5, din_dark);
            painter.rect_stroke(
                box_r,
                2.5,
                Stroke::new(1.2_f32, wire_stroke),
                StrokeKind::Outside,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use loxone_core::parser::load_loxone_file;

    #[test]
    fn test_klimczaki_periphery_icons_match_reference_screenshot() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Klimczaki.Loxone");
        if !path.exists() {
            return;
        }
        let project = load_loxone_file(&path).expect("Klimczaki.Loxone should parse");
        let loxlive = project
            .periphery_tree
            .iter()
            .find(|n| n.control_type == "LoxLIVE")
            .expect("LoxLIVE node should exist");
        assert_eq!(icon_for_tree_node(loxlive), LoxoneIcon::Miniserver);

        let check_child = |title: &str, expected: LoxoneIcon| {
            let child = loxlive
                .children
                .iter()
                .find(|c| c.title == title)
                .unwrap_or_else(|| panic!("Missing LoxLIVE child '{}'", title));
            assert_eq!(icon_for_tree_node(child), expected, "Mismatch for {}", title);
        };

        check_child("Wejścia cyfrowe", LoxoneIcon::DigitalInput);
        check_child("Wejścia analogowe", LoxoneIcon::AnalogInput);
        check_child("Wyjścia cyfrowe", LoxoneIcon::DigitalOutput);
        check_child("Wejścia wirtualne", LoxoneIcon::VirtualInput);
        check_child("Wirtualne wyjścia", LoxoneIcon::VirtualOutput);
        check_child("Intercom", LoxoneIcon::Intercom);
        check_child("Peryferia sieciowe", LoxoneIcon::NetworkPeriphery);
        check_child("Wiadomości", LoxoneIcon::MessagesLog);
        check_child("Grupy oświetlenia", LoxoneIcon::LightingGroup);
        check_child("Karta SD", LoxoneIcon::SdCard);
        check_child("Link", LoxoneIcon::LoxoneLink);
        check_child("Tree", LoxoneIcon::LoxoneTree);

        let link = loxlive.children.iter().find(|c| c.title == "Link").unwrap();
        let check_ext = |title: &str, expected: LoxoneIcon| {
            let ext = link
                .children
                .iter()
                .find(|c| c.title == title)
                .unwrap_or_else(|| panic!("Missing Link extension '{}'", title));
            assert_eq!(icon_for_tree_node(ext), expected, "Mismatch for {}", title);
        };
        check_ext("Air Base Extension", LoxoneIcon::ExtensionBadge("Air"));
        check_ext("Dimmer Extension", LoxoneIcon::ExtensionBadge("Dim"));
        check_ext("Modbus Extension", LoxoneIcon::ExtensionBadge("MB"));
        check_ext("Relay Extension", LoxoneIcon::ExtensionBadge("Rel"));
    }
}
