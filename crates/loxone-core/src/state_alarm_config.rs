use crate::factory::generate_loxone_uuid;
use crate::model::{PageBlock, RawNode};
use crate::writer::ProjectEdit;

/// Comparison operator used in `<StateText Cond0..Cond3="...">` inside a `State` block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StateConditionOp {
    #[default]
    Equal,
    NotEqual,
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
}

impl StateConditionOp {
    pub const ALL: [StateConditionOp; 6] = [
        StateConditionOp::Equal,
        StateConditionOp::NotEqual,
        StateConditionOp::Greater,
        StateConditionOp::GreaterOrEqual,
        StateConditionOp::Less,
        StateConditionOp::LessOrEqual,
    ];

    pub fn from_attr(val: Option<&str>) -> Self {
        match val.unwrap_or("0").trim() {
            "1" => Self::NotEqual,
            "2" => Self::Greater,
            "3" => Self::GreaterOrEqual,
            "4" => Self::Less,
            "5" => Self::LessOrEqual,
            _ => Self::Equal,
        }
    }

    pub fn to_attr_opt(self) -> Option<&'static str> {
        match self {
            Self::Equal => None,
            Self::NotEqual => Some("1"),
            Self::Greater => Some("2"),
            Self::GreaterOrEqual => Some("3"),
            Self::Less => Some("4"),
            Self::LessOrEqual => Some("5"),
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Equal => "=",
            Self::NotEqual => "≠",
            Self::Greater => ">",
            Self::GreaterOrEqual => "≥",
            Self::Less => "<",
            Self::LessOrEqual => "≤",
        }
    }
}

/// One row (`<StateText .../>`) inside a `State` (`Status`) block's `<StateTexts>` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateRuleRow {
    pub input0: Option<u8>,
    pub op0: StateConditionOp,
    pub val0: String,
    /// `false` = AND (default), `true` = OR (`AndOr1="1"` or `"2"`).
    pub join_or: bool,
    pub input1: Option<u8>,
    pub op1: StateConditionOp,
    pub val1: String,
    pub text: String,
    pub value: String,
    pub icon: String,
    pub icon_color: String,
    pub preserved_attrs: Vec<(String, String)>,
}

impl Default for StateRuleRow {
    fn default() -> Self {
        Self {
            input0: Some(1),
            op0: StateConditionOp::Equal,
            val0: "1".to_string(),
            join_or: false,
            input1: None,
            op1: StateConditionOp::Equal,
            val1: "0".to_string(),
            text: String::new(),
            value: "0".to_string(),
            icon: "00000000-0000-0000-0000000000000000".to_string(),
            icon_color: "7405453".to_string(),
            preserved_attrs: vec![
                ("Valid".to_string(), "true".to_string()),
                ("ValidV".to_string(), "true".to_string()),
            ],
        }
    }
}

impl StateRuleRow {
    pub fn is_fallback(&self) -> bool {
        self.input0.is_none() && self.input1.is_none()
    }

    pub fn from_raw_node(node: &RawNode) -> Self {
        let input0 = node
            .attr("Input0")
            .and_then(|s| s.trim().parse::<u8>().ok())
            .filter(|&v| v > 0);
        let op0 = StateConditionOp::from_attr(node.attr("Cond0"));
        let val0 = node.attr("CondV0").unwrap_or("0").to_string();
        let join_or = matches!(node.attr("AndOr1"), Some("1" | "2"));
        let input1 = node
            .attr("Input1")
            .and_then(|s| s.trim().parse::<u8>().ok())
            .filter(|&v| v > 0);
        let op1 = StateConditionOp::from_attr(node.attr("Cond1"));
        let val1 = node.attr("CondV1").unwrap_or("0").to_string();
        let text = node.attr("Text").unwrap_or("").to_string();
        let value = node.attr("TextV").unwrap_or("0").to_string();
        let icon = node
            .attr("Icon")
            .unwrap_or("00000000-0000-0000-0000000000000000")
            .to_string();
        let icon_color = node.attr("IcC").unwrap_or("7405453").to_string();

        let managed = [
            "Text", "TextV", "Icon", "IcC", "Input0", "Cond0", "CondV0", "AndOr1", "Input1",
            "Cond1", "CondV1",
        ];
        let preserved_attrs = node
            .attributes
            .iter()
            .filter(|(k, _)| !managed.contains(&k.as_str()))
            .cloned()
            .collect();

        Self {
            input0,
            op0,
            val0,
            join_or,
            input1,
            op1,
            val1,
            text,
            value,
            icon,
            icon_color,
            preserved_attrs,
        }
    }

    pub fn to_raw_node(&self) -> RawNode {
        let mut attrs = Vec::new();
        // Keep Valid / ValidV first if present in preserved_attrs
        for (k, v) in &self.preserved_attrs {
            if k == "Valid" || k == "ValidV" {
                attrs.push((k.clone(), v.clone()));
            }
        }
        if !attrs.iter().any(|(k, _)| k == "Valid") {
            attrs.push(("Valid".to_string(), "true".to_string()));
        }
        if !attrs.iter().any(|(k, _)| k == "ValidV") {
            attrs.push(("ValidV".to_string(), "true".to_string()));
        }

        attrs.push(("Text".to_string(), self.text.clone()));
        attrs.push(("TextV".to_string(), self.value.clone()));
        attrs.push(("Icon".to_string(), self.icon.clone()));
        if !self.icon_color.is_empty() {
            attrs.push(("IcC".to_string(), self.icon_color.clone()));
        }

        if let Some(i0) = self.input0 {
            attrs.push(("Input0".to_string(), i0.to_string()));
            if let Some(op_str) = self.op0.to_attr_opt() {
                attrs.push(("Cond0".to_string(), op_str.to_string()));
            }
            let v0 = self.val0.trim();
            if !v0.is_empty() && v0 != "0" {
                attrs.push(("CondV0".to_string(), v0.to_string()));
            }
        }

        if let Some(i1) = self.input1 {
            if self.join_or {
                attrs.push(("AndOr1".to_string(), "1".to_string()));
            }
            attrs.push(("Input1".to_string(), i1.to_string()));
            if let Some(op_str) = self.op1.to_attr_opt() {
                attrs.push(("Cond1".to_string(), op_str.to_string()));
            }
            let v1 = self.val1.trim();
            if !v1.is_empty() && v1 != "0" {
                attrs.push(("CondV1".to_string(), v1.to_string()));
            }
        }

        for (k, v) in &self.preserved_attrs {
            if k != "Valid" && k != "ValidV" {
                attrs.push((k.clone(), v.clone()));
            }
        }

        RawNode {
            tag: "StateText".to_string(),
            attributes: attrs,
            text: None,
            children: Vec::new(),
        }
    }
}

/// Configuration model for `State` (`Status`) blocks.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StateBlockConfig {
    pub rules: Vec<StateRuleRow>,
}

impl StateBlockConfig {
    pub fn is_state_block(block_type: &str) -> bool {
        block_type == "State"
    }

    pub fn from_block(block: &PageBlock) -> Self {
        let mut rules = Vec::new();
        if let Some(st_node) = block.extra_children.iter().find(|n| n.tag == "StateTexts") {
            for child in &st_node.children {
                if child.tag == "StateText" {
                    rules.push(StateRuleRow::from_raw_node(child));
                }
            }
        }
        Self { rules }
    }

    pub fn add_rule(&mut self) {
        let new_rule = StateRuleRow::default();
        if self.rules.last().is_some_and(|r| r.is_fallback()) {
            let insert_idx = self.rules.len() - 1;
            self.rules.insert(insert_idx, new_rule);
        } else {
            self.rules.push(new_rule);
        }
    }

    pub fn remove_rule(&mut self, index: usize) {
        if index < self.rules.len() {
            self.rules.remove(index);
        }
    }

    pub fn move_rule_up(&mut self, index: usize) {
        if index > 0 && index < self.rules.len() {
            self.rules.swap(index, index - 1);
        }
    }

    pub fn move_rule_down(&mut self, index: usize) {
        if index + 1 < self.rules.len() {
            self.rules.swap(index, index + 1);
        }
    }

    pub fn to_edits(&self, block: &PageBlock) -> Vec<ProjectEdit> {
        let current = Self::from_block(block);
        if &current == self {
            return Vec::new();
        }

        let mut new_extra = block.extra_children.clone();
        let state_texts_node = RawNode {
            tag: "StateTexts".to_string(),
            attributes: vec![("Num".to_string(), self.rules.len().to_string())],
            text: None,
            children: self.rules.iter().map(StateRuleRow::to_raw_node).collect(),
        };

        if let Some(existing) = new_extra.iter_mut().find(|n| n.tag == "StateTexts") {
            *existing = state_texts_node;
        } else {
            new_extra.push(state_texts_node);
        }

        vec![ProjectEdit::SetExtraChildren {
            control_uuid: block.uuid.clone(),
            extra_children: new_extra,
        }]
    }
}

/// Bitmask constants for `Modes` on `<AlarmClock>` entries.
pub const ALARM_MODE_HOLIDAY: u32 = 1 << 0; // 1
pub const ALARM_MODE_VACATION: u32 = 1 << 1; // 2
pub const ALARM_MODE_SICK: u32 = 1 << 2; // 4
pub const ALARM_MODE_MON: u32 = 1 << 3; // 8
pub const ALARM_MODE_TUE: u32 = 1 << 4; // 16
pub const ALARM_MODE_WED: u32 = 1 << 5; // 32
pub const ALARM_MODE_THU: u32 = 1 << 6; // 64
pub const ALARM_MODE_FRI: u32 = 1 << 7; // 128
pub const ALARM_MODE_SAT: u32 = 1 << 8; // 256
pub const ALARM_MODE_SUN: u32 = 1 << 9; // 512

pub const ALARM_MODES_WEEKDAYS: u32 =
    ALARM_MODE_MON | ALARM_MODE_TUE | ALARM_MODE_WED | ALARM_MODE_THU | ALARM_MODE_FRI; // 248
pub const ALARM_MODES_WEEKEND: u32 = ALARM_MODE_SAT | ALARM_MODE_SUN; // 768
pub const ALARM_MODES_ALL: u32 = 1023;

/// One `<AlarmClock .../>` child entry inside an `AlarmClock` (`Budzik`) block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlarmClockEntry {
    pub id: Option<u32>,
    pub name: String,
    /// Seconds since midnight (`0..86400`), e.g. `21600` = `06:00`, `25200` = `07:00`.
    pub time_sec: u32,
    /// Bitmask of operating modes (`248` = Mon–Fri, `1023` = All modes).
    pub modes_mask: u32,
    /// `true` for the built-in default alarm (`Int="true"`).
    pub is_internal_default: bool,
    pub uuid: String,
}

impl AlarmClockEntry {
    pub fn format_time_hhmm(&self) -> String {
        let total_min = (self.time_sec / 60).min(23 * 60 + 59);
        let hh = total_min / 60;
        let mm = total_min % 60;
        format!("{:02}:{:02}", hh, mm)
    }

    pub fn set_time_hhmm(&mut self, input: &str) -> bool {
        let trimmed = input.trim();
        let parts: Vec<&str> = trimmed.split(':').collect();
        if parts.len() == 2 {
            if let (Ok(hh), Ok(mm)) = (parts[0].trim().parse::<u32>(), parts[1].trim().parse::<u32>())
            {
                if hh < 24 && mm < 60 {
                    self.time_sec = hh * 3600 + mm * 60;
                    return true;
                }
            }
        }
        false
    }

    pub fn has_mode_bit(&self, bit: u32) -> bool {
        (self.modes_mask & bit) != 0
    }

    pub fn set_mode_bit(&mut self, bit: u32, enabled: bool) {
        if enabled {
            self.modes_mask |= bit;
        } else {
            self.modes_mask &= !bit;
        }
    }

    pub fn from_raw_node(node: &RawNode) -> Self {
        let id = node
            .attr("Id")
            .and_then(|s| s.trim().parse::<u32>().ok());
        let name = node.attr("Name").unwrap_or("Budzik").to_string();
        let time_sec = node
            .attr("Time")
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(25200);
        let modes_mask = node
            .attr("Modes")
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(ALARM_MODES_WEEKDAYS);
        let is_internal_default = matches!(node.attr("Int"), Some("true" | "1"));
        let uuid = node
            .attr("U")
            .map(str::to_string)
            .unwrap_or_else(generate_loxone_uuid);

        Self {
            id,
            name,
            time_sec,
            modes_mask,
            is_internal_default,
            uuid,
        }
    }

    pub fn to_raw_node(&self) -> RawNode {
        let mut attrs = Vec::new();
        if let Some(id) = self.id {
            attrs.push(("Id".to_string(), id.to_string()));
        }
        attrs.push(("Name".to_string(), self.name.clone()));
        attrs.push(("Time".to_string(), self.time_sec.to_string()));
        attrs.push(("Modes".to_string(), self.modes_mask.to_string()));
        if self.is_internal_default {
            attrs.push(("Int".to_string(), "true".to_string()));
        }
        attrs.push(("U".to_string(), self.uuid.clone()));

        RawNode {
            tag: "AlarmClock".to_string(),
            attributes: attrs,
            text: None,
            children: Vec::new(),
        }
    }
}

/// Configuration model for `AlarmClock` (`Budzik`) blocks.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AlarmClockConfig {
    pub entries: Vec<AlarmClockEntry>,
    pub alarm_time_sec: u32,
    pub prep_time_sec: u32,
    pub snooze_time_sec: u32,
    pub wake_sound: u32,
    pub wake_volume: u32,
}

impl AlarmClockConfig {
    pub fn is_alarm_clock_block(block_type: &str) -> bool {
        block_type == "AlarmClock"
    }

    pub fn from_block(block: &PageBlock) -> Self {
        let mut entries = Vec::new();
        for child in &block.extra_children {
            if child.tag == "AlarmClock" {
                entries.push(AlarmClockEntry::from_raw_node(child));
            }
        }
        if entries.is_empty() {
            entries.push(AlarmClockEntry {
                id: None,
                name: "Alarm domyślny".to_string(),
                time_sec: 21600,
                modes_mask: ALARM_MODES_ALL,
                is_internal_default: true,
                uuid: generate_loxone_uuid(),
            });
        }

        let pin_u32 = |key: &str, default: u32| -> u32 {
            block
                .pins
                .iter()
                .find(|p| p.key == key)
                .and_then(|p| p.default_value.as_deref())
                .and_then(|s| s.trim().parse::<u32>().ok())
                .unwrap_or(default)
        };

        Self {
            entries,
            alarm_time_sec: pin_u32("AlarmTime", 120),
            prep_time_sec: pin_u32("PrepTime", 180),
            snooze_time_sec: pin_u32("SnoozeTime", 300),
            wake_sound: pin_u32("ParWakeAlarmSound", 1),
            wake_volume: pin_u32("ParWakeAlarmVolume", 100),
        }
    }

    pub fn add_entry(&mut self) {
        let next_id = self
            .entries
            .iter()
            .filter_map(|e| e.id)
            .max()
            .unwrap_or(0)
            + 1;
        self.entries.push(AlarmClockEntry {
            id: Some(next_id),
            name: format!("Budzik {}", next_id),
            time_sec: 7 * 3600,
            modes_mask: ALARM_MODES_WEEKDAYS,
            is_internal_default: false,
            uuid: generate_loxone_uuid(),
        });
    }

    pub fn remove_entry(&mut self, index: usize) {
        if let Some(entry) = self.entries.get(index) {
            if !entry.is_internal_default {
                self.entries.remove(index);
            }
        }
    }

    pub fn to_edits(&self, block: &PageBlock) -> Vec<ProjectEdit> {
        let current = Self::from_block(block);
        let mut edits = Vec::new();

        if self.entries != current.entries {
            let mut new_extra: Vec<RawNode> = block
                .extra_children
                .iter()
                .filter(|n| n.tag != "AlarmClock")
                .cloned()
                .collect();
            for entry in &self.entries {
                new_extra.push(entry.to_raw_node());
            }
            edits.push(ProjectEdit::SetExtraChildren {
                control_uuid: block.uuid.clone(),
                extra_children: new_extra,
            });
            edits.push(ProjectEdit::SetControlAttr {
                control_uuid: block.uuid.clone(),
                attr_name: "N".to_string(),
                value: Some(self.entries.len().to_string()),
            });
        }

        for (key, new_val, old_val) in [
            ("AlarmTime", self.alarm_time_sec, current.alarm_time_sec),
            ("PrepTime", self.prep_time_sec, current.prep_time_sec),
            ("SnoozeTime", self.snooze_time_sec, current.snooze_time_sec),
            ("ParWakeAlarmSound", self.wake_sound, current.wake_sound),
            ("ParWakeAlarmVolume", self.wake_volume, current.wake_volume),
        ] {
            if new_val != old_val {
                edits.push(ProjectEdit::SetPinDefault {
                    control_uuid: block.uuid.clone(),
                    pin_key: key.to_string(),
                    default_value: Some(new_val.to_string()),
                });
            }
        }

        edits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::load_loxone_file;
    use crate::writer::apply_edit_with_inverse;
    use std::path::PathBuf;

    #[test]
    fn test_parse_and_roundtrip_state_and_alarm_clock_from_klimczaki() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("Klimczaki.Loxone");
        let mut proj = load_loxone_file(&path).expect("parse Klimczaki.Loxone");

        // 1. Verify State ("Ogrzewanie") block parsing & roundtrip
        let state_uuid = "1abec360-009a-9ee2-ffff62ed8a266bff";
        let state_block = proj
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .find(|b| b.uuid == state_uuid)
            .expect("find Ogrzewanie State block")
            .clone();
        assert!(StateBlockConfig::is_state_block(&state_block.block_type));
        let mut state_cfg = StateBlockConfig::from_block(&state_block);
        assert_eq!(state_cfg.rules.len(), 4);
        assert_eq!(state_cfg.rules[1].text, "Ogrzewam");
        assert_eq!(state_cfg.rules[1].op0, StateConditionOp::NotEqual);
        assert_eq!(state_cfg.rules[2].text, "Schładzam");
        assert!(state_cfg.rules[3].is_fallback());
        assert!(state_cfg.to_edits(&state_block).is_empty());

        state_cfg.rules[1].text = "Grzanie aktywne".to_string();
        state_cfg.add_rule();
        let edits = state_cfg.to_edits(&state_block);
        assert!(!edits.is_empty());
        let inv = apply_edit_with_inverse(&mut proj, &ProjectEdit::Batch(edits)).unwrap();

        let updated_state = proj
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .find(|b| b.uuid == state_uuid)
            .unwrap();
        let updated_cfg = StateBlockConfig::from_block(updated_state);
        assert_eq!(updated_cfg.rules.len(), 5);
        assert_eq!(updated_cfg.rules[1].text, "Grzanie aktywne");

        apply_edit_with_inverse(&mut proj, &inv).unwrap();
        let restored_state = proj
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .find(|b| b.uuid == state_uuid)
            .unwrap();
        assert_eq!(StateBlockConfig::from_block(restored_state).rules.len(), 4);

        // 2. Verify AlarmClock ("Touch Nightlight Air") block parsing & roundtrip
        let alarm_uuid = "1c83a514-0155-bc42-ffff51535bcccc04";
        let alarm_block = proj
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .find(|b| b.uuid == alarm_uuid)
            .expect("find Touch Nightlight Air AlarmClock block")
            .clone();
        assert!(AlarmClockConfig::is_alarm_clock_block(
            &alarm_block.block_type
        ));
        let mut alarm_cfg = AlarmClockConfig::from_block(&alarm_block);
        assert_eq!(alarm_cfg.entries.len(), 2);
        assert_eq!(alarm_cfg.entries[0].name, "Alarm domyślny");
        assert_eq!(alarm_cfg.entries[0].format_time_hhmm(), "06:00");
        assert!(alarm_cfg.entries[0].is_internal_default);
        assert_eq!(alarm_cfg.entries[0].modes_mask, ALARM_MODES_ALL);
        assert_eq!(alarm_cfg.entries[1].name, "Budzik");
        assert_eq!(alarm_cfg.entries[1].format_time_hhmm(), "07:00");
        assert_eq!(alarm_cfg.entries[1].modes_mask, ALARM_MODES_WEEKDAYS);
        assert_eq!(alarm_cfg.alarm_time_sec, 120);
        assert_eq!(alarm_cfg.prep_time_sec, 180);
        assert_eq!(alarm_cfg.snooze_time_sec, 300);
        assert!(alarm_cfg.to_edits(&alarm_block).is_empty());

        assert!(alarm_cfg.entries[1].set_time_hhmm("06:30"));
        alarm_cfg.add_entry();
        alarm_cfg.snooze_time_sec = 600;
        let alarm_edits = alarm_cfg.to_edits(&alarm_block);
        assert!(!alarm_edits.is_empty());
        let alarm_inv =
            apply_edit_with_inverse(&mut proj, &ProjectEdit::Batch(alarm_edits)).unwrap();

        let updated_alarm = proj
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .find(|b| b.uuid == alarm_uuid)
            .unwrap();
        let updated_alarm_cfg = AlarmClockConfig::from_block(updated_alarm);
        assert_eq!(updated_alarm_cfg.entries.len(), 3);
        assert_eq!(updated_alarm_cfg.entries[1].format_time_hhmm(), "06:30");
        assert_eq!(updated_alarm_cfg.snooze_time_sec, 600);

        apply_edit_with_inverse(&mut proj, &alarm_inv).unwrap();
        let restored_alarm = proj
            .pages
            .iter()
            .flat_map(|p| &p.blocks)
            .find(|b| b.uuid == alarm_uuid)
            .unwrap();
        let restored_alarm_cfg = AlarmClockConfig::from_block(restored_alarm);
        assert_eq!(restored_alarm_cfg.entries.len(), 2);
        assert_eq!(restored_alarm_cfg.entries[1].format_time_hhmm(), "07:00");
        assert_eq!(restored_alarm_cfg.snooze_time_sec, 300);
    }
}
