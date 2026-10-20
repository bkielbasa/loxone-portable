use std::collections::HashMap;

use crate::block_schema::lookup_block_schema;
use crate::model::{LoxoneProject, PageBlock, RawNode, TreeNode};
use crate::writer::ProjectEdit;

/// Returns `true` if `block` is an Intelligent Room Controller (`HeatIRoomController2` / `HeatIRoomController`)
/// that uses the dedicated 3-tab Room Controller & Schedule configuration modal.
pub fn supports_room_controller_config(block: &PageBlock) -> bool {
    matches!(
        block.block_type.as_str(),
        "HeatIRoomController2" | "HeatIRoomController"
    )
}

/// Temperature setpoint mode (`V` attribute in `<Entry>` or `DefValue` in `<Timer>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoomTempMode {
    /// `0` — Temperatura ekonomiczna
    Economy,
    /// `1` — Temperatura komfortowa
    #[default]
    Comfort,
    /// `2` — Ochrona budynku
    BuildingProtection,
    /// `3` — Stała wartość
    FixedValue,
}

impl RoomTempMode {
    pub fn from_code(code: &str) -> Self {
        match code.trim() {
            "1" => Self::Comfort,
            "2" => Self::BuildingProtection,
            "3" => Self::FixedValue,
            _ => Self::Economy,
        }
    }

    pub fn to_code(self) -> &'static str {
        match self {
            Self::Economy => "0",
            Self::Comfort => "1",
            Self::BuildingProtection => "2",
            Self::FixedValue => "3",
        }
    }

    pub fn label_pl(self) -> &'static str {
        match self {
            Self::Comfort => "Temperatura komfortowa",
            Self::Economy => "Temperatura ekonomiczna",
            Self::BuildingProtection => "Ochrona budynku",
            Self::FixedValue => "Stała wartość",
        }
    }

    pub fn all() -> &'static [RoomTempMode] {
        &[
            Self::Comfort,
            Self::Economy,
            Self::BuildingProtection,
            Self::FixedValue,
        ]
    }
}

/// A single row (`Grupa 1`, `Grupa 2`, ...) in the `Źródło energii` tab (`02.loxone-config.png`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnergySourceGroup {
    pub object_uuid: String,
    pub src_name: String,
    /// Formatted as `"{Title} ({RoomOrPage})"` (e.g. `"Sterownik klimatu (Funkcje centralne)"`).
    pub display_name: String,
    /// Localized object type label (e.g. `"Sterownik ogrzewania i chłodzenia"`, `"HVAC Controller"`).
    pub object_type_label: String,
    /// Bit 0 (`0x01`) of the group's capability byte in `Cap`
    pub heating: bool,
    /// Bit 1 (`0x02`) of the group's capability byte in `Cap`
    pub cooling: bool,
    /// Bit 2 (`0x04`) of the group's capability byte in `Cap`
    pub pwm: bool,
}

impl EnergySourceGroup {
    /// Formatted label shown in the `Ogrzewanie` and `Chłodzenie` priority tables:
    /// `"Grupa {N}:{display_name} - {object_type_label}"`.
    pub fn priority_row_label(&self, group_one_based_index: usize) -> String {
        format!(
            "Grupa {}:{} - {}",
            group_one_based_index, self.display_name, self.object_type_label
        )
    }

    pub fn cap_byte(&self) -> u32 {
        let mut b = 0u32;
        if self.heating {
            b |= 1;
        }
        if self.cooling {
            b |= 2;
        }
        if self.pwm {
            b |= 4;
        }
        b
    }
}

/// Candidate climate/HVAC controller block that can be added as an energy source group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnergySourceCandidate {
    pub object_uuid: String,
    pub src_name: String,
    pub display_name: String,
    pub object_type_label: String,
}

/// A row in the `Edytuj harmonogram` (`<Timer>`) schedule grid (`05.harmonogram.png`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimerModeRow {
    pub mode_uuid: String,
    /// Formatted as `"{Title} ({ModeNumber})"`, e.g. `"Dom w głębokim śnie (-3)"` or `"Poniedziałek (3)"`.
    pub label: String,
    pub is_user_mode: bool,
}

/// A time block entry (`<Entry>`) inside `<Timer>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimerEntry {
    pub row_ix: usize,
    /// Start minute of the day (`0..=1440`)
    pub from_min: u16,
    /// End minute of the day (`0..=1440`)
    pub to_min: u16,
    pub mode: RoomTempMode,
}

/// Schedule configuration (`<Timer>`) for `HeatIRoomController2`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RoomScheduleConfig {
    pub rows: Vec<TimerModeRow>,
    pub entries: Vec<TimerEntry>,
    pub default_mode: RoomTempMode,
}

impl RoomScheduleConfig {
    pub fn user_mode_count(&self) -> usize {
        self.rows.iter().take_while(|r| r.is_user_mode).count()
    }

    pub fn entries_for_row(&self, row_ix: usize) -> Vec<&TimerEntry> {
        let mut list: Vec<&TimerEntry> = self
            .entries
            .iter()
            .filter(|e| e.row_ix == row_ix && e.to_min > e.from_min)
            .collect();
        list.sort_by_key(|e| e.from_min);
        list
    }

    /// Adds a user operating mode row at the end of the user-modes section (just before Monday).
    pub fn add_user_mode(&mut self, mode_uuid: String, label: String) -> usize {
        if let Some(pos) = self.rows.iter().position(|r| r.mode_uuid == mode_uuid) {
            return pos;
        }
        let insert_ix = self.user_mode_count();
        for entry in &mut self.entries {
            if entry.row_ix >= insert_ix {
                entry.row_ix += 1;
            }
        }
        self.rows.insert(
            insert_ix,
            TimerModeRow {
                mode_uuid,
                label,
                is_user_mode: true,
            },
        );
        insert_ix
    }

    /// Removes a user operating mode row at `row_ix` (only valid if `self.rows[row_ix].is_user_mode`).
    pub fn remove_user_mode(&mut self, row_ix: usize) -> bool {
        if row_ix >= self.rows.len() || !self.rows[row_ix].is_user_mode {
            return false;
        }
        self.rows.remove(row_ix);
        self.entries.retain(|e| e.row_ix != row_ix);
        for entry in &mut self.entries {
            if entry.row_ix > row_ix {
                entry.row_ix -= 1;
            }
        }
        true
    }

    /// Moves a user operating mode row up by 1 position within the user-modes section.
    pub fn move_user_mode_up(&mut self, row_ix: usize) -> bool {
        if row_ix == 0 || row_ix >= self.user_mode_count() {
            return false;
        }
        self.rows.swap(row_ix, row_ix - 1);
        for entry in &mut self.entries {
            if entry.row_ix == row_ix {
                entry.row_ix = row_ix - 1;
            } else if entry.row_ix == row_ix - 1 {
                entry.row_ix = row_ix;
            }
        }
        true
    }

    /// Moves a user operating mode row down by 1 position within the user-modes section.
    pub fn move_user_mode_down(&mut self, row_ix: usize) -> bool {
        let count = self.user_mode_count();
        if count == 0 || row_ix + 1 >= count {
            return false;
        }
        self.rows.swap(row_ix, row_ix + 1);
        for entry in &mut self.entries {
            if entry.row_ix == row_ix {
                entry.row_ix = row_ix + 1;
            } else if entry.row_ix == row_ix + 1 {
                entry.row_ix = row_ix;
            }
        }
        true
    }

    /// Sets the temperature mode for `[from_min, to_min]` on `row_ix`, splitting or trimming
    /// any existing entries on that row and merging adjacent identical entries.
    pub fn set_interval(&mut self, row_ix: usize, from_min: u16, to_min: u16, mode: RoomTempMode) {
        let start = from_min.min(1440);
        let end = to_min.min(1440);
        if end <= start || row_ix >= self.rows.len() {
            return;
        }

        let mut other_entries = Vec::new();
        let mut row_entries = Vec::new();
        for e in self.entries.drain(..) {
            if e.row_ix == row_ix {
                row_entries.push(e);
            } else {
                other_entries.push(e);
            }
        }

        let mut updated_row = Vec::new();
        for e in row_entries {
            if e.to_min <= start || e.from_min >= end {
                updated_row.push(e);
            } else {
                if e.from_min < start {
                    updated_row.push(TimerEntry {
                        row_ix,
                        from_min: e.from_min,
                        to_min: start,
                        mode: e.mode,
                    });
                }
                if e.to_min > end {
                    updated_row.push(TimerEntry {
                        row_ix,
                        from_min: end,
                        to_min: e.to_min,
                        mode: e.mode,
                    });
                }
            }
        }

        if mode != self.default_mode {
            updated_row.push(TimerEntry {
                row_ix,
                from_min: start,
                to_min: end,
                mode,
            });
        }

        updated_row.sort_by_key(|e| e.from_min);
        let mut merged: Vec<TimerEntry> = Vec::new();
        for e in updated_row {
            if let Some(last) = merged.last_mut() {
                if last.mode == e.mode && last.to_min == e.from_min {
                    last.to_min = e.to_min;
                    continue;
                }
            }
            merged.push(e);
        }

        other_entries.extend(merged);
        other_entries.sort_by(|a, b| a.row_ix.cmp(&b.row_ix).then(a.from_min.cmp(&b.from_min)));
        self.entries = other_entries;
    }

    pub fn clear_row(&mut self, row_ix: usize) {
        self.entries.retain(|e| e.row_ix != row_ix);
    }

    /// Copies the schedule entries of `source_row_ix` to all weekday rows (`!is_user_mode`).
    pub fn copy_row_to_weekdays(&mut self, source_row_ix: usize) {
        let template: Vec<(u16, u16, RoomTempMode)> = self
            .entries
            .iter()
            .filter(|e| e.row_ix == source_row_ix)
            .map(|e| (e.from_min, e.to_min, e.mode))
            .collect();
        let weekday_indices: Vec<usize> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(_, r)| !r.is_user_mode)
            .map(|(i, _)| i)
            .collect();
        self.entries.retain(|e| !weekday_indices.contains(&e.row_ix));
        for wk_ix in weekday_indices {
            for &(from_min, to_min, mode) in &template {
                self.entries.push(TimerEntry {
                    row_ix: wk_ix,
                    from_min,
                    to_min,
                    mode,
                });
            }
        }
        self.entries
            .sort_by(|a, b| a.row_ix.cmp(&b.row_ix).then(a.from_min.cmp(&b.from_min)));
    }
}

/// Complete configuration state for `HeatIRoomController2` (`Źródło energii`, `Ogrzewanie`, `Chłodzenie`, and `Harmonogram`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RoomControllerConfig {
    pub groups: Vec<EnergySourceGroup>,
    pub prio_h: Vec<usize>,
    pub prio_c: Vec<usize>,
    pub schedule: RoomScheduleConfig,
}

fn get_attr<'a>(attrs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

fn format_climate_object_type(block_type: &str) -> String {
    match block_type {
        "HVACController" | "ClimateController" | "HcController" => {
            "Sterownik ogrzewania i chłodzenia".to_string()
        }
        "ClimateControllerUS" => "HVAC Controller".to_string(),
        other => lookup_block_schema(other)
            .map(|s| s.display_name_en.to_string())
            .unwrap_or_else(|| other.to_string()),
    }
}

fn resolve_energy_source_metadata(
    project: &LoxoneProject,
    object_uuid: &str,
    fallback_src_name: Option<&str>,
) -> (String, String, String) {
    let room_map: HashMap<&str, &str> = project
        .places
        .values()
        .map(|p| (p.uuid.as_str(), p.title.as_str()))
        .collect();

    for page in &project.pages {
        for block in &page.blocks {
            if block.uuid == object_uuid {
                let base_title = if block.title.is_empty() {
                    fallback_src_name.unwrap_or(&block.block_type).to_string()
                } else {
                    block.title.clone()
                };
                let location = block
                    .room_uuid
                    .as_deref()
                    .and_then(|ru| room_map.get(ru).copied())
                    .filter(|s| !s.is_empty())
                    .unwrap_or(page.title.as_str());
                let display_name = if location.is_empty() {
                    base_title.clone()
                } else {
                    format!("{} ({})", base_title, location)
                };
                let object_type_label = format_climate_object_type(&block.block_type);
                return (base_title, display_name, object_type_label);
            }
        }
    }

    let fallback = fallback_src_name.unwrap_or(object_uuid).to_string();
    (
        fallback.clone(),
        fallback,
        "Sterownik ogrzewania i chłodzenia".to_string(),
    )
}

/// Collects all suitable climate/HVAC controller blocks in `project` that are not already in `existing_uuids`.
pub fn collect_energy_source_candidates(
    project: &LoxoneProject,
    existing_uuids: &[String],
) -> Vec<EnergySourceCandidate> {
    let mut out = Vec::new();
    for page in &project.pages {
        for block in &page.blocks {
            if matches!(
                block.block_type.as_str(),
                "HVACController" | "ClimateControllerUS" | "ClimateController" | "HcController"
            ) && !existing_uuids.iter().any(|u| u == &block.uuid)
            {
                let (src_name, display_name, object_type_label) =
                    resolve_energy_source_metadata(project, &block.uuid, None);
                out.push(EnergySourceCandidate {
                    object_uuid: block.uuid.clone(),
                    src_name,
                    display_name,
                    object_type_label,
                });
            }
        }
    }
    out
}

fn collect_mode_nodes_map(project: &LoxoneProject) -> HashMap<String, (String, i32)> {
    fn walk(nodes: &[TreeNode], map: &mut HashMap<String, (String, i32)>) {
        for n in nodes {
            if n.control_type == "Mode" && !n.uuid.is_empty() {
                let mode_num = get_attr(&n.raw_attributes, "Mode")
                    .and_then(|s| s.trim().parse::<i32>().ok())
                    .unwrap_or(0);
                let label = if mode_num != 0 {
                    format!("{} ({})", n.title, mode_num)
                } else {
                    n.title.clone()
                };
                map.insert(n.uuid.clone(), (label, mode_num));
            }
            walk(&n.children, map);
        }
    }
    let mut map = HashMap::new();
    walk(&project.system_tree, &mut map);
    walk(&project.periphery_tree, &mut map);
    map
}

/// Collects all user operating modes (`Mode < 0`, e.g. `Alarm (-1)`, `Symulacja obecności (-2)`, `Dom w głębokim śnie (-3)`)
/// defined in `project.system_tree` for the `Tryby użytkownika` dropdown in `Edytuj harmonogram`.
pub fn collect_user_operating_modes_for_timer(project: &LoxoneProject) -> Vec<(String, String)> {
    fn walk(nodes: &[TreeNode], out: &mut Vec<(String, String, i32)>) {
        for n in nodes {
            if n.control_type == "Mode" && !n.uuid.is_empty() {
                let mode_num = get_attr(&n.raw_attributes, "Mode")
                    .and_then(|s| s.trim().parse::<i32>().ok())
                    .unwrap_or(0);
                if mode_num < 0 && !out.iter().any(|(u, _, _)| u == &n.uuid) {
                    out.push((
                        n.uuid.clone(),
                        format!("{} ({})", n.title, mode_num),
                        mode_num,
                    ));
                }
            }
            walk(&n.children, out);
        }
    }
    let mut raw = Vec::new();
    walk(&project.system_tree, &mut raw);
    raw.into_iter().map(|(u, l, _)| (u, l)).collect()
}

fn default_weekday_rows() -> Vec<TimerModeRow> {
    vec![
        ("00000000-0000-0004-1500000000000000", "Poniedziałek (3)"),
        ("00000000-0000-0005-1500000000000000", "Wtorek (4)"),
        ("00000000-0000-0006-1500000000000000", "Środa (5)"),
        ("00000000-0000-0007-1500000000000000", "Czwartek (6)"),
        ("00000000-0000-0008-1500000000000000", "Piątek (7)"),
        ("00000000-0000-0009-1500000000000000", "Sobota (8)"),
        ("00000000-0000-000a-1500000000000000", "Niedziela (9)"),
    ]
    .into_iter()
    .map(|(u, l)| TimerModeRow {
        mode_uuid: u.to_string(),
        label: l.to_string(),
        is_user_mode: false,
    })
    .collect()
}

impl RoomControllerConfig {
    pub fn from_block(project: &LoxoneProject, block: &PageBlock) -> Self {
        let objects_str = get_attr(&block.raw_attributes, "Objects").unwrap_or("");
        let object_uuids: Vec<String> = objects_str
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();

        let src_names: Vec<&str> = get_attr(&block.raw_attributes, "srcName")
            .unwrap_or("")
            .split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();

        let cap_words: Vec<u32> = get_attr(&block.raw_attributes, "Cap")
            .unwrap_or("")
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse::<u32>().ok())
            .collect();

        let mut groups = Vec::with_capacity(object_uuids.len());
        for (idx, obj_uuid) in object_uuids.iter().enumerate() {
            let word = cap_words.get(idx / 4).copied().unwrap_or(0);
            let byte = (word >> ((idx % 4) * 8)) & 0xFF;
            let heating = (byte & 1) != 0;
            let cooling = (byte & 2) != 0;
            let pwm = (byte & 4) != 0;

            let (src_name, display_name, object_type_label) =
                resolve_energy_source_metadata(project, obj_uuid, src_names.get(idx).copied());

            groups.push(EnergySourceGroup {
                object_uuid: obj_uuid.clone(),
                src_name,
                display_name,
                object_type_label,
                heating,
                cooling,
                pwm,
            });
        }

        let parse_prio = |attr_key: &str| -> Vec<usize> {
            get_attr(&block.raw_attributes, attr_key)
                .unwrap_or("")
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .filter_map(|s| s.parse::<usize>().ok())
                .filter(|&i| i < groups.len())
                .collect()
        };

        let prio_h = parse_prio("PrioH");
        let prio_c = parse_prio("PrioC");

        // Parse <Timer> schedule
        let mode_map = collect_mode_nodes_map(project);
        let schedule = if let Some(timer_node) =
            block.extra_children.iter().find(|n| n.tag == "Timer")
        {
            let user_modes_count = timer_node
                .attr("UserModes")
                .and_then(|s| s.trim().parse::<usize>().ok())
                .unwrap_or(0);
            let default_mode = RoomTempMode::from_code(timer_node.attr("DefValue").unwrap_or("0"));
            let mode_uuids: Vec<String> = timer_node
                .attr("Modes")
                .unwrap_or("")
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();

            let rows: Vec<TimerModeRow> = if mode_uuids.is_empty() {
                default_weekday_rows()
            } else {
                mode_uuids
                    .into_iter()
                    .enumerate()
                    .map(|(i, u)| {
                        let (label, mode_num) = mode_map
                            .get(&u)
                            .cloned()
                            .unwrap_or_else(|| (u.clone(), 0));
                        let is_user_mode = i < user_modes_count || mode_num < 0;
                        TimerModeRow {
                            mode_uuid: u,
                            label,
                            is_user_mode,
                        }
                    })
                    .collect()
            };

            let mut entries = Vec::new();
            for child in &timer_node.children {
                if child.tag == "Entry" {
                    let row_ix = child
                        .attr("Ix")
                        .and_then(|s| s.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    let from_min = child
                        .attr("From")
                        .and_then(|s| s.trim().parse::<u16>().ok())
                        .unwrap_or(0);
                    let to_min = child
                        .attr("To")
                        .and_then(|s| s.trim().parse::<u16>().ok())
                        .unwrap_or(1440);
                    let mode = RoomTempMode::from_code(child.attr("V").unwrap_or("1"));
                    if row_ix < rows.len() && to_min > from_min {
                        entries.push(TimerEntry {
                            row_ix,
                            from_min,
                            to_min,
                            mode,
                        });
                    }
                }
            }

            RoomScheduleConfig {
                rows,
                entries,
                default_mode,
            }
        } else {
            RoomScheduleConfig {
                rows: default_weekday_rows(),
                entries: Vec::new(),
                default_mode: RoomTempMode::Economy,
            }
        };

        Self {
            groups,
            prio_h,
            prio_c,
            schedule,
        }
    }

    pub fn add_group(&mut self, candidate: &EnergySourceCandidate) {
        if self
            .groups
            .iter()
            .any(|g| g.object_uuid == candidate.object_uuid)
        {
            return;
        }
        let idx = self.groups.len();
        self.groups.push(EnergySourceGroup {
            object_uuid: candidate.object_uuid.clone(),
            src_name: candidate.src_name.clone(),
            display_name: candidate.display_name.clone(),
            object_type_label: candidate.object_type_label.clone(),
            heating: true,
            cooling: true,
            pwm: false,
        });
        self.prio_h.push(idx);
        self.prio_c.push(idx);
    }

    pub fn remove_group(&mut self, group_idx: usize) {
        if group_idx >= self.groups.len() {
            return;
        }
        self.groups.remove(group_idx);
        let adjust = |prio: &mut Vec<usize>| {
            prio.retain(|&i| i != group_idx);
            for i in prio.iter_mut() {
                if *i > group_idx {
                    *i -= 1;
                }
            }
        };
        adjust(&mut self.prio_h);
        adjust(&mut self.prio_c);
    }

    pub fn set_group_heating(&mut self, group_idx: usize, enabled: bool) {
        if let Some(g) = self.groups.get_mut(group_idx) {
            g.heating = enabled;
            if enabled {
                if !self.prio_h.contains(&group_idx) {
                    self.prio_h.push(group_idx);
                }
            } else {
                self.prio_h.retain(|&i| i != group_idx);
            }
        }
    }

    pub fn set_group_cooling(&mut self, group_idx: usize, enabled: bool) {
        if let Some(g) = self.groups.get_mut(group_idx) {
            g.cooling = enabled;
            if enabled {
                if !self.prio_c.contains(&group_idx) {
                    self.prio_c.push(group_idx);
                }
            } else {
                self.prio_c.retain(|&i| i != group_idx);
            }
        }
    }

    pub fn move_heating_prio_up(&mut self, pos: usize) {
        if pos > 0 && pos < self.prio_h.len() {
            self.prio_h.swap(pos, pos - 1);
        }
    }

    pub fn move_heating_prio_down(&mut self, pos: usize) {
        if pos + 1 < self.prio_h.len() {
            self.prio_h.swap(pos, pos + 1);
        }
    }

    pub fn move_cooling_prio_up(&mut self, pos: usize) {
        if pos > 0 && pos < self.prio_c.len() {
            self.prio_c.swap(pos, pos - 1);
        }
    }

    pub fn move_cooling_prio_down(&mut self, pos: usize) {
        if pos + 1 < self.prio_c.len() {
            self.prio_c.swap(pos, pos + 1);
        }
    }

    pub fn encode_cap_attribute(&self) -> String {
        let word_count = (self.groups.len().div_ceil(4)).max(2);
        let mut words = vec![0u32; word_count];
        for (idx, g) in self.groups.iter().enumerate() {
            words[idx / 4] |= (g.cap_byte() & 0xFF) << ((idx % 4) * 8);
        }
        words
            .into_iter()
            .map(|w| w.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn to_edits(&self, block: &PageBlock) -> Vec<ProjectEdit> {
        let mut edits = Vec::new();
        let uuid = block.uuid.clone();

        let new_objects = if self.groups.is_empty() {
            None
        } else {
            Some(
                self.groups
                    .iter()
                    .map(|g| g.object_uuid.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            )
        };
        if new_objects.as_deref() != get_attr(&block.raw_attributes, "Objects") {
            edits.push(ProjectEdit::SetControlAttr {
                control_uuid: uuid.clone(),
                attr_name: "Objects".to_string(),
                value: new_objects,
            });
        }

        let new_src_name = if self.groups.is_empty() {
            None
        } else {
            Some(
                self.groups
                    .iter()
                    .map(|g| g.src_name.as_str())
                    .collect::<Vec<_>>()
                    .join(";"),
            )
        };
        if new_src_name.as_deref() != get_attr(&block.raw_attributes, "srcName") {
            edits.push(ProjectEdit::SetControlAttr {
                control_uuid: uuid.clone(),
                attr_name: "srcName".to_string(),
                value: new_src_name,
            });
        }

        let new_cap = Some(self.encode_cap_attribute());
        if new_cap.as_deref() != get_attr(&block.raw_attributes, "Cap") {
            edits.push(ProjectEdit::SetControlAttr {
                control_uuid: uuid.clone(),
                attr_name: "Cap".to_string(),
                value: new_cap,
            });
        }

        let new_prio_h = if self.prio_h.is_empty() {
            None
        } else {
            Some(
                self.prio_h
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            )
        };
        if new_prio_h.as_deref() != get_attr(&block.raw_attributes, "PrioH") {
            edits.push(ProjectEdit::SetControlAttr {
                control_uuid: uuid.clone(),
                attr_name: "PrioH".to_string(),
                value: new_prio_h,
            });
        }

        let new_prio_c = if self.prio_c.is_empty() {
            None
        } else {
            Some(
                self.prio_c
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            )
        };
        if new_prio_c.as_deref() != get_attr(&block.raw_attributes, "PrioC") {
            edits.push(ProjectEdit::SetControlAttr {
                control_uuid: uuid.clone(),
                attr_name: "PrioC".to_string(),
                value: new_prio_c,
            });
        }

        // Build updated <Timer> node inside extra_children
        let mut new_extra_children = block.extra_children.clone();
        let modes_attr = self
            .schedule
            .rows
            .iter()
            .map(|r| r.mode_uuid.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let user_modes_str = self.schedule.user_mode_count().to_string();
        let def_val_str = self.schedule.default_mode.to_code().to_string();
        let n_str = self.schedule.entries.len().to_string();

        let mut entry_nodes = Vec::with_capacity(self.schedule.entries.len());
        for entry in &self.schedule.entries {
            let mut attrs = Vec::new();
            if entry.from_min > 0 {
                attrs.push(("From".to_string(), entry.from_min.to_string()));
            }
            attrs.push(("To".to_string(), entry.to_min.to_string()));
            if entry.row_ix > 0 {
                attrs.push(("Ix".to_string(), entry.row_ix.to_string()));
            }
            attrs.push(("V".to_string(), entry.mode.to_code().to_string()));
            entry_nodes.push(RawNode {
                tag: "Entry".to_string(),
                attributes: attrs,
                text: None,
                children: Vec::new(),
            });
        }

        let timer_node = RawNode {
            tag: "Timer".to_string(),
            attributes: vec![
                ("M".to_string(), "5".to_string()),
                ("UserModes".to_string(), user_modes_str),
                ("DefValue".to_string(), def_val_str),
                ("N".to_string(), n_str),
                ("Modes".to_string(), modes_attr),
            ],
            text: None,
            children: entry_nodes,
        };

        if let Some(existing_timer) = new_extra_children.iter_mut().find(|n| n.tag == "Timer") {
            *existing_timer = timer_node;
        } else {
            new_extra_children.push(timer_node);
        }

        if new_extra_children != block.extra_children {
            edits.push(ProjectEdit::SetExtraChildren {
                control_uuid: uuid,
                extra_children: new_extra_children,
            });
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::load_loxone_file;
    use crate::writer::{apply_edit_with_inverse, ProjectEdit};
    use std::path::PathBuf;

    #[test]
    fn test_gabinet_room_controller_config_matches_loxone_screenshots() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Klimczaki.Loxone");
        let proj = load_loxone_file(&path).expect("parse Klimczaki.Loxone");

        let gabinet_block = proj
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .find(|b| b.uuid == "19eb19a9-001b-384c-ffff51535bcccc04")
            .expect("find Gabinet HeatIRoomController2");

        assert!(supports_room_controller_config(gabinet_block));
        let cfg = RoomControllerConfig::from_block(&proj, gabinet_block);

        // 02.loxone-config.png: 2 energy source groups
        assert_eq!(cfg.groups.len(), 2);
        // Grupa 1: Sterownik klimatu (Funkcje centralne) | Sterownik ogrzewania i chłodzenia | Ogrzewanie=true, Chłodzenie=true, PWM=false
        assert_eq!(
            cfg.groups[0].display_name,
            "Sterownik klimatu (Funkcje centralne)"
        );
        assert_eq!(
            cfg.groups[0].object_type_label,
            "Sterownik ogrzewania i chłodzenia"
        );
        assert!(cfg.groups[0].heating);
        assert!(cfg.groups[0].cooling);
        assert!(!cfg.groups[0].pwm);

        // Grupa 2: HVAC Controller (Gabinet) | HVAC Controller | Ogrzewanie=true, Chłodzenie=false, PWM=true
        assert_eq!(cfg.groups[1].display_name, "HVAC Controller (Gabinet)");
        assert_eq!(cfg.groups[1].object_type_label, "HVAC Controller");
        assert!(cfg.groups[1].heating);
        assert!(!cfg.groups[1].cooling);
        assert!(cfg.groups[1].pwm);

        // 03.loxone-config.png: Ogrzewanie priority rows
        assert_eq!(cfg.prio_h, vec![0, 1]);
        assert_eq!(
            cfg.groups[cfg.prio_h[0]].priority_row_label(1),
            "Grupa 1:Sterownik klimatu (Funkcje centralne) - Sterownik ogrzewania i chłodzenia"
        );
        assert_eq!(
            cfg.groups[cfg.prio_h[1]].priority_row_label(2),
            "Grupa 2:HVAC Controller (Gabinet) - HVAC Controller"
        );

        // 04.loxone-config.png: Chłodzenie priority rows (empty in Gabinet)
        assert!(cfg.prio_c.is_empty());

        // 05.harmonogram.png: Schedule rows and entries
        assert_eq!(cfg.schedule.rows.len(), 8);
        assert_eq!(cfg.schedule.rows[0].label, "Dom w głębokim śnie (-3)");
        assert!(cfg.schedule.rows[0].is_user_mode);
        assert_eq!(cfg.schedule.rows[1].label, "Poniedziałek (3)");
        assert_eq!(cfg.schedule.rows[7].label, "Niedziela (9)");
        assert_eq!(cfg.schedule.entries.len(), 6);
        assert_eq!(
            cfg.schedule.entries[0],
            TimerEntry {
                row_ix: 0,
                from_min: 0,
                to_min: 1440,
                mode: RoomTempMode::BuildingProtection,
            }
        );
        assert_eq!(
            cfg.schedule.entries[1],
            TimerEntry {
                row_ix: 1,
                from_min: 840,
                to_min: 1140,
                mode: RoomTempMode::Comfort,
            }
        );

        // Candidate list when both controllers are already assigned should be empty ("Nie znaleziono żadnych odpowiednich obiektów")
        let existing_uuids: Vec<String> =
            cfg.groups.iter().map(|g| g.object_uuid.clone()).collect();
        let candidates = collect_energy_source_candidates(&proj, &existing_uuids);
        assert!(candidates.is_empty());

        // User operating modes dropdown starts with "Alarm (-1)"
        let user_modes = collect_user_operating_modes_for_timer(&proj);
        assert!(!user_modes.is_empty());
        assert_eq!(user_modes[0].1, "Alarm (-1)");
    }

    #[test]
    fn test_room_controller_config_roundtrip_and_undo() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Klimczaki.Loxone");
        let mut proj = load_loxone_file(&path).expect("parse Klimczaki.Loxone");

        let gabinet_uuid = "19eb19a9-001b-384c-ffff51535bcccc04";
        let block = proj
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .find(|b| b.uuid == gabinet_uuid)
            .unwrap()
            .clone();

        let mut cfg = RoomControllerConfig::from_block(&proj, &block);
        // Unchanged config produces 0 edits
        assert!(cfg.to_edits(&block).is_empty());

        // Swap heating priority, enable cooling on group 0, and add Saturday 10:00..18:00 Comfort (like 05.harmonogram.png)
        cfg.move_heating_prio_down(0);
        cfg.set_group_cooling(0, true);
        cfg.schedule
            .set_interval(6, 600, 1080, RoomTempMode::Comfort);

        let edits = cfg.to_edits(&block);
        assert!(!edits.is_empty());

        let inverse = apply_edit_with_inverse(&mut proj, &ProjectEdit::Batch(edits))
            .expect("apply room controller edits");

        let updated_block = proj
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .find(|b| b.uuid == gabinet_uuid)
            .unwrap();
        let updated_cfg = RoomControllerConfig::from_block(&proj, updated_block);
        assert_eq!(updated_cfg.prio_h, vec![1, 0]);
        assert_eq!(updated_cfg.prio_c, vec![0]);
        assert_eq!(
            updated_cfg.schedule.entries_for_row(6),
            vec![&TimerEntry {
                row_ix: 6,
                from_min: 600,
                to_min: 1080,
                mode: RoomTempMode::Comfort,
            }]
        );

        // Undo restores original state
        apply_edit_with_inverse(&mut proj, &inverse).expect("undo room controller edits");
        let restored_block = proj
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .find(|b| b.uuid == gabinet_uuid)
            .unwrap();
        let restored_cfg = RoomControllerConfig::from_block(&proj, restored_block);
        assert_eq!(restored_cfg.prio_h, vec![0, 1]);
        assert!(restored_cfg.prio_c.is_empty());
        assert!(restored_cfg.schedule.entries_for_row(6).is_empty());
    }
}
