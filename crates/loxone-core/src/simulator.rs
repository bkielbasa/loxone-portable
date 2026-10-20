use std::collections::BTreeMap;

use crate::miniserver::{LiveControlSnapshot, LiveValueStore};
use crate::model::{ConnectorPin, LoxoneProject, Page, PageBlock};

/// Pure offline logic simulation engine for Loxone schematics.
///
/// Produces and updates a [`LiveValueStore`] directly from the project's pages, blocks,
/// default pin parameters, and wire links—enabling interactive input toggling, energized
/// green wire visualization, multi-hop logic evaluation, and cross-page `Memory` (`Merker`)
/// propagation without requiring a network connection to a physical Miniserver.
#[derive(Debug, Clone, Default)]
pub struct OfflineSimulator {
    pub enabled: bool,
    pub live_store: LiveValueStore,
    pub tick_count: u64,
    pub forced_inputs: BTreeMap<String, String>,
    pub block_latches: BTreeMap<String, f64>,
}

impl OfflineSimulator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts offline simulation mode and seeds initial values for all blocks in the project.
    pub fn start(&mut self, project: &LoxoneProject) {
        self.enabled = true;
        self.tick_count = 0;
        self.live_store.seed_project_defaults(project);
        self.recompute(project);
    }

    /// Stops offline simulation mode.
    pub fn stop(&mut self) {
        self.enabled = false;
    }

    /// Toggles offline simulation mode on or off, returning the new `enabled` state.
    pub fn toggle(&mut self, project: &LoxoneProject) -> bool {
        if self.enabled {
            self.stop();
        } else {
            self.start(project);
        }
        self.enabled
    }

    /// Resets all forced inputs and stateful block latches back to project defaults.
    pub fn reset(&mut self, project: &LoxoneProject) {
        self.forced_inputs.clear();
        self.block_latches.clear();
        self.tick_count = 0;
        self.live_store.controls.clear();
        self.live_store.pin_overrides.clear();
        self.live_store.seed_project_defaults(project);
        self.recompute(project);
    }

    /// Applies an interactive user command (such as clicking an `InputRef` pill, a `Switch`
    /// block, or a pin toggle in the inspector) and recomputes downstream logic across the project.
    pub fn apply_command(
        &mut self,
        project: &LoxoneProject,
        control_uuid: &str,
        pin_key: Option<&str>,
        raw_value: &str,
    ) {
        if !self.enabled {
            self.start(project);
        }

        let key = match pin_key {
            Some(pk) if !pk.is_empty() => format!("{control_uuid}:{pk}"),
            _ => control_uuid.to_string(),
        };
        self.forced_inputs.insert(key, raw_value.to_string());

        let numeric = parse_signal_numeric(raw_value);
        if pin_key.is_none() || pin_key == Some("Q") || pin_key == Some("O") || pin_key == Some("OutputAPI") {
            self.block_latches.insert(control_uuid.to_string(), numeric);
        }

        self.live_store
            .apply_interactive_command(Some(project), control_uuid, pin_key, raw_value);

        // If the user triggered a Central block (CentralLight, CentralShade, CentralMusic),
        // propagate the command to all member blocks listed in its `rec` attribute.
        for page in &project.pages {
            if let Some(block) = page.blocks.iter().find(|b| b.uuid == control_uuid) {
                self.propagate_central_block_members(block, numeric);
            }
        }

        self.recompute(project);
    }

    /// Advances the simulation clock by one tick and recomputes time-dependent blocks.
    pub fn step_tick(&mut self, project: &LoxoneProject) {
        if !self.enabled {
            self.start(project);
        }
        self.tick_count += 1;

        for page in &project.pages {
            for block in &page.blocks {
                match block.block_type.as_str() {
                    "IntervalImpulse" => {
                        let next_val = if self.tick_count % 2 == 1 { 1.0 } else { 0.0 };
                        self.block_latches.insert(block.uuid.clone(), next_val);
                    }
                    "Stopwatch" | "OperatingHoursCounter" => {
                        let current = self.block_latches.get(&block.uuid).copied().unwrap_or(0.0);
                        let reset_val = self.max_input_for_keys(
                            block,
                            page,
                            project,
                            &["R", "Reset", "Rr"],
                        );
                        if reset_val > 0.0 {
                            self.block_latches.insert(block.uuid.clone(), 0.0);
                        } else {
                            let active = self.max_input_for_keys(
                                block,
                                page,
                                project,
                                &["I", "Tr", "S", "On", "En"],
                            );
                            if active > 0.0 {
                                self.block_latches.insert(block.uuid.clone(), current + 1.0);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        self.recompute(project);
    }

    /// Evaluates all pages and blocks in the project over multiple passes so multi-hop wires
    /// and cross-page `Memory` (`Merker`) references converge.
    pub fn recompute(&mut self, project: &LoxoneProject) {
        const PASSES: usize = 4;
        for _ in 0..PASSES {
            for page in &project.pages {
                for block in &page.blocks {
                    if block.is_note() {
                        continue;
                    }
                    self.evaluate_block(block, page, project);
                }
            }
        }
        self.live_store.poll_count += 1;
    }

    fn propagate_central_block_members(&mut self, block: &PageBlock, active_val: f64) {
        let is_central = matches!(
            block.block_type.as_str(),
            "CentralLight"
                | "CentralLightController"
                | "CentralShade"
                | "CentralJalousie"
                | "CentralMusic"
                | "CentralAudio"
                | "CentralGate"
        );
        if !is_central {
            return;
        }
        if let Some(rec) = block.attr("rec") {
            for member_uuid in rec.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                self.block_latches.insert(member_uuid.to_string(), active_val);
                let raw = if active_val > 0.0 { "1" } else { "0" };
                self.live_store.set_control_value(member_uuid, raw);
            }
        }
    }

    fn evaluate_block(&mut self, block: &PageBlock, page: &Page, project: &LoxoneProject) {
        // 1. Reference bars (`InputRef`, `OutputRef`, `OutputRefLM`, `Memory`)
        if block.is_ref_bar() {
            if !block.is_input_ref_bar() {
                // OutputRef / Memory writer: check incoming wire links
                let mut wired_raw: Option<String> = None;
                for pin in &block.pins {
                    for link in &pin.incoming_links {
                        if link.is_hidden() {
                            continue;
                        }
                        if let Some(sig) = self.live_store.resolve_pin_by_uuid(
                            &link.source_pin_uuid,
                            page,
                            project,
                            0,
                        ) {
                            wired_raw = Some(sig.raw);
                            break;
                        }
                    }
                    if wired_raw.is_some() {
                        break;
                    }
                }

                if let Some(raw) = wired_raw {
                    self.set_block_output_pin(&block.uuid, "O", &raw, true);
                    self.set_block_output_pin(&block.uuid, "I", &raw, false);
                    if let Some(ref_uuid) = &block.ref_uuid {
                        self.set_block_output_pin(ref_uuid, "O", &raw, true);
                        // Propagate to any reader reference blocks (e.g. Memory flags on other pages)
                        for p in &project.pages {
                            for other in &p.blocks {
                                if other.uuid != block.uuid
                                    && other.ref_uuid.as_deref() == Some(ref_uuid.as_str())
                                    && other.pins.iter().all(|cp| cp.incoming_links.is_empty())
                                {
                                    self.set_block_output_pin(&other.uuid, "O", &raw, true);
                                }
                            }
                        }
                    }
                }
            }
            return;
        }

        // 2. Function blocks
        match block.block_type.as_str() {
            "And" => {
                let inputs = self.collect_connected_input_values(block, page, project);
                let q = if !inputs.is_empty() && inputs.iter().all(|&v| v > 0.0) {
                    1.0
                } else {
                    0.0
                };
                self.write_digital_outputs(block, q);
            }
            "Or" => {
                let inputs = self.collect_connected_input_values(block, page, project);
                let q = if inputs.iter().any(|&v| v > 0.0) {
                    1.0
                } else {
                    0.0
                };
                self.write_digital_outputs(block, q);
            }
            "Xor" => {
                let inputs = self.collect_connected_input_values(block, page, project);
                let active_count = inputs.iter().filter(|&&v| v > 0.0).count();
                let q = if active_count % 2 == 1 { 1.0 } else { 0.0 };
                self.write_digital_outputs(block, q);
            }
            "Not" => {
                let inputs = self.collect_connected_input_values(block, page, project);
                let any_active = inputs.iter().any(|&v| v > 0.0);
                let q = if any_active { 0.0 } else { 1.0 };
                self.write_digital_outputs(block, q);
            }
            "Switch"
            | "Pushbutton"
            | "PushButton"
            | "RSFlipFlop"
            | "Monoflop"
            | "StairwellLight"
            | "OffDelay"
            | "PresenceLight" => {
                let reset = self.max_input_for_keys(
                    block,
                    page,
                    project,
                    &["R", "Reset", "Off", "Dis", "InputDisable"],
                );
                let set = self.max_input_for_keys(
                    block,
                    page,
                    project,
                    &[
                        "Tr",
                        "I",
                        "S",
                        "On",
                        "Presence",
                        "Move",
                        "InputTrigger",
                        "Plus",
                    ],
                );
                let latched = self.block_latches.get(&block.uuid).copied().unwrap_or(0.0);
                let q = if reset > 0.0 {
                    self.block_latches.insert(block.uuid.clone(), 0.0);
                    0.0
                } else if set > 0.0 || latched > 0.0 {
                    1.0
                } else {
                    0.0
                };
                self.write_digital_outputs(block, q);
            }
            "LightController2" | "LightControllerV2" | "LightController" => {
                let reset = self.max_input_for_keys(
                    block,
                    page,
                    project,
                    &["Reset", "R", "Off", "Dis", "InputDisable"],
                );
                let trigger = self.max_input_for_keys(
                    block,
                    page,
                    project,
                    &[
                        "Trigger",
                        "Tr",
                        "Presence",
                        "Move",
                        "Gesture",
                        "Plus",
                        "On",
                        "I",
                    ],
                );
                let latched = self.block_latches.get(&block.uuid).copied().unwrap_or(0.0);
                let active = reset <= 0.0 && (trigger > 0.0 || latched > 0.0);
                let ww_str = if active { "WW(100%)" } else { "WW(0%)" };
                let num_str = if active { "100" } else { "0" };
                let dig_str = if active { "1" } else { "0" };

                self.set_block_output_pin(&block.uuid, "OutputAPI", ww_str, true);
                self.set_block_output_pin(&block.uuid, "AQ1", ww_str, false);
                for idx in 1..=8 {
                    let aq = format!("AQ{idx}");
                    let q = format!("Q{idx}");
                    if block.pins.iter().any(|p| p.key == aq) {
                        self.set_block_output_pin(&block.uuid, &aq, num_str, false);
                    }
                    if block.pins.iter().any(|p| p.key == q) {
                        self.set_block_output_pin(&block.uuid, &q, dig_str, false);
                    }
                }
            }
            "ThresholdSwitch" | "Hysteresis" => {
                let input = self.first_input_or_default(block, page, project, &["I", "AI"], 0.0);
                let threshold = self.first_input_or_default(
                    block,
                    page,
                    project,
                    &["S", "Thon", "Threshold", "Min"],
                    10.0,
                );
                let q = if input >= threshold { 1.0 } else { 0.0 };
                self.write_digital_outputs(block, q);
            }
            "Limiter" => {
                let input = self.first_input_or_default(block, page, project, &["I", "AI"], 0.0);
                let min_v = self.first_input_or_default(block, page, project, &["Min"], 0.0);
                let max_v = self.first_input_or_default(block, page, project, &["Max"], 100.0);
                let clamped = input.clamp(min_v, max_v);
                let val_str = format_compact_f64(clamped);
                self.set_block_output_pin(&block.uuid, "AQ", &val_str, true);
                self.set_block_output_pin(&block.uuid, "Q", &val_str, false);
            }
            "Average" => {
                let inputs = self.collect_connected_input_values(block, page, project);
                let avg = if inputs.is_empty() {
                    0.0
                } else {
                    inputs.iter().sum::<f64>() / (inputs.len() as f64)
                };
                let val_str = format_compact_f64(avg);
                self.set_block_output_pin(&block.uuid, "AQ", &val_str, true);
                self.set_block_output_pin(&block.uuid, "Q", &val_str, false);
            }
            "MinMax" => {
                let inputs = self.collect_connected_input_values(block, page, project);
                let (min_v, max_v) = if inputs.is_empty() {
                    (0.0, 0.0)
                } else {
                    let min_v = inputs.iter().copied().fold(f64::INFINITY, f64::min);
                    let max_v = inputs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                    (min_v, max_v)
                };
                let min_str = format_compact_f64(min_v);
                let max_str = format_compact_f64(max_v);
                self.set_block_output_pin(&block.uuid, "AQmin", &min_str, false);
                self.set_block_output_pin(&block.uuid, "Min", &min_str, false);
                self.set_block_output_pin(&block.uuid, "AQmax", &max_str, true);
                self.set_block_output_pin(&block.uuid, "Max", &max_str, false);
            }
            "Scale" => {
                let input = self.first_input_or_default(block, page, project, &["I", "AI"], 0.0);
                let il = self.first_input_or_default(block, page, project, &["Il"], 0.0);
                let ih = self.first_input_or_default(block, page, project, &["Ih"], 10.0);
                let ol = self.first_input_or_default(block, page, project, &["Ol"], 0.0);
                let oh = self.first_input_or_default(block, page, project, &["Oh"], 100.0);
                let span = (ih - il).abs();
                let scaled = if span > 1e-6 {
                    ol + ((input - il) / (ih - il)) * (oh - ol)
                } else {
                    ol
                };
                let val_str = format_compact_f64(scaled);
                self.set_block_output_pin(&block.uuid, "AQ", &val_str, true);
                self.set_block_output_pin(&block.uuid, "O", &val_str, false);
            }
            "Jalousie" | "AutomaticShading" | "RoofWindowShading" => {
                let up = self.max_input_for_keys(block, page, project, &["Up", "Cu", "FullUp"]);
                let down =
                    self.max_input_for_keys(block, page, project, &["Down", "Cd", "FullDown", "S"]);
                let (qu, qd, pos) = if up > 0.0 && down <= 0.0 {
                    ("1", "0", "0")
                } else if down > 0.0 {
                    ("0", "1", "100")
                } else {
                    ("0", "0", "50")
                };
                self.set_block_output_pin(&block.uuid, "Qu", qu, false);
                self.set_block_output_pin(&block.uuid, "Qup", qu, false);
                self.set_block_output_pin(&block.uuid, "Qd", qd, false);
                self.set_block_output_pin(&block.uuid, "Qdown", qd, false);
                self.set_block_output_pin(&block.uuid, "Pos", pos, true);
            }
            "CentralLight"
            | "CentralLightController"
            | "CentralShade"
            | "CentralJalousie"
            | "CentralMusic"
            | "CentralAudio"
            | "CentralGate" => {
                let reset =
                    self.max_input_for_keys(block, page, project, &["Reset", "R", "Off", "Dis"]);
                let trigger = self.max_input_for_keys(
                    block,
                    page,
                    project,
                    &["Trigger", "Tr", "On", "Up", "Down", "Play", "I"],
                );
                if reset > 0.0 {
                    self.propagate_central_block_members(block, 0.0);
                    self.write_digital_outputs(block, 0.0);
                } else if trigger > 0.0 {
                    self.propagate_central_block_members(block, 1.0);
                    self.write_digital_outputs(block, 1.0);
                }
            }
            "IntervalImpulse" | "Stopwatch" | "OperatingHoursCounter" => {
                let val = self.block_latches.get(&block.uuid).copied().unwrap_or(0.0);
                let val_str = format_compact_f64(val);
                self.set_block_output_pin(&block.uuid, "Q", &val_str, true);
                self.set_block_output_pin(&block.uuid, "AQ", &val_str, false);
            }
            _ => {}
        }
    }

    fn write_digital_outputs(&mut self, block: &PageBlock, q: f64) {
        let q_str = if q > 0.0 { "1" } else { "0" };
        let qn_str = if q > 0.0 { "0" } else { "1" };
        self.set_block_output_pin(&block.uuid, "Q", q_str, true);
        self.set_block_output_pin(&block.uuid, "O", q_str, false);
        self.set_block_output_pin(&block.uuid, "Qon", q_str, false);
        self.set_block_output_pin(&block.uuid, "Qn", qn_str, false);
        self.set_block_output_pin(&block.uuid, "Qoff", qn_str, false);
    }

    fn set_block_output_pin(
        &mut self,
        block_uuid: &str,
        pin_key: &str,
        raw_value: &str,
        is_primary: bool,
    ) {
        self.live_store.pin_overrides.insert(
            (block_uuid.to_string(), pin_key.to_string()),
            raw_value.to_string(),
        );
        let snap = self
            .live_store
            .controls
            .entry(block_uuid.to_string())
            .or_insert_with(|| LiveControlSnapshot {
                control_id: block_uuid.to_string(),
                primary_value: None,
                code: 200,
                outputs: BTreeMap::new(),
            });
        if is_primary {
            snap.primary_value = Some(raw_value.to_string());
        }
        snap.outputs
            .insert(pin_key.to_string(), raw_value.to_string());
    }

    fn collect_connected_input_values(
        &self,
        block: &PageBlock,
        page: &Page,
        project: &LoxoneProject,
    ) -> Vec<f64> {
        let mut values = Vec::new();
        for pin in &block.pins {
            if pin.incoming_links.is_empty() {
                continue;
            }
            for link in &pin.incoming_links {
                if link.is_hidden() {
                    continue;
                }
                if let Some(sig) =
                    self.live_store
                        .resolve_pin_by_uuid(&link.source_pin_uuid, page, project, 0)
                {
                    values.push(parse_signal_numeric(&sig.raw));
                }
            }
        }
        values
    }

    fn max_input_for_keys(
        &self,
        block: &PageBlock,
        page: &Page,
        project: &LoxoneProject,
        keys: &[&str],
    ) -> f64 {
        let mut max_val = 0.0_f64;
        for pin in &block.pins {
            let matches_key = keys.iter().any(|k| pin.key.eq_ignore_ascii_case(k))
                || (keys.contains(&"I") && pin.key.starts_with('I'));
            if !matches_key {
                continue;
            }
            if let Some(v) = self.resolve_input_pin_numeric(block, pin, page, project) {
                if v > max_val {
                    max_val = v;
                }
            }
        }
        max_val
    }

    fn first_input_or_default(
        &self,
        block: &PageBlock,
        page: &Page,
        project: &LoxoneProject,
        keys: &[&str],
        fallback: f64,
    ) -> f64 {
        for key in keys {
            if let Some(pin) = block
                .pins
                .iter()
                .find(|p| p.key.eq_ignore_ascii_case(key))
            {
                if let Some(v) = self.resolve_input_pin_numeric(block, pin, page, project) {
                    return v;
                }
            }
        }
        fallback
    }

    fn resolve_input_pin_numeric(
        &self,
        block: &PageBlock,
        pin: &ConnectorPin,
        page: &Page,
        project: &LoxoneProject,
    ) -> Option<f64> {
        if let Some(ov) = self
            .live_store
            .pin_overrides
            .get(&(block.uuid.clone(), pin.key.clone()))
        {
            return Some(parse_signal_numeric(ov));
        }

        let mut acc: Option<f64> = None;
        for link in &pin.incoming_links {
            if link.is_hidden() {
                continue;
            }
            if let Some(sig) =
                self.live_store
                    .resolve_pin_by_uuid(&link.source_pin_uuid, page, project, 0)
            {
                let v = parse_signal_numeric(&sig.raw);
                acc = Some(acc.map_or(v, |prev| prev.max(v)));
            }
        }
        if acc.is_some() {
            return acc;
        }

        if let Some(def) = &pin.default_value {
            if !def.is_empty() {
                return Some(parse_signal_numeric(def));
            }
        }
        None
    }
}

pub fn parse_signal_numeric(raw: &str) -> f64 {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return 0.0;
    }
    if trimmed.eq_ignore_ascii_case("on") || trimmed.eq_ignore_ascii_case("true") {
        return 1.0;
    }
    if trimmed.eq_ignore_ascii_case("off") || trimmed.eq_ignore_ascii_case("false") {
        return 0.0;
    }
    if let Some(inner) = trimmed
        .strip_prefix("WW(")
        .and_then(|s| s.strip_suffix("%)"))
    {
        return inner.trim().parse::<f64>().unwrap_or(0.0);
    }
    if let Some( slash_parts ) = trimmed.contains('/').then(|| trimmed.split('/')) {
        let any_nonzero = slash_parts
            .filter_map(|p| p.trim().parse::<f64>().ok())
            .any(|v| v > 0.0);
        return if any_nonzero { 1.0 } else { 0.0 };
    }
    let clean: String = trimmed
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    clean.parse::<f64>().unwrap_or(0.0)
}

fn format_compact_f64(val: f64) -> String {
    if (val - val.round()).abs() < 1e-6 {
        format!("{}", val.round() as i64)
    } else {
        format!("{val:.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factory::{new_page, new_page_block, new_reference_block, new_tree_node};
    use crate::model::WireLink;

    #[test]
    fn test_offline_simulator_evaluates_multi_hop_logic_and_cross_page_memory() {
        let xml = std::fs::read_to_string("../../Klimczaki.Loxone")
            .expect("Klimczaki.Loxone fixture should exist");
        let mut project = crate::parser::parse_loxone_xml(&xml).expect("parse should succeed");

        let mut page1 = new_page("Sim Page 1");
        let mut page2 = new_page("Sim Page 2");

        let in_node_1 = new_tree_node("VirtualIn", "Button A");
        let in_node_2 = new_tree_node("VirtualIn", "Button B");
        let merker_node = new_tree_node("Merker", "Shared Alarm Flag");

        project.periphery_tree.push(in_node_1.clone());
        project.periphery_tree.push(in_node_2.clone());
        project.system_tree.push(merker_node.clone());

        let ref_in_a = new_reference_block("InputRef", &in_node_1, 64.0, 64.0);
        let ref_in_b = new_reference_block("InputRef", &in_node_2, 64.0, 192.0);
        let mut and_block = new_page_block("And", "Both Pressed", 320.0, 128.0);
        let mut mem_writer = new_reference_block("Memory", &merker_node, 640.0, 128.0);

        let src_pin_a = ref_in_a.pins[0].uuid.clone();
        let src_pin_b = ref_in_b.pins[0].uuid.clone();

        let and_in_pin = and_block
            .pins
            .iter_mut()
            .find(|p| p.key == "I")
            .unwrap();
        and_in_pin.incoming_links.push(WireLink {
            source_pin_uuid: src_pin_a,
            flags: None,
        });
        and_in_pin.incoming_links.push(WireLink {
            source_pin_uuid: src_pin_b,
            flags: None,
        });

        let and_q_uuid = and_block
            .pins
            .iter()
            .find(|p| p.key == "Q")
            .unwrap()
            .uuid
            .clone();
        mem_writer.pins[0].incoming_links.push(WireLink {
            source_pin_uuid: and_q_uuid,
            flags: None,
        });

        let and_uuid = and_block.uuid.clone();
        let ref_a_uuid = ref_in_a.uuid.clone();
        let ref_b_uuid = ref_in_b.uuid.clone();

        page1.blocks.push(ref_in_a);
        page1.blocks.push(ref_in_b);
        page1.blocks.push(and_block);
        page1.blocks.push(mem_writer);

        // Page 2 reads the Merker via a Memory reference block
        let mem_reader = new_reference_block("Memory", &merker_node, 64.0, 64.0);
        let mem_reader_uuid = mem_reader.uuid.clone();
        page2.blocks.push(mem_reader);

        project.pages.push(page1);
        project.pages.push(page2);
        project.xref = crate::xref::XrefIndex::build(&project);

        let mut sim = OfflineSimulator::new();
        sim.start(&project);

        // Initially both inputs are 0 -> And Q is 0
        let page1_ref = project.pages.iter().find(|p| p.title == "Sim Page 1").unwrap();
        let page2_ref = project.pages.iter().find(|p| p.title == "Sim Page 2").unwrap();
        let and_blk = page1_ref.blocks.iter().find(|b| b.uuid == and_uuid).unwrap();
        let q_pin = and_blk.pins.iter().find(|p| p.key == "Q").unwrap();
        assert!(!sim.live_store.resolve_pin_signal(and_blk, q_pin, page1_ref, &project).is_active);

        // Turn on Input A only -> And Q is still 0
        sim.apply_command(&project, &ref_a_uuid, None, "1");
        assert!(!sim.live_store.resolve_pin_signal(and_blk, q_pin, page1_ref, &project).is_active);

        // Turn on Input B -> And Q becomes 1 and propagates to Page 2's Memory reader!
        sim.apply_command(&project, &ref_b_uuid, None, "1");
        assert!(sim.live_store.resolve_pin_signal(and_blk, q_pin, page1_ref, &project).is_active);

        let reader_blk = page2_ref
            .blocks
            .iter()
            .find(|b| b.uuid == mem_reader_uuid)
            .unwrap();
        let reader_sig = sim
            .live_store
            .resolve_ref_bar_signal(reader_blk, page2_ref, &project);
        assert!(reader_sig.is_active);

        // Reset restores all forced inputs and outputs back to 0
        sim.reset(&project);
        assert!(!sim.live_store.resolve_pin_signal(and_blk, q_pin, page1_ref, &project).is_active);
    }

    #[test]
    fn test_offline_simulator_step_tick_and_central_light_propagation() {
        let xml = std::fs::read_to_string("../../Klimczaki.Loxone")
            .expect("Klimczaki.Loxone fixture should exist");
        let mut project = crate::parser::parse_loxone_xml(&xml).expect("parse should succeed");

        let mut page = new_page("Tick & Central Page");
        let impulse = new_page_block("IntervalImpulse", "Blink", 64.0, 64.0);
        let impulse_uuid = impulse.uuid.clone();

        let light1 = new_page_block("LightController2", "Salon Light", 320.0, 64.0);
        let light1_uuid = light1.uuid.clone();

        let mut central = new_page_block("CentralLight", "All Lights", 64.0, 256.0);
        central
            .raw_attributes
            .push(("rec".to_string(), light1_uuid.clone()));
        let central_uuid = central.uuid.clone();

        page.blocks.push(impulse);
        page.blocks.push(light1);
        page.blocks.push(central);
        project.pages.push(page);
        project.xref = crate::xref::XrefIndex::build(&project);

        let mut sim = OfflineSimulator::new();
        sim.start(&project);

        let page_ref = project
            .pages
            .iter()
            .find(|p| p.title == "Tick & Central Page")
            .unwrap();
        let imp_blk = page_ref.blocks.iter().find(|b| b.uuid == impulse_uuid).unwrap();
        let imp_q = imp_blk.pins.iter().find(|p| p.key == "Q").unwrap();

        assert!(!sim.live_store.resolve_pin_signal(imp_blk, imp_q, page_ref, &project).is_active);

        sim.step_tick(&project);
        assert_eq!(sim.tick_count, 1);
        assert!(sim.live_store.resolve_pin_signal(imp_blk, imp_q, page_ref, &project).is_active);

        sim.step_tick(&project);
        assert_eq!(sim.tick_count, 2);
        assert!(!sim.live_store.resolve_pin_signal(imp_blk, imp_q, page_ref, &project).is_active);

        // Trigger CentralLight -> propagates to member LightController2
        sim.apply_command(&project, &central_uuid, None, "1");
        let light_blk = page_ref.blocks.iter().find(|b| b.uuid == light1_uuid).unwrap();
        let light_out = light_blk
            .pins
            .iter()
            .find(|p| p.key == "OutputAPI")
            .unwrap();
        assert!(sim.live_store.resolve_pin_signal(light_blk, light_out, page_ref, &project).is_active);
    }
}
