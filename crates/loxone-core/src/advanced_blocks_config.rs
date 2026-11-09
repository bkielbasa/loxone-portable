use crate::model::{LoxoneProject, PageBlock, RawNode, TreeNode};
use crate::writer::ProjectEdit;

/// Master helper returning `true` when `block_type` supports the dedicated Advanced Block
/// Double-Click Configuration Modal (`Central`, `AudioZone` / `MusicPlayer`,
/// `Heatmixer2` / `ClimateController`, `Sequence`, `AutoJalousie` / `Jalousie`,
/// `Doorcontroller` / `Intercom` / `Door` / `NfcCodeTouch`, or `PresenceDetector`).
pub fn supports_advanced_block_modal(block_type: &str) -> bool {
    CentralBlockConfig::is_central_block(block_type)
        || AudioZoneConfig::is_audio_zone_block(block_type)
        || ClimateControllerConfig::is_climate_controller_block(block_type)
        || SequenceConfig::is_sequence_block(block_type)
        || ShadingBlockConfig::is_shading_block(block_type)
        || AccessBlockConfig::is_access_block(block_type)
        || PresenceBlockConfig::is_presence_block(block_type)
}

fn find_attr<'a>(attrs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

fn find_pin_def<'a>(block: &'a PageBlock, key: &str) -> Option<&'a str> {
    block
        .pins
        .iter()
        .find(|p| p.key == key)
        .and_then(|p| p.default_value.as_deref())
}

fn parse_pin_f32(block: &PageBlock, key: &str, default: f32) -> f32 {
    find_pin_def(block, key)
        .and_then(|v| v.trim().parse::<f32>().ok())
        .unwrap_or(default)
}

fn parse_pin_u32(block: &PageBlock, key: &str, default: u32) -> u32 {
    find_pin_def(block, key)
        .and_then(|v| v.trim().parse::<u32>().ok())
        .unwrap_or(default)
}

fn parse_pin_i32(block: &PageBlock, key: &str, default: i32) -> i32 {
    find_pin_def(block, key)
        .and_then(|v| v.trim().parse::<i32>().ok())
        .unwrap_or(default)
}

fn fmt_f32(val: f32) -> String {
    if (val - val.round()).abs() < 1e-4 {
        format!("{}", val.round() as i64)
    } else {
        format!("{:.3}", val)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

// ============================================================================
// 1. CENTRAL BLOCKS (`CentralLight`, `CentralShade`, `CentralMusic`, `CentralGate`)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CentralKind {
    Lighting,
    Shading,
    Audio,
    Gate,
}

impl CentralKind {
    pub fn from_block_type(block_type: &str) -> Option<Self> {
        match block_type {
            "CentralLight" | "CentralLightController" => Some(Self::Lighting),
            "CentralShade" | "CentralJalousie" => Some(Self::Shading),
            "CentralMusic" | "CentralAudio" => Some(Self::Audio),
            "CentralGate" => Some(Self::Gate),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Lighting => "Central Lighting",
            Self::Shading => "Central Shading",
            Self::Audio => "Central Audio",
            Self::Gate => "Central Gate / Access",
        }
    }

    pub fn matches_target_type(self, candidate_type: &str) -> bool {
        match self {
            Self::Lighting => matches!(
                candidate_type,
                "LightController2"
                    | "LightController"
                    | "MoodSwitch"
                    | "LightScene"
                    | "Dimmer"
                    | "Switch"
                    | "StairwellLight"
                    | "PresenceLight"
            ),
            Self::Shading => matches!(
                candidate_type,
                "Jalousie" | "AutoJalousie" | "RoofWindowShading" | "Gate"
            ),
            Self::Audio => matches!(
                candidate_type,
                "MusicPlayer" | "AudioZone" | "StereoExtension"
            ),
            Self::Gate => matches!(
                candidate_type,
                "Gate" | "UpDownDigital" | "Switch" | "AccessCode"
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CentralCandidate {
    pub uuid: String,
    pub title: String,
    pub block_type: String,
    pub room_name: String,
    pub category_name: String,
    pub page_title: String,
}

pub fn collect_central_candidates(
    project: &LoxoneProject,
    central_block_type: &str,
) -> Vec<CentralCandidate> {
    let Some(kind) = CentralKind::from_block_type(central_block_type) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for page in &project.pages {
        for block in &page.blocks {
            if kind.matches_target_type(&block.block_type) {
                let room_name = block
                    .room_uuid
                    .as_deref()
                    .or(page.room_uuid.as_deref())
                    .and_then(|u| project.places.get(u))
                    .map(|r| r.title.clone())
                    .unwrap_or_else(|| "Unassigned".to_string());
                let category_name = block
                    .category_uuid
                    .as_deref()
                    .or(page.category_uuid.as_deref())
                    .and_then(|u| project.categories.get(u))
                    .map(|c| c.title.clone())
                    .unwrap_or_else(|| "General".to_string());
                out.push(CentralCandidate {
                    uuid: block.uuid.clone(),
                    title: if block.title.is_empty() {
                        block.block_type.clone()
                    } else {
                        block.title.clone()
                    },
                    block_type: block.block_type.clone(),
                    room_name,
                    category_name,
                    page_title: page.title.clone(),
                });
            }
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CentralBlockConfig {
    pub kind: CentralKind,
    pub selected_uuids: Vec<String>,
}

impl CentralBlockConfig {
    pub fn is_central_block(block_type: &str) -> bool {
        CentralKind::from_block_type(block_type).is_some()
    }

    pub fn from_block(block: &PageBlock) -> Self {
        let kind = CentralKind::from_block_type(&block.block_type).unwrap_or(CentralKind::Lighting);
        let selected_uuids = find_attr(&block.raw_attributes, "rec")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        Self {
            kind,
            selected_uuids,
        }
    }

    pub fn is_selected(&self, uuid: &str) -> bool {
        self.selected_uuids.iter().any(|u| u == uuid)
    }

    pub fn toggle_uuid(&mut self, uuid: &str) {
        if let Some(pos) = self.selected_uuids.iter().position(|u| u == uuid) {
            self.selected_uuids.remove(pos);
        } else {
            self.selected_uuids.push(uuid.to_string());
        }
    }

    pub fn select_all(&mut self, candidates: &[CentralCandidate]) {
        for cand in candidates {
            if !self.is_selected(&cand.uuid) {
                self.selected_uuids.push(cand.uuid.clone());
            }
        }
    }

    pub fn clear_all(&mut self) {
        self.selected_uuids.clear();
    }

    pub fn to_project_edit(&self, _page_uuid: &str, block_uuid: &str) -> ProjectEdit {
        let joined = self.selected_uuids.join(",");
        ProjectEdit::SetControlAttr {
            control_uuid: block_uuid.to_string(),
            attr_name: "rec".to_string(),
            value: (!joined.is_empty()).then_some(joined),
        }
    }
}

// ============================================================================
// 2. AUDIO ZONE / MUSIC PLAYER (`MusicPlayer`, `AudioZone`, `StereoExtension`)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioOutChannel {
    pub uuid: String,
    pub title: String,
    pub parent_title: String,
    pub assigned_player_uuid: Option<String>,
    pub max_vol: u32,
    pub line_out_vol: u32,
}

fn collect_audio_out_recursive(
    node: &TreeNode,
    parent_title: &str,
    out: &mut Vec<AudioOutChannel>,
) {
    if node.control_type == "AudioOut" {
        let assigned_player_uuid = find_attr(&node.raw_attributes, "Ply")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let max_vol = find_attr(&node.raw_attributes, "MxV")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(100);
        let line_out_vol = find_attr(&node.raw_attributes, "VOLL")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(30);
        out.push(AudioOutChannel {
            uuid: node.uuid.clone(),
            title: node.title.clone(),
            parent_title: parent_title.to_string(),
            assigned_player_uuid,
            max_vol,
            line_out_vol,
        });
    }
    let next_parent = if matches!(
        node.control_type.as_str(),
        "AudioServer" | "IntercomDevice" | "TreeDevice"
    ) && !node.title.is_empty()
    {
        node.title.as_str()
    } else {
        parent_title
    };
    for child in &node.children {
        collect_audio_out_recursive(child, next_parent, out);
    }
}

pub fn collect_audio_out_channels(project: &LoxoneProject) -> Vec<AudioOutChannel> {
    let mut out = Vec::new();
    for node in project.periphery_tree.iter().chain(project.system_tree.iter()) {
        collect_audio_out_recursive(node, &node.title, &mut out);
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioZoneConfig {
    pub speaker_cfg: u32,
    pub presence_enabled: bool,
    pub airplay_enabled: bool,
    pub spotify_enabled: bool,
    pub high_pass_filter: bool,
    pub default_vol: u32,
    pub max_vol: u32,
    pub step_vol: u32,
    pub alarm_vol: u32,
    pub bell_vol: u32,
    pub buzzer_vol: u32,
    pub tts_vol: u32,
    pub assigned_channel_uuids: Vec<String>,
}

impl AudioZoneConfig {
    pub fn is_audio_zone_block(block_type: &str) -> bool {
        matches!(block_type, "MusicPlayer" | "AudioZone" | "StereoExtension")
    }

    pub fn from_block(block: &PageBlock, project: &LoxoneProject) -> Self {
        let speaker_cfg = find_attr(&block.raw_attributes, "SpCfg")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(4);
        let presence_enabled = find_attr(&block.raw_attributes, "PR") == Some("true");
        let airplay_enabled = find_attr(&block.raw_attributes, "APL") == Some("true");
        let spotify_enabled = find_attr(&block.raw_attributes, "SPC") == Some("true");
        let high_pass_filter = find_attr(&block.raw_attributes, "HPF") == Some("true");

        let assigned_channel_uuids = collect_audio_out_channels(project)
            .into_iter()
            .filter(|ch| ch.assigned_player_uuid.as_deref() == Some(block.uuid.as_str()))
            .map(|ch| ch.uuid)
            .collect();

        Self {
            speaker_cfg,
            presence_enabled,
            airplay_enabled,
            spotify_enabled,
            high_pass_filter,
            default_vol: parse_pin_u32(block, "DefVol", 20),
            max_vol: parse_pin_u32(block, "MaxVol", 100),
            step_vol: parse_pin_u32(block, "Step", 3),
            alarm_vol: parse_pin_u32(block, "AlarmVol", 75),
            bell_vol: parse_pin_u32(block, "BellVol", 10),
            buzzer_vol: parse_pin_u32(block, "BuzzVol", 50),
            tts_vol: parse_pin_u32(block, "TTSVol", 40),
            assigned_channel_uuids,
        }
    }

    pub fn is_channel_assigned(&self, channel_uuid: &str) -> bool {
        self.assigned_channel_uuids.iter().any(|u| u == channel_uuid)
    }

    pub fn toggle_channel(&mut self, channel_uuid: &str) {
        if let Some(pos) = self
            .assigned_channel_uuids
            .iter()
            .position(|u| u == channel_uuid)
        {
            self.assigned_channel_uuids.remove(pos);
        } else {
            self.assigned_channel_uuids.push(channel_uuid.to_string());
        }
    }

    pub fn to_project_edit(
        &self,
        _page_uuid: &str,
        block_uuid: &str,
        all_channels: &[AudioOutChannel],
    ) -> ProjectEdit {
        let mut edits = vec![
            ProjectEdit::SetControlAttr {
                control_uuid: block_uuid.to_string(),
                attr_name: "SpCfg".to_string(),
                value: Some(self.speaker_cfg.to_string()),
            },
            ProjectEdit::SetControlAttr {
                control_uuid: block_uuid.to_string(),
                attr_name: "PR".to_string(),
                value: self.presence_enabled.then(|| "true".to_string()),
            },
            ProjectEdit::SetControlAttr {
                control_uuid: block_uuid.to_string(),
                attr_name: "APL".to_string(),
                value: self.airplay_enabled.then(|| "true".to_string()),
            },
            ProjectEdit::SetControlAttr {
                control_uuid: block_uuid.to_string(),
                attr_name: "SPC".to_string(),
                value: self.spotify_enabled.then(|| "true".to_string()),
            },
            ProjectEdit::SetControlAttr {
                control_uuid: block_uuid.to_string(),
                attr_name: "HPF".to_string(),
                value: self.high_pass_filter.then(|| "true".to_string()),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "DefVol".to_string(),
                default_value: Some(self.default_vol.to_string()),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "MaxVol".to_string(),
                default_value: Some(self.max_vol.to_string()),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "Step".to_string(),
                default_value: Some(self.step_vol.to_string()),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "AlarmVol".to_string(),
                default_value: Some(self.alarm_vol.to_string()),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "BellVol".to_string(),
                default_value: Some(self.bell_vol.to_string()),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "BuzzVol".to_string(),
                default_value: Some(self.buzzer_vol.to_string()),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "TTSVol".to_string(),
                default_value: Some(self.tts_vol.to_string()),
            },
        ];

        for ch in all_channels {
            let currently_assigned = ch.assigned_player_uuid.as_deref() == Some(block_uuid);
            let want_assigned = self.is_channel_assigned(&ch.uuid);
            if want_assigned && !currently_assigned {
                edits.push(ProjectEdit::SetControlAttr {
                    control_uuid: ch.uuid.clone(),
                    attr_name: "Ply".to_string(),
                    value: Some(block_uuid.to_string()),
                });
            } else if !want_assigned && currently_assigned {
                edits.push(ProjectEdit::SetControlAttr {
                    control_uuid: ch.uuid.clone(),
                    attr_name: "Ply".to_string(),
                    value: None,
                });
            }
        }

        ProjectEdit::Batch(edits)
    }
}

// ============================================================================
// 3. CLIMATE CONTROLLER (`HVACController`, `Heatmixer2`, `ClimateControllerUS`, `ClimateController`)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignedObjectEntry {
    pub uuid: String,
    pub badge_prefix: String,
    pub title: String,
    pub display_label: String,
    pub page_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockObjectAssignmentSummary {
    pub assigned: Vec<AssignedObjectEntry>,
    pub total_candidates: usize,
    pub show_api_fanout: bool,
}

fn polish_sort_key(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'ą' | 'Ą' => 'a',
            'ć' | 'Ć' => 'c',
            'ę' | 'Ę' => 'e',
            'ł' | 'Ł' => 'l',
            'ń' | 'Ń' => 'n',
            'ó' | 'Ó' => 'o',
            'ś' | 'Ś' => 's',
            'ź' | 'Ź' | 'ż' | 'Ż' => 'z',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

fn short_assigned_object_prefix(block_type: &str) -> &'static str {
    match block_type {
        "HeatIRoomController2" => "Irc2",
        "HeatIRoomController" => "Irc",
        "LightController2" | "LightController" => "Lico",
        "Jalousie" | "AutoJalousie" | "RoofWindowShading" => "Shd",
        "MusicPlayer" | "AudioZone" => "Aud",
        "Gate" | "Door" => "Gate",
        _ => "Obj",
    }
}

/// Resolves the assigned objects and total candidate count for blocks that display
/// `Przyporządkuj obiekty... (X/Y)` and the `API ⇄` fan-out tree on the canvas.
pub fn collect_block_assigned_objects(
    block: &PageBlock,
    project: &LoxoneProject,
) -> Option<BlockObjectAssignmentSummary> {
    if ClimateControllerConfig::is_climate_controller_block(&block.block_type) {
        let candidates = collect_irc_candidates(project);
        let total_candidates = candidates.len();
        let cfg = ClimateControllerConfig::from_block_with_project(block, Some(project));
        let mut assigned = Vec::new();
        for uuid in &cfg.assigned_irc_uuids {
            if let Some((page_idx, irc_block)) =
                project.pages.iter().enumerate().find_map(|(idx, p)| {
                    p.blocks
                        .iter()
                        .find(|b| &b.uuid == uuid)
                        .map(|b| (idx, b))
                })
            {
                let prefix = short_assigned_object_prefix(&irc_block.block_type).to_string();
                let title = if !irc_block.title.trim().is_empty() {
                    irc_block.title.clone()
                } else {
                    irc_block
                        .room_uuid
                        .as_deref()
                        .and_then(|u| project.places.get(u))
                        .map(|r| r.title.clone())
                        .unwrap_or_else(|| irc_block.block_type.clone())
                };
                let display_label = format!("{prefix}: {title}");
                assigned.push(AssignedObjectEntry {
                    uuid: irc_block.uuid.clone(),
                    badge_prefix: prefix,
                    title,
                    display_label,
                    page_index: Some(page_idx),
                });
            }
        }
        assigned.sort_by(|a, b| polish_sort_key(&a.title).cmp(&polish_sort_key(&b.title)));
        return Some(BlockObjectAssignmentSummary {
            assigned,
            total_candidates,
            show_api_fanout: true,
        });
    }

    if CentralBlockConfig::is_central_block(&block.block_type) {
        let candidates = collect_central_candidates(project, &block.block_type);
        let total_candidates = candidates.len();
        let cfg = CentralBlockConfig::from_block(block);
        let mut assigned = Vec::new();
        for uuid in &cfg.selected_uuids {
            if let Some((page_idx, target_block)) =
                project.pages.iter().enumerate().find_map(|(idx, p)| {
                    p.blocks
                        .iter()
                        .find(|b| &b.uuid == uuid)
                        .map(|b| (idx, b))
                })
            {
                let prefix = short_assigned_object_prefix(&target_block.block_type).to_string();
                let title = if !target_block.title.trim().is_empty() {
                    target_block.title.clone()
                } else {
                    target_block.block_type.clone()
                };
                let display_label = format!("{prefix}: {title}");
                assigned.push(AssignedObjectEntry {
                    uuid: target_block.uuid.clone(),
                    badge_prefix: prefix,
                    title,
                    display_label,
                    page_index: Some(page_idx),
                });
            }
        }
        return Some(BlockObjectAssignmentSummary {
            assigned,
            total_candidates,
            show_api_fanout: true,
        });
    }

    if AudioZoneConfig::is_audio_zone_block(&block.block_type) {
        let candidates = collect_audio_out_channels(project);
        let total_candidates = candidates.len();
        let cfg = AudioZoneConfig::from_block(block, project);
        let mut assigned = Vec::new();
        for uuid in &cfg.assigned_channel_uuids {
            if let Some(ch) = candidates.iter().find(|c| &c.uuid == uuid) {
                let display_label = format!("Aud: {}", ch.title);
                assigned.push(AssignedObjectEntry {
                    uuid: ch.uuid.clone(),
                    badge_prefix: "Aud".to_string(),
                    title: ch.title.clone(),
                    display_label,
                    page_index: None,
                });
            }
        }
        return Some(BlockObjectAssignmentSummary {
            assigned,
            total_candidates,
            show_api_fanout: false,
        });
    }

    if PresenceBlockConfig::is_presence_block(&block.block_type) {
        let candidates = collect_presence_sensor_candidates(project);
        let cfg = PresenceBlockConfig::from_block(block);
        let mut assigned = Vec::new();
        for uuid in &cfg.assigned_device_uuids {
            let title = candidates
                .iter()
                .find(|c| &c.uuid == uuid)
                .map(|c| c.title.clone())
                .unwrap_or_else(|| uuid.clone());
            let display_label = format!("Sens: {title}");
            assigned.push(AssignedObjectEntry {
                uuid: uuid.clone(),
                badge_prefix: "Sens".to_string(),
                title,
                display_label,
                page_index: None,
            });
        }
        let total_candidates = candidates.len().max(assigned.len());
        return Some(BlockObjectAssignmentSummary {
            assigned,
            total_candidates,
            show_api_fanout: false,
        });
    }

    if matches!(
        block.block_type.as_str(),
        "HeatIRoomController2" | "HeatIRoomController"
    ) {
        let obj_uuids: Vec<String> = find_attr(&block.raw_attributes, "Objects")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        let all_candidates =
            crate::room_controller_config::collect_energy_source_candidates(project, &[]);
        let mut assigned = Vec::new();
        for uuid in &obj_uuids {
            let page_idx = project
                .pages
                .iter()
                .position(|p| p.blocks.iter().any(|b| &b.uuid == uuid));
            let title = all_candidates
                .iter()
                .find(|c| &c.object_uuid == uuid)
                .map(|c| c.src_name.clone())
                .unwrap_or_else(|| uuid.clone());
            let display_label = format!("Hvac: {title}");
            assigned.push(AssignedObjectEntry {
                uuid: uuid.clone(),
                badge_prefix: "Hvac".to_string(),
                title,
                display_label,
                page_index: page_idx,
            });
        }
        let total_candidates = all_candidates.len().max(assigned.len());
        return Some(BlockObjectAssignmentSummary {
            assigned,
            total_candidates,
            show_api_fanout: false,
        });
    }

    if crate::object_config::supports_object_config(block) {
        let rows = crate::object_config::build_object_config_rows(project, "", &block.uuid);
        let objects_attr = find_attr(&block.raw_attributes, "Objects")
            .or_else(|| find_attr(&block.raw_attributes, "UO"))
            .unwrap_or_default();
        let explicit_uuids: Vec<&str> = objects_attr
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();

        let mut assigned = Vec::new();
        if !explicit_uuids.is_empty() {
            for u in &explicit_uuids {
                let page_idx = project
                    .pages
                    .iter()
                    .position(|p| p.blocks.iter().any(|b| b.uuid == *u));
                let title = project
                    .pages
                    .iter()
                    .flat_map(|p| &p.blocks)
                    .find(|b| b.uuid == *u)
                    .map(|b| {
                        if b.title.is_empty() {
                            b.block_type.clone()
                        } else {
                            b.title.clone()
                        }
                    })
                    .unwrap_or_else(|| (*u).to_string());
                assigned.push(AssignedObjectEntry {
                    uuid: (*u).to_string(),
                    badge_prefix: "Obj".to_string(),
                    display_label: format!("Obj: {title}"),
                    title,
                    page_index: page_idx,
                });
            }
        } else {
            for r in rows.iter().filter(|r| r.is_input || r.is_output) {
                let page_idx = match &r.target_kind {
                    crate::object_config::ObjectConfigTargetKind::PageFunctionBlock {
                        page_uuid,
                        ..
                    } => project.pages.iter().position(|p| &p.uuid == page_uuid),
                    _ => None,
                };
                assigned.push(AssignedObjectEntry {
                    uuid: r.target_uuid.clone(),
                    badge_prefix: "Obj".to_string(),
                    display_label: format!("Obj: {}", r.name),
                    title: r.name.clone(),
                    page_index: page_idx,
                });
            }
        }
        let unselected_count = rows.iter().filter(|r| !r.is_input && !r.is_output).count();
        let total_candidates = assigned.len() + unselected_count;
        if total_candidates > 0 || !assigned.is_empty() {
            return Some(BlockObjectAssignmentSummary {
                assigned,
                total_candidates,
                show_api_fanout: false,
            });
        }
    }

    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrcCandidate {
    pub uuid: String,
    pub title: String,
    pub room_name: String,
    pub page_title: String,
}

pub fn collect_irc_candidates(project: &LoxoneProject) -> Vec<IrcCandidate> {
    let mut out = Vec::new();
    for page in &project.pages {
        for block in &page.blocks {
            if matches!(
                block.block_type.as_str(),
                "HeatIRoomController2" | "HeatIRoomController"
            ) {
                let room_name = block
                    .room_uuid
                    .as_deref()
                    .or(page.room_uuid.as_deref())
                    .and_then(|u| project.places.get(u))
                    .map(|r| r.title.clone())
                    .unwrap_or_else(|| "Unassigned".to_string());
                out.push(IrcCandidate {
                    uuid: block.uuid.clone(),
                    title: if block.title.is_empty() {
                        block.block_type.clone()
                    } else {
                        block.title.clone()
                    },
                    room_name,
                    page_title: page.title.clone(),
                });
            }
        }
    }
    out.sort_by(|a, b| polish_sort_key(&a.title).cmp(&polish_sort_key(&b.title)));
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClimateControllerConfig {
    pub is_heatmixer: bool,
    pub controller_mode: u32,
    pub cooling_mode: u32,
    pub use_hvac: bool,
    pub assigned_irc_uuids: Vec<String>,
    pub min_flow_temp: f32,
    pub max_flow_temp: f32,
    pub buffer_inc: f32,
    pub prev_inc: f32,
    pub transconductance: f32,
    pub threshold: f32,
    pub gain: f32,
    pub capabilities: u32,
    pub device_type: u32,
    pub temp_limit_c: f32,
    pub temp_limit_h: f32,
    pub on_threshold: f32,
    pub humid_target: f32,
    pub time_off_pump: u32,
    pub fan_delay: u32,
    pub tp_min: f32,
    pub tp_max: f32,
}

impl ClimateControllerConfig {
    pub fn is_climate_controller_block(block_type: &str) -> bool {
        matches!(
            block_type,
            "HVACController" | "Heatmixer2" | "ClimateControllerUS" | "ClimateController"
        )
    }

    pub fn from_block(block: &PageBlock) -> Self {
        Self::from_block_with_project(block, None)
    }

    pub fn from_block_with_project(block: &PageBlock, project: Option<&LoxoneProject>) -> Self {
        let is_heatmixer = block.block_type == "Heatmixer2";
        let controller_mode = find_attr(&block.raw_attributes, "ControllerMode")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(1);
        let cooling_mode = find_attr(&block.raw_attributes, "Cool")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(2);
        let use_hvac = find_attr(&block.raw_attributes, "UseHVAc") == Some("1");
        let mut assigned_irc_uuids: Vec<String> = find_attr(&block.raw_attributes, "Irr")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        if let Some(proj) = project {
            for page in &proj.pages {
                for cand in &page.blocks {
                    if matches!(
                        cand.block_type.as_str(),
                        "HeatIRoomController2" | "HeatIRoomController"
                    ) {
                        if let Some(objs) = find_attr(&cand.raw_attributes, "Objects") {
                            if objs
                                .split(',')
                                .map(str::trim)
                                .any(|u| u == block.uuid.as_str())
                                && !assigned_irc_uuids.contains(&cand.uuid)
                            {
                                assigned_irc_uuids.push(cand.uuid.clone());
                            }
                        }
                    }
                }
            }
        }
        let capabilities = find_attr(&block.raw_attributes, "cCap")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(3);
        let device_type = find_attr(&block.raw_attributes, "DType")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(2);

        Self {
            is_heatmixer,
            controller_mode,
            cooling_mode,
            use_hvac,
            assigned_irc_uuids,
            min_flow_temp: parse_pin_f32(block, "Min", 5.0),
            max_flow_temp: parse_pin_f32(block, "Max", 40.0),
            buffer_inc: parse_pin_f32(block, "BufferInc", 5.0),
            prev_inc: parse_pin_f32(block, "PrevInc", 2.0),
            transconductance: parse_pin_f32(block, "Transconductance", 0.5),
            threshold: parse_pin_f32(block, "Treshold", 35.0),
            gain: parse_pin_f32(block, "Gain", 1.0),
            capabilities,
            device_type,
            temp_limit_c: parse_pin_f32(block, "TempLimitC", 15.0),
            temp_limit_h: parse_pin_f32(block, "TempLimitH", 18.0),
            on_threshold: parse_pin_f32(block, "OnThreshold", 30.0),
            humid_target: parse_pin_f32(block, "HTarget", 45.0),
            time_off_pump: parse_pin_u32(block, "TimeOffPump", 300),
            fan_delay: parse_pin_u32(block, "FanDelay", 90),
            tp_min: parse_pin_f32(block, "TPMin", 12.0),
            tp_max: parse_pin_f32(block, "TPMax", 19.0),
        }
    }

    pub fn is_irc_assigned(&self, irc_uuid: &str) -> bool {
        self.assigned_irc_uuids.iter().any(|u| u == irc_uuid)
    }

    pub fn toggle_irc(&mut self, irc_uuid: &str) {
        if let Some(pos) = self.assigned_irc_uuids.iter().position(|u| u == irc_uuid) {
            self.assigned_irc_uuids.remove(pos);
        } else {
            self.assigned_irc_uuids.push(irc_uuid.to_string());
        }
    }

    pub fn to_project_edit(&self, page_uuid: &str, block_uuid: &str) -> ProjectEdit {
        self.to_project_edit_with_project(page_uuid, block_uuid, None)
    }

    pub fn to_project_edit_with_project(
        &self,
        _page_uuid: &str,
        block_uuid: &str,
        project: Option<&LoxoneProject>,
    ) -> ProjectEdit {
        let mut edits = Vec::new();
        if self.is_heatmixer {
            let irr_str = self.assigned_irc_uuids.join(",");
            edits.extend([
                ProjectEdit::SetControlAttr {
                    control_uuid: block_uuid.to_string(),
                    attr_name: "ControllerMode".to_string(),
                    value: Some(self.controller_mode.to_string()),
                },
                ProjectEdit::SetControlAttr {
                    control_uuid: block_uuid.to_string(),
                    attr_name: "Cool".to_string(),
                    value: Some(self.cooling_mode.to_string()),
                },
                ProjectEdit::SetControlAttr {
                    control_uuid: block_uuid.to_string(),
                    attr_name: "UseHVAc".to_string(),
                    value: self.use_hvac.then(|| "1".to_string()),
                },
                ProjectEdit::SetControlAttr {
                    control_uuid: block_uuid.to_string(),
                    attr_name: "Irr".to_string(),
                    value: (!irr_str.is_empty()).then_some(irr_str),
                },
                ProjectEdit::SetPinDefault {
                    control_uuid: block_uuid.to_string(),
                    pin_key: "Min".to_string(),
                    default_value: Some(fmt_f32(self.min_flow_temp)),
                },
                ProjectEdit::SetPinDefault {
                    control_uuid: block_uuid.to_string(),
                    pin_key: "Max".to_string(),
                    default_value: Some(fmt_f32(self.max_flow_temp)),
                },
                ProjectEdit::SetPinDefault {
                    control_uuid: block_uuid.to_string(),
                    pin_key: "BufferInc".to_string(),
                    default_value: Some(fmt_f32(self.buffer_inc)),
                },
                ProjectEdit::SetPinDefault {
                    control_uuid: block_uuid.to_string(),
                    pin_key: "PrevInc".to_string(),
                    default_value: Some(fmt_f32(self.prev_inc)),
                },
                ProjectEdit::SetPinDefault {
                    control_uuid: block_uuid.to_string(),
                    pin_key: "Transconductance".to_string(),
                    default_value: Some(fmt_f32(self.transconductance)),
                },
                ProjectEdit::SetPinDefault {
                    control_uuid: block_uuid.to_string(),
                    pin_key: "Treshold".to_string(),
                    default_value: Some(fmt_f32(self.threshold)),
                },
                ProjectEdit::SetPinDefault {
                    control_uuid: block_uuid.to_string(),
                    pin_key: "Gain".to_string(),
                    default_value: Some(fmt_f32(self.gain)),
                },
            ]);
        } else {
            if let Some(proj) = project {
                for page in &proj.pages {
                    for cand in &page.blocks {
                        if matches!(
                            cand.block_type.as_str(),
                            "HeatIRoomController2" | "HeatIRoomController"
                        ) {
                            let mut current_objs: Vec<String> =
                                find_attr(&cand.raw_attributes, "Objects")
                                    .unwrap_or_default()
                                    .split(',')
                                    .map(str::trim)
                                    .filter(|s| !s.is_empty())
                                    .map(str::to_string)
                                    .collect();
                            let want = self.is_irc_assigned(&cand.uuid);
                            let has = current_objs.iter().any(|u| u == block_uuid);
                            if want && !has {
                                current_objs.push(block_uuid.to_string());
                                edits.push(ProjectEdit::SetControlAttr {
                                    control_uuid: cand.uuid.clone(),
                                    attr_name: "Objects".to_string(),
                                    value: Some(current_objs.join(",")),
                                });
                            } else if !want && has {
                                current_objs.retain(|u| u != block_uuid);
                                edits.push(ProjectEdit::SetControlAttr {
                                    control_uuid: cand.uuid.clone(),
                                    attr_name: "Objects".to_string(),
                                    value: (!current_objs.is_empty())
                                        .then(|| current_objs.join(",")),
                                });
                            }
                        }
                    }
                }
            }
            edits.extend([
                ProjectEdit::SetControlAttr {
                    control_uuid: block_uuid.to_string(),
                    attr_name: "cCap".to_string(),
                    value: Some(self.capabilities.to_string()),
                },
                ProjectEdit::SetControlAttr {
                    control_uuid: block_uuid.to_string(),
                    attr_name: "DType".to_string(),
                    value: Some(self.device_type.to_string()),
                },
                ProjectEdit::SetPinDefault {
                    control_uuid: block_uuid.to_string(),
                    pin_key: "TempLimitC".to_string(),
                    default_value: Some(fmt_f32(self.temp_limit_c)),
                },
                ProjectEdit::SetPinDefault {
                    control_uuid: block_uuid.to_string(),
                    pin_key: "TempLimitH".to_string(),
                    default_value: Some(fmt_f32(self.temp_limit_h)),
                },
                ProjectEdit::SetPinDefault {
                    control_uuid: block_uuid.to_string(),
                    pin_key: "OnThreshold".to_string(),
                    default_value: Some(fmt_f32(self.on_threshold)),
                },
                ProjectEdit::SetPinDefault {
                    control_uuid: block_uuid.to_string(),
                    pin_key: "HTarget".to_string(),
                    default_value: Some(fmt_f32(self.humid_target)),
                },
                ProjectEdit::SetPinDefault {
                    control_uuid: block_uuid.to_string(),
                    pin_key: "TimeOffPump".to_string(),
                    default_value: Some(self.time_off_pump.to_string()),
                },
                ProjectEdit::SetPinDefault {
                    control_uuid: block_uuid.to_string(),
                    pin_key: "FanDelay".to_string(),
                    default_value: Some(self.fan_delay.to_string()),
                },
                ProjectEdit::SetPinDefault {
                    control_uuid: block_uuid.to_string(),
                    pin_key: "TPMin".to_string(),
                    default_value: Some(fmt_f32(self.tp_min)),
                },
                ProjectEdit::SetPinDefault {
                    control_uuid: block_uuid.to_string(),
                    pin_key: "TPMax".to_string(),
                    default_value: Some(fmt_f32(self.tp_max)),
                },
            ]);
        }
        ProjectEdit::Batch(edits)
    }
}

// ============================================================================
// 4. SEQUENCE CONTROLLER (`Sequence`)
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct SequenceStepRow {
    pub label: String,
    pub condition: String,
    pub aq1: f32,
    pub aq2: f32,
    pub aq3: f32,
    pub aq4: f32,
    pub wait_sec: f32,
    pub comment: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequenceConfig {
    pub steps: Vec<SequenceStepRow>,
    other_extra_children: Vec<RawNode>,
}

impl SequenceConfig {
    pub fn is_sequence_block(block_type: &str) -> bool {
        block_type == "Sequence"
    }

    pub fn from_block(block: &PageBlock) -> Self {
        let mut steps = Vec::new();
        let mut other_extra_children = Vec::new();
        for child in &block.extra_children {
            if child.tag == "SequenceSteps" {
                for step_node in &child.children {
                    if step_node.tag == "Step" {
                        steps.push(SequenceStepRow {
                            label: step_node.attr("Label").unwrap_or("Step").to_string(),
                            condition: step_node.attr("Cond").unwrap_or("AI1 > 0").to_string(),
                            aq1: step_node
                                .attr("AQ1")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0.0),
                            aq2: step_node
                                .attr("AQ2")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0.0),
                            aq3: step_node
                                .attr("AQ3")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0.0),
                            aq4: step_node
                                .attr("AQ4")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0.0),
                            wait_sec: step_node
                                .attr("Wait")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(5.0),
                            comment: step_node.attr("Comment").unwrap_or_default().to_string(),
                        });
                    }
                }
            } else {
                other_extra_children.push(child.clone());
            }
        }

        if steps.is_empty() {
            steps = vec![
                SequenceStepRow {
                    label: "Step 1 — Init".to_string(),
                    condition: "Tr == 1".to_string(),
                    aq1: 1.0,
                    aq2: 0.0,
                    aq3: 0.0,
                    aq4: 0.0,
                    wait_sec: 5.0,
                    comment: "Activate primary stage".to_string(),
                },
                SequenceStepRow {
                    label: "Step 2 — Hold".to_string(),
                    condition: "AI1 >= 1".to_string(),
                    aq1: 1.0,
                    aq2: 1.0,
                    aq3: 0.0,
                    aq4: 0.0,
                    wait_sec: 10.0,
                    comment: "Secondary output stage".to_string(),
                },
            ];
        }

        Self {
            steps,
            other_extra_children,
        }
    }

    pub fn add_step(&mut self) {
        let next_no = self.steps.len() + 1;
        self.steps.push(SequenceStepRow {
            label: format!("Step {next_no}"),
            condition: "1".to_string(),
            aq1: 0.0,
            aq2: 0.0,
            aq3: 0.0,
            aq4: 0.0,
            wait_sec: 5.0,
            comment: String::new(),
        });
    }

    pub fn remove_step(&mut self, index: usize) {
        if index < self.steps.len() {
            self.steps.remove(index);
        }
    }

    pub fn move_up(&mut self, index: usize) {
        if index > 0 && index < self.steps.len() {
            self.steps.swap(index, index - 1);
        }
    }

    pub fn move_down(&mut self, index: usize) {
        if index + 1 < self.steps.len() {
            self.steps.swap(index, index + 1);
        }
    }

    pub fn format_script(&self) -> String {
        self.steps
            .iter()
            .enumerate()
            .map(|(idx, s)| {
                format!(
                    "{}: IF ({}) THEN AQ1={}, AQ2={}, AQ3={}, AQ4={} WAIT {}s",
                    idx + 1,
                    s.condition,
                    fmt_f32(s.aq1),
                    fmt_f32(s.aq2),
                    fmt_f32(s.aq3),
                    fmt_f32(s.aq4),
                    fmt_f32(s.wait_sec)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn to_project_edit(&self, _page_uuid: &str, block_uuid: &str) -> ProjectEdit {
        let mut extra_children = self.other_extra_children.clone();
        let step_children = self
            .steps
            .iter()
            .map(|s| RawNode {
                tag: "Step".to_string(),
                attributes: vec![
                    ("Label".to_string(), s.label.clone()),
                    ("Cond".to_string(), s.condition.clone()),
                    ("AQ1".to_string(), fmt_f32(s.aq1)),
                    ("AQ2".to_string(), fmt_f32(s.aq2)),
                    ("AQ3".to_string(), fmt_f32(s.aq3)),
                    ("AQ4".to_string(), fmt_f32(s.aq4)),
                    ("Wait".to_string(), fmt_f32(s.wait_sec)),
                    ("Comment".to_string(), s.comment.clone()),
                ],
                text: None,
                children: Vec::new(),
            })
            .collect();
        extra_children.push(RawNode {
            tag: "SequenceSteps".to_string(),
            attributes: vec![("Count".to_string(), self.steps.len().to_string())],
            text: None,
            children: step_children,
        });

        ProjectEdit::Batch(vec![
            ProjectEdit::SetControlAttr {
                control_uuid: block_uuid.to_string(),
                attr_name: "Script".to_string(),
                value: Some(self.format_script()),
            },
            ProjectEdit::SetExtraChildren {
                control_uuid: block_uuid.to_string(),
                extra_children,
            },
        ])
    }
}

// ============================================================================
// 5. SHADING / AUTOMATIC BLINDS (`AutoJalousie`, `Jalousie`, `RoofWindowShading`)
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct ShadingBlockConfig {
    pub sun_automation: bool,
    pub blind_type: u32,
    pub compass_dir: u32,
    pub time_up: f32,
    pub time_down: f32,
    pub turn_offset: f32,
    pub min_move: f32,
    pub back_turn: f32,
    pub dir_tol_start: f32,
    pub dir_tol_end: f32,
    pub slat_width_mm: u32,
    pub slat_space_mm: u32,
    pub auto_shade_end: u32,
    pub auto_mode: u32,
    pub auto_shade_time_min: u32,
    pub sunrise_offset_min: i32,
    pub sunset_offset_min: i32,
    pub linked_actuator_uuid: Option<String>,
}

impl ShadingBlockConfig {
    pub fn is_shading_block(block_type: &str) -> bool {
        matches!(
            block_type,
            "AutoJalousie" | "Jalousie" | "RoofWindowShading"
        )
    }

    pub fn from_block(block: &PageBlock) -> Self {
        let sun_automation = find_attr(&block.raw_attributes, "Sun")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(true);
        let linked_actuator_uuid = find_attr(&block.raw_attributes, "Ref")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .or_else(|| block.ref_uuid.clone());

        Self {
            sun_automation,
            blind_type: parse_pin_u32(block, "Type", 1),
            compass_dir: parse_pin_u32(block, "Dir", 8),
            time_up: parse_pin_f32(block, "TimeEnd", 30.0),
            time_down: parse_pin_f32(block, "TimeEndDown", 28.0),
            turn_offset: parse_pin_f32(block, "TurnOffset", 0.15),
            min_move: parse_pin_f32(block, "MinMove", 0.4),
            back_turn: parse_pin_f32(block, "Back", 0.8),
            dir_tol_start: parse_pin_f32(block, "DirTol", 85.0),
            dir_tol_end: parse_pin_f32(block, "DirTol2", 85.0),
            slat_width_mm: parse_pin_u32(block, "Width", 70),
            slat_space_mm: parse_pin_u32(block, "Space", 60),
            auto_shade_end: parse_pin_u32(block, "AutoShadeEnd", 1),
            auto_mode: parse_pin_u32(block, "AutMode", 1),
            auto_shade_time_min: parse_pin_u32(block, "AutoShadeTime", 120),
            sunrise_offset_min: parse_pin_i32(block, "SRoff", 30),
            sunset_offset_min: parse_pin_i32(block, "SSoff", -30),
            linked_actuator_uuid,
        }
    }

    pub fn to_project_edit(&self, _page_uuid: &str, block_uuid: &str) -> ProjectEdit {
        let mut edits = vec![
            ProjectEdit::SetControlAttr {
                control_uuid: block_uuid.to_string(),
                attr_name: "Sun".to_string(),
                value: Some(if self.sun_automation {
                    "true".to_string()
                } else {
                    "false".to_string()
                }),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "Type".to_string(),
                default_value: Some(self.blind_type.to_string()),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "Dir".to_string(),
                default_value: Some(self.compass_dir.to_string()),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "TimeEnd".to_string(),
                default_value: Some(fmt_f32(self.time_up)),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "TimeEndDown".to_string(),
                default_value: Some(fmt_f32(self.time_down)),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "TurnOffset".to_string(),
                default_value: Some(fmt_f32(self.turn_offset)),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "MinMove".to_string(),
                default_value: Some(fmt_f32(self.min_move)),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "Back".to_string(),
                default_value: Some(fmt_f32(self.back_turn)),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "DirTol".to_string(),
                default_value: Some(fmt_f32(self.dir_tol_start)),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "DirTol2".to_string(),
                default_value: Some(fmt_f32(self.dir_tol_end)),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "Width".to_string(),
                default_value: Some(self.slat_width_mm.to_string()),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "Space".to_string(),
                default_value: Some(self.slat_space_mm.to_string()),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "AutoShadeEnd".to_string(),
                default_value: Some(self.auto_shade_end.to_string()),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "AutMode".to_string(),
                default_value: Some(self.auto_mode.to_string()),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "AutoShadeTime".to_string(),
                default_value: Some(self.auto_shade_time_min.to_string()),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "SRoff".to_string(),
                default_value: Some(self.sunrise_offset_min.to_string()),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "SSoff".to_string(),
                default_value: Some(self.sunset_offset_min.to_string()),
            },
        ];
        if let Some(ref_u) = &self.linked_actuator_uuid {
            edits.push(ProjectEdit::SetControlAttr {
                control_uuid: block_uuid.to_string(),
                attr_name: "Ref".to_string(),
                value: (!ref_u.trim().is_empty()).then(|| ref_u.trim().to_string()),
            });
        }
        ProjectEdit::Batch(edits)
    }
}

// ============================================================================
// 6. ACCESS & INTERCOM (`Doorcontroller`, `Intercom`, `Door`, `NfcCodeTouch`, `Gate`)
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct AccessBlockConfig {
    pub block_type: String,
    pub out_name_1: String,
    pub out_name_2: String,
    pub out_name_3: String,
    pub nfc_output_labels: [String; 6],
    pub intercom_answers: String,
    pub door_number: u32,
    pub ctrl_type: u32,
    pub history_size: u32,
    pub timeout_sec: f32,
    pub pulse_time_sec: f32,
    pub pulse_open_sec: f32,
    pub pulse_close_sec: f32,
    pub led_brightness: u32,
    pub acoustic_feedback: u32,
    pub assigned_audio_uuid: Option<String>,
}

impl AccessBlockConfig {
    pub fn is_access_block(block_type: &str) -> bool {
        matches!(
            block_type,
            "Doorcontroller" | "Intercom" | "Door" | "NfcCodeTouch" | "Gate"
        )
    }

    pub fn from_block(block: &PageBlock) -> Self {
        let mut nfc_output_labels = [
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ];
        if let Some(ostr) = find_attr(&block.raw_attributes, "OStr") {
            for (i, part) in ostr.split(';').take(6).enumerate() {
                nfc_output_labels[i] = part.trim().to_string();
            }
        }
        let assigned_audio_uuid = find_attr(&block.raw_attributes, "Objects")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);

        Self {
            block_type: block.block_type.clone(),
            out_name_1: find_attr(&block.raw_attributes, "OutName1")
                .unwrap_or_default()
                .to_string(),
            out_name_2: find_attr(&block.raw_attributes, "OutName2")
                .unwrap_or_default()
                .to_string(),
            out_name_3: find_attr(&block.raw_attributes, "OutName3")
                .unwrap_or_default()
                .to_string(),
            nfc_output_labels,
            intercom_answers: find_attr(&block.raw_attributes, "ans")
                .unwrap_or_default()
                .to_string(),
            door_number: find_attr(&block.raw_attributes, "DoorNumber")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1),
            ctrl_type: find_attr(&block.raw_attributes, "CtrlType")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1),
            history_size: find_attr(&block.raw_attributes, "HSize")
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            timeout_sec: parse_pin_f32(block, "Timeout", 60.0),
            pulse_time_sec: parse_pin_f32(block, "PulseTime", 5.0),
            pulse_open_sec: parse_pin_f32(block, "PulseOpen", 60.0),
            pulse_close_sec: parse_pin_f32(block, "PulseClose", 60.0),
            led_brightness: parse_pin_u32(block, "Lbr", 50),
            acoustic_feedback: if block.block_type == "Intercom" {
                parse_pin_u32(block, "Tp", 1)
            } else {
                parse_pin_u32(block, "Au", 1)
            },
            assigned_audio_uuid,
        }
    }

    pub fn to_project_edit(&self, _page_uuid: &str, block_uuid: &str) -> ProjectEdit {
        let mut edits = Vec::new();
        match self.block_type.as_str() {
            "NfcCodeTouch" => {
                let ostr_joined = self
                    .nfc_output_labels
                    .iter()
                    .map(|s| if s.trim().is_empty() { " " } else { s.trim() })
                    .collect::<Vec<_>>()
                    .join(";");
                edits.extend([
                    ProjectEdit::SetControlAttr {
                        control_uuid: block_uuid.to_string(),
                        attr_name: "OStr".to_string(),
                        value: Some(ostr_joined),
                    },
                    ProjectEdit::SetControlAttr {
                        control_uuid: block_uuid.to_string(),
                        attr_name: "HSize".to_string(),
                        value: Some(self.history_size.to_string()),
                    },
                    ProjectEdit::SetPinDefault {
                        control_uuid: block_uuid.to_string(),
                        pin_key: "PulseTime".to_string(),
                        default_value: Some(fmt_f32(self.pulse_time_sec)),
                    },
                    ProjectEdit::SetPinDefault {
                        control_uuid: block_uuid.to_string(),
                        pin_key: "Lbr".to_string(),
                        default_value: Some(self.led_brightness.to_string()),
                    },
                    ProjectEdit::SetPinDefault {
                        control_uuid: block_uuid.to_string(),
                        pin_key: "Au".to_string(),
                        default_value: Some(self.acoustic_feedback.to_string()),
                    },
                ]);
            }
            "Door" | "Gate" => {
                edits.extend([
                    ProjectEdit::SetControlAttr {
                        control_uuid: block_uuid.to_string(),
                        attr_name: "CtrlType".to_string(),
                        value: Some(self.ctrl_type.to_string()),
                    },
                    ProjectEdit::SetPinDefault {
                        control_uuid: block_uuid.to_string(),
                        pin_key: "PulseOpen".to_string(),
                        default_value: Some(fmt_f32(self.pulse_open_sec)),
                    },
                    ProjectEdit::SetPinDefault {
                        control_uuid: block_uuid.to_string(),
                        pin_key: "PulseClose".to_string(),
                        default_value: Some(fmt_f32(self.pulse_close_sec)),
                    },
                ]);
            }
            "Intercom" => {
                edits.extend([
                    ProjectEdit::SetControlAttr {
                        control_uuid: block_uuid.to_string(),
                        attr_name: "OutName1".to_string(),
                        value: (!self.out_name_1.is_empty()).then(|| self.out_name_1.clone()),
                    },
                    ProjectEdit::SetControlAttr {
                        control_uuid: block_uuid.to_string(),
                        attr_name: "OutName2".to_string(),
                        value: (!self.out_name_2.is_empty()).then(|| self.out_name_2.clone()),
                    },
                    ProjectEdit::SetControlAttr {
                        control_uuid: block_uuid.to_string(),
                        attr_name: "OutName3".to_string(),
                        value: (!self.out_name_3.is_empty()).then(|| self.out_name_3.clone()),
                    },
                    ProjectEdit::SetControlAttr {
                        control_uuid: block_uuid.to_string(),
                        attr_name: "ans".to_string(),
                        value: (!self.intercom_answers.is_empty())
                            .then(|| self.intercom_answers.clone()),
                    },
                    ProjectEdit::SetControlAttr {
                        control_uuid: block_uuid.to_string(),
                        attr_name: "Objects".to_string(),
                        value: self
                            .assigned_audio_uuid
                            .as_ref()
                            .filter(|s| !s.trim().is_empty())
                            .cloned(),
                    },
                    ProjectEdit::SetPinDefault {
                        control_uuid: block_uuid.to_string(),
                        pin_key: "Timeout".to_string(),
                        default_value: Some(fmt_f32(self.timeout_sec)),
                    },
                    ProjectEdit::SetPinDefault {
                        control_uuid: block_uuid.to_string(),
                        pin_key: "Lbr".to_string(),
                        default_value: Some(self.led_brightness.to_string()),
                    },
                    ProjectEdit::SetPinDefault {
                        control_uuid: block_uuid.to_string(),
                        pin_key: "Tp".to_string(),
                        default_value: Some(self.acoustic_feedback.to_string()),
                    },
                ]);
            }
            _ => {
                // Doorcontroller
                edits.extend([
                    ProjectEdit::SetControlAttr {
                        control_uuid: block_uuid.to_string(),
                        attr_name: "OutName1".to_string(),
                        value: (!self.out_name_1.is_empty()).then(|| self.out_name_1.clone()),
                    },
                    ProjectEdit::SetControlAttr {
                        control_uuid: block_uuid.to_string(),
                        attr_name: "OutName2".to_string(),
                        value: (!self.out_name_2.is_empty()).then(|| self.out_name_2.clone()),
                    },
                    ProjectEdit::SetControlAttr {
                        control_uuid: block_uuid.to_string(),
                        attr_name: "OutName3".to_string(),
                        value: (!self.out_name_3.is_empty()).then(|| self.out_name_3.clone()),
                    },
                    ProjectEdit::SetControlAttr {
                        control_uuid: block_uuid.to_string(),
                        attr_name: "DoorNumber".to_string(),
                        value: Some(self.door_number.to_string()),
                    },
                    ProjectEdit::SetControlAttr {
                        control_uuid: block_uuid.to_string(),
                        attr_name: "Objects".to_string(),
                        value: self
                            .assigned_audio_uuid
                            .as_ref()
                            .filter(|s| !s.trim().is_empty())
                            .cloned(),
                    },
                    ProjectEdit::SetPinDefault {
                        control_uuid: block_uuid.to_string(),
                        pin_key: "Timeout".to_string(),
                        default_value: Some(fmt_f32(self.timeout_sec)),
                    },
                ]);
            }
        }
        ProjectEdit::Batch(edits)
    }
}

// ============================================================================
// 7. PRESENCE DETECTOR (`PresenceDetector`)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceSensorCandidate {
    pub uuid: String,
    pub title: String,
    pub control_type: String,
    pub parent_title: String,
    pub serial: String,
}

fn collect_presence_sensors_recursive(
    node: &TreeNode,
    parent_title: &str,
    out: &mut Vec<PresenceSensorCandidate>,
) {
    if matches!(
        node.control_type.as_str(),
        "TreeDevice" | "AirDevice" | "IntercomDevice"
    ) && !node.uuid.is_empty()
    {
        let serial = find_attr(&node.raw_attributes, "Serial")
            .unwrap_or_default()
            .to_string();
        let title_l = node.title.to_lowercase();
        let is_sensor_candidate = title_l.contains("czujnik")
            || title_l.contains("obecno")
            || title_l.contains("presence")
            || title_l.contains("motion")
            || title_l.contains("touch")
            || title_l.contains("intercom")
            || node
                .children
                .iter()
                .any(|c| matches!(c.control_type.as_str(), "SensorIn" | "DaytimerIn" | "SoundIn"));
        if is_sensor_candidate {
            out.push(PresenceSensorCandidate {
                uuid: node.uuid.clone(),
                title: if node.title.is_empty() {
                    node.control_type.clone()
                } else {
                    node.title.clone()
                },
                control_type: node.control_type.clone(),
                parent_title: parent_title.to_string(),
                serial,
            });
        }
    }
    let next_parent = if !node.title.is_empty() {
        node.title.as_str()
    } else {
        parent_title
    };
    for child in &node.children {
        collect_presence_sensors_recursive(child, next_parent, out);
    }
}

pub fn collect_presence_sensor_candidates(project: &LoxoneProject) -> Vec<PresenceSensorCandidate> {
    let mut out = Vec::new();
    for node in project
        .periphery_tree
        .iter()
        .chain(project.system_tree.iter())
    {
        collect_presence_sensors_recursive(node, &node.title, &mut out);
    }
    out.sort_by(|a, b| polish_sort_key(&a.title).cmp(&polish_sort_key(&b.title)));
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct PresenceBlockConfig {
    pub hold_time_sec: f32,
    pub warn_time_sec: f32,
    pub track_ne: u32,
    pub track_max: u32,
    pub assigned_device_uuids: Vec<String>,
}

impl PresenceBlockConfig {
    pub fn is_presence_block(block_type: &str) -> bool {
        block_type == "PresenceDetector"
    }

    pub fn from_block(block: &PageBlock) -> Self {
        let assigned_device_uuids = find_attr(&block.raw_attributes, "DEVS")
            .unwrap_or_default()
            .split(|c| c == ',' || c == ';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        let track_ne = find_attr(&block.raw_attributes, "TRK_NE")
            .and_then(|v| v.parse().ok())
            .unwrap_or(50);
        let track_max = find_attr(&block.raw_attributes, "TRK_MAX")
            .and_then(|v| v.parse().ok())
            .unwrap_or(50);

        Self {
            hold_time_sec: parse_pin_f32(block, "ParamTOn", 900.0),
            warn_time_sec: parse_pin_f32(block, "ParamTWarn", 15.0),
            track_ne,
            track_max,
            assigned_device_uuids,
        }
    }

    pub fn is_device_assigned(&self, uuid: &str) -> bool {
        self.assigned_device_uuids.iter().any(|u| u == uuid)
    }

    pub fn toggle_device(&mut self, uuid: &str) {
        if let Some(pos) = self.assigned_device_uuids.iter().position(|u| u == uuid) {
            self.assigned_device_uuids.remove(pos);
        } else {
            self.assigned_device_uuids.push(uuid.to_string());
        }
    }

    pub fn to_project_edit(&self, _page_uuid: &str, block_uuid: &str) -> ProjectEdit {
        let devs_joined = self.assigned_device_uuids.join(",");
        ProjectEdit::Batch(vec![
            ProjectEdit::SetControlAttr {
                control_uuid: block_uuid.to_string(),
                attr_name: "DEVS".to_string(),
                value: (!devs_joined.is_empty()).then_some(devs_joined),
            },
            ProjectEdit::SetControlAttr {
                control_uuid: block_uuid.to_string(),
                attr_name: "TRK_NE".to_string(),
                value: Some(self.track_ne.to_string()),
            },
            ProjectEdit::SetControlAttr {
                control_uuid: block_uuid.to_string(),
                attr_name: "TRK_MAX".to_string(),
                value: Some(self.track_max.to_string()),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "ParamTOn".to_string(),
                default_value: Some(fmt_f32(self.hold_time_sec)),
            },
            ProjectEdit::SetPinDefault {
                control_uuid: block_uuid.to_string(),
                pin_key: "ParamTWarn".to_string(),
                default_value: Some(fmt_f32(self.warn_time_sec)),
            },
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factory::new_page_block;
    use crate::parser::load_loxone_file;
    use crate::writer::apply_edit_with_inverse;
    use std::path::PathBuf;

    #[test]
    fn test_klimczaki_advanced_block_configs_roundtrip() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Sample.Loxone");
        let mut proj = load_loxone_file(&path).expect("load Sample.Loxone");

        // 1. CentralLight (`19eb19aa-03b9-3ff5-ffff51535bcccc04`)
        let cl_uuid = "19eb19aa-03b9-3ff5-ffff51535bcccc04";
        let (cl_page_uuid, cl_block) = proj
            .pages
            .iter()
            .find_map(|p| {
                p.blocks
                    .iter()
                    .find(|b| b.uuid == cl_uuid)
                    .map(|b| (p.uuid.clone(), b.clone()))
            })
            .expect("find CentralLight");
        assert!(supports_advanced_block_modal(&cl_block.block_type));
        let mut cl_cfg = CentralBlockConfig::from_block(&cl_block);
        assert_eq!(cl_cfg.kind, CentralKind::Lighting);
        assert_eq!(cl_cfg.selected_uuids.len(), 10);
        let candidates = collect_central_candidates(&proj, &cl_block.block_type);
        assert!(candidates.len() >= 10);
        let first_uuid = cl_cfg.selected_uuids[0].clone();
        cl_cfg.toggle_uuid(&first_uuid);
        let edit = cl_cfg.to_project_edit(&cl_page_uuid, cl_uuid);
        let inv = apply_edit_with_inverse(&mut proj, &edit).unwrap();
        apply_edit_with_inverse(&mut proj, &inv).unwrap();

        // 2. MusicPlayer (`19eb19a5-0392-28eb-ffff51535bcccc04`)
        let mp_uuid = "19eb19a5-0392-28eb-ffff51535bcccc04";
        let (mp_page_uuid, mp_block) = proj
            .pages
            .iter()
            .find_map(|p| {
                p.blocks
                    .iter()
                    .find(|b| b.uuid == mp_uuid)
                    .map(|b| (p.uuid.clone(), b.clone()))
            })
            .expect("find MusicPlayer");
        let all_channels = collect_audio_out_channels(&proj);
        assert!(!all_channels.is_empty());
        let mut mp_cfg = AudioZoneConfig::from_block(&mp_block, &proj);
        assert_eq!(mp_cfg.default_vol, 20);
        assert_eq!(mp_cfg.alarm_vol, 75);
        assert_eq!(mp_cfg.assigned_channel_uuids.len(), 4);
        mp_cfg.default_vol = 25;
        let mp_edit = mp_cfg.to_project_edit(&mp_page_uuid, mp_uuid, &all_channels);
        let mp_inv = apply_edit_with_inverse(&mut proj, &mp_edit).unwrap();
        apply_edit_with_inverse(&mut proj, &mp_inv).unwrap();

        // 3. Heatmixer2 (`19eb19ab-001b-406d-ffff51535bcccc04`) & ClimateControllerUS
        let hm_uuid = "19eb19ab-001b-406d-ffff51535bcccc04";
        let (hm_page_uuid, hm_block) = proj
            .pages
            .iter()
            .find_map(|p| {
                p.blocks
                    .iter()
                    .find(|b| b.uuid == hm_uuid)
                    .map(|b| (p.uuid.clone(), b.clone()))
            })
            .expect("find Heatmixer2");
        let mut hm_cfg = ClimateControllerConfig::from_block(&hm_block);
        assert!(hm_cfg.is_heatmixer);
        assert_eq!(hm_cfg.assigned_irc_uuids.len(), 7);
        assert_eq!(hm_cfg.max_flow_temp, 40.0);
        hm_cfg.max_flow_temp = 42.0;
        let hm_edit = hm_cfg.to_project_edit(&hm_page_uuid, hm_uuid);
        let hm_inv = apply_edit_with_inverse(&mut proj, &hm_edit).unwrap();
        apply_edit_with_inverse(&mut proj, &hm_inv).unwrap();

        // 4. Sequence Controller
        let seq_block = new_page_block("Sequence", "Test Sequence", 100.0, 100.0);
        let mut seq_cfg = SequenceConfig::from_block(&seq_block);
        assert_eq!(seq_cfg.steps.len(), 2);
        seq_cfg.add_step();
        assert_eq!(seq_cfg.steps.len(), 3);

        // 5. AutoJalousie
        let (aj_page_uuid, aj_block) = proj
            .pages
            .iter()
            .find_map(|p| {
                p.blocks
                    .iter()
                    .find(|b| b.block_type == "AutoJalousie")
                    .map(|b| (p.uuid.clone(), b.clone()))
            })
            .expect("find AutoJalousie");
        let aj_uuid = &aj_block.uuid;
        assert!(supports_advanced_block_modal(&aj_block.block_type));
        let mut aj_cfg = ShadingBlockConfig::from_block(&aj_block);
        assert!(aj_cfg.sun_automation);
        assert_eq!(aj_cfg.blind_type, 1);
        assert!(aj_cfg.time_up > 10.0);
        assert!(aj_cfg.time_down > 10.0);
        aj_cfg.time_up = 31.5;
        let aj_edit = aj_cfg.to_project_edit(&aj_page_uuid, aj_uuid);
        let aj_inv = apply_edit_with_inverse(&mut proj, &aj_edit).unwrap();
        apply_edit_with_inverse(&mut proj, &aj_inv).unwrap();

        // 6. Access blocks: Intercom, NfcCodeTouch
        let (ic_page_uuid, ic_block) = proj
            .pages
            .iter()
            .find_map(|p| {
                p.blocks
                    .iter()
                    .find(|b| b.block_type == "Intercom")
                    .map(|b| (p.uuid.clone(), b.clone()))
            })
            .expect("find Intercom");
        let ic_uuid = &ic_block.uuid;
        assert!(supports_advanced_block_modal(&ic_block.block_type));
        let mut ic_cfg = AccessBlockConfig::from_block(&ic_block);
        assert_eq!(ic_cfg.out_name_1, "Otwórz furtkę");
        assert!(ic_cfg.intercom_answers.contains("Nie ma nas w domu."));
        ic_cfg.timeout_sec = 45.0;
        let ic_edit = ic_cfg.to_project_edit(&ic_page_uuid, ic_uuid);
        let ic_inv = apply_edit_with_inverse(&mut proj, &ic_edit).unwrap();
        apply_edit_with_inverse(&mut proj, &ic_inv).unwrap();

        let (nfc_page_uuid, nfc_block) = proj
            .pages
            .iter()
            .find_map(|p| {
                p.blocks
                    .iter()
                    .find(|b| b.block_type == "NfcCodeTouch")
                    .map(|b| (p.uuid.clone(), b.clone()))
            })
            .expect("find NfcCodeTouch");
        let nfc_uuid = &nfc_block.uuid;
        let mut nfc_cfg = AccessBlockConfig::from_block(&nfc_block);
        assert_eq!(nfc_cfg.nfc_output_labels[0], "wiatrołap: elektrozaczep");
        assert_eq!(nfc_cfg.nfc_output_labels[1], "Brama");
        nfc_cfg.pulse_time_sec = 6.0;
        let nfc_edit = nfc_cfg.to_project_edit(&nfc_page_uuid, nfc_uuid);
        let nfc_inv = apply_edit_with_inverse(&mut proj, &nfc_edit).unwrap();
        apply_edit_with_inverse(&mut proj, &nfc_inv).unwrap();

        // 7. PresenceDetector
        let (pd_page_uuid, pd_block) = proj
            .pages
            .iter()
            .find_map(|p| {
                p.blocks
                    .iter()
                    .find(|b| b.block_type == "PresenceDetector")
                    .map(|b| (p.uuid.clone(), b.clone()))
            })
            .expect("find PresenceDetector");
        let pd_uuid = &pd_block.uuid;
        assert!(supports_advanced_block_modal(&pd_block.block_type));
        let mut pd_cfg = PresenceBlockConfig::from_block(&pd_block);
        assert_eq!(pd_cfg.hold_time_sec, 900.0);
        assert_eq!(pd_cfg.warn_time_sec, 15.0);
        assert!(!pd_cfg.assigned_device_uuids.is_empty());
        let pd_candidates = collect_presence_sensor_candidates(&proj);
        assert!(!pd_candidates.is_empty());
        pd_cfg.hold_time_sec = 600.0;
        let pd_edit = pd_cfg.to_project_edit(&pd_page_uuid, pd_uuid);
        let pd_inv = apply_edit_with_inverse(&mut proj, &pd_edit).unwrap();
        apply_edit_with_inverse(&mut proj, &pd_inv).unwrap();
    }

    #[test]
    fn test_hvac_and_climate_controllers_resolve_assigned_irc2_objects_and_short_labels() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Sample.Loxone");
        let proj = load_loxone_file(&path).expect("load Sample.Loxone");

        // HVACController ("Sterownik klimatu"), Heatmixer2 ("Inteligentny sterownik temperatury"),
        // and ClimateControllerUS ("HVAC Controller") on "Funkcje centralne Ogrzewanie"
        for uuid in [
            "19eb19aa-03bc-4048-ffff51535bcccc04",
            "19eb19ab-001b-406d-ffff51535bcccc04",
            "19f5278e-0184-ed04-ffff51535bcccc04",
        ] {
            let block = proj
                .pages
                .iter()
                .flat_map(|p| &p.blocks)
                .find(|b| b.uuid == uuid)
                .expect("climate block should exist");
            assert!(
                ClimateControllerConfig::is_climate_controller_block(&block.block_type),
                "{} should be recognized as climate controller block",
                block.block_type
            );
            let summary = collect_block_assigned_objects(block, &proj)
                .expect("climate block should have object assignment summary");
            assert_eq!(summary.assigned.len(), 7);
            assert_eq!(summary.total_candidates, 7);
            assert_eq!(summary.assigned[0].display_label, "Irc2: Gabinet");
            assert_eq!(summary.assigned[6].display_label, "Irc2: Wiatrołap");
        }

        assert_eq!(
            crate::miniserver::loxone_short_pin_label("HVACController", "Temp"),
            "ϑo"
        );
        assert_eq!(
            crate::miniserver::loxone_short_pin_label("HVACController", "Stop"),
            "Off"
        );
        assert_eq!(
            crate::miniserver::loxone_short_pin_label("HVACController", "CoolAvailable"),
            "Ec"
        );
        assert_eq!(
            crate::miniserver::loxone_short_pin_label("HVACController", "HeatAvailable"),
            "Eh"
        );
        assert_eq!(
            crate::miniserver::loxone_short_pin_label("HVACController", "OutHeat1"),
            "H"
        );
        assert_eq!(
            crate::miniserver::loxone_short_pin_label("HVACController", "OutHeat2"),
            "H2"
        );
        assert_eq!(
            crate::miniserver::loxone_short_pin_label("HVACController", "OutCool1"),
            "C"
        );
        assert_eq!(
            crate::miniserver::loxone_short_pin_label("HVACController", "OutCool2"),
            "C2"
        );
        assert_eq!(
            crate::miniserver::loxone_short_pin_label("HVACController", "OutSecHeat"),
            "Ah"
        );
        assert_eq!(
            crate::miniserver::loxone_short_pin_label("HVACController", "OutValve"),
            "Sv"
        );
        assert_eq!(
            crate::miniserver::loxone_short_pin_label("HVACController", "OutFan"),
            "F"
        );
        assert_eq!(
            crate::miniserver::loxone_short_pin_label("HVACController", "OutFilter"),
            "Fc"
        );
        assert_eq!(
            crate::miniserver::loxone_short_pin_label("HVACController", "OutAverageTemp"),
            "ϑoa"
        );
        assert_eq!(
            crate::miniserver::loxone_short_pin_label("HVACController", "OutputAPI"),
            "API"
        );
        assert_eq!(
            crate::miniserver::loxone_short_pin_label("ClimateControllerUS", "OutCompressor"),
            "C"
        );
        assert_eq!(
            crate::miniserver::loxone_short_pin_label("ClimateControllerUS", "OutHumid"),
            "G"
        );
        assert_eq!(
            crate::miniserver::loxone_short_pin_label("Heatmixer2", "Outer"),
            "ϑo"
        );
    }
}
